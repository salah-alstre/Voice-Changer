//! Built-in voice presets. These are data only; the DSP chain is the real
//! processing that realises them.

use super::params::*;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub params: VoiceParams,
}

fn eq_with(gains: [(usize, f32); 4]) -> EqParams {
    let mut e = EqParams::default();
    for (i, g) in gains {
        e.bands[i].gain_db = g;
    }
    e
}

fn base() -> VoiceParams {
    VoiceParams::default()
}

pub fn builtin_presets() -> Vec<VoicePreset> {
    let mut v = Vec::new();
    let mut add = |id, name, description, params: VoiceParams| {
        v.push(VoicePreset {
            id,
            name,
            description,
            params: params.sanitized(),
        });
    };

    add(
        "natural",
        "Natural",
        "No processing. Your voice exactly as captured.",
        base(),
    );

    let mut p = base();
    p.noise_suppression = NoiseSuppressionParams {
        enabled: true,
        strength: 1.0,
    };
    p.gate = GateParams {
        enabled: true,
        threshold_db: -55.0,
        ..Default::default()
    };
    p.filters.highpass_enabled = true;
    p.filters.highpass_hz = 90.0;
    p.eq = eq_with([(1, -2.0), (4, 2.0), (5, 1.5), (0, 0.0)]);
    p.compressor.enabled = true;
    add(
        "clean-mic",
        "Clean Mic",
        "Noise suppression, gate, high-pass, gentle presence and compression.",
        p,
    );

    let mut p = base();
    p.noise_suppression = NoiseSuppressionParams {
        enabled: true,
        strength: 0.8,
    };
    p.filters.highpass_enabled = true;
    p.filters.highpass_hz = 70.0;
    p.eq = eq_with([(0, 3.0), (1, 2.0), (4, 1.0), (5, 0.5)]);
    p.compressor = CompressorParams {
        enabled: true,
        threshold_db: -20.0,
        ratio: 3.5,
        makeup_db: 4.0,
        ..Default::default()
    };
    add(
        "podcast",
        "Podcast",
        "Warm low end, tight dynamics and broadcast-style presence.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = -4.0;
    p.formant = FormantParams {
        enabled: true,
        shift: -2.5,
    };
    p.eq = eq_with([(0, 4.0), (1, 2.0), (5, -1.5), (6, -2.0)]);
    p.compressor.enabled = true;
    add(
        "deep",
        "Deep Voice",
        "Lower pitch with a longer vocal tract for a larger voice.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = 4.0;
    p.formant = FormantParams {
        enabled: true,
        shift: 3.0,
    };
    p.eq = eq_with([(0, -4.0), (1, -2.0), (5, 2.0), (6, 2.0)]);
    add(
        "bright",
        "Bright Voice",
        "Higher pitch with a smaller vocal tract.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = 10.0;
    p.formant = FormantParams {
        enabled: false,
        shift: 0.0,
    };
    p.eq = eq_with([(0, -6.0), (1, -3.0), (5, 3.0), (6, 3.0)]);
    add("chipmunk", "Chipmunk", "Pitch and formants up together.", p);

    let mut p = base();
    p.pitch.semitones = -8.0;
    p.formant = FormantParams {
        enabled: false,
        shift: 0.0,
    };
    p.eq = eq_with([(0, 5.0), (1, 3.0), (5, -3.0), (6, -4.0)]);
    p.compressor.enabled = true;
    add("giant", "Giant", "Pitch and formants down together.", p);

    let mut p = base();
    p.pitch.semitones = -2.0;
    p.formant = FormantParams {
        enabled: true,
        shift: 0.5,
    };
    p.eq = eq_with([(1, 1.0), (4, 2.0), (5, 3.0), (6, 1.0)]);
    p.compressor.enabled = true;
    add(
        "ranger",
        "Ranger",
        "Tight, gritty mid-range with a touch of drive.",
        {
            let mut q = p;
            q.effects.saturation_enabled = true;
            q.effects.saturation_drive = 3.0;
            q.effects.saturation_mix = 0.3;
            q
        },
    );

    let mut p = base();
    p.filters = FilterParams {
        highpass_enabled: true,
        highpass_hz: 400.0,
        lowpass_enabled: true,
        lowpass_hz: 3200.0,
    };
    p.effects.saturation_enabled = true;
    p.effects.saturation_drive = 6.0;
    p.effects.saturation_mix = 0.4;
    p.compressor = CompressorParams {
        enabled: true,
        threshold_db: -24.0,
        ratio: 8.0,
        makeup_db: 6.0,
        ..Default::default()
    };
    add(
        "radio",
        "Radio",
        "Narrow band, compressed and lightly distorted.",
        p,
    );

    let mut p = base();
    p.filters = FilterParams {
        highpass_enabled: true,
        highpass_hz: 600.0,
        lowpass_enabled: true,
        lowpass_hz: 2800.0,
    };
    p.eq = eq_with([(3, 4.0), (4, 3.0), (5, -2.0), (1, -4.0)]);
    p.effects.saturation_enabled = true;
    p.effects.saturation_drive = 10.0;
    p.effects.saturation_mix = 0.3;
    add(
        "telephone",
        "Telephone",
        "Classic band-limited phone line.",
        p,
    );

    let mut p = base();
    p.filters = FilterParams {
        highpass_enabled: true,
        highpass_hz: 500.0,
        lowpass_enabled: true,
        lowpass_hz: 4000.0,
    };
    p.effects.saturation_enabled = true;
    p.effects.saturation_drive = 14.0;
    p.effects.saturation_mix = 0.6;
    p.compressor = CompressorParams {
        enabled: true,
        threshold_db: -26.0,
        ratio: 10.0,
        makeup_db: 8.0,
        ..Default::default()
    };
    p.eq = eq_with([(3, 5.0), (4, 4.0), (1, -3.0), (6, -6.0)]);
    add(
        "walkie",
        "Walkie-Talkie",
        "Hard-limited narrow band with crunch.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = -3.0;
    p.formant = FormantParams {
        enabled: true,
        shift: -4.0,
    };
    p.effects.ring_enabled = true;
    p.effects.ring_hz = 45.0;
    p.effects.ring_mix = 0.45;
    p.eq = eq_with([(0, 4.0), (5, -3.0), (6, -5.0), (2, 1.0)]);
    p.reverb = ReverbParams {
        enabled: true,
        room_size: 0.4,
        decay: 0.4,
        mix: 0.12,
    };
    p.compressor.enabled = true;
    add(
        "monster",
        "Monster",
        "Low, growling and slightly metallic.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = -1.0;
    p.formant = FormantParams {
        enabled: true,
        shift: 1.0,
    };
    p.effects.ring_enabled = true;
    p.effects.ring_hz = 28.0;
    p.effects.ring_mix = 0.35;
    p.effects.vibrato_enabled = true;
    p.effects.vibrato_rate_hz = 6.0;
    p.effects.vibrato_depth = 0.25;
    p.eq = eq_with([(1, -2.0), (4, 2.0), (5, 3.0), (6, 2.0)]);
    add(
        "robot",
        "Robot",
        "Metallic ring modulation with a slight wobble.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = 2.0;
    p.formant = FormantParams {
        enabled: true,
        shift: 1.0,
    };
    p.effects.vibrato_enabled = true;
    p.effects.vibrato_rate_hz = 4.5;
    p.effects.vibrato_depth = 0.4;
    p.reverb = ReverbParams {
        enabled: true,
        room_size: 0.9,
        decay: 0.85,
        mix: 0.4,
    };
    p.delay = DelayParams {
        enabled: true,
        time_ms: 320.0,
        feedback: 0.4,
        mix: 0.25,
    };
    p.eq = eq_with([(0, -3.0), (5, 2.0), (6, 3.0), (7, 3.0)]);
    add(
        "ethereal",
        "Ethereal",
        "Airy, shimmering voice with a long tail.",
        p,
    );

    let mut p = base();
    p.reverb = ReverbParams {
        enabled: true,
        room_size: 0.8,
        decay: 0.7,
        mix: 0.3,
    };
    p.eq = eq_with([(0, 1.0), (1, 1.0), (5, 1.0), (6, 1.0)]);
    add("cathedral", "Cathedral", "Large stone space reverb.", p);

    let mut p = base();
    p.delay = DelayParams {
        enabled: true,
        time_ms: 380.0,
        feedback: 0.45,
        mix: 0.4,
    };
    p.reverb = ReverbParams {
        enabled: true,
        room_size: 0.3,
        decay: 0.3,
        mix: 0.1,
    };
    add("echo", "Echo", "Repeating slap-back echo.", p);

    let mut p = base();
    p.pitch.semitones = -5.0;
    p.formant = FormantParams {
        enabled: true,
        shift: -1.5,
    };
    p.filters = FilterParams {
        highpass_enabled: true,
        highpass_hz: 120.0,
        lowpass_enabled: true,
        lowpass_hz: 6500.0,
    };
    p.noise_suppression = NoiseSuppressionParams {
        enabled: true,
        strength: 0.6,
    };
    p.compressor = CompressorParams {
        enabled: true,
        threshold_db: -22.0,
        ratio: 4.0,
        makeup_db: 5.0,
        ..Default::default()
    };
    p.eq = eq_with([(0, 2.0), (1, 1.0), (4, -1.0), (5, -2.0)]);
    add(
        "anonymous",
        "Anonymous",
        "Altered pitch and timbre to mask a voice for privacy.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = 3.0;
    p.formant = FormantParams {
        enabled: true,
        shift: 2.0,
    };
    p.filters.highpass_enabled = true;
    p.filters.highpass_hz = 100.0;
    p.eq = eq_with([(0, -3.0), (2, -1.0), (4, 2.0), (5, 2.5)]);
    p.compressor.enabled = true;
    add(
        "female-ish",
        "Higher Tone",
        "Raised pitch and lighter timbre.",
        p,
    );

    let mut p = base();
    p.pitch.semitones = -3.0;
    p.formant = FormantParams {
        enabled: true,
        shift: -2.0,
    };
    p.eq = eq_with([(0, 3.0), (1, 2.0), (5, -1.0), (6, -1.5)]);
    p.compressor.enabled = true;
    add(
        "male-ish",
        "Lower Tone",
        "Lowered pitch and fuller timbre.",
        p,
    );

    let mut p = base();
    p.noise_suppression = NoiseSuppressionParams {
        enabled: true,
        strength: 1.0,
    };
    p.gate = GateParams {
        enabled: true,
        threshold_db: -48.0,
        attack_ms: 3.0,
        hold_ms: 120.0,
        release_ms: 150.0,
    };
    p.filters.highpass_enabled = true;
    p.filters.highpass_hz = 100.0;
    p.eq = eq_with([(1, -2.0), (3, 1.0), (4, 2.5), (5, 2.0)]);
    p.compressor = CompressorParams {
        enabled: true,
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 6.0,
        release_ms: 100.0,
        makeup_db: 4.0,
    };
    p.limiter.ceiling_db = -2.0;
    add(
        "streamer",
        "Streamer",
        "Loud, clear and consistent for live streaming.",
        p,
    );

    v
}

pub fn preset_by_id(id: &str) -> Option<VoicePreset> {
    builtin_presets().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn has_twenty_unique_presets() {
        let p = builtin_presets();
        assert_eq!(p.len(), 20);
        let ids: HashSet<_> = p.iter().map(|x| x.id).collect();
        assert_eq!(ids.len(), p.len());
    }

    #[test]
    fn presets_are_already_sanitized() {
        for p in builtin_presets() {
            assert_eq!(p.params, p.params.sanitized(), "{}", p.id);
        }
    }

    #[test]
    fn natural_is_identity_default() {
        assert_eq!(
            preset_by_id("natural").unwrap().params,
            VoiceParams::default()
        );
    }
}
