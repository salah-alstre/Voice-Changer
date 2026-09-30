//! Phase-vocoder pitch shifter with independent spectral-envelope (formant)
//! control. Mono, 1024-point FFT, 4x overlap (256-sample hop): algorithmic
//! latency is `FFT - HOP` = 768 samples = 16 ms at 48 kHz.

use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::f32::consts::PI;
use std::sync::Arc;

pub const FFT: usize = 1024;
pub const OSAMP: usize = 4;
pub const HOP: usize = FFT / OSAMP;
pub const LATENCY: usize = FFT - HOP;
const BINS: usize = FFT / 2 + 1;
/// Half-width (in bins) of the moving average used to estimate the envelope.
const ENV_HALF: usize = 6;

pub struct PitchShifter {
    fwd: Arc<dyn RealToComplex<f32>>,
    inv: Arc<dyn ComplexToReal<f32>>,
    window: Vec<f32>,
    in_fifo: Vec<f32>,
    out_fifo: Vec<f32>,
    out_accum: Vec<f32>,
    last_phase: Vec<f32>,
    sum_phase: Vec<f32>,
    rover: usize,
    time: Vec<f32>,
    spec: Vec<Complex<f32>>,
    fwd_scratch: Vec<Complex<f32>>,
    inv_scratch: Vec<Complex<f32>>,
    ana_magn: Vec<f32>,
    ana_freq: Vec<f32>,
    env: Vec<f32>,
    syn_magn: Vec<f32>,
    syn_freq: Vec<f32>,
    sr: f32,
}

impl PitchShifter {
    pub fn new(sr: f32) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fwd = planner.plan_fft_forward(FFT);
        let inv = planner.plan_fft_inverse(FFT);
        let fwd_scratch = fwd.make_scratch_vec();
        let inv_scratch = inv.make_scratch_vec();
        let window = (0..FFT)
            .map(|i| -0.5 * (2.0 * PI * i as f32 / FFT as f32).cos() + 0.5)
            .collect();
        Self {
            fwd,
            inv,
            window,
            in_fifo: vec![0.0; FFT],
            out_fifo: vec![0.0; HOP],
            out_accum: vec![0.0; 2 * FFT],
            last_phase: vec![0.0; BINS],
            sum_phase: vec![0.0; BINS],
            rover: LATENCY,
            time: vec![0.0; FFT],
            spec: vec![Complex::new(0.0, 0.0); BINS],
            fwd_scratch,
            inv_scratch,
            ana_magn: vec![0.0; BINS],
            ana_freq: vec![0.0; BINS],
            env: vec![0.0; BINS],
            syn_magn: vec![0.0; BINS],
            syn_freq: vec![0.0; BINS],
            sr,
        }
    }

    pub fn reset(&mut self) {
        for v in [
            &mut self.in_fifo,
            &mut self.out_fifo,
            &mut self.out_accum,
            &mut self.last_phase,
            &mut self.sum_phase,
        ] {
            v.fill(0.0);
        }
        self.rover = LATENCY;
    }

    /// Feed one sample, receive one (delayed) sample.
    /// `pitch_ratio` scales pitch; `formant_ratio` scales the spectral envelope.
    #[inline]
    pub fn process(&mut self, x: f32, pitch_ratio: f32, formant_ratio: f32) -> f32 {
        self.in_fifo[self.rover] = x;
        let out = self.out_fifo[self.rover - LATENCY];
        self.rover += 1;
        if self.rover >= FFT {
            self.rover = LATENCY;
            self.process_frame(pitch_ratio, formant_ratio);
        }
        out
    }

    fn process_frame(&mut self, pitch_ratio: f32, formant_ratio: f32) {
        for k in 0..FFT {
            self.time[k] = self.in_fifo[k] * self.window[k];
        }
        let _ =
            self.fwd
                .process_with_scratch(&mut self.time, &mut self.spec, &mut self.fwd_scratch);

        let expct = 2.0 * PI * HOP as f32 / FFT as f32;
        let freq_per_bin = self.sr / FFT as f32;
        for k in 0..BINS {
            let c = self.spec[k];
            let magn = (c.re * c.re + c.im * c.im).sqrt();
            let phase = c.im.atan2(c.re);
            let mut tmp = phase - self.last_phase[k];
            self.last_phase[k] = phase;
            tmp -= k as f32 * expct;
            let mut qpd = (tmp / PI) as i32;
            qpd += if qpd >= 0 { qpd & 1 } else { -(qpd & 1) };
            tmp -= PI * qpd as f32;
            tmp = OSAMP as f32 * tmp / (2.0 * PI);
            self.ana_magn[k] = magn;
            self.ana_freq[k] = k as f32 * freq_per_bin + tmp * freq_per_bin;
        }

        // Spectral envelope via moving average of magnitudes.
        for k in 0..BINS {
            let lo = k.saturating_sub(ENV_HALF);
            let hi = (k + ENV_HALF).min(BINS - 1);
            let mut s = 0.0;
            for j in lo..=hi {
                s += self.ana_magn[j];
            }
            self.env[k] = s / (hi - lo + 1) as f32;
        }

        self.syn_magn.fill(0.0);
        self.syn_freq.fill(0.0);
        for k in 0..BINS {
            let index = (k as f32 * pitch_ratio) as usize;
            if index < BINS {
                let flat = self.ana_magn[k] / self.env[k].max(1e-9);
                self.syn_magn[index] += flat;
                self.syn_freq[index] = self.ana_freq[k] * pitch_ratio;
            }
        }

        for k in 0..BINS {
            // Envelope sampled at k / formant_ratio with linear interpolation.
            let src = k as f32 / formant_ratio;
            let env = if src >= (BINS - 1) as f32 {
                0.0
            } else {
                let i = src as usize;
                let f = src - i as f32;
                self.env[i] * (1.0 - f) + self.env[i + 1] * f
            };
            let magn = self.syn_magn[k] * env;
            let mut tmp = self.syn_freq[k];
            tmp -= k as f32 * freq_per_bin;
            tmp /= freq_per_bin;
            tmp = 2.0 * PI * tmp / OSAMP as f32;
            tmp += k as f32 * expct;
            self.sum_phase[k] += tmp;
            let (s, c) = self.sum_phase[k].sin_cos();
            self.spec[k] = Complex::new(magn * c, magn * s);
        }
        self.spec[0].im = 0.0;
        self.spec[BINS - 1].im = 0.0;
        let _ =
            self.inv
                .process_with_scratch(&mut self.spec, &mut self.time, &mut self.inv_scratch);

        let scale = 1.0 / ((FFT / 2) as f32 * OSAMP as f32);
        for k in 0..FFT {
            self.out_accum[k] += self.window[k] * self.time[k] * scale;
        }
        self.out_fifo.copy_from_slice(&self.out_accum[..HOP]);
        self.out_accum.copy_within(HOP.., 0);
        let n = self.out_accum.len();
        self.out_accum[n - HOP..].fill(0.0);
        self.in_fifo.copy_within(HOP.., 0);
    }
}

