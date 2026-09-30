# Building

Prerequisites: Windows 10/11 x64, Rust stable (MSVC toolchain + Visual Studio Build Tools), Node 20+, WebView2 runtime. NSIS is downloaded automatically by the Tauri bundler.

```bash
npm install
npm run tauri dev        # development (Vite dev server + app)
npm test                 # frontend tests
npm run typecheck
(cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
npm run tauri build      # release exe + NSIS installer
```

Outputs:
- `src-tauri/target/release/auralis.exe`
- `src-tauri/target/release/bundle/nsis/Auralis_1.0.0_x64-setup.exe`

The release exe embeds the frontend and needs no dev server.

## Regenerating screenshots
Start the release exe with remote debugging, then run the capture script:

```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 src-tauri/target/release/auralis.exe &
node --experimental-websocket scripts/screenshots.mjs
```
