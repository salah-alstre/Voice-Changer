//! Live monitoring engine: microphone -> voice chain -> monitor output.
//!
//! Threads: one WASAPI capture thread pushes mono 48 kHz samples into a lock-free SPSC ring; one
//! WASAPI render thread drains the ring, runs the DSP chain and writes the monitor buffer. The
//! render callback never allocates, blocks, logs or touches the disk. Shared state is atomics
//! plus one `try_lock`ed parameter block.
//!
//! Latency is the sum of measured/reported buffer stages (capture stream latency + ring
//! occupancy + DSP algorithmic latency + render stream latency + queued render frames). It is
//! a buffer-path measurement, not an acoustic loopback measurement, and is labelled so.

use super::devices;
use super::stream::{self, StreamHandle, StreamInfo, SAMPLE_RATE};
use crate::dsp::{VoiceChain, VoiceParams};
use crate::error::AppResult;
use parking_lot::Mutex;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

const BLOCK: usize = 2048;
const RING_CAPACITY: usize = 9600; // 200 ms
const PRIME_SAMPLES: usize = 720; // 15 ms
const MAX_FILL: usize = 2880; // 60 ms; above this we drop to DRIFT_TARGET
const DRIFT_TARGET: usize = 960;
pub const WAVE_POINTS: usize = 256;
/// Samples per waveform point (10 ms).
const WAVE_SPAN: usize = 480;

fn f2u(v: f32) -> u32 {
    v.to_bits()
}
fn u2f(v: u32) -> f32 {
    f32::from_bits(v)
}
pub fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Lock-free level meter: peak (max since last read), rms of the latest block, clip counter.
#[derive(Default)]
pub struct Meter {
    peak: AtomicU32, // positive f32 bit patterns order like integers
    rms: AtomicU32,
    clips: AtomicU32,
}

#[derive(Clone, Copy, Debug, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeterReading {
    pub peak: f32,
    pub rms: f32,
    pub clipped: bool,
    pub clip_count: u32,
}

impl Meter {
    pub fn update(&self, block: &[f32]) {
        if block.is_empty() {
            return;
        }
        let mut peak = 0.0f32;
        let mut sum = 0.0f32;
        let mut clips = 0u32;
        for &s in block {
            let a = s.abs();
            if a > peak {
                peak = a;
            }
            if a >= 0.999 {
                clips += 1;
            }
            sum += s * s;
        }
        self.peak.fetch_max(f2u(peak), Ordering::Relaxed);
        self.rms
            .store(f2u((sum / block.len() as f32).sqrt()), Ordering::Relaxed);
        if clips > 0 {
            self.clips.fetch_add(clips, Ordering::Relaxed);
        }
    }
    pub fn read(&self) -> MeterReading {
        let clip_count = self.clips.load(Ordering::Relaxed);
        MeterReading {
            peak: u2f(self.peak.swap(0, Ordering::Relaxed)),
            rms: u2f(self.rms.load(Ordering::Relaxed)),
            clipped: clip_count > 0,
            clip_count,
        }
    }
    pub fn reset_clips(&self) {
        self.clips.store(0, Ordering::Relaxed);
    }
}

struct Wave {
    points: Vec<AtomicU32>,
    head: AtomicUsize,
}

impl Wave {
    fn new() -> Self {
        Self {
            points: (0..WAVE_POINTS).map(|_| AtomicU32::new(0)).collect(),
            head: AtomicUsize::new(0),
        }
    }
    fn push(&self, v: f32) {
        let h = self.head.load(Ordering::Relaxed);
        self.points[h % WAVE_POINTS].store(f2u(v), Ordering::Relaxed);
        self.head.store(h.wrapping_add(1), Ordering::Release);
    }
    /// Oldest to newest.
    fn snapshot(&self) -> Vec<f32> {
        let h = self.head.load(Ordering::Acquire);
        (0..WAVE_POINTS)
            .map(|i| u2f(self.points[(h.wrapping_add(i)) % WAVE_POINTS].load(Ordering::Relaxed)))
            .collect()
    }
}

