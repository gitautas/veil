//! A virtual Sony DualSense, created through /dev/uhid.
//!
//! Rather than imitating the DualSense's evdev nodes, this creates a USB HID
//! device and lets the kernel's own hid-playstation driver bind to it, the way
//! it does for a real pad. That gives the same evdev nodes (gamepad, motion
//! sensors, touchpad) *and* a hidraw node, which Steam and SDL read directly
//! for their full DualSense support (button glyphs, Steam Input, rumble).
//!
//! The driver asks for calibration, firmware and pairing reports while it
//! probes, and sends output reports for rumble and LEDs; a small thread per
//! pad answers those. Input reports are written whenever the pad changes,
//! and repeated periodically like a real pad's.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use anyhow::Context as _;
use tracing::{debug, info, trace, warn};

use super::control::GamepadState;
use super::input::Rumble;

/// The name hid-playstation's gamepad node gets for a real pad.
pub const DEVICE_NAME: &str = "Sony Interactive Entertainment DualSense Wireless Controller";
const VENDOR: u32 = 0x054c;
const PRODUCT: u32 = 0x0ce6;
const BUS_USB: u16 = 0x03;

// struct uhid_event: a u32 type, then the largest member of the union
// (uhid_create2_req, 4372 bytes).
const EVENT_SIZE: usize = 4 + 4372;
const UHID_DATA_MAX: usize = 4096;

const UHID_DESTROY: u32 = 1;
const UHID_START: u32 = 2;
const UHID_STOP: u32 = 3;
const UHID_OPEN: u32 = 4;
const UHID_CLOSE: u32 = 5;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;

const INPUT_REPORT_ID: u8 = 0x01;
const INPUT_REPORT_SIZE: usize = 64;
const OUTPUT_REPORT_ID: u8 = 0x02;
const FEATURE_CALIBRATION: u8 = 0x05;
const FEATURE_PAIRING_INFO: u8 = 0x09;
const FEATURE_FIRMWARE_INFO: u8 = 0x20;

/// Real pads report continuously; repeat the current state this often.
const REPORT_INTERVAL: Duration = Duration::from_millis(50);

/// A USB report descriptor with the DualSense's report IDs and sizes: input
/// report 1 (63 bytes), output report 2 (47 bytes), and the feature reports
/// the driver and SDL request. hid-playstation parses reports by offset, so
/// only the top-level gamepad usage and the sizes matter.
#[rustfmt::skip]
const REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,       // Usage Page (Generic Desktop)
    0x09, 0x05,       // Usage (Game Pad)
    0xa1, 0x01,       // Collection (Application)
    0x85, INPUT_REPORT_ID,
    // Sticks and triggers: X, Y, Z, Rz (sticks), Rx, Ry (L2, R2).
    0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35, 0x09, 0x33, 0x09, 0x34,
    0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x06, 0x81, 0x02,
    // Sequence number.
    0x06, 0x00, 0xff, 0x09, 0x20, 0x95, 0x01, 0x81, 0x02,
    // Hat switch (4 bits, 8 = centred).
    0x05, 0x01, 0x09, 0x39, 0x15, 0x00, 0x25, 0x07, 0x35, 0x00, 0x46, 0x3b, 0x01,
    0x65, 0x14, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x65, 0x00,
    // 15 buttons, then 13 vendor bits to fill the four button bytes.
    0x05, 0x09, 0x19, 0x01, 0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f,
    0x81, 0x02,
    0x06, 0x00, 0xff, 0x09, 0x21, 0x95, 0x0d, 0x81, 0x02,
    // Motion, touch and battery data.
    0x06, 0x00, 0xff, 0x09, 0x22, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x34,
    0x81, 0x02,
    // Output report 2: rumble and LEDs.
    0x85, OUTPUT_REPORT_ID, 0x09, 0x23, 0x95, 0x2f, 0x91, 0x02,
    // Feature reports.
    0x85, FEATURE_CALIBRATION, 0x09, 0x24, 0x95, 0x28, 0xb1, 0x02,
    0x85, 0x08, 0x09, 0x25, 0x95, 0x2f, 0xb1, 0x02,
    0x85, FEATURE_PAIRING_INFO, 0x09, 0x26, 0x95, 0x13, 0xb1, 0x02,
    0x85, FEATURE_FIRMWARE_INFO, 0x09, 0x27, 0x95, 0x3f, 0xb1, 0x02,
    0xc0,             // End Collection
];

/// What the reactor and the device thread share.
struct Shared {
    state: GamepadState,
    seq: u8,
    started: Instant,
}

