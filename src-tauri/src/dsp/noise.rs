//! RNNoise-based noise suppression (nnnoiseless, BSD-3-Clause).
//!
//! RNNoise works on 480-sample frames at 48 kHz, so this stage adds exactly one
//! frame (10 ms) of latency. The dry path is delayed by the same amount so the
//! strength control can blend them without comb filtering.

use nnnoiseless::DenoiseState;

pub const FRAME: usize = DenoiseState::FRAME_SIZE;

pub struct NoiseSuppressor {
    state: Box<DenoiseState<'static>>,
    in_buf: [f32; FRAME],
    out_buf: [f32; FRAME],
    wet: [f32; FRAME],
    pos: usize,
    primed: bool,
    /// Voice-activity probability of the last processed frame (0..1).
    pub vad: f32,
}

impl Default for NoiseSuppressor {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseSuppressor {
    pub fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            in_buf: [0.0; FRAME],
            out_buf: [0.0; FRAME],
            wet: [0.0; FRAME],
            pos: 0,
            primed: false,
            vad: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.state = DenoiseState::new();
        self.in_buf = [0.0; FRAME];
        self.out_buf = [0.0; FRAME];
        self.pos = 0;
        self.primed = false;
    }

    #[inline]
    pub fn process(&mut self, x: f32, strength: f32) -> f32 {
        self.in_buf[self.pos] = x * 32768.0;
        let y = self.out_buf[self.pos] / 32768.0;
        self.pos += 1;
        if self.pos == FRAME {
            self.vad = self.state.process_frame(&mut self.wet, &self.in_buf);
            // The first frame out of RNNoise contains a fade-in artefact; drop it.
            for i in 0..FRAME {
                let wet = if self.primed {
                    self.wet[i]
                } else {
                    self.in_buf[i]
                };
                self.out_buf[i] = self.in_buf[i] * (1.0 - strength) + wet * strength;
            }
            self.primed = true;
            self.pos = 0;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random white noise in [-amp, amp].
    fn noise(n: usize, amp: f32) -> Vec<f32> {
        let mut s = 0x1234_5678u32;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                ((s >> 8) as f32 / 8_388_608.0 - 1.0) * amp
            })
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn suppresses_steady_white_noise() {
        let mut ns = NoiseSuppressor::new();
        let input = noise(48000 * 2, 0.02);
        let out: Vec<f32> = input.iter().map(|x| ns.process(*x, 1.0)).collect();
        let tail_in = rms(&input[48000..]);
        let tail_out = rms(&out[48000..]);
        assert!(tail_out < tail_in * 0.6, "in {tail_in} out {tail_out}");
    }

    #[test]
    fn zero_strength_is_a_delayed_passthrough() {
        let mut ns = NoiseSuppressor::new();
        let input = noise(FRAME * 10, 0.3);
        let out: Vec<f32> = input.iter().map(|x| ns.process(*x, 0.0)).collect();
        for i in FRAME..input.len() {
            assert!((out[i] - input[i - FRAME]).abs() < 1e-4);
        }
    }
}