/// User-adjustable live controls.
#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorControls {
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub monitor_gain_db: f32,
    pub monitor_mute: bool,
    /// true = processed (A), false = raw microphone (B).
    pub processed: bool,
}

impl Default for MonitorControls {
    fn default() -> Self {
        Self {
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            monitor_gain_db: -6.0,
            monitor_mute: false,
            processed: true,
        }
    }
}

impl MonitorControls {
    pub fn sanitized(mut self) -> Self {
        let c = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 0.0 };
        self.input_gain_db = c(self.input_gain_db, -60.0, 24.0);
        self.output_gain_db = c(self.output_gain_db, -60.0, 24.0);
        self.monitor_gain_db = c(self.monitor_gain_db, -80.0, 12.0);
        self
    }
}

struct Shared {
    controls: Mutex<MonitorControls>,
    controls_version: AtomicU64,
    params: Mutex<VoiceParams>,
    params_version: AtomicU64,
    stop: Arc<AtomicBool>,
    mic_in: Meter,
    dsp_in: Meter,
    dsp_out: Meter,
    monitor_out: Meter,
    wave_in: Wave,
    wave_out: Wave,
    underruns: AtomicU32,
    overruns: AtomicU32,
    drift_drops: AtomicU32,
    ring_fill: AtomicU32,
    dsp_latency_samples: AtomicU32,
    feedback_suspected: AtomicBool,
    frames_rendered: AtomicU64,
    frames_captured: AtomicU64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyReport {
    pub capture_ms: f32,
    pub ring_ms: f32,
    pub dsp_ms: f32,
    pub render_ms: f32,
    pub total_ms: f32,
    /// Always `"buffer-path"`: sum of reported/measured buffer stages, not an acoustic measurement.
    pub method: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    pub running: bool,
    pub error: Option<String>,
    pub input_device: String,
    pub output_device: String,
    pub mic_in: MeterReading,
    pub dsp_in: MeterReading,
    pub dsp_out: MeterReading,
    pub monitor_out: MeterReading,
    pub wave_in: Vec<f32>,
    pub wave_out: Vec<f32>,
    pub latency: LatencyReport,
    pub underruns: u32,
    pub overruns: u32,
    pub drift_corrections: u32,
    pub feedback_suspected: bool,
    pub controls: MonitorControls,
    pub frames_rendered: u64,
}

pub struct MonitorEngine {
    shared: Arc<Shared>,
    capture: StreamHandle,
    render: StreamHandle,
    padding: Arc<AtomicU32>,
    input: StreamInfo,
    output: StreamInfo,
}

impl MonitorEngine {
    /// Start live monitoring. Fails synchronously with a clear error when a device cannot be opened.
    pub fn start(
        input_id: Option<String>,
        output_id: Option<String>,
        params: VoiceParams,
        controls: MonitorControls,
    ) -> AppResult<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            controls: Mutex::new(controls.sanitized()),
            controls_version: AtomicU64::new(1),
            params: Mutex::new(params.sanitized()),
            params_version: AtomicU64::new(1),
            stop: stop.clone(),
            mic_in: Meter::default(),
            dsp_in: Meter::default(),
            dsp_out: Meter::default(),
            monitor_out: Meter::default(),
            wave_in: Wave::new(),
            wave_out: Wave::new(),
            underruns: AtomicU32::new(0),
            overruns: AtomicU32::new(0),
            drift_drops: AtomicU32::new(0),
            ring_fill: AtomicU32::new(0),
            dsp_latency_samples: AtomicU32::new(0),
            feedback_suspected: AtomicBool::new(false),
            frames_rendered: AtomicU64::new(0),
            frames_captured: AtomicU64::new(0),
        });

        let (mut prod, mut cons) = HeapRb::<f32>::new(RING_CAPACITY).split();

        // Capture callback: meter + push. No allocation, no locks.
        let cap_shared = shared.clone();
        let capture = stream::start_capture(input_id, stop.clone(), move |samples| {
            cap_shared.mic_in.update(samples);
            cap_shared
                .frames_captured
                .fetch_add(samples.len() as u64, Ordering::Relaxed);
            let pushed = prod.push_slice(samples);
            if pushed < samples.len() {
                cap_shared.overruns.fetch_add(1, Ordering::Relaxed);
            }
        })?;

        // Render callback state (all preallocated).
        let r_shared = shared.clone();
        let mut chain = VoiceChain::new(SAMPLE_RATE as f32);
        let mut local_params = params.sanitized();
        let mut local_controls = controls.sanitized();
        let (mut seen_p, mut seen_c) = (1u64, 1u64);
        let mut raw = vec![0.0f32; BLOCK];
        let mut proc = vec![0.0f32; BLOCK];
        let mut primed = false;
        let mut wave_acc_in = 0.0f32;
        let mut wave_acc_out = 0.0f32;
        let mut wave_n = 0usize;
        let mut fb_blocks = 0u32;
        let mut last_rms = 0.0f32;
        let mut block_acc = 0usize;
        let mut block_sum = 0.0f32;

        let render_result = stream::start_render(output_id, stop.clone(), move |out| {
            let sh = &*r_shared;
            // Pick up control/parameter changes without ever blocking.
            let pv = sh.params_version.load(Ordering::Acquire);
            if pv != seen_p {
                if let Some(g) = sh.params.try_lock() {
                    local_params = *g;
                    seen_p = pv;
                }
            }
            let cv = sh.controls_version.load(Ordering::Acquire);
            if cv != seen_c {
                if let Some(g) = sh.controls.try_lock() {
                    local_controls = *g;
                    seen_c = cv;
                }
            }
            sh.dsp_latency_samples.store(
                VoiceChain::latency_samples(&local_params) as u32,
                Ordering::Relaxed,
            );

            let in_gain = db_to_lin(local_controls.input_gain_db);
            let out_gain = db_to_lin(local_controls.output_gain_db);
            let mon_gain = if local_controls.monitor_mute {
                0.0
            } else {
                db_to_lin(local_controls.monitor_gain_db)
            };

            let total_frames = out.len() / 2;
            let mut done = 0usize;
            while done < total_frames {
                let n = (total_frames - done).min(BLOCK);

                // Drift control: keep the ring short.
                let mut occ = cons.occupied_len();
                if occ > MAX_FILL {
                    let drop = occ - DRIFT_TARGET;
                    cons.skip(drop);
                    sh.drift_drops.fetch_add(1, Ordering::Relaxed);
                    occ = DRIFT_TARGET;
                }
                if !primed && occ >= PRIME_SAMPLES {
                    primed = true;
                }
                let got = if primed {
                    cons.pop_slice(&mut raw[..n])
                } else {
                    0
                };
                if primed && got < n {
                    sh.underruns.fetch_add(1, Ordering::Relaxed);
                    primed = false;
                }
                for s in raw[got..n].iter_mut() {
                    *s = 0.0;
                }
                sh.ring_fill
                    .store(cons.occupied_len() as u32, Ordering::Relaxed);

                for s in raw[..n].iter_mut() {
                    *s *= in_gain;
                }
                sh.dsp_in.update(&raw[..n]);
                if local_controls.processed {
                    chain.process_block(&raw[..n], &mut proc[..n], &local_params);
                } else {
                    proc[..n].copy_from_slice(&raw[..n]);
                }
                for s in proc[..n].iter_mut() {
                    *s *= out_gain;
                }
                sh.dsp_out.update(&proc[..n]);

                // Monitor output (stereo), hard-limited to the legal range.
                let mut clipped_block = false;
                for i in 0..n {
                    let mut v = proc[i] * mon_gain;
                    if v.abs() > 1.0 {
                        clipped_block = true;
                        v = v.clamp(-1.0, 1.0);
                    }
                    out[(done + i) * 2] = v;
                    out[(done + i) * 2 + 1] = v;
                    proc[i] = v;
                }
                let _ = clipped_block;
                sh.monitor_out.update(&proc[..n]);

                // Waveform points (peak per 10 ms) and feedback heuristic.
                for i in 0..n {
                    wave_acc_in = wave_acc_in.max(raw[i].abs());
                    wave_acc_out = wave_acc_out.max(proc[i].abs());
                    wave_n += 1;
                    if wave_n >= WAVE_SPAN {
                        sh.wave_in.push(wave_acc_in);
                        sh.wave_out.push(wave_acc_out);
                        wave_acc_in = 0.0;
                        wave_acc_out = 0.0;
                        wave_n = 0;
                    }
                    block_sum += raw[i] * raw[i];
                    block_acc += 1;
                    if block_acc >= 4800 {
                        // 100 ms windows: a loud, steady-level input while monitoring is audible
                        // is the signature of an acoustic loop (howl).
                        let rms = (block_sum / block_acc as f32).sqrt();
                        let steady = last_rms > 0.0 && (rms - last_rms).abs() / last_rms < 0.08;
                        if rms > 0.2 && steady && mon_gain > 0.0 {
                            fb_blocks += 1;
                        } else {
                            fb_blocks = fb_blocks.saturating_sub(2);
                        }
                        if fb_blocks >= 15 {
                            sh.feedback_suspected.store(true, Ordering::Relaxed);
                        } else if fb_blocks == 0 {
                            sh.feedback_suspected.store(false, Ordering::Relaxed);
                        }
                        last_rms = rms;
                        block_sum = 0.0;
                        block_acc = 0;
                    }
                }
                done += n;
            }
            sh.frames_rendered
                .fetch_add(total_frames as u64, Ordering::Relaxed);
            true
        });

        let (render, extras) = match render_result {
            Ok(r) => r,
            Err(e) => {
                stop.store(true, Ordering::Release);
                return Err(e);
            }
        };
        let input = capture.info.clone();
        let output = render.info.clone();
        Ok(Self {
            shared,
            capture,
            render,
            padding: extras.padding_frames,
            input,
            output,
        })
    }

    pub fn set_params(&self, p: VoiceParams) {
        *self.shared.params.lock() = p.sanitized();
        self.shared.params_version.fetch_add(1, Ordering::Release);
    }

    pub fn set_controls(&self, c: MonitorControls) {
        *self.shared.controls.lock() = c.sanitized();
        self.shared.controls_version.fetch_add(1, Ordering::Release);
    }

    pub fn controls(&self) -> MonitorControls {
        *self.shared.controls.lock()
    }

    pub fn reset_clips(&self) {
        for m in [
            &self.shared.mic_in,
            &self.shared.dsp_in,
            &self.shared.dsp_out,
            &self.shared.monitor_out,
        ] {
            m.reset_clips();
        }
    }

    /// Emergency stop: mutes the monitor immediately (even before threads wind down) and stops streams.
    pub fn emergency_stop(&mut self) {
        {
            let mut c = self.shared.controls.lock();
            c.monitor_mute = true;
        }
        self.shared.controls_version.fetch_add(1, Ordering::Release);
        self.stop();
    }

    pub fn stop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.capture.join();
        self.render.join();
    }

    pub fn is_running(&self) -> bool {
        !self.shared.stop.load(Ordering::Acquire)
            && !self.capture.finished.load(Ordering::Acquire)
            && !self.render.finished.load(Ordering::Acquire)
    }

    fn stream_error(&self) -> Option<String> {
        self.capture
            .error
            .lock()
            .clone()
            .or_else(|| self.render.error.lock().clone())
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let sh = &self.shared;
        let ms = |samples: f32| samples / SAMPLE_RATE as f32 * 1000.0;
        let capture_ms = self.input.stream_latency_ms;
        let ring_ms = ms(sh.ring_fill.load(Ordering::Relaxed) as f32);
        let dsp_ms = if sh.controls.lock().processed {
            ms(sh.dsp_latency_samples.load(Ordering::Relaxed) as f32)
        } else {
            0.0
        };
        let render_ms =
            self.output.stream_latency_ms + ms(self.padding.load(Ordering::Relaxed) as f32);
        EngineSnapshot {
            running: self.is_running(),
            error: self.stream_error(),
            input_device: self.input.device_name.clone(),
            output_device: self.output.device_name.clone(),
            mic_in: sh.mic_in.read(),
            dsp_in: sh.dsp_in.read(),
            dsp_out: sh.dsp_out.read(),
            monitor_out: sh.monitor_out.read(),
            wave_in: sh.wave_in.snapshot(),
            wave_out: sh.wave_out.snapshot(),
            latency: LatencyReport {
                capture_ms,
                ring_ms,
                dsp_ms,
                render_ms,
                total_ms: capture_ms + ring_ms + dsp_ms + render_ms,
                method: "buffer-path",
            },
            underruns: sh.underruns.load(Ordering::Relaxed),
            overruns: sh.overruns.load(Ordering::Relaxed),
            drift_corrections: sh.drift_drops.load(Ordering::Relaxed),
            feedback_suspected: sh.feedback_suspected.load(Ordering::Relaxed),
            controls: *sh.controls.lock(),
            frames_rendered: sh.frames_rendered.load(Ordering::Relaxed),
        }
    }

    /// Warn when the chosen output is likely to feed back into the chosen microphone.
    pub fn feedback_risk(input_id: Option<&str>, output_id: Option<&str>) -> Option<String> {
        let out = devices::list_devices(devices::Flow::Render).ok()?;
        let dev = match output_id {
            Some(id) if !id.is_empty() => out.into_iter().find(|d| d.id == id),
            _ => out.into_iter().find(|d| d.is_default),
        }?;
        let headphone_like = matches!(
            dev.form_factor.as_str(),
            "Headphones" | "Headset" | "Handset"
        );
        if headphone_like {
            return None;
        }
        let _ = input_id;
        Some(format!("'{}' is not a headphone-type output. Live monitoring through speakers can cause howling feedback; use headphones or keep the monitor volume low.", dev.name))
    }
}

