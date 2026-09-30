//! The complete voice processing chain (mono, real-time safe).
//!
//! Order: input gain -> noise suppression -> gate -> HPF/LPF -> EQ -> pitch &
//! formant -> colour effects -> delay -> reverb -> compressor -> output gain ->
//! limiter (final safety stage) -> wet/dry mix.
//!
//! Nothing in `process_block` allocates, locks, logs or touches the disk.

use super::biquad::{Biquad, Coeffs, ParametricEq};
use super::dynamics::{db_to_lin, Compressor, Gate, Limiter};
use super::effects::{Colour, Delay, Reverb};
use super::noise::{NoiseSuppressor, FRAME as NS_FRAME};
use super::params::VoiceParams;
use super::pitch::{semitones_to_ratio, PitchShifter, LATENCY as PITCH_LATENCY};

const MAX_DRY_DELAY: usize = 4096;

pub struct VoiceChain {
    sr: f32,
    ns: NoiseSuppressor,
    gate: Gate,
    hp: [Biquad; 2],
    lp: [Biquad; 2],
    filter_key: (bool, f32, bool, f32),
    eq: ParametricEq,
    pitch: PitchShifter,
    pitch_active: bool,
    colour: Colour,
    delay: Delay,
    reverb: Reverb,
    comp: Compressor,
    limiter: Limiter,
    in_gain: f32,
    out_gain: f32,
    dry: Vec<f32>,
    dry_pos: usize,
    /// Gain reduction applied by the compressor at the end of the last block (dB, <= 0).
    pub gain_reduction_db: f32,
}

