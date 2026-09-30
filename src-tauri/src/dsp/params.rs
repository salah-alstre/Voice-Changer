//! Serializable, `Copy` parameter set for the voice processing chain.
//!
//! Every struct is plain-old-data so the audio thread can copy a fresh snapshot
//! out of the shared cell without allocating.

use serde::{Deserialize, Serialize};

pub const EQ_BANDS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BandKind {
    LowShelf,
    #[default]
    Peak,
    HighShelf,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EqBand {
    pub kind: BandKind,
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            kind: BandKind::Peak,
            freq: 1000.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EqParams {
    pub enabled: bool,
    pub bands: [EqBand; EQ_BANDS],
}

impl Default for EqParams {
    fn default() -> Self {
        Self {
            enabled: true,
            bands: default_eq_bands(),
        }
    }
}

pub fn default_eq_bands() -> [EqBand; EQ_BANDS] {
    let b = |kind, freq, q| EqBand {
        kind,
        freq,
        gain_db: 0.0,
        q,
    };
    [
        b(BandKind::LowShelf, 80.0, 0.7),
        b(BandKind::Peak, 200.0, 1.0),
        b(BandKind::Peak, 500.0, 1.0),
        b(BandKind::Peak, 1000.0, 1.0),
        b(BandKind::Peak, 2500.0, 1.0),
        b(BandKind::Peak, 4500.0, 1.0),
        b(BandKind::Peak, 8000.0, 1.0),
        b(BandKind::HighShelf, 12000.0, 0.7),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NoiseSuppressionParams {
    pub enabled: bool,
    /// 0 = fully dry, 1 = fully suppressed.
    pub strength: f32,
}
impl Default for NoiseSuppressionParams {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GateParams {
    pub enabled: bool,
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
}
impl Default for GateParams {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -50.0,
            attack_ms: 5.0,
            hold_ms: 80.0,
            release_ms: 120.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FilterParams {
    pub highpass_enabled: bool,
    pub highpass_hz: f32,
    pub lowpass_enabled: bool,
    pub lowpass_hz: f32,
}
impl Default for FilterParams {
    fn default() -> Self {
        Self {
            highpass_enabled: false,
            highpass_hz: 80.0,
            lowpass_enabled: false,
            lowpass_hz: 12000.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PitchParams {
    pub enabled: bool,
    pub semitones: f32,
    pub cents: f32,
}
impl Default for PitchParams {
    fn default() -> Self {
        Self {
            enabled: true,
            semitones: 0.0,
            cents: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FormantParams {
    pub enabled: bool,
    /// Spectral-envelope shift in semitones, independent of pitch.
    pub shift: f32,
}
impl Default for FormantParams {
    fn default() -> Self {
        Self {
            enabled: true,
            shift: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EffectsParams {
    pub ring_enabled: bool,
    pub ring_hz: f32,
    pub ring_mix: f32,
    pub saturation_enabled: bool,
    pub saturation_drive: f32,
    pub saturation_mix: f32,
    pub vibrato_enabled: bool,
    pub vibrato_rate_hz: f32,
    pub vibrato_depth: f32,
}
impl Default for EffectsParams {
    fn default() -> Self {
        Self {
            ring_enabled: false,
            ring_hz: 60.0,
            ring_mix: 0.7,
            saturation_enabled: false,
            saturation_drive: 4.0,
            saturation_mix: 0.5,
            vibrato_enabled: false,
            vibrato_rate_hz: 5.0,
            vibrato_depth: 0.3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompressorParams {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}
impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            makeup_db: 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LimiterParams {
    pub enabled: bool,
    pub ceiling_db: f32,
}
impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_db: -1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReverbParams {
    pub enabled: bool,
    pub room_size: f32,
    pub decay: f32,
    pub mix: f32,
}
impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            enabled: false,
            room_size: 0.5,
            decay: 0.5,
            mix: 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DelayParams {
    pub enabled: bool,
    pub time_ms: f32,
    pub feedback: f32,
    pub mix: f32,
}
impl Default for DelayParams {
    fn default() -> Self {
        Self {
            enabled: false,
            time_ms: 250.0,
            feedback: 0.35,
            mix: 0.3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VoiceParams {
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    /// Wet/dry balance of the whole chain: 0 = original, 1 = processed.
    pub mix: f32,
    pub noise_suppression: NoiseSuppressionParams,
    pub gate: GateParams,
    pub filters: FilterParams,
    pub eq: EqParams,
    pub pitch: PitchParams,
    pub formant: FormantParams,
    pub effects: EffectsParams,
    pub delay: DelayParams,
    pub reverb: ReverbParams,
    pub compressor: CompressorParams,
    pub limiter: LimiterParams,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            mix: 1.0,
            noise_suppression: Default::default(),
            gate: Default::default(),
            filters: Default::default(),
            eq: Default::default(),
            pitch: Default::default(),
            formant: Default::default(),
            effects: Default::default(),
            delay: Default::default(),
            reverb: Default::default(),
            compressor: Default::default(),
            limiter: Default::default(),
        }
    }
}

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_nan() {
        lo
    } else {
        v.clamp(lo, hi)
    }
}

impl VoiceParams {
    /// Clamp every field into the range the DSP modules support. Always call
    /// this on parameters that arrive over IPC or from disk.
    pub fn sanitized(mut self) -> Self {
        self.input_gain_db = clamp(self.input_gain_db, -24.0, 24.0);
        self.output_gain_db = clamp(self.output_gain_db, -24.0, 24.0);
        self.mix = clamp(self.mix, 0.0, 1.0);
        self.noise_suppression.strength = clamp(self.noise_suppression.strength, 0.0, 1.0);
        let g = &mut self.gate;
        g.threshold_db = clamp(g.threshold_db, -90.0, 0.0);
        g.attack_ms = clamp(g.attack_ms, 0.5, 200.0);
        g.hold_ms = clamp(g.hold_ms, 0.0, 1000.0);
        g.release_ms = clamp(g.release_ms, 5.0, 2000.0);
        let f = &mut self.filters;
        f.highpass_hz = clamp(f.highpass_hz, 20.0, 1000.0);
        f.lowpass_hz = clamp(f.lowpass_hz, 1000.0, 20000.0);
        for b in self.eq.bands.iter_mut() {
            b.freq = clamp(b.freq, 20.0, 20000.0);
            b.gain_db = clamp(b.gain_db, -18.0, 18.0);
            b.q = clamp(b.q, 0.1, 10.0);
        }
        self.pitch.semitones = clamp(self.pitch.semitones, -12.0, 12.0);
        self.pitch.cents = clamp(self.pitch.cents, -100.0, 100.0);
        self.formant.shift = clamp(self.formant.shift, -12.0, 12.0);
        let e = &mut self.effects;
        e.ring_hz = clamp(e.ring_hz, 5.0, 2000.0);
        e.ring_mix = clamp(e.ring_mix, 0.0, 1.0);
        e.saturation_drive = clamp(e.saturation_drive, 1.0, 30.0);
        e.saturation_mix = clamp(e.saturation_mix, 0.0, 1.0);
        e.vibrato_rate_hz = clamp(e.vibrato_rate_hz, 0.5, 20.0);
        e.vibrato_depth = clamp(e.vibrato_depth, 0.0, 1.0);
        let c = &mut self.compressor;
        c.threshold_db = clamp(c.threshold_db, -60.0, 0.0);
        c.ratio = clamp(c.ratio, 1.0, 20.0);
        c.attack_ms = clamp(c.attack_ms, 0.5, 200.0);
        c.release_ms = clamp(c.release_ms, 10.0, 2000.0);
        c.makeup_db = clamp(c.makeup_db, 0.0, 24.0);
        self.limiter.ceiling_db = clamp(self.limiter.ceiling_db, -12.0, 0.0);
        let r = &mut self.reverb;
        r.room_size = clamp(r.room_size, 0.0, 1.0);
        r.decay = clamp(r.decay, 0.0, 1.0);
        r.mix = clamp(r.mix, 0.0, 1.0);
        let d = &mut self.delay;
        d.time_ms = clamp(d.time_ms, 10.0, 1000.0);
        d.feedback = clamp(d.feedback, 0.0, 0.9);
        d.mix = clamp(d.mix, 0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_stable() {
        let p = VoiceParams::default();
        assert_eq!(p, p.sanitized());
    }

    #[test]
    fn sanitize_clamps_extremes_and_nan() {
        let mut p = VoiceParams::default();
        p.input_gain_db = 999.0;
        p.pitch.semitones = f32::NAN;
        p.delay.feedback = 5.0;
        p.eq.bands[3].gain_db = -100.0;
        let s = p.sanitized();
        assert_eq!(s.input_gain_db, 24.0);
        assert_eq!(s.pitch.semitones, -12.0);
        assert_eq!(s.delay.feedback, 0.9);
        assert_eq!(s.eq.bands[3].gain_db, -18.0);
    }

    #[test]
    fn serde_round_trip_with_missing_fields() {
        let p: VoiceParams = serde_json::from_str(r#"{"pitch":{"semitones":-4.0}}"#).unwrap();
        assert_eq!(p.pitch.semitones, -4.0);
        assert!(p.limiter.enabled);
        let json = serde_json::to_string(&p).unwrap();
        let back: VoiceParams = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
