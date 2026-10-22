#![allow(clippy::too_many_arguments)]

use std::ffi::{CStr, CString, c_void};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd, IntoRawFd as _, OwnedFd};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use ash::vk;
use ash::{ext, khr};
use libc::dev_t;
use tracing::{debug, error, info, instrument, warn};

// ============================================================================
// vk_chain! macro for building pinned Vulkan struct chains
// ============================================================================

/// Used to construct a pinned chain of vulkan structures.
///
/// Ash provides the builder pattern for generating temporary input structs on
/// the stack, but it doesn't work well with structs stored on the heap for
/// re-use. Part of the reason for that is that the `p_next` pointer mechanism
/// is out of scope of the borrow checker.
///
/// If we want to store a chain of structs in a `Box`, we should also `Pin` it,
/// since the holding struct is effectively self-referential. This macro handles
/// the boilerplate for that, by:
///
///  - Generating a constructor for the struct that returns `Pin<Box<Self>>`
///  - Generating `AsRef<T>` and `AsMut<T>` for the first struct in the chain
///  - Generating `with_foo` methods that allow you to replace one struct in the
///    chain (presumably using the builder pattern).
///
/// Besides letting us reuse allocations for heavy structs, this also achieves a
/// level of polymorphism, since calling code can take an `impl AsRef<T>` where
/// `T` is the first struct, and generalize over the remaining chain.
///
/// Note that the chain is always created and maintained in declaration order,
/// with the first field being the "head" and the head's `p_next` pointer
/// pointing to the second field, and so on.
macro_rules! vk_chain {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Chain:ident <$lifetime:lifetime> {
            $(#[$head_meta:meta])*
            pub $HeadName:ident: $HeadStruct:ty,
            $(
                $(#[$field_meta:meta])*
                pub $Name:ident: $Struct:ty,
            )+
        }
    ) => {
        paste::paste! {
            $(#[$meta])*
            $vis struct [<$Chain Inner>] <$lifetime> {
                $(#[$head_meta])*
                pub $HeadName: $HeadStruct,
                $(
                    $(#[$field_meta])*
                    pub $Name: $Struct,
                )*
            }

            $vis struct $Chain(std::pin::Pin<Box<[<$Chain Inner>] <'static> >>);

            unsafe impl Send for $Chain {}

            #[allow(dead_code)]
            impl $Chain {
                pub fn new<$lifetime: 'static>($HeadName: $HeadStruct, $($Name: $Struct,)*) -> Self {
                    let mut ch = Box::pin([<$Chain Inner>] {
                        $HeadName,
                        $($Name,)*
                    });

                    __set_p_next!(ch, $HeadName, $($Name),*);
                    Self(ch)
                }

                $(
                    #[doc = "Replaces the `" $Name "` field with the new (or modified) struct returned by `f`. Maintains the `p_next` chain."]
                    pub fn [<with_ $Name>]<$lifetime: 'static, F>(&mut self, f: F)
                    where
                        F: FnOnce($Struct) -> $Struct,
                    {
                        let p_next = self.0.$Name.p_next;
                        self.0.$Name = f(self.$Name);
                        self.0.$Name.p_next = p_next;
                    }
                )*
            }

            impl Default for $Chain {
                fn default() -> Self {
                    Self::new(__replace_expr!(($HeadStruct) Default::default()), $(__replace_expr!(($Struct) Default::default()),)*)
                }
            }

            impl std::ops::Deref for $Chain {
                type Target = [<$Chain Inner>]<'static>;

                fn deref(&self) -> &Self::Target {
                    std::pin::Pin::deref(&self.0)
                }
            }
        }

        impl<$lifetime: 'static> AsRef<$HeadStruct> for $Chain {
            fn as_ref(&self) -> &$HeadStruct {
                &self.0.as_ref().get_ref().$HeadName
            }
        }

        impl<$lifetime: 'static> AsMut<$HeadStruct> for $Chain {
            fn as_mut(&mut self) -> &mut $HeadStruct {
                &mut self.0.as_mut().get_mut().$HeadName
            }
        }
    };
}

macro_rules! __set_p_next(
    ($target:ident, $head:ident, $next:ident) => {
        $target.$head.p_next = <*mut _>::cast(&mut $target.$next);
    };
    ($target:ident, $head:ident, $next:ident, $($tail:ident),+) => {
        $target.$head.p_next = <*mut _>::cast(&mut $target.$next);
        __set_p_next!($target, $next, $($tail),+);
    };
);

macro_rules! __replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

pub(crate) use __replace_expr;
pub(crate) use __set_p_next;
pub(crate) use vk_chain;

// ============================================================================
// DRM device wrapper
// ============================================================================

pub struct DrmDevice(File);

impl AsFd for DrmDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for DrmDevice {}
impl drm::control::Device for DrmDevice {}

impl DrmDevice {
    pub fn new(dev: dev_t) -> anyhow::Result<Self> {
        let path = drm::node::DrmNode::from_dev_id(dev)?
            .dev_path()
            .ok_or(anyhow!("no device file found"))?;

        let mut options = OpenOptions::new();
        options.read(true);
        options.write(true);

        Ok(Self(options.open(path)?))
    }
}

// ============================================================================
// Timeline semaphore wrapper
// ============================================================================

#[derive(Clone)]
pub struct VkTimelineSemaphore(Arc<TimelineInner>);

struct TimelineInner {
    vk: Arc<VkContext>,
    sema: vk::Semaphore,
}

#[derive(Clone)]
pub struct VkTimelinePoint(Arc<TimelineInner>, u64);

impl From<VkTimelinePoint> for u64 {
    fn from(value: VkTimelinePoint) -> Self {
        value.1
    }
}

impl std::ops::Add<u64> for VkTimelinePoint {
    type Output = Self;

    fn add(self, rhs: u64) -> Self {
        Self(self.0.clone(), self.1 + rhs)
    }
}

impl std::ops::Add<u64> for &VkTimelinePoint {
    type Output = VkTimelinePoint;

    fn add(self, rhs: u64) -> Self::Output {
        VkTimelinePoint(self.0.clone(), self.1 + rhs)
    }
}

impl std::ops::AddAssign<u64> for VkTimelinePoint {
    fn add_assign(&mut self, rhs: u64) {
        self.1 += rhs
    }
}

impl VkTimelineSemaphore {
    pub fn new(vk: Arc<VkContext>, initial_value: u64) -> anyhow::Result<Self> {
        let sema = unsafe {
            let mut type_info = vk::SemaphoreTypeCreateInfo::default()
                .semaphore_type(vk::SemaphoreType::TIMELINE)
                .initial_value(initial_value);

            let mut create_info = vk::SemaphoreCreateInfo::default();
            create_info.p_next = &mut type_info as *mut _ as *mut std::ffi::c_void;

            vk.device
                .create_semaphore(&create_info, None)
                .context("VkCreateSemaphore")?
        };

        Ok(Self(Arc::new(TimelineInner { vk, sema })))
    }

    pub fn from_syncobj_fd(vk: Arc<VkContext>, fd: OwnedFd) -> anyhow::Result<Self> {
        let sema = Self::new(vk.clone(), 0)?;

        let import_info = vk::ImportSemaphoreFdInfoKHR::default()
            .semaphore(sema.as_semaphore())
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD)
            .fd(fd.into_raw_fd()); // Vulkan owns the fd now.

        unsafe {
            vk.external_semaphore_api
                .import_semaphore_fd(&import_info)
                .context("VkImportSemaphoreFdKHR")?;
        }

        Ok(sema)
    }

    pub fn new_point(&self, value: u64) -> VkTimelinePoint {
        VkTimelinePoint(self.0.clone(), value)
    }

    pub fn as_semaphore(&self) -> vk::Semaphore {
        self.0.sema
    }
}

impl VkTimelinePoint {
    pub fn value(&self) -> u64 {
        self.1
    }

    pub fn timeline(&self) -> VkTimelineSemaphore {
        VkTimelineSemaphore(self.0.clone())
    }

    #[instrument(skip_all)]
    pub unsafe fn wait(&self) -> anyhow::Result<()> {
        let device = &self.0.vk.device;
        unsafe {
            device.wait_semaphores(
                &vk::SemaphoreWaitInfo::default()
                    .semaphores(&[self.0.sema])
                    .values(&[self.1]),
                1_000_000_000, // 1 second
            )?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub unsafe fn signal(&self) -> anyhow::Result<()> {
        let device = &self.0.vk.device;
        unsafe {
            device.signal_semaphore(
                &vk::SemaphoreSignalInfo::default()
                    .semaphore(self.0.sema)
                    .value(self.1),
            )?;
        }

        Ok(())
    }

    pub unsafe fn poll(&self) -> anyhow::Result<bool> {
        let device = &self.0.vk.device;
        let value = unsafe { device.get_semaphore_counter_value(self.0.sema)? };
        Ok(value >= self.1)
    }
}

impl Drop for TimelineInner {
    fn drop(&mut self) {
        unsafe {
            self.vk.device.device_wait_idle().unwrap();
            self.vk.device.destroy_semaphore(self.sema, None)
        };
    }
}

// ============================================================================
// Vulkan context and device management
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Amd,
    Nvidia,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverVersion {
    MesaRadv { major: u32, minor: u32, patch: u32 },
    NvidiaProprietary { major: u32, minor: u32 },
    Other(String),
}

impl std::fmt::Display for DriverVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MesaRadv { major, minor, patch } => write!(f, "Mesa RADV {major}.{minor}.{patch}"),
            Self::NvidiaProprietary { major, minor } => write!(f, "NVIDIA {major}.{minor:02}"),
            Self::Other(info) => f.write_str(info),
        }
    }
}

