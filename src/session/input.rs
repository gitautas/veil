//! Virtual gamepads.
//!
//! Each client gamepad becomes either a virtual Xbox 360 wired controller
//! (through uinput) that matches what the kernel's xpad driver exposes for a
//! real one (045e:028e): the same buttons, axes, ranges and force feedback;
//! or a DualSense (through uhid, see `dualsense.rs`) that the kernel's own
//! driver binds to. Steam, SDL and games then treat it exactly like the real
//! thing, including their built-in mappings, which are index-based and break
//! on any extra button or axis.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt as _;

use anyhow::Context as _;
use tracing::{debug, info, warn};

use input_linux::{
    AbsoluteAxis, AbsoluteInfo, AbsoluteInfoSetup, EventKind, EventTime, ForceFeedbackKind,
    InputEvent, InputId, Key, SynchronizeKind, UInputHandle,
};
use input_linux::sys;

use super::compositor::ButtonState;
use super::control::GamepadState;
use super::dualsense::UhidDualSense;

// ─── Gamepad layout ──────────────────────────────────────────────────────────

/// Which controller a virtual gamepad emulates.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GamepadLayout {
    /// A wired Xbox 360 controller.
    #[default]
    Xbox360,
    /// A Sony DualSense (PS5), for PlayStation pads.
    DualSense,
}

impl GamepadLayout {
    /// The wire value: 0 = Xbox 360, 1 = DualSense.
    pub fn from_wire(b: u8) -> Self {
        match b {
            1 => Self::DualSense,
            _ => Self::Xbox360,
        }
    }
}

// ─── What a real Xbox 360 pad looks like ─────────────────────────────────────

/// The name xpad gives a wired Xbox 360 controller.
const DEVICE_NAME: &str = "Microsoft X-Box 360 pad";
const DEVICE_ID: InputId = InputId {
    bustype: 0x03,   // BUS_USB
    vendor: 0x045e,  // Microsoft
    product: 0x028e, // Xbox 360 Wired Controller
    version: 0x0114, // bcdDevice of a retail pad; SDL's database keys on it
};
/// Force-feedback effects a client may upload at once.
const FF_EFFECTS_MAX: u32 = 16;

/// W3C standard-mapping button index → evdev key, as xpad reports them.
///
/// xpad predates the BTN_NORTH/BTN_WEST naming and reports X as BTN_X
/// (0x133, which is BTN_NORTH) and Y as BTN_Y (0x134, BTN_WEST). Software
/// expects that for this VID/PID, so we copy it rather than "fixing" it.
/// `None`: triggers (6, 7) are analog axes and the d-pad (12-15) is a hat,
/// so neither has a button. A real 360 pad has no other buttons, and any
/// extra key would shift the index-based mappings games use.
const BUTTON_MAP: [Option<Key>; 17] = [
    Some(Key::ButtonSouth),  // 0: A      (BTN_A)
    Some(Key::ButtonEast),   // 1: B      (BTN_B)
    Some(Key::ButtonNorth),  // 2: X      (BTN_X, 0x133)
    Some(Key::ButtonWest),   // 3: Y      (BTN_Y, 0x134)
    Some(Key::ButtonTL),     // 4: LB
    Some(Key::ButtonTR),     // 5: RB
    None,                    // 6: LT     → ABS_Z
    None,                    // 7: RT     → ABS_RZ
    Some(Key::ButtonSelect), // 8: Back
    Some(Key::ButtonStart),  // 9: Start
    Some(Key::ButtonThumbl), // 10: LS
    Some(Key::ButtonThumbr), // 11: RS
    None,                    // 12: Up    → HAT0Y -1
    None,                    // 13: Down  → HAT0Y +1
    None,                    // 14: Left  → HAT0X -1
    None,                    // 15: Right → HAT0X +1
    Some(Key::ButtonMode),   // 16: Guide
];

