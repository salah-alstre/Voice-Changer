# Troubleshooting

**Blank window / "…" forever** – WebView2 runtime missing. Install it from Microsoft and restart.

**Virtual mic says "No virtual cable detected"** – Auralis ships no driver. Install a virtual cable (e.g. VB-Cable) yourself, then restart Auralis; it will show *detected*. Select the cable as the monitor/output target and as the input in your chat app.

**Loud howling when monitoring** – feedback. Use headphones, lower monitor volume, or press Emergency stop.

**Latency seems higher than shown** – the UI shows software buffer latency only. Bluetooth headsets add tens of milliseconds that Windows cannot report.

**Per-app routing shows an error** – your Windows build may not support the per-app endpoint policy. This is reported honestly; use Windows Settings → Sound → Volume mixer instead.

**Hotkey rejected** – it conflicts with another Auralis action or is already registered by another app. Pick another combination.

**Logs** – `%APPDATA%\Auralis\logs`. Audio content is never logged. Use Diagnostics → copy report when filing issues.

**Reset** – close the app and delete `%APPDATA%\Auralis\*.json` (keeps logs).

**Arabic shows some English text** – the UI, presets, dialogs and hotkey notices are translated. Error messages returned by the Rust backend, device/app names, OS-provided names (e.g. "System sounds") and log lines are shown as provided, in English.