pub struct VkContext {
    pub entry: ash::Entry,

    // Prefer the ash `Device` wrappers for extension commands, so we keep any additional
    // method shims provided by ash and avoid calling into raw function tables that
    // may not implement all helpers (notably for Vulkan Video).
    pub push_ds_api: khr::push_descriptor::Device,
    pub external_memory_api: khr::external_memory_fd::Device,
    pub external_semaphore_api: khr::external_semaphore_fd::Device,

    // Vulkan Video commands are split across instance-level and device-level wrappers.
    // Store them separately so call sites can use the correct loader for each command.
    pub video_apis: Option<VideoApis>,

    pub instance: ash::Instance,
    pub debug: Option<VkDebugContext>,
    pub device: ash::Device,
    pub device_info: VkDeviceInfo,
    /// Names of every GPU that was eligible for encoding, including this one.
    pub eligible_gpus: Vec<String>,
    pub drm_device: DrmDevice,
    pub graphics_queue: VkQueue,
    pub encode_queue: Option<VkQueue>,
    pub descriptor_pool: vk::DescriptorPool,
}

pub struct VideoApis {
    pub video_instance: khr::video_queue::Instance,
    pub video_device: khr::video_queue::Device,
    pub encode_device: khr::video_encode_queue::Device,
}

pub struct VkDebugContext {
    debug: ext::debug_utils::Instance,
    messenger: vk::DebugUtilsMessengerEXT,
}