/// Stick axes in W3C order (left X, left Y, right X, right Y).
const STICK_AXES: [AbsoluteAxis; 4] =
    [AbsoluteAxis::X, AbsoluteAxis::Y, AbsoluteAxis::RX, AbsoluteAxis::RY];

fn stick_info() -> AbsoluteInfo {
    AbsoluteInfo { value: 0, minimum: -32768, maximum: 32767, fuzz: 16, flat: 128, resolution: 0 }
}

fn trigger_info() -> AbsoluteInfo {
    AbsoluteInfo { value: 0, minimum: 0, maximum: 255, fuzz: 0, flat: 0, resolution: 0 }
}

fn hat_info() -> AbsoluteInfo {
    AbsoluteInfo { value: 0, minimum: -1, maximum: 1, fuzz: 0, flat: 0, resolution: 0 }
}

/// Split a server-wide pad ID into (viewer, index in that viewer's browser).
pub fn split_pad_id(id: u64) -> (u32, u8) {
    ((id >> 8) as u32, (id & 0xff) as u8)
}

// ─── InputDeviceManager ──────────────────────────────────────────────────────

/// Manages virtual input devices backed by Linux uinput.
pub struct InputDeviceManager;

impl InputDeviceManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    /// Create and plug a virtual gamepad. A DualSense that can't be created
    /// (usually because /dev/uhid isn't accessible) falls back to an Xbox pad.
    pub fn plug_gamepad(
        &mut self,
        id: u64,
        layout: GamepadLayout,
        permanent: bool,
    ) -> anyhow::Result<GamepadHandle> {
        let mut notice = None;
        if layout == GamepadLayout::DualSense {
            match UhidDualSense::new(id) {
                Ok(ds) => {
                    return Ok(GamepadHandle::new(
                        Some(VirtualPad::DualSense(ds)),
                        layout,
                        permanent,
                        None,
                        None,
                    ));
                }
                Err(e) => {
                    warn!(id, "failed to create virtual DualSense, using an Xbox pad: {e:#}");
                    notice = Some(format!(
                        "Emulating an Xbox pad instead of a DualSense: {}",
                        describe_open_error(&e, "/dev/uhid"),
                    ));
                }
            }
        }

        let (inner, device, error) = match UInputGamepad::new(id) {
            Ok(gp) => {
                let device = gp.handle.evdev_path().ok().map(|p| p.display().to_string());
                (Some(VirtualPad::Xbox(gp)), device, None)
            }
            Err(e) => {
                warn!(id, permanent, "failed to create virtual gamepad: {e:#}");
                (None, None, Some(describe_open_error(&e, "/dev/uinput")))
            }
        };
        let mut handle = GamepadHandle::new(inner, layout, permanent, device, error);
        handle.notice = notice;
        Ok(handle)
    }
}

// ─── GamepadHandle ───────────────────────────────────────────────────────────

/// A rumble request from a game, to forward to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rumble {
    pub strong: u16,
    pub weak: u16,
    /// 0 with zero magnitudes means stop.
    pub duration_ms: u16,
}

/// The device behind a gamepad handle.
enum VirtualPad {
    Xbox(UInputGamepad),
    DualSense(UhidDualSense),
}

/// An error opening a device node, with a hint if it's a permissions problem.
fn describe_open_error(e: &anyhow::Error, node: &str) -> String {
    let denied = e
        .chain()
        .filter_map(|c| c.downcast_ref::<std::io::Error>())
        .any(|io| io.kind() == std::io::ErrorKind::PermissionDenied);
    if denied {
        format!("{e:#}: {node} isn't writable by this user (see udev/61-veil.rules)")
    } else {
        format!("{e:#}")
    }
}

