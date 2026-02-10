use std::time::{Instant, Duration};
use std::collections::HashMap;

#[derive(Debug)]
pub struct APMTracker {
    actions: Vec<f64>,
    recording_start: Option<Instant>,
    last_key_time: HashMap<u32, f64>, // vk -> last time (s)
    last_mouse_button_time: Option<f64>,

    debounce_ms: f64,
    mouse_button_debounce_ms: f64,
}

impl APMTracker {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            recording_start: None,
            last_key_time: HashMap::new(),
            last_mouse_button_time: None,
            debounce_ms: 40.0,
            mouse_button_debounce_ms: 30.0,
        }
    }

    pub fn start_recording(&mut self) {
        self.recording_start = Some(Instant::now());
        self.actions.clear();
        self.last_key_time.clear();
        self.last_mouse_button_time = None;
    }

    pub fn stop_recording(&mut self) {
        self.recording_start = None;
    }

    fn now_secs(&self) -> Option<f64> {
        self.recording_start.map(|start| Instant::now().duration_since(start).as_secs_f64())
    }

    /// Record a key down event; vk = virtual key code
    pub fn record_key(&mut self, vk: u32) {
        let t = match self.now_secs() { Some(t) => t, None => return };
        let last = self.last_key_time.get(&vk).copied().unwrap_or(-9999.0);
        if t - last < (self.debounce_ms / 1000.0) {
            // considered a repeat/hold -> ignore
            return;
        }
        self.last_key_time.insert(vk, t);
        self.actions.push(t);
    }

    /// Record a mouse button down
    pub fn record_mouse_button(&mut self) {
        let t = match self.now_secs() { Some(t) => t, None => return };
        if let Some(last) = self.last_mouse_button_time {
            if t - last < (self.mouse_button_debounce_ms / 1000.0) {
                return;
            }
        }
        self.last_mouse_button_time = Some(t);
        self.actions.push(t);
    }

    pub fn record_mouse_wheel(&mut self, delta: i32) {
        let t = match self.now_secs() { Some(t) => t, None => return };
        let notches = (delta / 120).abs(); // integer notches
        let count = if notches == 0 { 1 } else { notches as usize };
        for _ in 0..count {
            self.actions.push(t);
        }
    }

    /// Compute per-second APM series.
    pub fn compute_apm_series(&self, window_secs: f64, step_secs: f64, normalize_using_fixed_window: bool) -> Vec<(f64, u32)> {
        if self.actions.is_empty() || window_secs <= 0.0 || step_secs <= 0.0 {
            return vec![];
        }

        // actions already in increasing order if recorded sequentially
        let actions = &self.actions;
        let total_duration = *actions.last().unwrap();
        let last_second = (total_duration).ceil() as i64;

        let mut series = Vec::with_capacity((last_second as f64 / step_secs) as usize + 2);

        for k in 0..=last_second {
            let sample_t = (k as f64) * step_secs;
            // trailing window: (sample_t - window_secs, sample_t]
            let window_start = if sample_t > window_secs { sample_t - window_secs } else { 0.0 };
            let window_end = sample_t;

            // indices via partition_point
            let start_idx = actions.partition_point(|&x| x < window_start);
            let end_idx = actions.partition_point(|&x| x <= window_end);
            let count = end_idx.saturating_sub(start_idx);

            let actual_window = if normalize_using_fixed_window {
                window_secs
            } else {
                // if we're before window_secs elapsed, use smaller window to reflect real rate
                let w = window_end - window_start;
                if w <= 0.0 { 1e-9 } else { w }
            };

            let apm = if actual_window > 0.0 {
                ((count as f64 / actual_window) * 60.0).round() as u32
            } else {
                0u32
            };

            series.push((sample_t, apm));
        }

        series
    }
}