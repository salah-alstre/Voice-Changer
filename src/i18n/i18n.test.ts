import { describe, expect, it } from "vitest";
import en from "./en.json";
import ar from "./ar.json";
import { hotkeyMessage, translate } from "./index";

const sources = import.meta.glob(["../**/*.ts", "../**/*.tsx", "!../**/*.test.*", "!../test/**"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const E = en as Record<string, string>;
const A = ar as Record<string, string>;
const vars = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("i18n", () => {
  it("has identical key sets in en and ar", () => {
    expect(Object.keys(A).sort()).toEqual(Object.keys(E).sort());
  });

  it("uses the same placeholders in both languages", () => {
    for (const k of Object.keys(E)) expect(vars(A[k] ?? ""), k).toEqual(vars(E[k]));
  });

  it("has no empty strings", () => {
    for (const [k, v] of [...Object.entries(E), ...Object.entries(A)]) expect(v.trim(), k).not.toBe("");
  });

  it("defines every literal key used in the sources", () => {
    const missing = new Set<string>();
    expect(Object.keys(sources).length).toBeGreaterThan(10);
    for (const [f, text] of Object.entries(sources)) {
      for (const m of text.matchAll(/\bt\(\s*"([a-zA-Z0-9_.]+)"/g)) if (!(m[1] in E)) missing.add(`${m[1]} (${f})`);
    }
    expect([...missing]).toEqual([]);
  });

  it("defines every dynamic key family", () => {
    const fams: [string, string[]][] = [
      ["theme.", ["dark", "light", "oled", "system"]],
      ["hotkey.", ["toggleMicMute", "toggleMonitor", "toggleVoiceFx", "nextPreset", "prevPreset", "emergencyStop", "masterMute", "showWindow"]],
      ["eq.kind.", ["lowShelf", "peak", "highShelf"]],
      ["state.", ["active", "inactive", "expired", "disabled", "notPresent", "unplugged"]],
      ["nav.", ["dashboard", "mixer", "microphone", "voiceTest", "voiceChanger", "equalizer", "devices", "routing", "profiles", "hotkeys", "diagnostics", "settings", "about"]],
      ["voiceChanger.m.", ["input", "noise", "gate", "filters", "pitch", "formant", "effects", "delay", "reverb", "compressor", "limiter", "output"]],
      ["onb.", ["welcome", "language", "output", "mic", "safety", "virtual", "hotkeys", "done"].flatMap((s) => [`${s}.title`, `${s}.text`])],
    ];
    for (const [p, ids] of fams) for (const id of ids) expect(E[p + id], p + id).toBeTruthy();
  });

  it("falls back to English, then to the key", () => {
    expect(translate("en", "nav.mixer")).toBe("Mixer");
    expect(translate("ar", "nav.mixer")).toBe("المازج");
    expect(translate("en", "no.such.key")).toBe("no.such.key");
    expect(translate("en", "profiles.activated", { name: "X" })).toContain("X");
  });

  it("localises hotkey status lines and preset names", () => {
    expect(hotkeyMessage("ar", "Microphone muted")).toBe("تم كتم الميكروفون");
    expect(hotkeyMessage("ar", "Preset: Clean Mic")).toBe("النمط: ميكروفون نقي");
    expect(hotkeyMessage("en", "Preset: Clean Mic")).toBe("Preset: Clean Mic");
    expect(hotkeyMessage("ar", "something else")).toBe("something else");
  });
});
