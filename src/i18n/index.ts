import en from "./en.json";
import ar from "./ar.json";
import { useApp } from "../stores/app";
import type { Lang } from "../types";

type Dict = Record<string, string>;
const dicts: Record<Lang, Dict> = { en: en as Dict, ar: ar as Dict };

export function translate(lang: Lang, key: string, vars?: Record<string, string | number>): string {
  let s = dicts[lang][key] ?? dicts.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) s = s.replaceAll(`{${k}}`, String(v));
  }
  return s;
}

export type TFn = (key: string, vars?: Record<string, string | number>) => string;

/** React hook returning a translator bound to the active language. */
export function useT(): TFn {
  const lang = useApp((s) => s.settings?.language ?? "en");
  return (key, vars) => translate(lang, key, vars);
}

export const isRtl = (lang: Lang) => lang === "ar";
export const allDicts = dicts;

/** Translated text for a backend-supplied string; falls back to the backend text when no key exists. */
export function tOr(t: TFn, key: string, fallback: string): string {
  const s = t(key);
  return s === key ? fallback : s;
}

const HOTKEY_MESSAGES: Record<string, string> = {
  "Microphone muted": "hkmsg.micMuted",
  "Microphone unmuted": "hkmsg.micUnmuted",
  "Output muted": "hkmsg.outMuted",
  "Output unmuted": "hkmsg.outUnmuted",
  "Monitoring started": "hkmsg.monStarted",
  "Monitoring stopped": "hkmsg.monStopped",
  "Voice effect on": "hkmsg.fxOn",
  "Voice effect bypassed (raw)": "hkmsg.fxOff",
  "Emergency stop: audio stopped": "hkmsg.stopped",
  "Window toggled": "hkmsg.window",
};

/** Localises the English status line the backend returns for a hotkey action. */
export function hotkeyMessage(lang: Lang, message: string): string {
  const key = HOTKEY_MESSAGES[message];
  if (key) return translate(lang, key);
  if (message.startsWith("Preset: ")) {
    const name = message.slice(8);
    const id = builtinPresetId(name);
    return translate(lang, "hkmsg.preset", { name: id ? translate(lang, `preset.${id}.name`) : name });
  }
  return message;
}

function builtinPresetId(name: string): string | undefined {
  return Object.keys(en as Dict)
    .find((k) => k.startsWith("preset.") && k.endsWith(".name") && (en as Dict)[k] === name)
    ?.slice(7, -5);
}
