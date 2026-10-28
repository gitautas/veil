//! HID passthrough: devices a viewer opened in the browser with WebHID,
//! recreated on the server with uhid, so drivers and games see the real
//! device (e.g. a DualSense with its touchpad, motion sensors and lightbar).
//!
//! The browser sends the device's report descriptor (rebuilt from what WebHID
//! exposes) and its input reports; the server sends back output reports and
//! the kernel's feature report requests for the browser to perform on the
//! real device. A viewer's devices are unplugged when it leaves.

use std::collections::BTreeMap;

use anyhow::{Context as _, bail};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::session::uhid;

/// How many devices one viewer may pass through.
const MAX_DEVICES: usize = 8;

/// A viewer opened a device (stream 0x40, JSON).
#[derive(Debug, serde::Deserialize)]
pub struct OpenRequest {
    /// The browser's number for the device, used in every later message.
    pub handle: u8,
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    /// "usb" (default) or "bluetooth": drivers parse some devices' reports
    /// differently over Bluetooth (e.g. Sony pads).
    #[serde(default)]
    pub bus: Option<String>,
    pub descriptor: Vec<u8>,
}

/// What the browser should do on the real device (stream 0x1D).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToClient {
    /// Send an output report (rumble, LEDs, ...).
    Output { handle: u8, data: Vec<u8> },
    /// Read a feature report and reply with it.
    GetFeature { handle: u8, request: u32, report_number: u8 },
    /// Write a feature (`feature: true`) or output report, and reply.
    SetReport { handle: u8, request: u32, report_number: u8, feature: bool, data: Vec<u8> },
}

/// The result of opening a device, for the viewer (MSG 0x1E, JSON).
#[derive(Debug, serde::Serialize)]
pub struct Status {
    pub handle: u8,
    pub ok: bool,
    pub error: Option<String>,
}

/// One viewer's passed-through devices.
pub struct Passthrough {
    viewer: u32,
    devices: BTreeMap<u8, uhid::Device>,
    to_client: mpsc::UnboundedSender<ToClient>,
}

// uhid report types.
const FEATURE_REPORT: u8 = 0;
const OUTPUT_REPORT: u8 = 1;

impl Passthrough {
    pub fn new(viewer: u32, to_client: mpsc::UnboundedSender<ToClient>) -> Self {
        Self { viewer, devices: BTreeMap::new(), to_client }
    }

    pub fn open(&mut self, req: OpenRequest) -> Status {
        let handle = req.handle;
        let result = self.try_open(req);
        if let Err(e) = &result {
            warn!(viewer = self.viewer, handle, "HID passthrough refused: {e:#}");
        }
        Status { handle, ok: result.is_ok(), error: result.err().map(|e| format!("{e:#}")) }
    }

    fn try_open(&mut self, req: OpenRequest) -> anyhow::Result<()> {
        // Replacing a handle is fine; adding beyond the limit isn't.
        self.devices.remove(&req.handle);
        if self.devices.len() >= MAX_DEVICES {
            bail!("at most {MAX_DEVICES} devices per viewer");
        }
        check_descriptor(&req.descriptor)?;

        let bus = match req.bus.as_deref() {
            Some("bluetooth") => uhid::BUS_BLUETOOTH,
            _ => uhid::BUS_USB,
        };
        let handle = req.handle;
        let viewer = self.viewer;
        let to_client = self.to_client.clone();
        let device = uhid::Device::new(
            &uhid::DeviceInfo {
                name: &req.name,
                phys: &format!("veil/hid{viewer}.{handle}"),
                uniq: "",
                bus,
                vendor: req.vendor as u32,
                product: req.product as u32,
                version: 0,
                report_descriptor: &req.descriptor,
            },
            format!("hid {viewer}.{handle}"),
            None,
            move |writer, event| match event {
                uhid::Event::Output { data, .. } => {
                    let _ = to_client.send(ToClient::Output { handle, data });
                }
                uhid::Event::GetReport { id, report_number, report_type } => {
                    // WebHID can read feature reports, not input or output ones.
                    if report_type != FEATURE_REPORT
                        || to_client.send(ToClient::GetFeature { handle, request: id, report_number }).is_err()
                    {
                        let _ = writer.get_report_reply(id, Err(libc::EIO as u16));
                    }
                }
                uhid::Event::SetReport { id, report_number, report_type, data } => {
                    let msg = ToClient::SetReport {
                        handle,
                        request: id,
                        report_number,
                        feature: report_type != OUTPUT_REPORT,
                        data,
                    };
                    if to_client.send(msg).is_err() {
                        let _ = writer.set_report_reply(id, libc::EIO as u16);
                    }
                }
                uhid::Event::Start | uhid::Event::Stop | uhid::Event::Tick => {}
            },
        )
        .context("creating the uhid device")?;

        info!(viewer, handle, name = req.name, vendor = req.vendor, product = req.product, "HID device passed through");
        self.devices.insert(handle, device);
        Ok(())
    }

    pub fn close(&mut self, handle: u8) {
        if self.devices.remove(&handle).is_some() {
            info!(viewer = self.viewer, handle, "HID passthrough closed");
        }
    }

    /// An input report from the real device.
    pub fn input(&self, handle: u8, report: &[u8]) {
        if let Some(device) = self.devices.get(&handle) {
            if let Err(e) = device.writer().input(report) {
                debug!(handle, "HID input write failed: {e}");
            }
        }
    }