#[derive(Clone)]
pub struct VkQueue {
    pub family: u32,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool, // TODO: synchronize access
    /// For GPU spans in the Tracy profiler, if the queue supports timestamps.
    #[cfg(feature = "tracy")]
    pub tracy_context: Option<tracy_client::GpuContext>,
}

impl VkQueue {
    #[cfg_attr(not(feature = "tracy"), allow(unused_variables))]
    pub fn new(
        device: &ash::Device,
        pdevice: &VkDeviceInfo,
        props: vk::QueueFamilyProperties,
        family: u32,
        name: &str,
    ) -> Result<Self> {
        let queue = unsafe { device.get_device_queue(family, 0) };

        let command_pool = unsafe {
            let create_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(family)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            device.create_command_pool(&create_info, None)?
        };

        #[cfg(feature = "tracy")]
        let tracy_context = Self::tracy_context(device, pdevice, props, queue, command_pool, name)
            .unwrap_or_else(|e| {
                warn!(name, "no tracy gpu context: {e:#}");
                None
            });

        Ok(Self {
            family,
            queue,
            command_pool,
            #[cfg(feature = "tracy")]
            tracy_context,
        })
    }

    /// Create a Tracy GPU context, calibrated from a timestamp written on this
    /// queue.
    #[cfg(feature = "tracy")]
    fn tracy_context(
        device: &ash::Device,
        pdevice: &VkDeviceInfo,
        props: vk::QueueFamilyProperties,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        name: &str,
    ) -> Result<Option<tracy_client::GpuContext>> {
        // RADV asserts when timestamps are written in encode command buffers.
        if props.timestamp_valid_bits == 0
            || matches!(pdevice.driver_version, DriverVersion::MesaRadv { .. })
        {
            return Ok(None);
        }
        let Some(client) = tracy_client::Client::running() else {
            return Ok(None);
        };

        let pool = create_timestamp_query_pool(device, 1)?;
        let timestamp = unsafe {
            let cb = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )?[0];
            device.begin_command_buffer(
                cb,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            pool.cmd_reset(device, cb);
            device.cmd_write_timestamp(cb, vk::PipelineStageFlags::BOTTOM_OF_PIPE, pool.pool, 0);
            device.end_command_buffer(cb)?;

            let fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            let cbs = [cb];
            let result = device
                .queue_submit(queue, &[vk::SubmitInfo::default().command_buffers(&cbs)], fence)
                .and_then(|()| device.wait_for_fences(&[fence], true, u64::MAX))
                .map_err(anyhow::Error::from)
                .and_then(|()| pool.fetch_results(device));

            device.destroy_fence(fence, None);
            device.free_command_buffers(command_pool, &cbs);
            device.destroy_query_pool(pool.pool, None);
            result?[0]
        };

        Ok(client
            .new_gpu_context(
                Some(name),
                tracy_client::GpuContextType::Vulkan,
                timestamp,
                pdevice.limits.timestamp_period,
            )
            .ok())
    }
}

pub struct VkDeviceInfo {
    pub pdevice: vk::PhysicalDevice,
    pub device_name: CString,
    pub device_type: vk::PhysicalDeviceType,
    pub device_vendor: Vendor,
    /// PCI vendor and device IDs.
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_version: DriverVersion,
    pub limits: vk::PhysicalDeviceLimits,
    pub drm_node: libc::dev_t,
    pub graphics_family: u32,
    pub encode_family: Option<u32>,
    pub supports_h264: bool,
    pub supports_h265: bool,
    pub supports_av1: bool,
    pub memory_props: vk::PhysicalDeviceMemoryProperties,
    pub host_visible_mem_type_index: u32,
    pub host_mem_is_cached: bool,
    pub selected_extensions: Vec<CString>,
}

