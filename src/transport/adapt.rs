//! Fitting the stream to its viewers.
//!
//! There's one encoder for everyone, so it follows the most constrained
//! viewer: the bitrate is capped at the lowest network budget any viewer's
//! session has estimated (see [`NetEstimator`]), and in Auto mode the codec
//! is the one every viewer decodes best, preferring hardware decoding.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tracing::info;

use crate::codec::VideoCodec;
use crate::session::control::{ControlMessage, StreamInfo, StreamParamsRequest};

// ─── Decode support (reported by each browser) ───────────────────────────────

/// How well a browser can decode a codec at the stream's size.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Support {
    #[default]
    None,
    Software,
    Hardware,
}

/// A browser's decode support, sent as datagram 0x32 (JSON).
#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DecodeCaps {
    pub h264: Support,
    pub h265: Support,
    pub av1: Support,
    /// Codecs the browser's decoder couldn't keep up with.
    pub struggling: Vec<VideoCodec>,
}

impl DecodeCaps {
    fn support(&self, codec: VideoCodec) -> Support {
        if self.struggling.contains(&codec) {
            return Support::None;
        }
        match codec {
            VideoCodec::H264 => self.h264,
            VideoCodec::H265 => self.h265,
            VideoCodec::Av1 => self.av1,
        }
    }
}

/// The codec to use, given what the GPU encodes and every viewer's support:
/// the best support all viewers share, then (with hardware decoding) the most
/// efficient codec, or (in software) the cheapest to decode.
pub fn choose_codec<'a>(
    gpu_codecs: &[VideoCodec],
    viewers: impl Iterator<Item = &'a DecodeCaps> + Clone,
) -> Option<(VideoCodec, Support)> {
    gpu_codecs
        .iter()
        .map(|&codec| {
            let support = viewers.clone().map(|caps| caps.support(codec)).min().unwrap_or(Support::None);
            (codec, support)
        })
        .filter(|(_, support)| *support > Support::None)
        .max_by_key(|&(codec, support)| {
            let preference = match (support, codec) {
                // Hardware: compression wins. AV1 is experimental, so H.265 first.
                (Support::Hardware, VideoCodec::H265) => 3,
                (Support::Hardware, VideoCodec::Av1) => 2,
                (Support::Hardware, VideoCodec::H264) => 1,
                // Software: decode cost wins.
                (_, VideoCodec::H264) => 3,
                (_, VideoCodec::H265) => 2,
                (_, VideoCodec::Av1) => 1,
            };
            (support, preference)
        })
}

// ─── Shared state ────────────────────────────────────────────────────────────

/// Mirrors the Auto setting, so a GPU switch (which re-runs Veil) keeps it.
static AUTO_CODEC: AtomicBool = AtomicBool::new(false);

pub fn auto_codec_enabled() -> bool {
    AUTO_CODEC.load(Ordering::Relaxed)
}

pub struct Adaptation {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    auto_codec: bool,
    viewers: BTreeMap<u32, ViewerState>,
    /// What the GPU encodes and what it's encoding, from the latest
    /// StreamInfo.
    gpu_codecs: Vec<VideoCodec>,
    current_codec: Option<VideoCodec>,
    /// The codec last asked for, so a change is only requested once.
    requested_codec: Option<VideoCodec>,
    /// Why the current codec was picked, for display.
    codec_reason: Option<String>,
    /// The bitrate limit last sent to the reactor.
    applied_limit: Option<Option<u64>>,
}

#[derive(Default)]
struct ViewerState {
    budget: Option<u64>,
    decode: Option<DecodeCaps>,
}

/// What a viewer is told about adaptation (MSG 0x1C, with its own network
/// numbers added by the session).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdaptationStatus {
    pub auto_codec: bool,
    pub codec_reason: Option<String>,
    /// The bitrate cap across all viewers, if any.
    pub limit_bps: Option<u64>,
}

impl Adaptation {
    pub fn new(auto_codec: bool) -> Self {
        AUTO_CODEC.store(auto_codec, Ordering::Relaxed);
        Self { inner: Mutex::new(Inner { auto_codec, ..Default::default() }) }
    }

    pub fn join(&self, id: u32) {
        self.inner.lock().unwrap().viewers.insert(id, ViewerState::default());
    }

    /// A viewer left; its budget and decode support no longer count.
    pub fn leave(&self, id: u32) -> Vec<ControlMessage> {
        let mut inner = self.inner.lock().unwrap();
        inner.viewers.remove(&id);
        let mut out = inner.limit_messages();
        out.extend(inner.codec_messages());
        out
    }

