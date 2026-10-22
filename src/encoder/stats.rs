use std::{
    sync::{Arc, Mutex},
    time,
};

#[derive(Default, Clone)]
pub struct EncodeStats {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    start: time::Instant,
    stream_stats: LayerStats,
    keyframe_stats: LayerStats,
    layer_stats: Vec<LayerStats>,
    window: Window,
}

/// Counters since the last [`EncodeStats::take_window`].
struct Window {
    start: time::Instant,
    frames: u64,
    keyframes: u64,
    bytes: u64,
    encode_total: time::Duration,
    encode_max: time::Duration,
}

impl Window {
    fn new() -> Self {
        Self {
            start: time::Instant::now(),
            frames: 0,
            keyframes: 0,
            bytes: 0,
            encode_total: time::Duration::ZERO,
            encode_max: time::Duration::ZERO,
        }
    }
}

impl Default for Inner {
    fn default() -> Self {
        let start = time::Instant::now();

        Self {
            start,
            stream_stats: LayerStats::new(start),
            keyframe_stats: LayerStats::new(start),
            layer_stats: Vec::new(),
            window: Window::new(),
        }
    }
}

struct LayerStats {
    start: time::Instant,
    min: usize,
    max: usize,
    total: u64,
}

impl LayerStats {
    fn new(start: time::Instant) -> Self {
        Self {
            start,
            min: 0,
            max: 0,
            total: 0,
        }
    }

    fn record_frame_size(&mut self, len: usize) {
        self.total += len as u64;

        if self.min == 0 || len < self.min {
            self.min = len;
        }

        if len > self.max {
            self.max = len;
        }
    }
}

impl std::fmt::Debug for LayerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let period = self.start.elapsed();

        let mut f = f.debug_struct("EncodeStats");

        f.field("frame_min", &self.min);
        f.field("frame_max", &self.max);
        f.field("rate", &calculate_rate(period, self.total));

        f.finish()
    }
}

impl EncodeStats {
    pub fn record_frame_size(&self, is_keyframe: bool, layer: u32, len: usize) {
        let mut inner = self.inner.lock().unwrap();

        inner.stream_stats.record_frame_size(len);
        inner.window.frames += 1;
        inner.window.bytes += len as u64;
        if is_keyframe {
            inner.window.keyframes += 1;
        }
        if is_keyframe {
            inner.keyframe_stats.record_frame_size(len);
        } else {
            let layer = layer as usize;
            let layers = (layer + 1).max(inner.layer_stats.len());

            let start = inner.start;
            inner
                .layer_stats
                .resize_with(layers, || LayerStats::new(start));

            inner.layer_stats[layer].record_frame_size(len);
        }
    }
}

impl EncodeStats {
    /// Record how long a frame took from submission to its bitstream being
    /// ready.
    pub fn record_encode_time(&self, dur: time::Duration) {
        let mut inner = self.inner.lock().unwrap();
        inner.window.encode_total += dur;
        inner.window.encode_max = inner.window.encode_max.max(dur);
    }

    /// Return the counters since the last call, and start a new window.
    pub fn take_window(&self) -> crate::session::control::EncoderStats {
        let mut inner = self.inner.lock().unwrap();
        let w = std::mem::replace(&mut inner.window, Window::new());

        let ms = |d: time::Duration| d.as_secs_f64() * 1000.0;
        crate::session::control::EncoderStats {
            window_ms: w.start.elapsed().as_millis() as u64,
            frames: w.frames,
            keyframes: w.keyframes,
            bytes: w.bytes,
            encode_ms_avg: if w.frames > 0 { ms(w.encode_total) / w.frames as f64 } else { 0.0 },
            encode_ms_max: ms(w.encode_max),
        }
    }
}

impl std::fmt::Debug for EncodeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().unwrap();

        let mut f = f.debug_struct("EncodeStats");

        f.field("duration", &inner.start.elapsed());
        f.field("totals", &inner.stream_stats);
        f.field("IDR", &inner.keyframe_stats);
        for (layer, stats) in inner.layer_stats.iter().enumerate() {
            f.field(&format!("P{layer}"), &stats);
        }

        f.finish()
    }
}

fn calculate_rate(dur: time::Duration, total: u64) -> f32 {
    // Total is in bytes, we want mbit/s.
    let total_mbits = total as f32 / (1024.0 * 1024.0) * 8.0;
    total_mbits / dur.as_secs_f32()
}
