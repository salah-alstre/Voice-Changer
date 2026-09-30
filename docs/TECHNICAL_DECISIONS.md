# Technical decisions

| Decision | Choice | Why / alternatives |
|---|---|---|
| Shell | Tauri 2 | Small installer (~3.4 MB), Rust backend for WASAPI; Electron would need native addons and ship ~100 MB. |
| Audio API | `windows` crate (Core Audio/WASAPI) | Official bindings, MIT/Apache. `cpal` hides session/endpoint control and lacks per-app sessions; we still use the same shared-mode event-driven model. |
| Exclusive mode | Not used | Would block other apps; shared mode is sufficient (~32 ms buffer path). |
| Noise suppression | `nnnoiseless` (RNNoise port, BSD-3) | Permissive, pure Rust, real-time. Alternatives (DeepFilterNet, WebRTC NS) are heavier or awkward to license/build. |
| FFT | `realfft` (MIT/Apache) | Fast, allocation-free planning. |
| Ring buffer | `ringbuf` (lock-free SPSC) | Audio thread never blocks. |
| Pitch shifting | Own phase vocoder | Avoids GPL (Rubber Band, SoundTouch LGPL concerns). Quality is "voice-effect" grade, not studio grade. |
| Virtual mic | Detect an installed cable; no bundled driver | Writing/signing a kernel audio driver is out of scope; bundling third-party drivers needs explicit user consent. Status is `detected`/`notDetected`; there is no separate Installed/Unavailable/Error state, and it is never faked. |
| Per-app routing / default device | Undocumented `IAudioPolicyConfigFactory`, `IPolicyConfig` | Only way to do what Windows Settings does. Isolated in one module with `Unsupported` fallback. |
| Persistence | JSON in `%APPDATA%\Auralis`, atomic writes | Human-inspectable, no DB needed. |
| Frontend | React 19, TypeScript, Vite, Tailwind 4, Zustand | Requested stack; CSS-variable themes, RTL via `dir`. |
| i18n | Flat JSON keys, own tiny `translate()` | No runtime dependency; tests enforce key/placeholder parity. |
| Tests | cargo test, vitest + Testing Library | Backend DSP/logic unit tests; frontend store/i18n/ui tests with mocked API. |

Licensing: all direct dependencies are MIT/Apache-2.0/BSD; no GPL.
