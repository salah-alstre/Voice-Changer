# Contributing

1. Install Rust (stable, MSVC), Node 20+, and WebView2.
2. `npm install`, then `npm run tauri dev`.
3. Before opening a PR, everything must pass:

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && npm run typecheck && npm test && npm run build
```

Rules of the project:

- No fake features: if something cannot work, return an honest error (`Unsupported`, `NotFound`, …).
- The audio render callback must not allocate, lock, log or touch the disk.
- Never log raw audio; no telemetry or network calls.
- Validate all IPC input in Rust. Do not expose shell or filesystem access to the frontend.
- All UI strings go through `src/i18n/en.json` and `ar.json` (a test enforces key parity).
- Keep `unsafe` minimal and commented.
- Prefer permissive dependency licenses (MIT/Apache/BSD); no GPL.
