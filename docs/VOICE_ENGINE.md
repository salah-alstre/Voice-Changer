# Voice engine

Internal format: 48 kHz, mono, f32. Chain order (each module has an independent bypass):

`input gain → noise suppression → gate → filters (HP/LP + EQ) → pitch → formant → effects → delay → reverb → compressor → limiter → output gain`

| Module | Implementation |
|---|---|
| Noise suppression | RNNoise via `nnnoiseless` (BSD-3), 10 ms frames |
| Gate / compressor / limiter | Envelope followers with attack/release, soft-knee compressor, look-ahead-free peak limiter |
| Filters / EQ | RBJ biquads (LP, HP, shelves, peaking), coefficient smoothing |
| Pitch | Phase vocoder (realfft), ±12 semitones |
| Formant | Spectral-envelope shift (Chipmunk and Giant leave it off, so the vocoder moves pitch and formants together) |
| Effects | Ring modulator, vibrato, saturation; radio/telephone character comes from filters and EQ |
| Delay / reverb | Feedback delay line; Freeverb |

Real-time safety: all buffers are pre-allocated in `Chain::new`; parameter updates arrive as a full parameter block applied via `try_lock` at block boundaries with smoothed transitions, so no clicks and no blocking on the audio thread.

Presets (20): Natural, Clean Mic, Podcast, Deep Voice, Bright Voice, Ranger, Radio, Telephone, Walkie-Talkie, Monster, Robot, Ethereal, Anonymous, Higher Tone, Lower Tone, Streamer, Chipmunk, Giant, Cathedral and Echo, each a plain parameter set editable in the advanced editor.

Unit tests cover filter response, dynamics behaviour, pitch accuracy, parameter validation and preset sanity.
