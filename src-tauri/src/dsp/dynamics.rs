//! Noise gate, compressor and look-ahead-free brickwall limiter.

use super::params::{CompressorParams, GateParams, LimiterParams};

#[inline]
pub fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[inline]
pub fn lin_to_db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

fn coef(ms: f32, sr: f32) -> f32 {
    (-1.0 / (ms.max(0.01) * 0.001 * sr)).exp()
}

pub struct Gate {
    sr: f32,
    env: f32,
    gain: f32,
    hold_left: u32,
}

impl Gate {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            env: 0.0,
            gain: 0.0,
            hold_left: 0,
        }
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
        self.gain = 0.0;
        self.hold_left = 0;
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &GateParams) -> f32 {
        let a = x.abs();
        let env_coef = if a > self.env {
            coef(1.0, self.sr)
        } else {
            coef(20.0, self.sr)
        };
        self.env = a + env_coef * (self.env - a);
        let open = lin_to_db(self.env) > p.threshold_db;
        if open {
            self.hold_left = (p.hold_ms * 0.001 * self.sr) as u32;
        }
        let target = if open || self.hold_left > 0 { 1.0 } else { 0.0 };
        if !open && self.hold_left > 0 {
            self.hold_left -= 1;
        }
        let c = if target > self.gain {
            coef(p.attack_ms, self.sr)
        } else {
            coef(p.release_ms, self.sr)
        };
        self.gain = target + c * (self.gain - target);
        x * self.gain
    }
}

pub struct Compressor {
    sr: f32,
    env_db: f32,
    /// Most recent gain reduction in dB (<= 0), for UI metering.
    pub gain_reduction_db: f32,
}

impl Compressor {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            env_db: -120.0,
            gain_reduction_db: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.env_db = -120.0;
        self.gain_reduction_db = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &CompressorParams) -> f32 {
        let level = lin_to_db(x.abs());
        let c = if level > self.env_db {
            coef(p.attack_ms, self.sr)
        } else {
            coef(p.release_ms, self.sr)
        };
        self.env_db = level + c * (self.env_db - level);
        let over = self.env_db - p.threshold_db;
        let reduction = if over > 0.0 {
            over * (1.0 - 1.0 / p.ratio)
        } else {
            0.0
        };
        self.gain_reduction_db = -reduction;
        x * db_to_lin(-reduction + p.makeup_db)
    }
}

/// Instant-attack peak limiter with smooth release. Guarantees |y| <= ceiling
/// because the gain is applied on the same sample that triggers it.
pub struct Limiter {
    sr: f32,
    gain: f32,
}

impl Limiter {
    pub fn new(sr: f32) -> Self {
        Self { sr, gain: 1.0 }
    }

    pub fn reset(&mut self) {
        self.gain = 1.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &LimiterParams) -> f32 {
        let ceiling = db_to_lin(p.ceiling_db);
        let needed = if x.abs() > ceiling {
            ceiling / x.abs()
        } else {
            1.0
        };
        if needed < self.gain {
            self.gain = needed;
        } else {
            let c = coef(60.0, self.sr);
            self.gain = 1.0 + c * (self.gain - 1.0);
            if self.gain > needed {
                self.gain = needed;
            }
        }
        x * self.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SR: f32 = 48000.0;

    fn sine(amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / SR).sin())
            .collect()
    }

    #[test]
    fn limiter_never_exceeds_ceiling() {
        let mut l = Limiter::new(SR);
        let p = LimiterParams {
            enabled: true,
            ceiling_db: -3.0,
        };
        let ceil = db_to_lin(-3.0);
        for x in sine(4.0, 20000) {
            assert!(l.process(x, &p).abs() <= ceil + 1e-4);
        }
    }

    #[test]
    fn gate_closes_on_silence_and_opens_on_signal() {
        let mut g = Gate::new(SR);
        let p = GateParams {
            enabled: true,
            ..Default::default()
        };
        let quiet: Vec<f32> = sine(0.0005, 24000)
            .iter()
            .map(|x| g.process(*x, &p))
            .collect();
        assert!(quiet.iter().rev().take(100).all(|x| x.abs() < 0.0001));
        let loud: Vec<f32> = sine(0.5, 24000).iter().map(|x| g.process(*x, &p)).collect();
        let peak = loud
            .iter()
            .rev()
            .take(1000)
            .fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.45);
    }

    #[test]
    fn compressor_reduces_loud_signal() {
        let mut c = Compressor::new(SR);
        let p = CompressorParams {
            enabled: true,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            makeup_db: 0.0,
        };
        let out: Vec<f32> = sine(0.8, 48000).iter().map(|x| c.process(*x, &p)).collect();
        let peak = out
            .iter()
            .rev()
            .take(2000)
            .fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak < 0.5, "peak {peak}");
        assert!(c.gain_reduction_db < -3.0);
    }

    #[test]
    fn compressor_leaves_quiet_signal_alone() {
        let mut c = Compressor::new(SR);
        let p = CompressorParams {
            enabled: true,
            makeup_db: 0.0,
            ..Default::default()
        };
        let out: Vec<f32> = sine(0.01, 24000)
            .iter()
            .map(|x| c.process(*x, &p))
            .collect();
        let peak = out
            .iter()
            .rev()
            .take(1000)
            .fold(0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.01).abs() < 0.001);
    }
}