/// Handle to a plugged virtual gamepad.
pub struct GamepadHandle {
    inner: Option<VirtualPad>,
    /// The pad type the client asked for (even if it fell back).
    pub layout: GamepadLayout,
    pub permanent: bool,
    /// The evdev node of the virtual device, e.g. `/dev/input/event23`.
    device: Option<String>,
    /// Why the virtual device couldn't be created, if it couldn't.
    error: Option<String>,
    /// Input events received for this gamepad.
    events: u64,
    /// The newest snapshot applied, to drop stale or duplicate ones.
    last_seq: Option<u32>,
    /// The state the device currently reports.
    state: GamepadState,
    /// Rumble requests forwarded, for diagnostics.
    rumbles: u64,
    /// Why the device isn't the requested layout, if it isn't.
    notice: Option<String>,
    /// When we last looked for a DualSense's evdev node, which the driver
    /// creates shortly after the device.
    last_device_lookup: Option<std::time::Instant>,
}

impl GamepadHandle {
    fn new(
        inner: Option<VirtualPad>,
        layout: GamepadLayout,
        permanent: bool,
        device: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            inner,
            layout,
            permanent,
            device,
            error,
            events: 0,
            last_seq: None,
            state: GamepadState::default(),
            rumbles: 0,
            notice: None,
            last_device_lookup: None,
        }
    }

    pub fn status(&self, id: u64) -> crate::session::control::GamepadStatus {
        let (viewer, index) = split_pad_id(id);
        let (name, emulates) = match &self.inner {
            Some(VirtualPad::DualSense(_)) => (
                super::dualsense::DEVICE_NAME,
                "DualSense (054c:0ce6), through the kernel's hid-playstation driver",
            ),
            _ => (DEVICE_NAME, "Xbox 360 Wired Controller (045e:028e), as the xpad driver exposes it"),
        };
        crate::session::control::GamepadStatus {
            id,
            viewer,
            index,
            name: name.to_owned(),
            layout: self.layout,
            emulates,
            device: self.device.clone(),
            error: self.error.clone(),
            notice: self.notice.clone(),
            events: self.events,
        }
    }

    /// Apply a full snapshot. Returns false if it was older than (or the same
    /// as) one already applied.
    pub(crate) fn apply_state(&mut self, seq: u32, new: GamepadState) -> bool {
        if let Some(last) = self.last_seq {
            // Wrapping comparison, so a long session can overflow seq.
            if (seq.wrapping_sub(last) as i32) <= 0 {
                return false;
            }
        }
        self.last_seq = Some(seq);
        self.events += 1;

        let old = std::mem::replace(&mut self.state, new);
        if old == new {
            return true;
        }
        let gp = match &mut self.inner {
            Some(VirtualPad::Xbox(gp)) => gp,
            Some(VirtualPad::DualSense(ds)) => {
                ds.set_state(new);
                return true;
            }
            None => return true,
        };

        let mut events = Vec::with_capacity(8);
        for (i, key) in BUTTON_MAP.iter().enumerate() {
            let Some(key) = key else { continue };
            let bit = 1 << i;
            if (old.buttons ^ new.buttons) & bit != 0 {
                events.push(make_event(EventKind::Key, *key as u16, (new.buttons & bit != 0) as i32));
            }
        }
        for (i, axis) in STICK_AXES.iter().enumerate() {
            if old.axes[i] != new.axes[i] {
                events.push(make_event(EventKind::Absolute, *axis as u16, new.axes[i] as i32));
            }
        }
        if old.left_trigger != new.left_trigger {
            events.push(make_event(EventKind::Absolute, AbsoluteAxis::Z as u16, new.left_trigger as i32));
        }
        if old.right_trigger != new.right_trigger {
            events.push(make_event(EventKind::Absolute, AbsoluteAxis::RZ as u16, new.right_trigger as i32));
        }
        let (old_hat, new_hat) = (hat(old.buttons), hat(new.buttons));
        if old_hat.0 != new_hat.0 {
            events.push(make_event(EventKind::Absolute, AbsoluteAxis::Hat0X as u16, new_hat.0));
        }
        if old_hat.1 != new_hat.1 {
            events.push(make_event(EventKind::Absolute, AbsoluteAxis::Hat0Y as u16, new_hat.1));
        }
        events.push(make_event(EventKind::Synchronize, SynchronizeKind::Report as u16, 0));
        gp.emit(&events);
        true
    }

    /// Set an absolute stick axis in the range [-1.0, 1.0] (older clients).
    pub(crate) fn axis(&mut self, axis_code: u32, value: f64) {
        let mut state = self.state;
        let Some(slot) = state.axes.get_mut(axis_code as usize) else { return };
        *slot = (value.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        self.apply_legacy(state);
    }

    /// Set a trigger in the range [0.0, 1.0] (older clients).
    pub(crate) fn trigger(&mut self, trigger_code: u32, value: f64) {
        let mut state = self.state;
        let raw = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        match trigger_code {
            0 => state.left_trigger = raw,
            1 => state.right_trigger = raw,
            _ => return,
        }
        self.apply_legacy(state);
    }

    /// Press or release a button by W3C index (older clients).
    pub(crate) fn input(&mut self, button_code: u32, state: ButtonState) {
        if button_code >= 32 {
            return;
        }
        let mut new = self.state;
        let bit = 1 << button_code;
        if matches!(state, ButtonState::Pressed) {
            new.buttons |= bit;
        } else {
            new.buttons &= !bit;
        }
        self.apply_legacy(new);
    }

    /// Per-event messages have no sequence numbers, so apply them directly.
    fn apply_legacy(&mut self, state: GamepadState) {
        let next = self.last_seq.map_or(0, |s| s.wrapping_add(1));
        self.apply_state(next, state);
    }

    /// Kept for the reactor's per-frame flush; snapshots already end with a
    /// SYN_REPORT, so there's nothing left to flush.
    pub(crate) fn frame(&mut self) {}

    /// Handle force-feedback requests from games, returning rumble commands
    /// to forward to the client.
    pub(crate) fn poll_rumble(&mut self) -> Vec<Rumble> {
        let out = match &mut self.inner {
            Some(VirtualPad::Xbox(gp)) => gp.poll_ff(),
            Some(VirtualPad::DualSense(ds)) => ds.poll_rumble(),
            None => Vec::new(),
        };
        self.rumbles += out.len() as u64;
        out
    }

    /// Look for a DualSense's evdev node until it appears (at most once a
    /// second). Returns true when it was just found.
    pub(crate) fn find_device(&mut self) -> bool {
        let Some(VirtualPad::DualSense(ds)) = &self.inner else { return false };
        if self.device.is_some()
            || self.last_device_lookup.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(1))
        {
            return false;
        }
        self.last_device_lookup = Some(std::time::Instant::now());
        self.device = ds.evdev_path();
        self.device.is_some()
    }
}

