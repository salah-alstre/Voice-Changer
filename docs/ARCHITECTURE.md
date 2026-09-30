# Architecture

```
React UI (src/)  ──invoke/events──▶  Tauri commands (src-tauri/src/commands.rs)
   Zustand stores                        │
   i18n, pages                           ▼
                                   AppState (app_state.rs)
                          ┌──────────┬───┴─────┬───────────┬──────────┐
                          ▼          ▼         ▼           ▼          ▼
                      audio/*       dsp/*   profiles   settings   hotkeys/tray
                   (WASAPI, COM)  (voice chain) persistence.rs diagnostics logging
```

## Backend modules (`src-tauri/src`)

| Module | Role |
|---|---|
| `audio/devices.rs` | Endpoint enumeration, volume/mute, device-change watcher, virtual-cable detection |
| `audio/sessions.rs` | Per-app audio sessions (volume, mute, peak, process info) |
| `audio/icons.rs` | Icon extraction for session executables (PNG, base64) |
| `audio/policy.rs` | Undocumented policy interfaces: default device, per-app endpoint routing (isolated, honest failure) |
| `audio/stream.rs` | WASAPI capture/render streams (event-driven, shared mode) |
| `audio/engine.rs` | Live monitor: capture → ring → DSP → render, telemetry, feedback detection |
| `audio/recorder.rs` | RAM-only test takes and playback |
| `dsp/*` | Real-time-safe voice chain, presets, parameters |
| `profiles.rs`, `settings.rs`, `persistence.rs` | JSON persistence in `%APPDATA%\Auralis` with atomic writes |
| `hotkeys.rs`, `tray.rs` | Global shortcuts (with conflict detection) and tray |
| `diagnostics.rs`, `logging.rs` | System report, rolling log files (never raw audio) |
| `error.rs` | `AppError` serialised as `{code, message}` |

## Frontend (`src/`)

Pages (13) under `pages/`, shared controls in `components/`, state in `stores/` (`app` for settings/voice/profiles, `live` for telemetry/devices/sessions, `toast`). `services/api.ts` is the only place that calls `invoke`; outside Tauri it throws an `Unsupported` error rather than returning fake data. Telemetry is pushed by the backend at a throttled rate.

## Security

- Tauri capabilities are minimal; file dialogs, hotkeys and tray live in Rust so the webview needs no fs/shell permissions.
- All command arguments are validated in Rust (ranges, ids, accelerator syntax).
- Single-instance enforced; strict CSP in `tauri.conf.json`.
