use std::{ffi::CString, os::raw::c_char, path::PathBuf};

use xkbcommon::xkb;

// Use the local `slang-rs` bindings via the `shader-slang` crate (renamed here
// to `slang` for readability).
extern crate shader_slang as slang;

fn main() {
    system_deps::Config::new().probe().unwrap();

    // `shader-slang-sys` requires one of:
    // - SLANG_DIR (preferred): points at an extracted Slang SDK (must contain `include/` and `lib/`)
    // - SLANG_INCLUDE_DIR + SLANG_LIB_DIR
    // - VULKAN_SDK (if it contains Slang under include/slang)
    //
    // We intentionally don't set these here; the build environment should.

    let global = slang::GlobalSession::new().expect("Failed to create Slang GlobalSession");

    let out_dir = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .expect("OUT_DIR not set");

    // Make sure shader imports like `import color;` can resolve `veil/color.slang`.
    // Slang resolves `import foo;` by searching for `foo.slang` on its search path.
    // `color.slang` currently lives at the project root: `veil/color.slang`.
    let project_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .expect("CARGO_MANIFEST_DIR not set");

    compile_slang_to_spirv(
        &global,
        "src/session/video/composite.slang",
        out_dir.join("shaders/composite_vert.spv"),
        "vert",
        slang::Stage::Vertex,
        &[],
        &[
            "../shader-common",
            project_root
                .to_str()
                .expect("project root contains invalid UTF-8"),
        ],
        "glsl_460",
    );

    compile_slang_to_spirv(
        &global,
        "src/session/video/composite.slang",
        out_dir.join("shaders/composite_frag.spv"),
        "frag",
        slang::Stage::Fragment,
        &[],
        &[
            "../shader-common",
            project_root
                .to_str()
                .expect("project root contains invalid UTF-8"),
        ],
        "glsl_460",
    );

    compile_slang_to_spirv(
        &global,
        "src/session/video/convert.slang",
        out_dir.join("shaders/convert_multiplanar.spv"),
        "main",
        slang::Stage::Compute,
        &[],
        &[
            "../shader-common",
            project_root
                .to_str()
                .expect("project root contains invalid UTF-8"),
        ],
        "glsl_460",
    );

    compile_slang_to_spirv(
        &global,
        "src/session/video/convert.slang",
        out_dir.join("shaders/convert_semiplanar.spv"),
        "main",
        slang::Stage::Compute,
        &[("SEMIPLANAR", "1")],
        &[
            "../shader-common",
            project_root
                .to_str()
                .expect("project root contains invalid UTF-8"),
        ],
        "glsl_460",
    );

    // We need a keymap for the compositor, but it shouldn't affect much, since we
    // operate generally with physical keycodes and so do games. If this proves
    // limiting, we could allow the configuration of other virtual keyboards.
    let xkb_ctx = xkb::Context::new(0);
    save_keymap(
        &xkb_ctx,
        out_dir.join("keymaps/iso_us.txt").to_str().unwrap(),
        "",
        "pc105",
        "us",
        "",
        None,
    );
}