impl VkDeviceInfo {
    fn query(instance: &ash::Instance, device: vk::PhysicalDevice) -> Result<Self> {
        let mut drm_props = vk::PhysicalDeviceDrmPropertiesEXT::default();
        let mut _host_mem_props = vk::PhysicalDeviceExternalMemoryHostPropertiesEXT::default();
        let mut driver_props = vk::PhysicalDeviceDriverPropertiesKHR::default();

        // ash (git) removed `.push_next(...)`; wire `p_next` manually.
        // Chain: PhysicalDeviceProperties2 -> DrmPropertiesEXT -> ExternalMemoryHostPropertiesEXT -> DriverPropertiesKHR
        drm_props.p_next = &mut _host_mem_props as *mut _ as *mut std::ffi::c_void;
        _host_mem_props.p_next = &mut driver_props as *mut _ as *mut std::ffi::c_void;

        let mut props = vk::PhysicalDeviceProperties2::default();
        props.p_next = &mut drm_props as *mut _ as *mut std::ffi::c_void;

        unsafe { instance.get_physical_device_properties2(device, &mut props) };

        let limits = props.properties.limits;
        let device_type = props.properties.device_type;
        let device_name =
            unsafe { CStr::from_ptr(props.properties.device_name.as_ptr()).to_owned() };
        let device_vendor = match props.properties.vendor_id {
            0x1002 => Vendor::Amd,
            0x10de => Vendor::Nvidia,
            _ => Vendor::Other,
        };

        let version = props.properties.driver_version;
        let driver_version = match driver_props.driver_id {
            vk::DriverId::MESA_RADV => DriverVersion::MesaRadv {
                major: vk::api_version_major(version),
                minor: vk::api_version_minor(version),
                patch: vk::api_version_patch(version),
            },
            vk::DriverId::NVIDIA_PROPRIETARY => DriverVersion::NvidiaProprietary {
                major: (version >> 22) & 0x3ff,
                minor: (version >> 14) & 0x0ff,
            },
            _ => DriverVersion::Other(
                CStr::from_bytes_with_nul(&driver_props.driver_info.map(|x| x as u8)[..])
                    .unwrap_or(c"unknown")
                    .to_str()
                    .unwrap_or("unknown")
                    .to_owned(),
            ),
        };

        if drm_props.render_major != 226 || drm_props.render_minor < 128 {
            bail!("device {:?} is not a render node", device_name);
        }

        let drm_node = libc::makedev(drm_props.render_major as u32, drm_props.render_minor as u32);

        let queue_families = unsafe {
            instance
                .get_physical_device_queue_family_properties(device)
                .into_iter()
                .collect::<Vec<_>>()
        };

        let graphics_family = queue_families
            .iter()
            .enumerate()
            .find(|(_, properties)| {
                properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    && properties.queue_flags.contains(vk::QueueFlags::COMPUTE)
            })
            .map(|(index, _)| index as u32)
            .to_owned()
            .ok_or_else(|| anyhow::anyhow!("no graphics queue found"))?;

        let encode_family = queue_families
            .iter()
            .enumerate()
            .find(|(_, properties)| {
                properties
                    .queue_flags
                    .contains(vk::QueueFlags::VIDEO_ENCODE_KHR)
            })
            .map(|(index, _)| index as u32);

        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
                .into_iter()
                .map(|properties| CStr::from_ptr(&properties.extension_name as *const _).to_owned())
                .collect::<Vec<_>>()
        };

        let mut selected_extensions = vec![
            // Push descriptors for compositing.
            khr::push_descriptor::NAME.to_owned(),
            // All required for dma-buf import.
            khr::external_memory_fd::NAME.to_owned(),
            khr::external_semaphore_fd::NAME.to_owned(),
            ext::external_memory_dma_buf::NAME.to_owned(),
            ext::image_drm_format_modifier::NAME.to_owned(),
            ext::physical_device_drm::NAME.to_owned(),
            ext::queue_family_foreign::NAME.to_owned(),
        ];

        for ext in selected_extensions.iter() {
            if !contains_extension(&available_extensions, ext.as_c_str()) {
                bail!("extension {:?} not available", ext);
            }
        }

        let ext_video_queue = khr::video_queue::NAME;
        let ext_video_encode_queue = khr::video_encode_queue::NAME;
        let ext_h264 = khr::video_encode_h264::NAME;
        let ext_h265 = khr::video_encode_h265::NAME;
        let ext_av1 = khr::video_encode_av1::NAME;

        let mut supports_h264 = false;
        let mut supports_h265 = false;
        let mut supports_av1 = false;
        if encode_family.is_some()
            && contains_extension(&available_extensions, ext_video_queue)
            && contains_extension(&available_extensions, ext_video_encode_queue)
        {
            selected_extensions.push(ext_video_encode_queue.to_owned());
            selected_extensions.push(ext_video_queue.to_owned());

            if contains_extension(&available_extensions, ext_h264) {
                supports_h264 = true;
                selected_extensions.push(ext_h264.to_owned());
            }

            if contains_extension(&available_extensions, ext_h265) {
                supports_h265 = true;
                selected_extensions.push(ext_h265.to_owned());
            }
            // The extension alone isn't enough; the feature has to be there
            // too (and is enabled at device creation).
            if contains_extension(&available_extensions, ext_av1) {
                let mut av1_features = vk::PhysicalDeviceVideoEncodeAV1FeaturesKHR::default();
                let mut features = vk::PhysicalDeviceFeatures2::default();
                features.p_next = &mut av1_features as *mut _ as *mut std::ffi::c_void;
                unsafe { instance.get_physical_device_features2(device, &mut features) };

                if av1_features.video_encode_av1 == vk::TRUE {
                    supports_av1 = true;
                    selected_extensions.push(ext_av1.to_owned());
                }
            }
        }

        if !supports_av1 && !supports_h265 && !supports_h264 {
            bail!("hardware encode extensions not available");
        }

        // We want HOST_CACHED | HOST_COHERENT, but we can make do with just
        // HOST_VISIBLE.
        let memory_props = unsafe { instance.get_physical_device_memory_properties(device) };

        let (host_visible_mem_type_index, host_mem_is_cached) = {
            let mut cached = true;
            let mut idx = select_memory_type(
                &memory_props,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_CACHED
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                None,
            );

            if idx.is_none() {
                idx = select_memory_type(
                    &memory_props,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    None,
                );

                if idx.is_none() {
                    bail!("no host visible memory type found");
                }

                cached = false;
            }

            (idx.unwrap(), cached)
        };

        // Make sure we have the features needed for dmabuf import.
        let mut semaphore_props = vk::ExternalSemaphoreProperties::default();
        unsafe {
            let info = vk::PhysicalDeviceExternalSemaphoreInfo::default()
                .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);