impl Drop for MonitorEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_tracks_peak_rms_and_clips() {
        let m = Meter::default();
        m.update(&[0.0, 0.5, -0.5, 1.0]);
        let r = m.read();
        assert!((r.peak - 1.0).abs() < 1e-6);
        assert!(r.rms > 0.4 && r.rms < 0.7);
        assert!(r.clipped);
        // Peak is "since last read"; next read without new data is zero.
        assert_eq!(m.read().peak, 0.0);
        m.reset_clips();
        assert!(!m.read().clipped);
    }

    #[test]
    fn controls_are_sanitized() {
        let c = MonitorControls {
            input_gain_db: f32::NAN,
            output_gain_db: 500.0,
            monitor_gain_db: -500.0,
            monitor_mute: false,
            processed: true,
        }
        .sanitized();
        assert_eq!(c.input_gain_db, 0.0);
        assert_eq!(c.output_gain_db, 24.0);
        assert_eq!(c.monitor_gain_db, -80.0);
    }

    #[test]
    fn waveform_wraps_oldest_to_newest() {
        let w = Wave::new();
        for i in 0..(WAVE_POINTS + 3) {
            w.push(i as f32);
        }
        let s = w.snapshot();
        assert_eq!(s[WAVE_POINTS - 1], (WAVE_POINTS + 2) as f32);
        assert_eq!(s[0], 3.0);
    }

    #[test]
    #[ignore = "needs a real microphone and output device; runs muted, so nothing is audible"]
    fn live_monitoring_runs_muted_on_default_devices() {
        let controls = MonitorControls {
            monitor_mute: true,
            ..Default::default()
        };
        let mut eng = MonitorEngine::start(None, None, VoiceParams::default(), controls)
            .expect("start engine");
        std::thread::sleep(std::time::Duration::from_millis(2000));
        let s = eng.snapshot();
        println!("in='{}' out='{}'", s.input_device, s.output_device);
        println!("latency: {:?}", s.latency);
        println!(
            "underruns={} overruns={} drift={} frames={}",
            s.underruns, s.overruns, s.drift_corrections, s.frames_rendered
        );
        assert!(s.running, "engine stopped: {:?}", s.error);
        assert!(
            s.frames_rendered > 48_000,
            "render thread did not deliver ~2 s of audio"
        );
        assert!(s.latency.total_ms > 0.0);
        eng.emergency_stop();
        assert!(!eng.is_running());
    }
}