pub struct UhidDualSense {
    file: Arc<File>,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    rumble: mpsc::Receiver<Rumble>,
    /// The MAC address hid-playstation puts in its input nodes' `uniq`
    /// (it leaves `phys` empty), to find the gamepad's evdev node.
    uniq: String,
}

impl UhidDualSense {
    pub fn new(id: u64) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/uhid")
            .context("opening /dev/uhid")?;

        let phys = format!("veil/gamepad{id}");
        let mut ev = event(UHID_CREATE2);
        put_str(&mut ev[4..132], DEVICE_NAME);
        put_str(&mut ev[132..196], &phys);
        ev[260..262].copy_from_slice(&(REPORT_DESCRIPTOR.len() as u16).to_ne_bytes());
        ev[262..264].copy_from_slice(&BUS_USB.to_ne_bytes());
        ev[264..268].copy_from_slice(&VENDOR.to_ne_bytes());
        ev[268..272].copy_from_slice(&PRODUCT.to_ne_bytes());
        ev[272..276].copy_from_slice(&0x0100_u32.to_ne_bytes()); // bcdDevice
        ev[280..280 + REPORT_DESCRIPTOR.len()].copy_from_slice(REPORT_DESCRIPTOR);
        (&file).write_all(&ev).context("UHID_CREATE2")?;

        let file = Arc::new(file);
        let shared = Arc::new(Mutex::new(Shared {
            state: GamepadState::default(),
            seq: 0,
            started: Instant::now(),
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (rumble_tx, rumble) = mpsc::channel();

        let thread = {
            let (file, shared, stop) = (file.clone(), shared.clone(), stop.clone());
            std::thread::Builder::new()
                .name(format!("dualsense {id}"))
                .spawn(move || device_thread(id, &file, &shared, &stop, rumble_tx))?
        };

        let mac = mac_address(id);
        let uniq = mac.iter().rev().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":");

        info!(id, mac = uniq, "virtual DualSense plugged (/dev/uhid)");
        Ok(Self { file, shared, stop, thread: Some(thread), rumble, uniq })
    }

    /// Report a new state.
    pub fn set_state(&self, state: GamepadState) {
        let mut shared = self.shared.lock().unwrap();
        shared.state = state;
        if let Err(e) = write_input_report(&self.file, &mut shared) {
            warn!("dualsense uhid write error: {e}");
        }
    }

    /// Rumble the driver, Steam or a game asked for since the last call.
    pub fn poll_rumble(&self) -> Vec<Rumble> {
        self.rumble.try_iter().collect()
    }

    /// The gamepad's evdev node, once hid-playstation has created it.
    pub fn evdev_path(&self) -> Option<String> {
        find_evdev("uniq", &self.uniq, DEVICE_NAME)
    }
}

impl Drop for UhidDualSense {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Err(e) = (&*self.file).write_all(&event(UHID_DESTROY)) {
            warn!("failed to destroy virtual DualSense: {e}");
        }
    }
}