    /// A viewer's session estimated a new network budget (None: unlimited).
    pub fn set_budget(&self, id: u32, budget: Option<u64>) -> Vec<ControlMessage> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(v) = inner.viewers.get_mut(&id) {
            v.budget = budget;
        }
        inner.limit_messages()
    }

    /// A viewer reported what its browser decodes.
    pub fn set_decode(&self, id: u32, caps: DecodeCaps) -> Vec<ControlMessage> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(v) = inner.viewers.get_mut(&id) {
            v.decode = Some(caps);
        }
        inner.codec_messages()
    }

    pub fn set_auto_codec(&self, auto: bool) -> Vec<ControlMessage> {
        let mut inner = self.inner.lock().unwrap();
        inner.auto_codec = auto;
        AUTO_CODEC.store(auto, Ordering::Relaxed);
        if !auto {
            inner.codec_reason = None;
        }
        inner.codec_messages()
    }

    /// The pipeline was (re)built.
    pub fn stream_info(&self, info: &StreamInfo) -> Vec<ControlMessage> {
        let mut inner = self.inner.lock().unwrap();
        inner.gpu_codecs = info.codecs.clone();
        inner.current_codec = Some(info.codec);
        if inner.requested_codec == Some(info.codec) {
            inner.requested_codec = None;
        }
        inner.codec_messages()
    }

    pub fn status(&self) -> AdaptationStatus {
        let inner = self.inner.lock().unwrap();
        AdaptationStatus {
            auto_codec: inner.auto_codec,
            codec_reason: inner.codec_reason.clone(),
            limit_bps: inner.limit(),
        }
    }
}

impl Inner {
    fn limit(&self) -> Option<u64> {
        self.viewers.values().filter_map(|v| v.budget).min()
    }

    fn limit_messages(&mut self) -> Vec<ControlMessage> {
        let limit = self.limit();
        if self.applied_limit == Some(limit) {
            return Vec::new();
        }
        self.applied_limit = Some(limit);
        vec![ControlMessage::SetBitrateLimit(limit)]
    }

    fn codec_messages(&mut self) -> Vec<ControlMessage> {
        if !self.auto_codec || self.gpu_codecs.is_empty() {
            return Vec::new();
        }
        let reporting = self.viewers.values().filter_map(|v| v.decode.as_ref());
        if reporting.clone().next().is_none() {
            return Vec::new();
        }
        let Some((codec, support)) = choose_codec(&self.gpu_codecs, reporting) else {
            return Vec::new();
        };
        let name = codec_name(codec);
        let how = match support {
            Support::Hardware => "hardware",
            _ => "software",
        };
        let viewers = self.viewers.len();
        self.codec_reason = Some(if viewers == 1 {
            format!("{name}: decoded in {how} by your browser")
        } else {
            format!("{name}: decoded in {how} by all {viewers} viewers")
        });

        if Some(codec) == self.current_codec || Some(codec) == self.requested_codec {
            return Vec::new();
        }
        info!(?codec, ?support, "auto codec: switching");
        self.requested_codec = Some(codec);
        vec![ControlMessage::SetStreamParams(StreamParamsRequest {
            codec: Some(codec),
            ..Default::default()
        })]
    }
}

fn codec_name(codec: VideoCodec) -> &'static str {
    match codec {
        VideoCodec::H264 => "H.264",
        VideoCodec::H265 => "H.265",
        VideoCodec::Av1 => "AV1",
    }
}

// ─── Per-viewer network estimate ─────────────────────────────────────────────

/// Delivered video frames since the last tick: the time from writing each
/// frame to the viewer acknowledging all of it.
#[derive(Debug, Default)]
pub struct DeliveryWindow {
    pub frames: u32,
    pub delivery_ms_sum: f64,
    pub bytes: u64,
}

/// The numbers one tick of [`NetEstimator`] works from.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetSample {
    pub interval: Duration,
    /// Frames delivered, their average delivery time, and their size.
    pub frames: u32,
    pub delivery_ms: f64,
    pub bytes: u64,
    /// QUIC's minimum and current round trip.
    pub min_rtt_ms: f64,
    pub rtt_ms: f64,
    /// Packets sent and lost since the last tick.
    pub sent_packets: u64,
    pub lost_packets: u64,
    /// Video events this viewer fell too far behind to receive.
    pub skipped: u64,
    /// The encoder's target and the preset's (None outside VBR).
    pub target_bps: Option<u64>,
    pub preset_bps: Option<u64>,
}

