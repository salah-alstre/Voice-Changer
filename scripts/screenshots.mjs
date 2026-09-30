// Captures docs/images/*.png from a running Auralis build via WebView2 remote debugging.
// Usage: set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222, start the exe, then
//   node --experimental-websocket scripts/screenshots.mjs [eval "<js>"]
import { writeFileSync, mkdirSync } from "node:fs";

const PORT = 9222;
const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
const ws = new WebSocket(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) pending.get(m.id)(m);
};
const send = (method, params = {}) =>
  new Promise((res, rej) => {
    const i = ++id;
    pending.set(i, (m) => (m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result)));
    ws.send(JSON.stringify({ id: i, method, params }));
  });
const evalJs = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  return r.result?.value;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

if (process.argv[2] === "eval") {
  console.log(JSON.stringify(await evalJs(process.argv[3])));
  process.exit(0);
}

mkdirSync("docs/images", { recursive: true });
await send("Emulation.setDeviceMetricsOverride", { width: 1320, height: 840, deviceScaleFactor: 1, mobile: false });
const labels = { dashboard: "Dashboard", mixer: "Mixer", microphone: "Microphone", voiceTest: "Voice Test", voiceChanger: "Voice Changer", devices: "Devices", profiles: "Profiles", settings: "Settings" };
const pages = [
  ["dashboard", "dashboard"],
  ["mixer", "mixer"],
  ["microphone", "microphone"],
  ["voiceTest", "voice-test"],
  ["voiceChanger", "voice-changer"],
  ["devices", "devices"],
  ["profiles", "profiles"],
  ["settings", "settings"],
];
for (const [key, file] of pages) {
  await evalJs(`[...document.querySelectorAll("nav button, aside button")].find((b) => b.innerText.trim() === "${labels[key]}")?.click()`);
  await sleep(1500);
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(`docs/images/${file}.png`, Buffer.from(shot.data, "base64"));
  console.log("saved", file);
}
process.exit(0);