    /// The browser's answer to a [`ToClient::GetFeature`] or
    /// [`ToClient::SetReport`]: `status` 0 for success (with `data`, for a
    /// get), otherwise an errno.
    pub fn reply(&self, handle: u8, request: u32, is_get: bool, status: u8, data: &[u8]) {
        let Some(device) = self.devices.get(&handle) else { return };
        let writer = device.writer();
        let result = if is_get {
            writer.get_report_reply(request, if status == 0 { Ok(data) } else { Err(status as u16) })
        } else {
            writer.set_report_reply(request, status as u16)
        };
        if let Err(e) = result {
            debug!(handle, "HID reply failed: {e}");
        }
    }
}

/// Refuse devices that could take over the host: uhid devices are seen by the
/// whole system, not just Veil's compositor, so a keyboard or mouse would
/// type into and click on the server's own desktop. These are also the kinds
/// of device browsers keep from WebHID, so a normal client never sends them.
fn check_descriptor(descriptor: &[u8]) -> anyhow::Result<()> {
    if descriptor.is_empty() || descriptor.len() > uhid::UHID_DATA_MAX {
        bail!("report descriptor must be 1-{} bytes", uhid::UHID_DATA_MAX);
    }
    for (page, usage) in top_level_usages(descriptor)? {
        let protected = match page {
            // Generic Desktop: pointer, mouse, keyboard, keypad, system controls.
            0x01 => matches!(usage, 0x01 | 0x02 | 0x06 | 0x07 | 0x80..=0x8f),
            // Keyboard/keypad, consumer controls (media keys), digitizers.
            0x07 | 0x0c | 0x0d => true,
            _ => false,
        };
        if protected {
            bail!("keyboards, mice and similar devices can't be passed through (usage page {page:#x}, usage {usage:#x})");
        }
    }
    Ok(())
}

/// The (usage page, usage) of each top-level collection, and of any usage
/// page a main item uses directly with keyboard usages.
fn top_level_usages(descriptor: &[u8]) -> anyhow::Result<Vec<(u32, u32)>> {
    let mut out = Vec::new();
    let mut usage_page = 0u32;
    let mut last_usage: Option<u32> = None;
    let mut depth = 0u32;
    let mut i = 0;
    while i < descriptor.len() {
        let prefix = descriptor[i];
        if prefix == 0xfe {
            // Long item: [0xfe][size][tag][data].
            let size = *descriptor.get(i + 1).context("truncated long item")? as usize;
            i += 3 + size;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let data = descriptor.get(i + 1..i + 1 + size).context("truncated report descriptor")?;
        let value = data.iter().rev().fold(0u32, |acc, b| (acc << 8) | *b as u32);
        match prefix & 0xfc {
            0x04 => usage_page = value, // Usage Page (global)
            0x08 => last_usage = Some(value), // Usage (local)
            0xa0 => {
                // Collection
                if depth == 0 {
                    let usage = last_usage.unwrap_or(0);
                    // A 4-byte usage carries its own page.
                    let (page, usage) = if size == 4 { (usage >> 16, usage & 0xffff) } else { (usage_page, usage) };
                    out.push((page, usage));
                }
                depth += 1;
                last_usage = None;
            }
            0xc0 => depth = depth.saturating_sub(1), // End Collection
            0x80 | 0x90 | 0xb0 => {
                // Input, Output, Feature: anything on the keyboard page counts.
                if usage_page == 0x07 {
                    out.push((0x07, 0));
                }
                last_usage = None;
            }
            _ => {}
        }
        i += 1 + size;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamepads_pass_keyboards_and_mice_dont() {
        // Generic Desktop / Game Pad.
        let gamepad = [0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x09, 0x30, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0xc0];
        assert!(check_descriptor(&gamepad).is_ok());
        // Joystick, and a vendor-defined collection.
        assert!(check_descriptor(&[0x05, 0x01, 0x09, 0x04, 0xa1, 0x01, 0xc0]).is_ok());
        assert!(check_descriptor(&[0x06, 0x00, 0xff, 0x09, 0x01, 0xa1, 0x01, 0xc0]).is_ok());

        // Keyboard, mouse, consumer control.
        assert!(check_descriptor(&[0x05, 0x01, 0x09, 0x06, 0xa1, 0x01, 0xc0]).is_err());
        assert!(check_descriptor(&[0x05, 0x01, 0x09, 0x02, 0xa1, 0x01, 0xc0]).is_err());
        assert!(check_descriptor(&[0x05, 0x0c, 0x09, 0x01, 0xa1, 0x01, 0xc0]).is_err());
        // A "gamepad" that sneaks in key codes.
        let sneaky = [0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x05, 0x07, 0x19, 0xe0, 0x29, 0xe7, 0x81, 0x02, 0xc0];
        assert!(check_descriptor(&sneaky).is_err());
        // A second top-level collection that's a mouse.
        let combo = [0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0xc0, 0x09, 0x02, 0xa1, 0x01, 0xc0];
        assert!(check_descriptor(&combo).is_err());
    }

    #[test]
    fn bad_descriptors_are_refused() {
        assert!(check_descriptor(&[]).is_err());
        assert!(check_descriptor(&[0x05]).is_err()); // truncated
        assert!(check_descriptor(&vec![0u8; uhid::UHID_DATA_MAX + 1]).is_err());
    }
}