            instance.get_physical_device_external_semaphore_properties(
                device,
                &info,
                &mut semaphore_props,
            );
        }

        if !semaphore_props
            .external_semaphore_features
            .contains(vk::ExternalSemaphoreFeatureFlags::IMPORTABLE)
        {
            bail!("no support found for importable semaphores");
        }

        Ok(Self {
            pdevice: device,
            device_name,
            device_type,
            device_vendor,
            vendor_id: props.properties.vendor_id,
            device_id: props.properties.device_id,
            driver_version,
            limits,
            drm_node,
            graphics_family,
            encode_family,
            supports_h264,
            supports_h265,
            supports_av1,
            memory_props,
            host_visible_mem_type_index,
            host_mem_is_cached,
            selected_extensions,
        })
    }
}

impl VkContext {
    pub fn new(enable_debug: bool) -> Result<Self> {
        // Try to enable RADV's video support.
        unsafe {
            std::env::set_var(
                "RADV_PERFTEST",
                std::env::var("RADV_PERFTEST").unwrap_or("video_encode,video_decode".to_string()),
            )
        };

        let entry = unsafe { ash::Entry::load().context("failed to load vulkan libraries!") }?;
        debug!("creating vulkan instance");

        let (major, minor) = match unsafe { entry.try_enumerate_instance_version()? } {
            // Vulkan 1.1+
            Some(version) => (
                vk::api_version_major(version),
                vk::api_version_minor(version),
            ),
            // Vulkan 1.0
            None => (1, 0),
        };

        if major < 1 || (major == 1 && minor < 3) {
            return Err(anyhow::anyhow!("vulkan 1.3 or higher is required"));
        }

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Veil")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"ash")
            .engine_version(vk::make_api_version(0, 0, 1, 0)) // TODO: grab from cargo.toml
            .api_version(vk::make_api_version(0, major, minor, 0));

        let available_extensions = unsafe {
            entry
                .enumerate_instance_extension_properties(None)?
                .into_iter()
                .map(|properties| CStr::from_ptr(&properties.extension_name as *const _).to_owned())
                .collect::<Vec<_>>()
        };

        let mut extensions = Vec::new();
        let mut layers = Vec::new();

        if enable_debug {
            if !available_extensions
                .iter()
                .any(|ext| ext.as_c_str() == ext::debug_utils::NAME)
            {
                return Err(anyhow::anyhow!(
                    "debug utils extension requested, but not available"
                ));
            }

            warn!("vulkan validation layers enabled!");
            extensions.push(ext::debug_utils::NAME.as_ptr());

            unsafe {
                let validation_layer = c"VK_LAYER_KHRONOS_validation";
                if entry
                    .enumerate_instance_layer_properties()?
                    .into_iter()
                    .map(|properties| CStr::from_ptr(&properties.layer_name as *const _))
                    .any(|layer| layer == validation_layer)
                {
                    layers.push(validation_layer.as_ptr());
                } else {
                    warn!("validation layers requested, but not available!")
                }
            }
        }

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        // Enable validation layers and a debugging callback, if requested.
        let debug_utils = if enable_debug {
            let debug_utils = ext::debug_utils::Instance::load(&entry, &instance);

            let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                )
                .pfn_user_callback(Some(vulkan_debug_utils_callback));

            let messenger =
                unsafe { debug_utils.create_debug_utils_messenger(&create_info, None) }?;

            Some(VkDebugContext {
                debug: debug_utils,
                messenger,
            })
        } else {
            None
        };

        // Select a device based on encoding support.
        let devices = unsafe { instance.enumerate_physical_devices()? };
        let mut devices = devices
            .into_iter()
            .enumerate()
            .flat_map(|(index, dev)| match VkDeviceInfo::query(&instance, dev) {
                Ok(device) => Some((index as u32, device)),
                Err(err) => {
                    let device_name = unsafe {
                        CStr::from_ptr(
                            instance
                                .get_physical_device_properties(dev)
                                .device_name
                                .as_ptr(),
                        )
                        .to_owned()
                    };

                    info!("gpu {device_name:?} ineligible: {err}");
                    None
                }
            })
            .collect::<Vec<_>>();

        if devices.is_empty() {
            return Err(anyhow::anyhow!("no suitable gpu found"));
        }

        let eligible_gpus = devices
            .iter()
            .map(|(_, dev)| dev.device_name.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        devices.sort_by_key(|(_, dev)| {
            let mut score = match dev.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 10,
                _ => 20,
            };

            score += dev.encode_family.is_none() as u32;
            score += !dev.supports_h264 as u32;
            score += !dev.supports_h265 as u32;
            // AV1 is experimental, so it doesn't affect which GPU is picked.
            score
        });

        // VEIL_GPU overrides the choice, by device index or a case-insensitive
        // substring of the device name (e.g. "nvidia", "radv").
        let pick = match std::env::var("VEIL_GPU") {
            Ok(wanted) => {
                let wanted_lower = wanted.to_lowercase();
                devices
                    .iter()
                    .position(|(index, dev)| {
                        wanted.parse::<u32>() == Ok(*index)
                            || dev
                                .device_name
                                .to_string_lossy()
                                .to_lowercase()
                                .contains(&wanted_lower)
                    })
                    .ok_or_else(|| anyhow::anyhow!("VEIL_GPU={wanted:?} matches no eligible gpu"))?
            }
            Err(_) => 0,
        };

