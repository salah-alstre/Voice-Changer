//! RBJ-cookbook biquad filters (transposed direct form II).

use super::params::{BandKind, EqBand, EqParams, EQ_BANDS};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct Coeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Coeffs {
    pub const PASSTHROUGH: Coeffs = Coeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn normalized(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv = 1.0 / a0;
        Coeffs {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
        }
    }

    fn clamp_freq(freq: f32, sr: f32) -> f32 {
        freq.clamp(10.0, sr * 0.49)
    }

    pub fn highpass(freq: f32, q: f32, sr: f32) -> Self {
        let w = 2.0 * PI * Self::clamp_freq(freq, sr) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        Self::normalized(
            (1.0 + c) / 2.0,
            -(1.0 + c),
            (1.0 + c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    pub fn lowpass(freq: f32, q: f32, sr: f32) -> Self {
        let w = 2.0 * PI * Self::clamp_freq(freq, sr) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        Self::normalized(
            (1.0 - c) / 2.0,
            1.0 - c,
            (1.0 - c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    pub fn peak(freq: f32, gain_db: f32, q: f32, sr: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w = 2.0 * PI * Self::clamp_freq(freq, sr) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        Self::normalized(
            1.0 + alpha * a,
            -2.0 * c,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * c,
            1.0 - alpha / a,
        )
    }

    pub fn low_shelf(freq: f32, gain_db: f32, q: f32, sr: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w = 2.0 * PI * Self::clamp_freq(freq, sr) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        let t = 2.0 * a.sqrt() * alpha;
        Self::normalized(
            a * ((a + 1.0) - (a - 1.0) * c + t),
            2.0 * a * ((a - 1.0) - (a + 1.0) * c),
            a * ((a + 1.0) - (a - 1.0) * c - t),
            (a + 1.0) + (a - 1.0) * c + t,
            -2.0 * ((a - 1.0) + (a + 1.0) * c),
            (a + 1.0) + (a - 1.0) * c - t,
        )
    }

    pub fn high_shelf(freq: f32, gain_db: f32, q: f32, sr: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w = 2.0 * PI * Self::clamp_freq(freq, sr) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        let t = 2.0 * a.sqrt() * alpha;
        Self::normalized(
            a * ((a + 1.0) + (a - 1.0) * c + t),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
            a * ((a + 1.0) + (a - 1.0) * c - t),
            (a + 1.0) - (a - 1.0) * c + t,
            2.0 * ((a - 1.0) - (a + 1.0) * c),
            (a + 1.0) - (a - 1.0) * c - t,
        )
    }

    pub fn for_band(b: &EqBand, sr: f32) -> Self {
        if b.gain_db.abs() < 0.01 {
            return Self::PASSTHROUGH;
        }
        match b.kind {
            BandKind::LowShelf => Self::low_shelf(b.freq, b.gain_db, b.q, sr),
            BandKind::Peak => Self::peak(b.freq, b.gain_db, b.q, sr),
            BandKind::HighShelf => Self::high_shelf(b.freq, b.gain_db, b.q, sr),
        }
    }

    /// Magnitude response in dB at `freq`. Used for the EQ graph and tests.
    pub fn magnitude_db(&self, freq: f32, sr: f32) -> f32 {
        let w = 2.0 * PI * freq / sr;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        let num = nr * nr + ni * ni;
        let den = (dr * dr + di * di).max(1e-20);
        10.0 * (num / den).max(1e-20).log10()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    c: Coeffs,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            c: Coeffs::PASSTHROUGH,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl Biquad {
    pub fn set(&mut self, c: Coeffs) {
        self.c = c;
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.c.b0 * x + self.z1;
        self.z1 = self.c.b1 * x - self.c.a1 * y + self.z2;
        self.z2 = self.c.b2 * x - self.c.a2 * y;
        y
    }
}

/// Eight-band parametric EQ.
pub struct ParametricEq {
    bands: [Biquad; EQ_BANDS],
    last: Option<EqParams>,
    sr: f32,
}

impl ParametricEq {
    pub fn new(sr: f32) -> Self {
        Self {
            bands: [Biquad::default(); EQ_BANDS],
            last: None,
            sr,
        }
    }

    pub fn update(&mut self, p: &EqParams) {
        if self.last.as_ref() == Some(p) {
            return;
        }
        for (bq, band) in self.bands.iter_mut().zip(p.bands.iter()) {
            bq.set(Coeffs::for_band(band, self.sr));
        }
        self.last = Some(*p);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let mut y = x;
        for b in self.bands.iter_mut() {
            y = b.process(y);
        }
        y
    }

    pub fn reset(&mut self) {
        for b in self.bands.iter_mut() {
            b.reset();
        }
    }
}

/// Combined response of the full EQ in dB at the given frequencies (for UI graphing).
pub fn eq_response_db(p: &EqParams, freqs: &[f32], sr: f32) -> Vec<f32> {
    let coeffs: Vec<Coeffs> = p.bands.iter().map(|b| Coeffs::for_band(b, sr)).collect();
    freqs
        .iter()
        .map(|&f| {
            if p.enabled {
                coeffs.iter().map(|c| c.magnitude_db(f, sr)).sum()
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    #[test]
    fn peak_gain_at_center() {
        let c = Coeffs::peak(1000.0, 6.0, 1.0, SR);
        assert!((c.magnitude_db(1000.0, SR) - 6.0).abs() < 0.05);
        assert!(c.magnitude_db(100.0, SR).abs() < 0.5);
    }

    #[test]
    fn shelves_reach_target_gain() {
        let lo = Coeffs::low_shelf(200.0, 9.0, 0.7, SR);
        assert!((lo.magnitude_db(20.0, SR) - 9.0).abs() < 0.3);
        let hi = Coeffs::high_shelf(4000.0, -9.0, 0.7, SR);
        assert!((hi.magnitude_db(20000.0, SR) + 9.0).abs() < 0.5);
    }

    #[test]
    fn highpass_attenuates_low_frequencies() {
        let c = Coeffs::highpass(200.0, 0.707, SR);
        assert!(c.magnitude_db(20.0, SR) < -30.0);
        assert!(c.magnitude_db(5000.0, SR).abs() < 0.5);
    }

    #[test]
    fn lowpass_attenuates_high_frequencies() {
        let c = Coeffs::lowpass(2000.0, 0.707, SR);
        assert!(c.magnitude_db(15000.0, SR) < -30.0);
        assert!(c.magnitude_db(100.0, SR).abs() < 0.5);
    }

    #[test]
    fn zero_gain_band_is_transparent() {
        let c = Coeffs::for_band(&EqBand::default(), SR);
        assert!(c.magnitude_db(1000.0, SR).abs() < 1e-3);
    }

    #[test]
    fn filter_is_stable_on_impulse() {
        let mut b = Biquad::default();
        b.set(Coeffs::peak(80.0, 18.0, 10.0, SR));
        let mut peak = 0f32;
        for i in 0..48000 {
            let y = b.process(if i == 0 { 1.0 } else { 0.0 });
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 50.0);
    }
}