/// D-pad bits (12-15) as a hat: (x, y), each -1, 0 or 1.
fn hat(buttons: u32) -> (i32, i32) {
    let b = |i: u32| buttons & (1 << i) != 0;
    let axis = |neg: bool, pos: bool| match (neg, pos) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0,
    };
    (axis(b(14), b(15)), axis(b(12), b(13)))
}

/// UI_SET_PHYS. input-linux-sys declares it with a 1-byte argument
/// (`c_char`), but the kernel's `_IOW('U', 108, char*)` encodes a pointer's
/// size, and uinput matches the number exactly, so the crate's version always
/// fails with EINVAL.
fn set_phys(handle: &UInputHandle<std::fs::File>, phys: &std::ffi::CStr) -> std::io::Result<()> {
    use std::os::fd::{AsFd as _, AsRawFd as _};
    const UI_SET_PHYS: libc::c_ulong = (1 << 30)
        | ((std::mem::size_of::<*const libc::c_char>() as libc::c_ulong) << 16)
        | ((b'U' as libc::c_ulong) << 8)
        | 108;
    let ret = unsafe { libc::ioctl(handle.as_fd().as_raw_fd(), UI_SET_PHYS, phys.as_ptr()) };
    if ret < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
}

fn make_event(kind: EventKind, code: u16, value: i32) -> sys::input_event {
    InputEvent { time: EventTime::new(0, 0), kind, code, value }.into_raw()
}

