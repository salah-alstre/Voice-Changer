# Changelog

## 1.0.0

Initial release.

- Per-app mixer, master output, device selection, microphone controls.
- Voice Test Studio: live monitoring, four meters, waveforms, buffer-path latency, feedback warning, emergency stop, RAM-only record/playback, multi-preset comparison.
- Voice changer: 20 presets, advanced editor with per-module bypass; RBJ biquads, gate, compressor, limiter, reverb, delay, phase-vocoder pitch/formant, RNNoise suppression.
- Virtual-mic detection with honest status.
- Profiles (CRUD, import/export, activate), global hotkeys with conflict detection, tray, autostart, diagnostics.
- English/Arabic (RTL), four themes, onboarding.
- NSIS per-user installer.

### Verification status

Automated: `cargo fmt`, `cargo test`, `cargo clippy`, `tsc --noEmit`, `vitest` (19 tests), production build and installer build all pass.

Verified on real hardware (HyperX Cloud Alpha Wireless, Windows 11): device and session enumeration, per-app routing round trip, muted live-monitor run (0 underruns/overruns, ~32 ms buffer-path latency), 5 s recorder capture, icon extraction, release exe launches standalone and the UI shows live device/session data.

Not verified: changing default devices, audible monitoring and the feedback heuristic, playback of takes, routing of other apps, virtual mic with a real cable, device hot-plug callbacks, global hotkeys, tray and autostart behaviour, the installer's install/uninstall flow.