impl VoiceChain {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            ns: NoiseSuppressor::new(),
            gate: Gate::new(sr),
            hp: [Biquad::default(); 2],
            lp: [Biquad::default(); 2],
            filter_key: (false, 0.0, false, 0.0),
            eq: ParametricEq::new(sr),
            pitch: PitchShifter::new(sr),
            pitch_active: false,
            colour: Colour::new(sr),
            delay: Delay::new(sr),
            reverb: Reverb::new(sr),
            comp: Compressor::new(sr),
            limiter: Limiter::new(sr),
            in_gain: 1.0,
            out_gain: 1.0,
            dry: vec![0.0; MAX_DRY_DELAY],
            dry_pos: 0,
            gain_reduction_db: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.ns.reset();
        self.gate.reset();
        for b in self.hp.iter_mut().chain(self.lp.iter_mut()) {
            b.reset();
        }
        self.eq.reset();
        self.pitch.reset();
        self.colour.reset();
        self.delay.reset();
        self.reverb.reset();
        self.comp.reset();
        self.limiter.reset();
        self.dry.fill(0.0);
    }

    /// (pitch ratio, formant ratio, active?) for the given parameters.
    fn pitch_plan(p: &VoiceParams) -> (f32, f32, bool) {
        let st = if p.pitch.enabled {
            p.pitch.semitones + p.pitch.cents / 100.0
        } else {
            0.0
        };
        let pr = semitones_to_ratio(st);
        let fr = if p.formant.enabled {
            semitones_to_ratio(p.formant.shift)
        } else {
            pr
        };
        let active = (pr - 1.0).abs() > 1e-4 || (fr - 1.0).abs() > 1e-4;
        (pr, fr, active)
    }

    /// Latency this chain adds for the given parameters, in samples.
    pub fn latency_samples(p: &VoiceParams) -> usize {
        let mut l = 0;
        if p.noise_suppression.enabled {
            l += NS_FRAME;
        }
        if Self::pitch_plan(p).2 {
            l += PITCH_LATENCY;
        }
        l
    }

    fn update_filters(&mut self, p: &VoiceParams) {
        let f = &p.filters;
        let key = (
            f.highpass_enabled,
            f.highpass_hz,
            f.lowpass_enabled,
            f.lowpass_hz,
        );
        if key != self.filter_key {
            // Two cascaded 2nd-order sections give a 24 dB/oct Butterworth-ish slope.
            let hp = if f.highpass_enabled {
                Coeffs::highpass(f.highpass_hz, std::f32::consts::FRAC_1_SQRT_2, self.sr)
            } else {
                Coeffs::PASSTHROUGH
            };
            let lp = if f.lowpass_enabled {
                Coeffs::lowpass(f.lowpass_hz, std::f32::consts::FRAC_1_SQRT_2, self.sr)
            } else {
                Coeffs::PASSTHROUGH
            };
            for b in self.hp.iter_mut() {
                b.set(hp);
            }
            for b in self.lp.iter_mut() {
                b.set(lp);
            }
            self.filter_key = key;
        }
    }

    pub fn process_block(&mut self, input: &[f32], output: &mut [f32], p: &VoiceParams) {
        debug_assert_eq!(input.len(), output.len());
        self.update_filters(p);
        self.eq.update(&p.eq);
        let (pr, fr, active) = Self::pitch_plan(p);
        if active != self.pitch_active {
            self.pitch.reset();
            self.pitch_active = active;
        }
        let in_target = db_to_lin(p.input_gain_db);
        let out_target = db_to_lin(p.output_gain_db);
        let latency = Self::latency_samples(p).min(MAX_DRY_DELAY - 1);
        let dry_len = self.dry.len();

        for (x, y) in input.iter().zip(output.iter_mut()) {
            self.in_gain += (in_target - self.in_gain) * 0.01;
            self.out_gain += (out_target - self.out_gain) * 0.01;

            let raw = *x;
            self.dry[self.dry_pos] = raw;
            let dry = self.dry[(self.dry_pos + dry_len - latency) % dry_len];
            self.dry_pos = (self.dry_pos + 1) % dry_len;

            let mut s = raw * self.in_gain;
            if p.noise_suppression.enabled {
                s = self.ns.process(s, p.noise_suppression.strength);
            }
            if p.gate.enabled {
                s = self.gate.process(s, &p.gate);
            }
            if p.filters.highpass_enabled {
                s = self.hp[0].process(s);
                s = self.hp[1].process(s);
            }
            if p.filters.lowpass_enabled {
                s = self.lp[0].process(s);
                s = self.lp[1].process(s);
            }
            if p.eq.enabled {
                s = self.eq.process(s);
            }
            if active {
                s = self.pitch.process(s, pr, fr);
            }
            s = self.colour.process(s, &p.effects);
            if p.delay.enabled {
                s = self.delay.process(s, &p.delay);
            }
            if p.reverb.enabled {
                s = self.reverb.process(s, &p.reverb);
            }
            if p.compressor.enabled {
                s = self.comp.process(s, &p.compressor);
            }
            s *= self.out_gain;
            if p.limiter.enabled {
                s = self.limiter.process(s, &p.limiter);
            }
            if p.mix < 0.999 {
                s = dry * (1.0 - p.mix) + s * p.mix;
            }
            // Never hand NaN/inf to the device.
            *y = if s.is_finite() { s } else { 0.0 };
        }
        self.gain_reduction_db = if p.compressor.enabled {
            self.comp.gain_reduction_db
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    const SR: f32 = 48000.0;

    fn tone(freq: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * PI * freq * i as f32 / SR).sin())
            .collect()
    }

    fn run(p: &VoiceParams, input: &[f32]) -> Vec<f32> {
        let mut c = VoiceChain::new(SR);
        let mut out = vec![0.0; input.len()];
        for (i, o) in input.chunks(480).zip(out.chunks_mut(480)) {
            c.process_block(i, o, p);
        }
        out
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn default_params_are_transparent() {
        let x = tone(1000.0, 0.3, 48000);
        let y = run(&VoiceParams::default(), &x);
        assert_eq!(VoiceChain::latency_samples(&VoiceParams::default()), 0);
        for i in 4800..x.len() {
            assert!((x[i] - y[i]).abs() < 0.003, "sample {i}");
        }
    }

    #[test]
    fn gains_apply_in_db() {
        let mut p = VoiceParams::default();
        p.input_gain_db = 6.0;
        p.limiter.enabled = false;
        let y = run(&p, &tone(1000.0, 0.1, 48000));
        assert!((peak(&y[24000..]) - 0.1 * db_to_lin(6.0)).abs() < 0.005);
    }

    #[test]
    fn limiter_protects_output_from_hot_input() {
        let mut p = VoiceParams::default();
        p.input_gain_db = 24.0;
        p.limiter.ceiling_db = -1.0;
        let y = run(&p, &tone(300.0, 0.9, 48000));
        assert!(peak(&y) <= db_to_lin(-1.0) + 1e-3);
    }

    #[test]
    fn mix_zero_returns_dry_signal_aligned() {
        let mut p = VoiceParams::default();
        p.pitch.semitones = 7.0;
        p.mix = 0.0;
        let x = tone(440.0, 0.3, 48000);
        let y = run(&p, &x);
        let lat = VoiceChain::latency_samples(&p);
        assert_eq!(lat, PITCH_LATENCY);
        for i in lat + 100..x.len() {
            assert!((y[i] - x[i - lat]).abs() < 1e-4);
        }
    }

    #[test]
    fn bypassed_modules_do_not_affect_signal() {
        let mut p = VoiceParams::default();
        p.compressor.enabled = false;
        p.compressor.ratio = 20.0;
        p.reverb.enabled = false;
        p.reverb.mix = 1.0;
        p.gate.enabled = false;
        p.gate.threshold_db = 0.0;
        let x = tone(800.0, 0.2, 24000);
        let y = run(&p, &x);
        for i in 2400..x.len() {
            assert!((x[i] - y[i]).abs() < 0.003);
        }
    }

    #[test]
    fn full_chain_stays_finite_with_extreme_settings() {
        let mut p = VoiceParams::default();
        p.noise_suppression.enabled = true;
        p.gate.enabled = true;
        p.filters.highpass_enabled = true;
        p.filters.lowpass_enabled = true;
        p.pitch.semitones = -12.0;
        p.formant.shift = 12.0;
        p.effects.ring_enabled = true;
        p.effects.saturation_enabled = true;
        p.effects.vibrato_enabled = true;
        p.delay.enabled = true;
        p.reverb.enabled = true;
        p.compressor.enabled = true;
        let p = p.sanitized();
        let y = run(&p, &tone(180.0, 0.8, 96000));
        assert!(y.iter().all(|v| v.is_finite()));
        assert!(peak(&y) <= 1.0);
    }
}
