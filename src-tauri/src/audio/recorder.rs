//! Voice test recording and playback. Recordings live in RAM only (never written to disk unless
//! the user explicitly exports them) and are discarded on request or when the app exits.

use super::stream::{self, StreamHandle, SAMPLE_RATE};
use crate::dsp::{VoiceChain, VoiceParams};
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

pub const MIN_SECONDS: u32 = 5;
pub const MAX_SECONDS: u32 = 30;

struct Buffer {
    data: Vec<AtomicU32>,
    len: AtomicUsize,
    peak: AtomicU32,
}

pub struct Recorder {
    buf: Arc<Buffer>,
    stop: Arc<AtomicBool>,
    handle: StreamHandle,
    target: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    pub recording: bool,
    pub elapsed_ms: u32,
    pub target_ms: u32,
    pub level: f32,
    pub error: Option<String>,
}

impl Recorder {
    pub fn start(input_id: Option<String>, seconds: u32) -> AppResult<Self> {
        if !(MIN_SECONDS..=MAX_SECONDS).contains(&seconds) {
            return Err(AppError::Invalid(format!(
                "recording length must be {MIN_SECONDS}-{MAX_SECONDS} seconds"
            )));
        }
        let target = seconds as usize * SAMPLE_RATE as usize;
        let buf = Arc::new(Buffer {
            data: (0..target).map(|_| AtomicU32::new(0)).collect(),
            len: AtomicUsize::new(0),
            peak: AtomicU32::new(0),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let (b, s) = (buf.clone(), stop.clone());
        let handle = stream::start_capture(input_id, stop.clone(), move |samples| {
            let len = b.len.load(Ordering::Relaxed);
            let room = target - len;
            let n = samples.len().min(room);
            let mut peak = 0.0f32;
            for (i, &v) in samples[..n].iter().enumerate() {
                b.data[len + i].store(v.to_bits(), Ordering::Relaxed);
                peak = peak.max(v.abs());
            }
            b.peak.store(peak.to_bits(), Ordering::Relaxed);
            b.len.store(len + n, Ordering::Release);
            if n == room {
                s.store(true, Ordering::Release);
            }
        })?;
        Ok(Self {
            buf,
            stop,
            handle,
            target,
        })
    }

    pub fn status(&self) -> RecorderStatus {
        let len = self.buf.len.load(Ordering::Acquire);
        RecorderStatus {
            recording: !self.stop.load(Ordering::Acquire)
                && !self.handle.finished.load(Ordering::Acquire),
            elapsed_ms: (len as u64 * 1000 / SAMPLE_RATE as u64) as u32,
            target_ms: (self.target as u64 * 1000 / SAMPLE_RATE as u64) as u32,
            level: f32::from_bits(self.buf.peak.load(Ordering::Relaxed)),
            error: self.handle.error.lock().clone(),
        }
    }

    /// Stop and take the recording. Fails if nothing was captured.
    pub fn finish(mut self) -> AppResult<Vec<f32>> {
        self.stop.store(true, Ordering::Release);
        self.handle.join();
        let len = self.buf.len.load(Ordering::Acquire);
        if len < SAMPLE_RATE as usize / 4 {
            return Err(AppError::Audio(
                self.handle
                    .error
                    .lock()
                    .clone()
                    .unwrap_or_else(|| "no audio was captured".into()),
            ));
        }
        Ok(self.buf.data[..len]
            .iter()
            .map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
            .collect())
    }

    /// Stop and throw the audio away.
    pub fn cancel(mut self) {
        self.stop.store(true, Ordering::Release);
        self.handle.join();
    }
}

/// Run a recording through the voice chain offline, compensating the chain's algorithmic delay so
/// the processed take is time-aligned with the original.
pub fn render_processed(input: &[f32], params: &VoiceParams) -> Vec<f32> {
    let params = params.sanitized();
    let lat = VoiceChain::latency_samples(&params);
    let mut chain = VoiceChain::new(SAMPLE_RATE as f32);
    let mut out = Vec::with_capacity(input.len() + lat);
    let mut block = vec![0.0f32; 480];
    let mut res = vec![0.0f32; 480];
    let total = input.len() + lat;
    let mut pos = 0;
    while pos < total {
        let n = (total - pos).min(480);
        for (i, b) in block[..n].iter_mut().enumerate() {
            *b = input.get(pos + i).copied().unwrap_or(0.0);
        }
        chain.process_block(&block[..n], &mut res[..n], &params);
        out.extend_from_slice(&res[..n]);
        pos += n;
    }
    let mut out: Vec<f32> = out.split_off(lat.min(out.len()));
    out.truncate(input.len());
    for s in out.iter_mut() {
        *s = s.clamp(-1.0, 1.0);
    }
    out
}

pub struct Player {
    handle: StreamHandle,
    stop: Arc<AtomicBool>,
    pos: Arc<AtomicUsize>,
    total: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatus {
    pub playing: bool,
    pub position_ms: u32,
    pub total_ms: u32,
}

impl Player {
    pub fn start(
        output_id: Option<String>,
        samples: Arc<Vec<f32>>,
        gain_db: f32,
    ) -> AppResult<Self> {
        let gain = super::engine::db_to_lin(if gain_db.is_finite() {
            gain_db.clamp(-80.0, 12.0)
        } else {
            0.0
        });
        let stop = Arc::new(AtomicBool::new(false));
        let pos = Arc::new(AtomicUsize::new(0));
        let (p2, total) = (pos.clone(), samples.len());
        let (handle, _) = stream::start_render(output_id, stop.clone(), move |out| {
            let mut p = p2.load(Ordering::Relaxed);
            for frame in out.chunks_exact_mut(2) {
                let v = if p < samples.len() {
                    (samples[p] * gain).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                frame[0] = v;
                frame[1] = v;
                p += 1;
            }
            p2.store(p.min(samples.len()), Ordering::Relaxed);
            p < samples.len()
        })?;
        Ok(Self {
            handle,
            stop,
            pos,
            total,
        })
    }

    pub fn status(&self) -> PlayerStatus {
        let ms = |s: usize| (s as u64 * 1000 / SAMPLE_RATE as u64) as u32;
        PlayerStatus {
            playing: !self.handle.finished.load(Ordering::Acquire)
                && !self.stop.load(Ordering::Acquire),
            position_ms: ms(self.pos.load(Ordering::Relaxed)),
            total_ms: ms(self.total),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.handle.join();
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Encode a mono take as a 16-bit PCM WAV (used only for an explicit user export).
pub fn encode_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    v.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| 0.3 * (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn processed_take_keeps_length_and_range() {
        let input = sine(48_000);
        let out = render_processed(&input, &VoiceParams::default());
        assert_eq!(out.len(), input.len());
        assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    }

    #[test]
    fn wav_header_is_well_formed() {
        let w = encode_wav(&sine(100));
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(w.len(), 44 + 200);
    }

    #[test]
    fn rejects_out_of_range_length() {
        assert!(Recorder::start(None, 1).is_err());
        assert!(Recorder::start(None, 99).is_err());
    }

    #[test]
    #[ignore = "needs a real microphone and output; records 5 s silently, plays nothing"]
    fn records_five_seconds_from_default_mic() {
        let r = Recorder::start(None, 5).expect("start");
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let st = r.status();
        println!("{st:?}");
        assert!(st.elapsed_ms > 1000);
        let take = r.finish().expect("finish");
        println!("captured {} samples", take.len());
        assert!(take.len() > 48_000);
    }
}