// ─── UInputGamepad internals ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct RumbleEffect {
    strong: u16,
    weak: u16,
    length_ms: u16,
}

struct UInputGamepad {
    handle: UInputHandle<std::fs::File>,
    /// Uploaded rumble effects by effect ID.
    effects: HashMap<i16, RumbleEffect>,
    /// FF_GAIN, 0-0xffff.
    gain: u32,
}

impl UInputGamepad {
    fn new(id: u64) -> anyhow::Result<Self> {
        // Non-blocking, so polling for force-feedback requests never stalls
        // the reactor.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")?;
        let handle = UInputHandle::new(file);

        for kind in [EventKind::Key, EventKind::Absolute, EventKind::Synchronize, EventKind::ForceFeedback] {
            handle.set_evbit(kind).with_context(|| format!("UI_SET_EVBIT {kind:?}"))?;
        }

        for key in BUTTON_MAP.iter().flatten() {
            handle.set_keybit(*key).with_context(|| format!("UI_SET_KEYBIT {key:?}"))?;
        }

        // Same axes as xpad, so the axis indices games map by line up:
        // X, Y, Z (LT), RX, RY, RZ (RT), then the hat.
        let abs_setups = [
            AbsoluteInfoSetup { axis: AbsoluteAxis::X, info: stick_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::Y, info: stick_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::Z, info: trigger_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::RX, info: stick_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::RY, info: stick_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::RZ, info: trigger_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::Hat0X, info: hat_info() },
            AbsoluteInfoSetup { axis: AbsoluteAxis::Hat0Y, info: hat_info() },
        ];
        for setup in &abs_setups {
            handle.set_absbit(setup.axis).with_context(|| format!("UI_SET_ABSBIT {:?}", setup.axis))?;
        }

        // xpad uses the kernel's memoryless FF, which offers rumble and gain.
        for kind in [ForceFeedbackKind::Rumble, ForceFeedbackKind::Gain] {
            handle.set_ffbit(kind).with_context(|| format!("UI_SET_FFBIT {kind:?}"))?;
        }

        let phys = std::ffi::CString::new(format!("veil/gamepad{id}"))?;
        set_phys(&handle, &phys).context("UI_SET_PHYS")?;
        handle
            .create(&DEVICE_ID, DEVICE_NAME.as_bytes(), FF_EFFECTS_MAX, &abs_setups)
            .context("creating the uinput device")?;

        info!(id, "virtual Xbox 360 pad plugged (/dev/uinput)");
        Ok(Self { handle, effects: HashMap::new(), gain: 0xffff })
    }

    fn emit(&self, events: &[sys::input_event]) {
        if let Err(e) = self.handle.write(events) {
            warn!("gamepad uinput write error: {e}");
        }
    }

