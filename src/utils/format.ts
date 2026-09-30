export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Linear amplitude (0..1) to dBFS, floored at -90. */
export function toDb(lin: number): number {
  return lin <= 0.0000316 ? -90 : 20 * Math.log10(lin);
}

/** dBFS to a 0..1 meter position across a span ending at 0 dB. */
export function dbToPos(db: number, floor = -60): number {
  return clamp((db - floor) / -floor, 0, 1);
}

export function linToPos(lin: number, floor = -60): number {
  return dbToPos(toDb(lin), floor);
}

export function fmtDb(db: number, digits = 1): string {
  if (db <= -89) return "-∞ dB";
  return `${db > 0 ? "+" : ""}${db.toFixed(digits)} dB`;
}

export function fmtHz(hz: number): string {
  if (hz < 1000) return `${Math.round(hz)} Hz`;
  const k = hz / 1000;
  return `${Number.isInteger(k) ? k : k.toFixed(k >= 10 ? 1 : 2).replace(/0+$/, "")} kHz`;
}

export function fmtMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${ms.toFixed(ms < 10 ? 1 : 0)} ms`;
}

export function fmtPct(v: number): string {
  return `${Math.round(v * 100)}%`;
}

export function fmtDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, "0")}`;
}

export function fmtUptime(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s % 60}s` : `${s}s`;
}

interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const KEY_MAP: Record<string, string> = {
  Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Delete: "Delete",
  Insert: "Insert", Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
  ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
  Minus: "-", Equal: "=", Comma: ",", Period: ".", Slash: "/", Semicolon: ";", Quote: "'",
  BracketLeft: "[", BracketRight: "]", Backslash: "\\", Backquote: "`",
};

/** Translate a keyboard event into a Tauri accelerator string, or null while only modifiers are held. */
export function eventToAccelerator(e: KeyLike): string | null {
  const c = e.code;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(c)) key = c.slice(3);
  else if (/^Digit\d$/.test(c)) key = c.slice(5);
  else if (/^F\d{1,2}$/.test(c)) key = c;
  else if (/^Numpad\d$/.test(c)) key = "Num" + c.slice(6);
  else key = KEY_MAP[c] ?? null;
  if (!key) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  return [...mods, key].join("+");
}