        let (index, device_info) = devices.remove(pick);
        info!("selected gpu: {:?} ({index})", device_info.device_name);

        let drm_device = DrmDevice::new(device_info.drm_node)?;

        let device = {
            let queue_priorities = &[1.0];
            let mut queue_indices = Vec::new();
            queue_indices.push(device_info.graphics_family);
            if let Some(idx) = device_info.encode_family {
                queue_indices.push(idx);
            }

            queue_indices.dedup();
            let queue_create_infos = queue_indices
                .iter()
                .map(|&index| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(index)
                        .queue_priorities(queue_priorities)
                })
                .collect::<Vec<_>>();

            // shader_draw_parameters: Slang emits the DrawParameters capability
            // for the composite vertex shader.
            let mut enabled_1_1_features = vk::PhysicalDeviceVulkan11Features::default()
                .sampler_ycbcr_conversion(true)
                .shader_draw_parameters(true);

            let mut _enabled_1_2_features = vk::PhysicalDeviceVulkan12Features::default()
                .timeline_semaphore(true)
                .host_query_reset(true);

            let mut enabled_1_3_features = vk::PhysicalDeviceVulkan13Features::default()
                .dynamic_rendering(true)
                .synchronization2(true);

            let mut av1_encode_features =
                vk::PhysicalDeviceVideoEncodeAV1FeaturesKHR::default().video_encode_av1(true);

            // ash (git) removed `.push_next(...)`; wire `p_next` manually.
            // Chain: DeviceCreateInfo -> Vulkan11Features -> Vulkan12Features -> Vulkan13Features
            //   (-> VideoEncodeAV1Features, if supported)
            if device_info.supports_av1 {
                enabled_1_3_features.p_next =
                    &mut av1_encode_features as *mut _ as *mut std::ffi::c_void;
            }
            enabled_1_1_features.p_next =
                &mut _enabled_1_2_features as *mut _ as *mut std::ffi::c_void;
            _enabled_1_2_features.p_next =
                &mut enabled_1_3_features as *mut _ as *mut std::ffi::c_void;

            let extension_names = device_info
                .selected_extensions
                .iter()
                .map(|v| v.as_c_str().as_ptr())
                .collect::<Vec<_>>();

            let mut device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&extension_names);

            device_create_info.p_next =
                &mut enabled_1_1_features as *mut _ as *mut std::ffi::c_void;

            unsafe { instance.create_device(device_info.pdevice, &device_create_info, None)? }
        };

        let qf_props =
            unsafe { instance.get_physical_device_queue_family_properties(device_info.pdevice) };

        let graphics_queue = VkQueue::new(
            &device,
            &device_info,
            qf_props[device_info.graphics_family as usize],
            device_info.graphics_family,
            "graphics",
        )?;

        let encode_queue = if device_info.encode_family.is_some() {
            info!(
                "hardware encoding support: (h264: {}, h265: {}, av1: {})",
                device_info.supports_h264, device_info.supports_h265, device_info.supports_av1
            );

            Some(VkQueue::new(
                &device,
                &device_info,
                qf_props[device_info.encode_family.unwrap() as usize],
                device_info.encode_family.unwrap(),
                "encode",
            )?)
        } else {
            warn!("no hardware encoding support found!");
            None
        };

        if !device_info.host_mem_is_cached {
            warn!("no cache-coherent memory type found on device!");
        }

        let external_memory_api = khr::external_memory_fd::Device::load(&instance, &device);
        let external_semaphore_api = khr::external_semaphore_fd::Device::load(&instance, &device);

        let video_apis = if device_info.encode_family.is_some() {
            let video_instance = khr::video_queue::Instance::load(&entry, &instance);
            let video_device = khr::video_queue::Device::load(&instance, &device);
            let encode_device = khr::video_encode_queue::Device::load(&instance, &device);

            Some(VideoApis {
                video_instance,
                video_device,
                encode_device,
            })
        } else {
            None
        };

        let push_ds_api = khr::push_descriptor::Device::load(&instance, &device);

        let descriptor_pool = {
            let pool_sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1024),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1024),
            ];

            let create_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .max_sets(1024);

            unsafe { device.create_descriptor_pool(&create_info, None)? }
        };

        Ok(Self {
            entry,
            push_ds_api,
            external_memory_api,
            external_semaphore_api,
            video_apis,
            instance,
            device,
            device_info,
            eligible_gpus,
            drm_device,
            graphics_queue,
            encode_queue,
            descriptor_pool,
            debug: debug_utils,
        })
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _userdata: *mut c_void,
) -> vk::Bool32 {
    let _ = std::panic::catch_unwind(|| {
        let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) }.to_string_lossy();
        let ty = format!("{:?}", message_type).to_lowercase();

        // TODO: these should all be debug.
        match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
                tracing::trace!(ty, "{}", message)
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
                debug!(target: "veil::vulkan::driver", ty, "{}", message)
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                debug!(target: "veil::vulkan::driver", ty, "{}", message)
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                error!(target: "veil::vulkan::driver", ty, "{}", message)
            }
            _ => (),
        }
    });

    // Must always return false.
    vk::FALSE
}