    /// Answer uinput's force-feedback requests (effect uploads and erases)
    /// and turn effect playback into rumble commands.
    fn poll_ff(&mut self) -> Vec<Rumble> {
        let mut out = Vec::new();
        let mut buf = [unsafe { std::mem::zeroed::<sys::input_event>() }; 16];
        loop {
            let n = match self.handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    debug!("uinput read error: {e}");
                    break;
                }
            };
            for ev in &buf[..n] {
                match ev.type_ as i32 {
                    sys::EV_UINPUT => self.handle_ff_request(ev.code as i32, ev.value as u32),
                    t if t == sys::EV_FF as i32 => {
                        if let Some(rumble) = self.handle_ff_event(ev.code, ev.value) {
                            out.push(rumble);
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }

    fn handle_ff_request(&mut self, code: i32, request_id: u32) {
        match code {
            sys::UI_FF_UPLOAD => {
                let mut upload: sys::uinput_ff_upload = unsafe { std::mem::zeroed() };
                upload.request_id = request_id;
                if let Err(e) = self.handle.ff_upload_begin(&mut upload) {
                    warn!("UI_BEGIN_FF_UPLOAD failed: {e}");
                    return;
                }
                let effect = upload.effect;
                if effect.type_ == sys::FF_RUMBLE {
                    let union: &sys::ff_effect_union = (&effect).into();
                    let r = union.rumble();
                    self.effects.insert(effect.id, RumbleEffect {
                        strong: r.strong_magnitude,
                        weak: r.weak_magnitude,
                        length_ms: effect.replay.length,
                    });
                    upload.retval = 0;
                } else {
                    // Like xpad, only rumble is supported.
                    upload.retval = -libc::EINVAL;
                }
                if let Err(e) = self.handle.ff_upload_end(&upload) {
                    warn!("UI_END_FF_UPLOAD failed: {e}");
                }
            }
            sys::UI_FF_ERASE => {
                let mut erase: sys::uinput_ff_erase = unsafe { std::mem::zeroed() };
                erase.request_id = request_id;
                if let Err(e) = self.handle.ff_erase_begin(&mut erase) {
                    warn!("UI_BEGIN_FF_ERASE failed: {e}");
                    return;
                }
                self.effects.remove(&(erase.effect_id as i16));
                erase.retval = 0;
                if let Err(e) = self.handle.ff_erase_end(&erase) {
                    warn!("UI_END_FF_ERASE failed: {e}");
                }
            }
            _ => {}
        }
    }

    /// EV_FF from a game: either FF_GAIN, or play/stop of an uploaded effect.
    fn handle_ff_event(&mut self, code: u16, value: i32) -> Option<Rumble> {
        if code == sys::FF_GAIN {
            self.gain = value.clamp(0, 0xffff) as u32;
            return None;
        }
        let effect = self.effects.get(&(code as i16))?;
        if value <= 0 {
            return Some(Rumble { strong: 0, weak: 0, duration_ms: 0 });
        }
        let scale = |m: u16| ((m as u32 * self.gain) / 0xffff) as u16;
        Some(Rumble {
            strong: scale(effect.strong),
            weak: scale(effect.weak),
            // 0 means "until stopped"; the browser needs a duration, and games
            // that do this stop the effect explicitly.
            duration_ms: if effect.length_ms == 0 { 5000 } else { effect.length_ms },
        })
    }
}

impl Drop for UInputGamepad {
    fn drop(&mut self) {
        if let Err(e) = self.handle.dev_destroy() {
            warn!("failed to destroy virtual gamepad: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hat_from_dpad_bits() {
        assert_eq!(hat(0), (0, 0));
        assert_eq!(hat(1 << 12), (0, -1)); // up
        assert_eq!(hat(1 << 13), (0, 1)); // down
        assert_eq!(hat(1 << 14), (-1, 0)); // left
        assert_eq!(hat((1 << 15) | (1 << 12)), (1, -1)); // up-right
        assert_eq!(hat((1 << 14) | (1 << 15)), (0, 0)); // both: centred
    }

    /// Needs a writable /dev/uinput: `cargo test -- --ignored create_device`.
    #[test]
    #[ignore]
    fn create_device() {
        let dev = UInputGamepad::new(0xfe).expect("creating the virtual pad");
        let path = dev.handle.evdev_path().expect("evdev path");
        println!("created {}", path.display());
    }

    #[test]
    fn pad_ids_round_trip() {
        assert_eq!(split_pad_id((7 << 8) | 2), (7, 2));
    }

    #[test]
    fn button_map_matches_xpad() {
        // The keys a real 360 pad reports, in code order: SDL's index-based
        // mapping for 045e:028e relies on there being exactly these.
        let mut keys: Vec<u16> = BUTTON_MAP.iter().flatten().map(|k| *k as u16).collect();
        keys.sort();
        assert_eq!(keys, [0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 0x13d, 0x13e]);
    }
}