/// Answer the driver: feature report requests, output reports (rumble), and
/// keep input reports flowing.
fn device_thread(
    id: u64,
    file: &File,
    shared: &Mutex<Shared>,
    stop: &AtomicBool,
    rumble_tx: mpsc::Sender<Rumble>,
) {
    let mut buf = vec![0u8; EVENT_SIZE];
    let mut started = false;
    let mut last_motors = (0u8, 0u8);
    let mut last_report = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let mut pfd = libc::pollfd { fd: file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ready = unsafe { libc::poll(&mut pfd, 1, REPORT_INTERVAL.as_millis() as i32) };

        if started && last_report.elapsed() >= REPORT_INTERVAL {
            if let Err(e) = write_input_report(file, &mut shared.lock().unwrap()) {
                debug!(id, "dualsense uhid write error: {e}");
            }
            last_report = Instant::now();
        }
        if ready <= 0 || pfd.revents & libc::POLLIN == 0 {
            continue;
        }

        let n = match (&*file).read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                warn!(id, "dualsense uhid read error: {e}");
                break;
            }
        };
        if n < 4 {
            continue;
        }

        match u32::from_ne_bytes(buf[0..4].try_into().unwrap()) {
            UHID_START => {
                debug!(id, "dualsense: driver started");
                started = true;
            }
            UHID_STOP => {
                debug!(id, "dualsense: driver stopped");
                started = false;
            }
            UHID_OPEN | UHID_CLOSE => {}
            UHID_GET_REPORT => {
                let req_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap());
                let rnum = buf[8];
                let reply = feature_report(id, rnum);
                if reply.is_none() {
                    debug!(id, rnum, "dualsense: unknown feature report requested");
                }
                let mut ev = event(UHID_GET_REPORT_REPLY);
                ev[4..8].copy_from_slice(&req_id.to_ne_bytes());
                match reply {
                    Some(data) => {
                        ev[10..12].copy_from_slice(&(data.len() as u16).to_ne_bytes());
                        ev[12..12 + data.len()].copy_from_slice(&data);
                    }
                    None => ev[8..10].copy_from_slice(&(libc::EIO as u16).to_ne_bytes()),
                }
                if let Err(e) = (&*file).write_all(&ev) {
                    warn!(id, "UHID_GET_REPORT_REPLY failed: {e}");
                }
            }
            UHID_SET_REPORT => {
                // Accept and ignore (e.g. Steam setting the lightbar this way).
                let req_id = u32::from_ne_bytes(buf[4..8].try_into().unwrap());
                trace!(id, rnum = buf[8], "dualsense: set report");
                let mut ev = event(UHID_SET_REPORT_REPLY);
                ev[4..8].copy_from_slice(&req_id.to_ne_bytes());
                if let Err(e) = (&*file).write_all(&ev) {
                    warn!(id, "UHID_SET_REPORT_REPLY failed: {e}");
                }
            }
            UHID_OUTPUT => {
                let size = u16::from_ne_bytes(buf[4 + UHID_DATA_MAX..4 + UHID_DATA_MAX + 2].try_into().unwrap());
                let data = &buf[4..4 + (size as usize).min(UHID_DATA_MAX)];
                trace!(id, report = ?&data[..data.len().min(48)], "dualsense: output report");
                if let Some(motors) = output_report_motors(data) {
                    if motors != last_motors {
                        last_motors = motors;
                        let _ = rumble_tx.send(motors_to_rumble(motors));
                    }
                }
            }
            other => trace!(id, event = other, "dualsense: uhid event"),
        }
    }
}

/// Rumble motor levels (right/weak, left/strong) from an output report, if
/// it sets them.
fn output_report_motors(data: &[u8]) -> Option<(u8, u8)> {
    // USB output report: [0x02, valid_flag0, valid_flag1, motor_right,
    // motor_left, ..., valid_flag2 at 39, ...].
    if data.len() < 5 || data[0] != OUTPUT_REPORT_ID {
        return None;
    }
    // valid_flag0: compatible vibration (bit 0) or haptics select (bit 1);
    // valid_flag2 bit 2: compatible vibration v2, used by newer firmware.
    let flag2 = data.get(39).copied().unwrap_or(0);
    if data[1] & 0x03 != 0 || flag2 & 0x04 != 0 {
        return Some((data[3], data[4]));
    }
    // SDL stops rumble with a report that sets no flags at all. Reports
    // that only update LEDs (flag 1) leave the motors alone.
    (data[1] == 0 && data[2] == 0 && flag2 == 0).then_some((0, 0))
}

fn motors_to_rumble((right, left): (u8, u8)) -> Rumble {
    Rumble {
        // The left motor is the heavy, low-frequency one.
        strong: left as u16 * 257,
        weak: right as u16 * 257,
        // DualSense motors hold their level until changed, so play until
        // told otherwise.
        duration_ms: if left == 0 && right == 0 { 0 } else { u16::MAX },
    }
}

/// Feature reports hid-playstation (and SDL) read while setting up.
fn feature_report(id: u64, rnum: u8) -> Option<Vec<u8>> {
    let mut r;
    match rnum {
        FEATURE_CALIBRATION => {
            r = vec![0u8; 41];
            let mut put = |off: usize, v: i16| r[off..off + 2].copy_from_slice(&v.to_le_bytes());
            // Gyro biases (pitch, yaw, roll) of zero, then plus/minus
            // references for each gyro axis, the gyro speed references,
            // and the accelerometers' ±1 g references.
            for (i, v) in [8192, -8192, 8192, -8192, 8192, -8192].into_iter().enumerate() {
                put(7 + 2 * i, v);
            }
            put(19, 540);
            put(21, 540);
            for (i, v) in [8192, -8192, 8192, -8192, 8192, -8192].into_iter().enumerate() {
                put(23 + 2 * i, v);
            }
        }
        FEATURE_PAIRING_INFO => {
            r = vec![0u8; 20];
            r[1..7].copy_from_slice(&mac_address(id));
        }
        FEATURE_FIRMWARE_INFO => {
            r = vec![0u8; 64];
            r[24..28].copy_from_slice(&0x0000_0613_u32.to_le_bytes()); // hardware
            r[28..32].copy_from_slice(&0x0110_002a_u32.to_le_bytes()); // firmware
            r[44..46].copy_from_slice(&0x0224_u16.to_le_bytes()); // update version
        }
        _ => return None,
    }
    r[0] = rnum;
    Some(r)
}