impl Drop for VkContext {
    fn drop(&mut self) {
        debug!("destroying vulkan instance");

        unsafe {
            if let Some(debug) = self.debug.as_ref() {
                debug
                    .debug
                    .destroy_debug_utils_messenger(debug.messenger, None);
            }

            self.device
                .destroy_command_pool(self.graphics_queue.command_pool, None);

            if let Some(encode_queue) = self.encode_queue.as_ref() {
                self.device
                    .destroy_command_pool(encode_queue.command_pool, None);
            }

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

pub fn select_memory_type(
    props: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
    memory_type_bits: Option<u32>,
) -> Option<u32> {
    for i in 0..props.memory_type_count {
        if let Some(mask) = memory_type_bits {
            if mask & (1 << i) == 0 {
                continue;
            }
        }

        if flags.is_empty()
            || props.memory_types[i as usize]
                .property_flags
                .contains(flags)
        {
            return Some(i);
        }
    }

    None
}

pub struct VkImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub memory: vk::DeviceMemory,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
    vk: Arc<VkContext>,
}

impl VkImage {
    pub fn new(
        vk: Arc<VkContext>,
        format: vk::Format,
        ignore_alpha: bool,
        width: u32,
        height: u32,
        usage: vk::ImageUsageFlags,
        sharing_mode: vk::SharingMode,
        flags: vk::ImageCreateFlags,
    ) -> anyhow::Result<Self> {
        let image = {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(usage)
                .sharing_mode(sharing_mode)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .flags(flags);

            unsafe {
                vk.device
                    .create_image(&create_info, None)
                    .context("VkCreateImage")?
            }
        };

        let memory =
            unsafe { bind_memory_for_image(&vk.device, &vk.device_info.memory_props, image)? };

        // Transfer-only images (e.g. readback staging) can't have a view.
        let view_usages = vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT
            | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
            | vk::ImageUsageFlags::INPUT_ATTACHMENT
            | vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR
            | vk::ImageUsageFlags::VIDEO_ENCODE_DPB_KHR;
        let view = if usage.intersects(view_usages) {
            unsafe { create_image_view(&vk.device, image, format, ignore_alpha)? }
        } else {
            vk::ImageView::null()
        };

        Ok(Self {
            image,
            view,
            memory,
            format,
            width,
            height,
            vk,
        })
    }

    pub fn wrap(
        vk: Arc<VkContext>,
        image: vk::Image,
        view: vk::ImageView,
        memory: vk::DeviceMemory,
        format: vk::Format,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            image,
            view,
            memory,
            format,
            width,
            height,
            vk,
        }
    }

    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn rect(&self) -> vk::Rect2D {
        vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.extent(),
        }
    }
}

impl Drop for VkImage {
    fn drop(&mut self) {
        unsafe {
            self.vk.device.destroy_image_view(self.view, None);
            self.vk.device.destroy_image(self.image, None);
            self.vk.device.free_memory(self.memory, None);
        }
    }
}

pub unsafe fn bind_memory_for_image(
    device: &ash::Device,
    props: &vk::PhysicalDeviceMemoryProperties,
    image: vk::Image,
) -> anyhow::Result<vk::DeviceMemory> {
    let image_memory_req = unsafe { device.get_image_memory_requirements(image) };

    let mem_type_index = select_memory_type(
        props,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        Some(image_memory_req.memory_type_bits),
    );

    if mem_type_index.is_none() {
        bail!(
            "no appropriate memory type found for reqs: {:?}",
            image_memory_req
        );
    }

    let memory = {
        let image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(image_memory_req.size)
            .memory_type_index(mem_type_index.unwrap());

        unsafe {
            device
                .allocate_memory(&image_allocate_info, None)
                .context("VkAllocateMemory")?
        }
    };

    unsafe {
        device
            .bind_image_memory(image, memory, 0)
            .context("VkBindImageMemory")?;
    }

    Ok(memory)
}

pub unsafe fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    ignore_alpha: bool,
) -> anyhow::Result<vk::ImageView> {
    let alpha_swizzle = if ignore_alpha {
        vk::ComponentSwizzle::ONE
    } else {
        vk::ComponentSwizzle::IDENTITY
    };

    let create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: alpha_swizzle,
        })
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        });

    unsafe {
        device
            .create_image_view(&create_info, None)
            .context("VkCreateImageView")
    }
}

pub struct VkHostBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub access: *mut c_void,
    pub len: usize,
    vk: Arc<VkContext>,
}

unsafe impl Send for VkHostBuffer {}

impl VkHostBuffer {
    pub fn new(
        vk: Arc<VkContext>,
        mem_type: u32,
        usage: vk::BufferUsageFlags,
        size: usize,
    ) -> anyhow::Result<Self> {
        let buffer = {
            let create_info = vk::BufferCreateInfo::default()
                .size(size as u64)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            unsafe {
                vk.device
                    .create_buffer(&create_info, None)
                    .context("VkCreateBuffer")?
            }
        };

        let requirements = unsafe { vk.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(mem_type);

        let memory = unsafe {
            vk.device
                .allocate_memory(&alloc_info, None)
                .context("VkAllocateMemory")?
        };

        unsafe {
            vk.device
                .bind_buffer_memory(buffer, memory, 0)
                .context("vkBindBufferMemory")?
        };

        let access = {
            unsafe {
                vk.device
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                    .context("VkMapMemory")?
            }
        };

        Ok(VkHostBuffer {
            buffer,
            memory,
            access,
            len: size,
            vk,
        })
    }

    pub(crate) fn wrap(
        vk: Arc<VkContext>,
        buf: vk::Buffer,
        memory: vk::DeviceMemory,
        buffer_size: usize,
    ) -> Self {
        let access = unsafe {
            vk.device
                .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                .context("failed to map buffer memory")
                .unwrap()
        };

        Self {
            buffer: buf,
            memory,
            access,
            len: buffer_size,
            vk,
        }
    }

    pub fn copy_from_slice(&mut self, src: &[u8]) {
        let dst = unsafe { std::slice::from_raw_parts_mut(self.access as *mut u8, self.len) };
        dst.copy_from_slice(src);
    }
}

impl Drop for VkHostBuffer {
    fn drop(&mut self) {
        unsafe {
            self.vk.device.unmap_memory(self.memory);
            self.vk.device.destroy_buffer(self.buffer, None);
            self.vk.device.free_memory(self.memory, None);
        }
    }
}

pub struct VkTimestampQueryPool {
    pub pool: vk::QueryPool,
    num_timestamps: u32,
}

impl VkTimestampQueryPool {
    pub unsafe fn cmd_reset(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_reset_query_pool(command_buffer, self.pool, 0, self.num_timestamps);
        }
    }

