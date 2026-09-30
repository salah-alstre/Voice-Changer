# Audio engine

## Mixer side
Sessions, endpoint volumes and peaks are read through Core Audio (`IAudioSessionManager2`, `IAudioEndpointVolume`, `IAudioMeterInformation`) on a dedicated COM-initialised worker. Session lists are pushed to the UI when they change; peaks are polled at a throttled rate.

Default-device changes and per-app endpoint routing use undocumented policy interfaces (`audio/policy.rs`). The per-app interface id differs between Windows builds; both known ids are tried and, if neither activates, the feature reports `Unsupported`. Microsoft may change these interfaces.

## Live monitor
```
capture thread (WASAPI event-driven) ─▶ lock-free SPSC ring (48 kHz mono f32) ─▶ render thread ─▶ DSP chain ─▶ monitor device
```
- Streams are opened in shared mode as 48 kHz mono f32; Windows converts to/from the device format (`AUTOCONVERTPCM`, default-quality SRC).
- The render callback never allocates, blocks, logs or touches the disk. Shared state is atomics plus one `try_lock`ed parameter block.
- Underruns/overruns are counted and shown in Diagnostics.

## Latency
Reported latency = capture stream latency + ring occupancy + DSP algorithmic latency + render stream latency + queued render frames. It is a **buffer-path measurement**, not an acoustic loopback test; Bluetooth/driver latency after the endpoint is not included and the UI says so. Measured on a wireless headset: ~32 ms.

## Feedback protection
A heuristic watches for sustained, growing loop gain between monitor output and mic input and raises a warning; the Emergency Stop button and hotkey stop monitoring immediately. The default monitor volume is -6 dB. *The heuristic has not been verified with real acoustic feedback.*

## Recorder
Takes (5–30 s) live in RAM only; nothing is written unless the user exports.