/// A locally administered MAC address, unique per pad (the driver refuses a
/// second device with the same one), in the report's byte order: the driver
/// shows it reversed, as 02:76:6c:xx:xx:xx.
fn mac_address(id: u64) -> [u8; 6] {
    let b = id.to_le_bytes();
    [b[0], b[1], b[2], 0x6c, 0x76, 0x02]
}

/// Write the current state as USB input report 1.
fn write_input_report(file: &File, shared: &mut Shared) -> std::io::Result<()> {
    shared.seq = shared.seq.wrapping_add(1);
    // Sensor timestamps count in units of 1/3 µs.
    let timestamp = (shared.started.elapsed().as_micros() as u64 * 3) as u32;
    let report = input_report(&shared.state, shared.seq, timestamp);

    let mut ev = event(UHID_INPUT2);
    ev[4..6].copy_from_slice(&(report.len() as u16).to_ne_bytes());
    ev[6..6 + report.len()].copy_from_slice(&report);
    (&*file).write_all(&ev)
}

/// Build USB input report 1 from a W3C standard-layout state.
fn input_report(state: &GamepadState, seq: u8, timestamp: u32) -> [u8; INPUT_REPORT_SIZE] {
    let mut r = [0u8; INPUT_REPORT_SIZE];
    let b = |i: u32| state.buttons & (1 << i) != 0;
    let stick = |v: i16| ((v as i32 + 32768) >> 8) as u8;

    r[0] = INPUT_REPORT_ID;
    // Left X/Y, right X/Y, L2, R2.
    r[1] = stick(state.axes[0]);
    r[2] = stick(state.axes[1]);
    r[3] = stick(state.axes[2]);
    r[4] = stick(state.axes[3]);
    r[5] = state.left_trigger;
    r[6] = state.right_trigger;
    r[7] = seq;

    // Buttons byte 0: the hat in the low nibble, then square, cross,
    // circle, triangle.
    r[8] = hat(b(12), b(13), b(14), b(15))
        | (b(2) as u8) << 4 // square   (W3C 2, left face button)
        | (b(0) as u8) << 5 // cross    (W3C 0, bottom)
        | (b(1) as u8) << 6 // circle   (W3C 1, right)
        | (b(3) as u8) << 7; // triangle (W3C 3, top)
    // Byte 1: L1, R1, L2, R2, Create, Options, L3, R3.
    r[9] = b(4) as u8
        | (b(5) as u8) << 1
        | (b(6) as u8) << 2
        | (b(7) as u8) << 3
        | (b(8) as u8) << 4
        | (b(9) as u8) << 5
        | (b(10) as u8) << 6
        | (b(11) as u8) << 7;
    // Byte 2: PS, touchpad click (Chrome's button 17).
    r[10] = b(16) as u8 | (b(17) as u8) << 1;

    // At rest: no rotation, 1 g on the accelerometer's Y axis.
    r[24..26].copy_from_slice(&8192_i16.to_le_bytes());
    r[28..32].copy_from_slice(&timestamp.to_le_bytes());
    // Both touch points inactive.
    r[33] = 0x80;
    r[37] = 0x80;
    // Battery full (charging status 2, level 10).
    r[53] = 0x2a;
    r
}

/// D-pad to the DualSense's hat value: 0 = up, clockwise to 7, 8 = centred.
fn hat(up: bool, down: bool, left: bool, right: bool) -> u8 {
    let y = (down as i8) - (up as i8);
    let x = (right as i8) - (left as i8);
    match (x, y) {
        (0, -1) => 0,
        (1, -1) => 1,
        (1, 0) => 2,
        (1, 1) => 3,
        (0, 1) => 4,
        (-1, 1) => 5,
        (-1, 0) => 6,
        (-1, -1) => 7,
        _ => 8,
    }
}

fn event(kind: u32) -> Vec<u8> {
    let mut ev = vec![0u8; EVENT_SIZE];
    ev[0..4].copy_from_slice(&kind.to_ne_bytes());
    ev
}

/// Copy a string into a fixed, NUL-terminated field.
fn put_str(field: &mut [u8], s: &str) {
    let n = s.len().min(field.len() - 1);
    field[..n].copy_from_slice(&s.as_bytes()[..n]);
}