    #[allow(dead_code)]
    pub fn fetch_results(&self, device: &ash::Device) -> anyhow::Result<Vec<i64>> {
        let mut results = vec![0_i64; self.num_timestamps as usize];
        unsafe {
            device
                .get_query_pool_results(self.pool, 0, &mut results, vk::QueryResultFlags::WAIT)
                .context("vkGetQueryPoolResults")?;
        }

        for v in &results {
            assert!(v > &0_i64, "invalid query pool results")
        }

        Ok(results)
    }
}

pub fn create_timestamp_query_pool(
    device: &ash::Device,
    num_timestamps: u32,
) -> anyhow::Result<VkTimestampQueryPool> {
    let create_info = vk::QueryPoolCreateInfo::default()
        .query_type(vk::QueryType::TIMESTAMP)
        .query_count(num_timestamps);

    let pool = unsafe {
        device
            .create_query_pool(&create_info, None)
            .context("vkCreateQueryPool")?
    };

    Ok(VkTimestampQueryPool {
        pool,
        num_timestamps,
    })
}

pub fn load_shader(device: &ash::Device, bytes: &[u8]) -> anyhow::Result<vk::ShaderModule> {
    let code = ash::util::read_spv(&mut std::io::Cursor::new(bytes))?;
    let create_info = vk::ShaderModuleCreateInfo::default().code(&code);

    let shader = unsafe { device.create_shader_module(&create_info, None)? };

    Ok(shader)
}

pub fn allocate_command_buffer(
    device: &ash::Device,
    pool: vk::CommandPool,
) -> anyhow::Result<vk::CommandBuffer> {
    let create_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(pool)
        .command_buffer_count(1);

    let cb = unsafe {
        device
            .allocate_command_buffers(&create_info)
            .context("failed to allocate render command buffer")?
            .pop()
            .unwrap()
    };

    Ok(cb)
}

#[instrument(level = "trace", skip_all)]
pub unsafe fn begin_command_buffer(
    device: &ash::Device,
    cb: vk::CommandBuffer,
) -> anyhow::Result<()> {
    unsafe {
        device.reset_command_buffer(cb, vk::CommandBufferResetFlags::empty())?;
        device.begin_command_buffer(
            cb,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;
    }
    Ok(())
}

pub fn insert_image_barrier(
    device: &ash::Device,
    cb: vk::CommandBuffer,
    image: vk::Image,
    queue_transfer: Option<(u32, u32)>,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags2,
    src_access: vk::AccessFlags2,
    dst_stage: vk::PipelineStageFlags2,
    dst_access: vk::AccessFlags2,
) {
    let (src_family, dst_family) =
        queue_transfer.unwrap_or((vk::QUEUE_FAMILY_IGNORED, vk::QUEUE_FAMILY_IGNORED));

    let barriers = [vk::ImageMemoryBarrier2::default()
        .src_stage_mask(src_stage)
        .src_access_mask(src_access)
        .dst_stage_mask(dst_stage)
        .dst_access_mask(dst_access)
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(src_family)
        .dst_queue_family_index(dst_family)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        })];

    unsafe {
        device.cmd_pipeline_barrier2(
            cb,
            &vk::DependencyInfo::default().image_memory_barriers(&barriers),
        )
    };
}

fn contains_extension(list: &[CString], str: &CStr) -> bool {
    list.iter().any(|v| v.as_c_str() == str)
}

#[cfg(test)]
mod tests {
    use ash::vk;

    #[test]
    fn test_chain() {
        vk_chain! {
            pub struct H264EncodeProfile<'a> {
                pub profile: vk::VideoProfileInfoKHR<'a>,
                pub encode_usage_info: vk::VideoEncodeUsageInfoKHR<'a>,
                pub h264_profile: vk::VideoEncodeH264ProfileInfoKHR<'a>,
            }
        }

        let mut chain = H264EncodeProfile::new(
            vk::VideoProfileInfoKHR::default(),
            vk::VideoEncodeUsageInfoKHR::default(),
            vk::VideoEncodeH264ProfileInfoKHR::default(),
        );

        chain.with_encode_usage_info(|info| {
            info.video_usage_hints(vk::VideoEncodeUsageFlagsKHR::STREAMING)
        });

        assert_eq!(
            chain.encode_usage_info.video_usage_hints,
            vk::VideoEncodeUsageFlagsKHR::STREAMING
        );
    }
}
