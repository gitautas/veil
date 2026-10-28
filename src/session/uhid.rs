//! Virtual HID devices through /dev/uhid.
//!
//! The kernel treats a uhid device like real hardware: HID drivers bind to it
//! (e.g. hid-playstation for a DualSense, hid-generic otherwise), and it gets
//! evdev and hidraw nodes. We write its input reports, and get its output
//! reports and feature report requests back. Each device has a thread that
//! reads those events and hands them to a handler.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use tracing::{trace, warn};

// struct uhid_event: a u32 type, then the largest member of the union
// (uhid_create2_req, 4372 bytes).
const EVENT_SIZE: usize = 4 + 4372;
/// The largest report or report descriptor uhid takes.
pub const UHID_DATA_MAX: usize = 4096;

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

pub const BUS_USB: u16 = 0x03;
pub const BUS_BLUETOOTH: u16 = 0x05;

/// What the device presents itself as.
pub struct DeviceInfo<'a> {
    pub name: &'a str,
    /// Shows up as the input nodes' `phys` (unless the driver overrides it).
    pub phys: &'a str,
    pub uniq: &'a str,
    pub bus: u16,
    pub vendor: u32,
    pub product: u32,
    pub version: u32,
    pub report_descriptor: &'a [u8],
}

/// Something the kernel (a driver, or hidraw users like Steam) did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A driver bound to (Start) or let go of (Stop) the device.
    Start,
    Stop,
    /// An output report, e.g. rumble or LEDs. `data` starts with the report
    /// ID if the descriptor uses them.
    Output { report_type: u8, data: Vec<u8> },
    /// A feature (or input) report request; answer with
    /// [`Writer::get_report_reply`] within 5 s, or the request fails.
    GetReport { id: u32, report_number: u8, report_type: u8 },
    /// Set a feature (or output) report; answer with
    /// [`Writer::set_report_reply`].
    SetReport { id: u32, report_number: u8, report_type: u8, data: Vec<u8> },
    /// No event for the tick interval (see [`Device::new`]).
    Tick,
}

/// Writes to a device: input reports and replies. Cheap to clone; usable from
/// any thread.
#[derive(Clone)]
pub struct Writer(Arc<File>);

impl Writer {
    /// Deliver an input report (starting with its report ID, if any).
    pub fn input(&self, report: &[u8]) -> std::io::Result<()> {
        if report.len() > UHID_DATA_MAX {
            return Err(std::io::Error::other("report too large"));
        }
        let mut ev = event(UHID_INPUT2);
        ev[4..6].copy_from_slice(&(report.len() as u16).to_ne_bytes());
        ev[6..6 + report.len()].copy_from_slice(report);
        (&*self.0).write_all(&ev)
    }

    /// Answer a [`Event::GetReport`] with the report (starting with its
    /// report ID), or an errno.
    pub fn get_report_reply(&self, id: u32, result: Result<&[u8], u16>) -> std::io::Result<()> {
        let mut ev = event(UHID_GET_REPORT_REPLY);
        ev[4..8].copy_from_slice(&id.to_ne_bytes());
        match result {
            Ok(data) => {
                let data = &data[..data.len().min(UHID_DATA_MAX)];
                ev[10..12].copy_from_slice(&(data.len() as u16).to_ne_bytes());
                ev[12..12 + data.len()].copy_from_slice(data);
            }
            Err(errno) => ev[8..10].copy_from_slice(&errno.to_ne_bytes()),
        }
        (&*self.0).write_all(&ev)
    }

    /// Answer a [`Event::SetReport`]: 0 for success, or an errno.
    pub fn set_report_reply(&self, id: u32, errno: u16) -> std::io::Result<()> {
        let mut ev = event(UHID_SET_REPORT_REPLY);
        ev[4..8].copy_from_slice(&id.to_ne_bytes());
        ev[8..10].copy_from_slice(&errno.to_ne_bytes());
        (&*self.0).write_all(&ev)
    }
}