/// Compile a Slang source file to a SPIR-V blob for a single entry point.
///
/// This uses the public `shader-slang` API:
/// - `GlobalSession` + `SessionDesc`/`TargetDesc`
/// - `Session::load_module_from_source_string`
/// - `Module::find_entry_point_by_name`
/// - `Session::create_composite_component_type` + `ComponentType::link`
/// - `ComponentType::entry_point_code`
///
/// Notes:
/// - `search_paths` are passed via `SessionDesc::search_paths`, which expects `&[*const c_char]`.
/// - Macro defines are passed via `CompilerOptions::macro_define`.
fn compile_slang_to_spirv(
    global: &slang::GlobalSession,
    in_path: &str,
    out_path: PathBuf,
    entry_point: &str,
    stage: slang::Stage,
    defines: &[(&str, &str)],
    search_paths: &[&str],
    profile_name: &str,
) {
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create shader output directory");
    }

    println!("cargo::rerun-if-changed={}", in_path);

    // Build compiler options. These are applied both at the target and session levels
    // (session uses them for module parsing; target uses them for codegen).
    let profile = global.find_profile(profile_name);

    let mut opts = slang::CompilerOptions::default()
        .language(slang::SourceLanguage::Slang)
        .stage(stage)
        .target(slang::CompileTarget::Spirv)
        .profile(profile)
        .optimization(slang::OptimizationLevel::Maximal)
        // This is commonly useful when piping SPIR-V into Vulkan tooling; keep it enabled.
        .emit_spirv_directly(true);

    for (k, v) in defines {
        opts = opts.macro_define(k, v);
    }

    // Target descriptor (SPIR-V + profile + options)
    let target = slang::TargetDesc::default()
        .format(slang::CompileTarget::Spirv)
        .profile(profile)
        .options(&opts);

    let targets = [target];

    // Slang wants search paths as C string pointers. Keep the CString storage alive
    // until after session creation and module loading.
    let mut sp_cstrings: Vec<CString> = Vec::with_capacity(search_paths.len());
    for p in search_paths {
        sp_cstrings.push(CString::new(*p).expect("search path contained NUL"));
    }
    let sp_ptrs: Vec<*const c_char> = sp_cstrings.iter().map(|s| s.as_ptr()).collect();

    // Session descriptor (targets + search paths + options)
    let mut desc = slang::SessionDesc::default()
        .targets(&targets)
        .options(&opts);
    if !sp_ptrs.is_empty() {
        desc = desc.search_paths(&sp_ptrs);
    }

    let session = global
        .create_session(&desc)
        .expect("Failed to create Slang Session");

    // Load the module from source string, but provide the original path so Slang can
    // produce correct diagnostics and resolve relative includes against it.
    let source = std::fs::read_to_string(in_path).expect("failed to read .slang source");

    let in_path_buf = PathBuf::from(in_path);
    let module_name = in_path_buf
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shader");

    let module = session
        .load_module_from_source_string(module_name, in_path, &source)
        .unwrap_or_else(|e| {
            panic!("Slang failed to load module `{module_name}` from {in_path}: {e}")
        });

    let ep = module
        .find_entry_point_by_name(entry_point)
        .unwrap_or_else(|| panic!("Slang failed to find entry point `{entry_point}` in {in_path}"));

    // Link a program consisting of just the requested entry point.
    let component: slang::ComponentType = ep.into();

    let program = session
        .create_composite_component_type(&[component])
        .unwrap_or_else(|e| {
            panic!("Slang failed to create composite program for {in_path}:{entry_point}: {e}")
        });

    let linked = program.link().unwrap_or_else(|e| {
        panic!("Slang failed to link program for {in_path}:{entry_point}: {e}")
    });

    // We have a single target and a single entry point in this composite.
    let blob = linked
        .entry_point_code(0, 0)
        .unwrap_or_else(|e| panic!("Slang failed to emit SPIR-V for {in_path}:{entry_point}: {e}"));

    std::fs::write(&out_path, blob.as_slice()).expect("failed to write SPIR-V to file");
}

fn save_keymap(
    ctx: &xkb::Context,
    out_path: &str,
    rules: &str,
    model: &str,
    layout: &str,
    variant: &str,
    options: Option<&str>,
) {
    std::fs::create_dir_all(PathBuf::from(out_path).parent().unwrap())
        .expect("failed to create output directory");

    let keymap = xkb::Keymap::new_from_names(
        ctx,
        rules,
        model,
        layout,
        variant,
        options.map(|s| s.to_string()),
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("failed to create keymap");

    let s = keymap.get_as_string(xkb::FORMAT_TEXT_V1);

    std::fs::write(out_path, CString::new(s).unwrap().to_bytes_with_nul())
        .expect("failed to write keymap bytes to file");
}