/// The evdev node whose input device has this name and `attr` (e.g. phys).
pub(crate) fn find_evdev(attr: &str, value: &str, name: &str) -> Option<String> {
    let entries = std::fs::read_dir("/sys/class/input").ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(node) = file_name.to_str().filter(|n| n.starts_with("event")) else { continue };
        let dev = entry.path().join("device");
        let read = |f: &str| std::fs::read_to_string(dev.join(f)).map(|s| s.trim().to_owned());
        if read(attr).ok().as_deref() == Some(value) && read("name").ok().as_deref() == Some(name) {
            return Some(format!("/dev/input/{node}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_reports_have_the_driver_sizes() {
        // hid-playstation requires input report 1 to be exactly 64 bytes.
        assert_eq!(input_report(&GamepadState::default(), 0, 0).len(), 64);
        assert_eq!(feature_report(0, FEATURE_CALIBRATION).unwrap().len(), 41);
        assert_eq!(feature_report(0, FEATURE_PAIRING_INFO).unwrap().len(), 20);
        assert_eq!(feature_report(0, FEATURE_FIRMWARE_INFO).unwrap().len(), 64);
        assert!(feature_report(0, 0x42).is_none());
    }

    #[test]
    fn neutral_report() {
        let r = input_report(&GamepadState::default(), 7, 0);
        assert_eq!(&r[1..5], &[128, 128, 128, 128]); // sticks centred
        assert_eq!(r[7], 7);
        assert_eq!(r[8], 8); // hat centred, no face buttons
        assert_eq!(r[9], 0);
        assert_eq!(r[10], 0);
    }

    #[test]
    fn buttons_map_to_dualsense_bits() {
        let state = GamepadState {
            buttons: (1 << 0) | (1 << 3) | (1 << 12) | (1 << 15) | (1 << 9) | (1 << 16) | (1 << 17),
            axes: [-32768, 32767, 0, 0],
            ..Default::default()
        };
        let r = input_report(&state, 0, 0);
        assert_eq!(r[1], 0); // left stick fully left
        assert_eq!(r[2], 255); // and down
        assert_eq!(r[8] & 0x0f, 1); // up-right
        assert_eq!(r[8] & 0xf0, 0x20 | 0x80); // cross, triangle
        assert_eq!(r[9], 0x20); // options
        assert_eq!(r[10], 0x03); // PS, touchpad
    }

    #[test]
    fn rumble_from_output_reports() {
        let mut report = vec![0u8; 48];
        report[0] = OUTPUT_REPORT_ID;
        report[1] = 0x03;
        report[3] = 0x40; // right (weak)
        report[4] = 0xff; // left (strong)
        assert_eq!(output_report_motors(&report), Some((0x40, 0xff)));
        let r = motors_to_rumble((0x40, 0xff));
        assert_eq!((r.strong, r.weak), (0xffff, 0x4040));

        // An LED-only update doesn't touch the motors...
        report[1] = 0;
        report[2] = 0x14;
        assert_eq!(output_report_motors(&report), None);
        // ...but a report with no flags at all is SDL's "stop".
        report[2] = 0;
        assert_eq!(output_report_motors(&report), Some((0, 0)));
        // Nor does another report ID.
        report[0] = 0x31;
        report[1] = 0x03;
        assert_eq!(output_report_motors(&report), None);
    }

    /// Needs a writable /dev/uhid: `cargo test -- --ignored create_dualsense`.
    /// VEIL_DUALSENSE_HOLD=<secs> keeps the device plugged for probing.
    #[test]
    #[ignore]
    fn create_dualsense() {
        let pad = UhidDualSense::new(0xfe).expect("creating the virtual DualSense");
        let hold: u64 = std::env::var("VEIL_DUALSENSE_HOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
        let deadline = Instant::now() + Duration::from_secs(hold.max(2));
        let mut path = None;
        while Instant::now() < deadline {
            for rumble in pad.poll_rumble() {
                println!("rumble {rumble:?}");
            }
            if path.is_none() {
                path = pad.evdev_path();
                if let Some(p) = &path {
                    println!("hid-playstation created {p}");
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(path.is_some(), "hid-playstation didn't bind (see `journalctl -k`)");
    }

    #[test]
    fn hat_directions() {
        assert_eq!(hat(false, false, false, false), 8);
        assert_eq!(hat(true, false, false, false), 0);
        assert_eq!(hat(false, true, false, false), 4);
        assert_eq!(hat(false, false, true, true), 8);
        assert_eq!(hat(true, false, true, false), 7);
    }
}