#[inline]
pub fn semitones_to_ratio(st: f32) -> f32 {
    2f32.powf(st / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SR: f32 = 48000.0;

    fn tone(freq: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| 0.5 * (2.0 * PI * freq * i as f32 / SR).sin())
            .collect()
    }

    /// Dominant frequency by zero-crossing rate over the last half of the buffer.
    fn estimate_freq(x: &[f32]) -> f32 {
        let s = &x[x.len() / 2..];
        let mut crossings = 0;
        for w in s.windows(2) {
            if w[0] <= 0.0 && w[1] > 0.0 {
                crossings += 1;
            }
        }
        crossings as f32 * SR / s.len() as f32
    }

    fn run(freq: f32, pitch: f32, formant: f32) -> Vec<f32> {
        let mut p = PitchShifter::new(SR);
        tone(freq, 48000)
            .iter()
            .map(|x| p.process(*x, pitch, formant))
            .collect()
    }

    #[test]
    fn unity_ratio_preserves_frequency_and_level() {
        let out = run(440.0, 1.0, 1.0);
        let f = estimate_freq(&out);
        assert!((f - 440.0).abs() < 8.0, "freq {f}");
        let peak = out[24000..].iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.35 && peak < 0.7, "peak {peak}");
    }

    #[test]
    fn octave_up_doubles_frequency() {
        let out = run(300.0, 2.0, 2.0);
        let f = estimate_freq(&out);
        assert!((f - 600.0).abs() < 20.0, "freq {f}");
    }

    #[test]
    fn fourth_down_lowers_frequency() {
        let r = semitones_to_ratio(-5.0);
        let out = run(500.0, r, r);
        let f = estimate_freq(&out);
        assert!((f - 500.0 * r).abs() < 15.0, "freq {f}");
    }

    #[test]
    fn output_stays_finite_under_parameter_sweeps() {
        let mut p = PitchShifter::new(SR);
        let x = tone(200.0, 48000);
        for (i, s) in x.iter().enumerate() {
            let r = semitones_to_ratio(((i / 2000) as f32 % 24.0) - 12.0);
            let y = p.process(*s, r, 1.0 / r);
            assert!(y.is_finite() && y.abs() < 4.0);
        }
    }
}