/// Queueing (delivery time beyond the path's round trip) that counts as
/// congestion.
const QUEUE_LIMIT_MS: f64 = 40.0;
/// Packet loss that counts as congestion.
const LOSS_LIMIT: f64 = 0.05;
/// The lowest budget we'll set.
const MIN_BUDGET: u64 = 500_000;
/// After cutting, wait this long before probing upwards again.
const HOLD_AFTER_CUT: Duration = Duration::from_secs(3);
/// Don't cut again this soon: the queue a cut is meant to drain takes a
/// moment to, and counting it twice overshoots.
const CUT_COOLDOWN: Duration = Duration::from_secs(2);

/// Estimates how much video one viewer's network carries: AIMD on the
/// bitrate, cutting when frames queue up (delivery takes much longer than a
/// round trip), packets are lost, or the viewer falls behind, and growing
/// back by 15% a second once it's been clear for a while.
#[derive(Debug, Default)]
pub struct NetEstimator {
    pub budget: Option<u64>,
    pub congested: bool,
    pub queue_ms: f64,
    pub loss: f64,
    pub delivered_bps: u64,
    hold_until: Option<Instant>,
    last_cut: Option<Instant>,
}

impl NetEstimator {
    /// Update the estimate from one tick's sample. Returns the new budget.
    pub fn tick(&mut self, s: &NetSample, now: Instant) -> Option<u64> {
        let secs = s.interval.as_secs_f64().max(0.001);
        self.delivered_bps = (s.bytes as f64 * 8.0 / secs) as u64;
        self.queue_ms = if s.frames > 0 { (s.delivery_ms - s.min_rtt_ms).max(0.0) } else { 0.0 };
        self.loss = if s.sent_packets >= 100 { s.lost_packets as f64 / s.sent_packets as f64 } else { 0.0 };

        // Without a VBR target (constant QP), there's no bitrate to adapt.
        let Some(target) = s.target_bps else {
            self.budget = None;
            self.congested = false;
            return None;
        };

        self.congested = s.skipped > 0
            || (s.frames >= 5 && self.queue_ms > QUEUE_LIMIT_MS)
            || self.loss > LOSS_LIMIT;

        if self.congested {
            let cooling = self.last_cut.is_some_and(|t| now < t + CUT_COOLDOWN);
            if !cooling {
                // Cut to 70%, or below what actually got through.
                let current = self.budget.unwrap_or(target).min(target);
                let mut cut = current * 7 / 10;
                if self.delivered_bps > 0 {
                    cut = cut.min(self.delivered_bps * 9 / 10);
                }
                self.budget = Some(cut.max(MIN_BUDGET));
                self.last_cut = Some(now);
            }
            self.hold_until = Some(now + HOLD_AFTER_CUT);
        } else {
            let holding = self.hold_until.is_some_and(|t| now < t);
            if let Some(budget) = self.budget {
                if !holding {
                    let grown = budget + budget / 7; // ~+15%
                    // Back at the preset's rate: no limit at all.
                    self.budget = match s.preset_bps {
                        Some(preset) if grown >= preset => None,
                        _ => Some(grown),
                    };
                }
            }
        }
        self.budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(h264: Support, h265: Support, av1: Support) -> DecodeCaps {
        DecodeCaps { h264, h265, av1, struggling: Vec::new() }
    }

    const ALL: [VideoCodec; 3] = [VideoCodec::H264, VideoCodec::H265, VideoCodec::Av1];

    #[test]
    fn prefers_hardware_then_efficiency() {
        use Support::*;
        // A Mac: hardware H.264/H.265, software AV1.
        let mac = caps(Hardware, Hardware, Software);
        assert_eq!(choose_codec(&ALL, [&mac].into_iter()), Some((VideoCodec::H265, Hardware)));
        // Everything in hardware: H.265 still beats experimental AV1.
        let all_hw = caps(Hardware, Hardware, Hardware);
        assert_eq!(choose_codec(&ALL, [&all_hw].into_iter()).unwrap().0, VideoCodec::H265);
        // A GPU without H.265 encode.
        assert_eq!(choose_codec(&[VideoCodec::H264, VideoCodec::Av1], [&all_hw].into_iter()).unwrap().0, VideoCodec::Av1);
    }

    #[test]
    fn software_only_picks_the_cheapest_decode() {
        use Support::*;
        let sw = caps(Software, Software, Software);
        assert_eq!(choose_codec(&ALL, [&sw].into_iter()), Some((VideoCodec::H264, Software)));
    }

    #[test]
    fn every_viewer_must_decode_it() {
        use Support::*;
        let a = caps(Hardware, Hardware, Hardware);
        let b = caps(Hardware, None, Hardware); // e.g. Firefox without H.265
        assert_eq!(choose_codec(&ALL, [&a, &b].into_iter()).unwrap().0, VideoCodec::Av1);
        let mut struggling = a.clone();
        struggling.struggling = vec![VideoCodec::H265];
        assert_eq!(choose_codec(&ALL, [&struggling].into_iter()).unwrap().0, VideoCodec::Av1);
    }

    fn sample(delivery_ms: f64, skipped: u64) -> NetSample {
        NetSample {
            interval: Duration::from_secs(1),
            frames: 60,
            delivery_ms,
            bytes: 1_000_000, // 8 Mbit/s delivered
            min_rtt_ms: 2.0,
            rtt_ms: 2.0,
            sent_packets: 1000,
            lost_packets: 0,
            skipped,
            target_bps: Some(20_000_000),
            preset_bps: Some(20_000_000),
        }
    }

    #[test]
    fn cuts_on_queueing_then_recovers() {
        let mut est = NetEstimator::default();
        let t0 = Instant::now();
        assert_eq!(est.tick(&sample(5.0, 0), t0), None); // clear: no limit

        // Frames take 200 ms to deliver: cut below what got through.
        let budget = est.tick(&sample(200.0, 0), t0).unwrap();
        assert!(est.congested);
        assert_eq!(budget, 7_200_000); // min(70% of 20M, 90% of 8M)

        // Clear again, but held for a few seconds after the cut.
        assert_eq!(est.tick(&sample(5.0, 0), t0 + Duration::from_secs(1)), Some(budget));
        assert_eq!(est.tick(&sample(5.0, 0), t0 + Duration::from_secs(2)), Some(budget));
        // Then it grows on each clear tick.
        est.tick(&sample(5.0, 0), t0 + Duration::from_secs(4));
        let grown = est.budget.unwrap();
        assert!(grown > budget);
        est.tick(&sample(5.0, 0), t0 + Duration::from_secs(5));
        assert!(est.budget.unwrap() > grown);
    }

    #[test]
    fn one_backlog_is_one_cut() {
        let mut est = NetEstimator::default();
        let t0 = Instant::now();
        let first = est.tick(&sample(500.0, 0), t0).unwrap();
        // The same queue, still draining a second later: no second cut.
        assert_eq!(est.tick(&sample(800.0, 2), t0 + Duration::from_secs(1)), Some(first));
        // Still congested after the cooldown: cut again.
        assert!(est.tick(&sample(300.0, 0), t0 + Duration::from_millis(2500)).unwrap() < first);
    }

    #[test]
    fn skipped_frames_count_as_congestion() {
        let mut est = NetEstimator::default();
        assert!(est.tick(&sample(3.0, 4), Instant::now()).is_some());
        assert!(est.congested);
    }

    #[test]
    fn grows_back_to_unlimited() {
        let mut est = NetEstimator { budget: Some(19_000_000), ..Default::default() };
        assert_eq!(est.tick(&sample(3.0, 0), Instant::now()), None);
    }

    #[test]
    fn constant_qp_is_never_limited() {
        let mut est = NetEstimator::default();
        let mut s = sample(500.0, 3);
        s.target_bps = None;
        assert_eq!(est.tick(&s, Instant::now()), None);
    }

    #[test]
    fn budget_is_the_slowest_viewer() {
        let adapt = Adaptation::new(false);
        adapt.join(1);
        adapt.join(2);
        let msgs = adapt.set_budget(1, Some(5_000_000));
        assert!(matches!(msgs[..], [ControlMessage::SetBitrateLimit(Some(5_000_000))]));
        let msgs = adapt.set_budget(2, Some(3_000_000));
        assert!(matches!(msgs[..], [ControlMessage::SetBitrateLimit(Some(3_000_000))]));
        // Unchanged: nothing to send.
        assert!(adapt.set_budget(1, Some(4_000_000)).is_empty());
        // The slow viewer leaves.
        let msgs = adapt.leave(2);
        assert!(matches!(msgs[..], [ControlMessage::SetBitrateLimit(Some(4_000_000))]));
    }
}