/// A uhid device. Destroyed (unplugged) when dropped.
pub struct Device {
    writer: Writer,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Device {
    /// Create the device, and start a thread that passes the kernel's events
    /// to `handler`. With `tick`, the handler also gets [`Event::Tick`] when
    /// that long passes without one.
    pub fn new(
        info: &DeviceInfo,
        thread_name: String,
        tick: Option<Duration>,
        mut handler: impl FnMut(&Writer, Event) + Send + 'static,
    ) -> anyhow::Result<Self> {
        if info.report_descriptor.is_empty() || info.report_descriptor.len() > UHID_DATA_MAX {
            bail!("report descriptor must be 1-{UHID_DATA_MAX} bytes");
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/uhid")
            .context("opening /dev/uhid")?;

        let mut ev = event(UHID_CREATE2);
        put_str(&mut ev[4..132], info.name);
        put_str(&mut ev[132..196], info.phys);
        put_str(&mut ev[196..260], info.uniq);
        ev[260..262].copy_from_slice(&(info.report_descriptor.len() as u16).to_ne_bytes());
        ev[262..264].copy_from_slice(&info.bus.to_ne_bytes());
        ev[264..268].copy_from_slice(&info.vendor.to_ne_bytes());
        ev[268..272].copy_from_slice(&info.product.to_ne_bytes());
        ev[272..276].copy_from_slice(&info.version.to_ne_bytes());
        ev[280..280 + info.report_descriptor.len()].copy_from_slice(info.report_descriptor);
        (&file).write_all(&ev).context("UHID_CREATE2")?;

        let writer = Writer(Arc::new(file));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (writer, stop) = (writer.clone(), stop.clone());
            std::thread::Builder::new()
                .name(thread_name)
                .spawn(move || read_events(&writer, &stop, tick, &mut handler))?
        };
        Ok(Self { writer, stop, thread: Some(thread) })
    }

    pub fn writer(&self) -> &Writer {
        &self.writer
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Err(e) = (&*self.writer.0).write_all(&event(UHID_DESTROY)) {
            warn!("failed to destroy uhid device: {e}");
        }
    }
}

fn read_events(
    writer: &Writer,
    stop: &AtomicBool,
    tick: Option<Duration>,
    handler: &mut impl FnMut(&Writer, Event),
) {
    let file = &*writer.0;
    let mut buf = vec![0u8; EVENT_SIZE];
    // Wake regularly to notice `stop`, and for ticks.
    let wait = tick.unwrap_or(Duration::from_millis(100)).min(Duration::from_millis(100));
    let mut last_tick = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let mut pfd = libc::pollfd { fd: file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ready = unsafe { libc::poll(&mut pfd, 1, wait.as_millis() as i32) };

        if let Some(tick) = tick {
            if last_tick.elapsed() >= tick {
                handler(writer, Event::Tick);
                last_tick = Instant::now();
            }
        }
        if ready <= 0 || pfd.revents & libc::POLLIN == 0 {
            continue;
        }

        let n = match (&*file).read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                warn!("uhid read error: {e}");
                break;
            }
        };
        if n < 4 {
            continue;
        }

        let u32_at = |i: usize| u32::from_ne_bytes(buf[i..i + 4].try_into().unwrap());
        let u16_at = |i: usize| u16::from_ne_bytes(buf[i..i + 2].try_into().unwrap());
        let event = match u32_at(0) {
            UHID_START => Event::Start,
            UHID_STOP => Event::Stop,
            UHID_OPEN | UHID_CLOSE => continue,
            UHID_OUTPUT => {
                let size = (u16_at(4 + UHID_DATA_MAX) as usize).min(UHID_DATA_MAX);
                Event::Output { report_type: buf[4 + UHID_DATA_MAX + 2], data: buf[4..4 + size].to_vec() }
            }
            UHID_GET_REPORT => Event::GetReport { id: u32_at(4), report_number: buf[8], report_type: buf[9] },
            UHID_SET_REPORT => {
                let size = (u16_at(10) as usize).min(UHID_DATA_MAX);
                Event::SetReport {
                    id: u32_at(4),
                    report_number: buf[8],
                    report_type: buf[9],
                    data: buf[12..12 + size].to_vec(),
                }
            }
            other => {
                trace!(event = other, "uhid event");
                continue;
            }
        };
        handler(writer, event);
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
