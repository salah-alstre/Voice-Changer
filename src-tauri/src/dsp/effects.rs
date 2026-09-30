//! Time-based and colouration effects. All buffers are allocated up front.

use super::params::{DelayParams, EffectsParams, ReverbParams};
use std::f32::consts::PI;

pub struct Delay {
    buf: Vec<f32>,
    pos: usize,
    sr: f32,
}

impl Delay {
    pub fn new(sr: f32) -> Self {
        Self {
            buf: vec![0.0; (sr * 1.05) as usize],
            pos: 0,
            sr,
        }
    }

    pub fn reset(&mut self) {
        self.buf.fill(0.0);
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &DelayParams) -> f32 {
        let len = self.buf.len();
        let d = ((p.time_ms * 0.001 * self.sr) as usize).clamp(1, len - 1);
        let read = (self.pos + len - d) % len;
        let delayed = self.buf[read];
        self.buf[self.pos] = x + delayed * p.feedback;
        self.pos = (self.pos + 1) % len;
        x + delayed * p.mix
    }
}

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
            store: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.pos];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.pos] = x + self.store * feedback;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        out
    }
}

struct AllPass {
    buf: Vec<f32>,
    pos: usize,
}

impl AllPass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.pos];
        let out = -x + b;
        self.buf[self.pos] = x + b * 0.5;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        out
    }
}

/// Freeverb-style mono reverb.
pub struct Reverb {
    combs: Vec<Comb>,
    allpass: Vec<AllPass>,
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];

impl Reverb {
    pub fn new(sr: f32) -> Self {
        let scale = sr / 44100.0;
        Self {
            combs: COMB_TUNING
                .iter()
                .map(|&t| Comb::new((t as f32 * scale) as usize))
                .collect(),
            allpass: ALLPASS_TUNING
                .iter()
                .map(|&t| AllPass::new((t as f32 * scale) as usize))
                .collect(),
        }
    }

    pub fn reset(&mut self) {
        for c in self.combs.iter_mut() {
            c.buf.fill(0.0);
            c.store = 0.0;
        }
        for a in self.allpass.iter_mut() {
            a.buf.fill(0.0);
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &ReverbParams) -> f32 {
        let feedback = 0.7 + 0.28 * (0.5 * p.room_size + 0.5 * p.decay);
        let damp = 0.2 + 0.3 * (1.0 - p.decay);
        let input = x * 0.015;
        let mut wet = 0.0;
        for c in self.combs.iter_mut() {
            wet += c.process(input, feedback, damp);
        }
        for a in self.allpass.iter_mut() {
            wet = a.process(wet);
        }
        x * (1.0 - p.mix * 0.5) + wet * p.mix * 3.0
    }
}

pub struct Colour {
    sr: f32,
    ring_phase: f32,
    lfo_phase: f32,
    vib_buf: Vec<f32>,
    vib_pos: usize,
}

impl Colour {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            ring_phase: 0.0,
            lfo_phase: 0.0,
            vib_buf: vec![0.0; (sr * 0.03) as usize],
            vib_pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vib_buf.fill(0.0);
        self.ring_phase = 0.0;
        self.lfo_phase = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32, p: &EffectsParams) -> f32 {
        let mut y = x;
        if p.vibrato_enabled {
            let len = self.vib_buf.len();
            self.vib_buf[self.vib_pos] = y;
            self.lfo_phase += p.vibrato_rate_hz / self.sr;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
            }
            let delay =
                (0.004 + 0.003 * p.vibrato_depth * (2.0 * PI * self.lfo_phase).sin()) * self.sr;
            let rp = self.vib_pos as f32 - delay;
            let rp = if rp < 0.0 { rp + len as f32 } else { rp };
            let i0 = rp as usize % len;
            let i1 = (i0 + 1) % len;
            let frac = rp - rp.floor();
            y = self.vib_buf[i0] * (1.0 - frac) + self.vib_buf[i1] * frac;
            self.vib_pos = (self.vib_pos + 1) % len;
        }
        if p.ring_enabled {
            self.ring_phase += p.ring_hz / self.sr;
            if self.ring_phase >= 1.0 {
                self.ring_phase -= 1.0;
            }
            let m = (2.0 * PI * self.ring_phase).sin();
            y = y * (1.0 - p.ring_mix) + y * m * p.ring_mix;
        }
        if p.saturation_enabled {
            let d = p.saturation_drive;
            let sat = (y * d).tanh() / d.tanh();
            y = y * (1.0 - p.saturation_mix) + sat * p.saturation_mix;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_echoes_impulse_at_requested_time() {
        let mut d = Delay::new(48000.0);
        let p = DelayParams {
            enabled: true,
            time_ms: 100.0,
            feedback: 0.0,
            mix: 1.0,
        };
        let mut out = vec![];
        for i in 0..6000 {
            out.push(d.process(if i == 0 { 1.0 } else { 0.0 }, &p));
        }
        assert!((out[4800] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn reverb_produces_a_tail_and_stays_finite() {
        let mut r = Reverb::new(48000.0);
        let p = ReverbParams {
            enabled: true,
            room_size: 1.0,
            decay: 1.0,
            mix: 1.0,
        };
        let mut energy = 0.0;
        for i in 0..48000 {
            let y = r.process(if i == 0 { 1.0 } else { 0.0 }, &p);
            assert!(y.is_finite());
            if i > 2000 {
                energy += y * y;
            }
        }
        assert!(energy > 1e-6);
    }

    #[test]
    fn saturation_is_bounded() {
        let mut c = Colour::new(48000.0);
        let p = EffectsParams {
            saturation_enabled: true,
            saturation_drive: 30.0,
            saturation_mix: 1.0,
            ..Default::default()
        };
        for i in 0..1000 {
            let y = c.process((i as f32 * 0.1).sin() * 3.0, &p);
            assert!(y.abs() <= 1.001);
        }
    }
}
