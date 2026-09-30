import { useRef } from "react";
import type { EqBand } from "../types";
import { clamp } from "../utils/format";

const SR = 48000;
const F_MIN = 20;
const F_MAX = 20000;
const DB_RANGE = 18;

/** Magnitude response (dB) of one RBJ biquad at `f` Hz. */
export function bandResponseDb(b: EqBand, f: number): number {
  const A = Math.pow(10, b.gainDb / 40);
  const w0 = (2 * Math.PI * b.freq) / SR;
  const cosw = Math.cos(w0);
  const sinw = Math.sin(w0);
  const alpha = sinw / (2 * Math.max(0.1, b.q));
  let b0: number, b1: number, b2: number, a0: number, a1: number, a2: number;
  if (b.kind === "peak") {
    b0 = 1 + alpha * A; b1 = -2 * cosw; b2 = 1 - alpha * A;
    a0 = 1 + alpha / A; a1 = -2 * cosw; a2 = 1 - alpha / A;
  } else {
    const sq = 2 * Math.sqrt(A) * alpha;
    if (b.kind === "lowShelf") {
      b0 = A * (A + 1 - (A - 1) * cosw + sq);
      b1 = 2 * A * (A - 1 - (A + 1) * cosw);
      b2 = A * (A + 1 - (A - 1) * cosw - sq);
      a0 = A + 1 + (A - 1) * cosw + sq;
      a1 = -2 * (A - 1 + (A + 1) * cosw);
      a2 = A + 1 + (A - 1) * cosw - sq;
    } else {
      b0 = A * (A + 1 + (A - 1) * cosw + sq);
      b1 = -2 * A * (A - 1 + (A + 1) * cosw);
      b2 = A * (A + 1 + (A - 1) * cosw - sq);
      a0 = A + 1 - (A - 1) * cosw + sq;
      a1 = 2 * (A - 1 - (A + 1) * cosw);
      a2 = A + 1 - (A - 1) * cosw - sq;
    }
  }
  const w = (2 * Math.PI * f) / SR;
  const c1 = Math.cos(w), s1 = Math.sin(w), c2 = Math.cos(2 * w), s2 = Math.sin(2 * w);
  const nr = b0 + b1 * c1 + b2 * c2, ni = -(b1 * s1 + b2 * s2);
  const dr = a0 + a1 * c1 + a2 * c2, di = -(a1 * s1 + a2 * s2);
  const mag = Math.sqrt((nr * nr + ni * ni) / (dr * dr + di * di));
  return 20 * Math.log10(Math.max(mag, 1e-9));
}

export function totalResponseDb(bands: EqBand[], f: number): number {
  return bands.reduce((s, b) => s + bandResponseDb(b, f), 0);
}

const W = 600;
const H = 220;
const xOf = (f: number) => (Math.log(f / F_MIN) / Math.log(F_MAX / F_MIN)) * W;
const fOf = (x: number) => F_MIN * Math.pow(F_MAX / F_MIN, x / W);
const yOf = (db: number) => H / 2 - (db / DB_RANGE) * (H / 2);
const dbOf = (y: number) => ((H / 2 - y) / (H / 2)) * DB_RANGE;

/** Interactive EQ curve: drag a node for frequency (x) and gain (y); wheel changes Q. */
export function EqCurve({
  bands, onChange, enabled, label,
}: { bands: EqBand[]; onChange: (i: number, patch: Partial<EqBand>) => void; enabled: boolean; label: string }) {
  const svg = useRef<SVGSVGElement>(null);
  const active = useRef<number | null>(null);

  const pts: string[] = [];
  for (let i = 0; i <= 240; i++) {
    const f = fOf((i / 240) * W);
    pts.push(`${((i / 240) * W).toFixed(1)},${yOf(clamp(totalResponseDb(bands, f), -DB_RANGE, DB_RANGE)).toFixed(1)}`);
  }

  const move = (e: React.PointerEvent) => {
    const i = active.current;
    const r = svg.current?.getBoundingClientRect();
    if (i == null || !r) return;
    let px = ((e.clientX - r.left) / r.width) * W;
    if (getComputedStyle(svg.current!).direction === "rtl") px = W - px;
    const py = ((e.clientY - r.top) / r.height) * H;
    onChange(i, {
      freq: Math.round(clamp(fOf(clamp(px, 0, W)), F_MIN, F_MAX)),
      gainDb: Math.round(clamp(dbOf(py), -DB_RANGE, DB_RANGE) * 2) / 2,
    });
  };

  const gridF = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
  return (
    <svg
      ref={svg}
      viewBox={`0 0 ${W} ${H}`}
      role="group"
      aria-label={label}
      className="block w-full touch-none rounded-lg bg-bg2 ring-1 ring-line"
      style={{ opacity: enabled ? 1 : 0.45, direction: "ltr" }}
      onPointerMove={move}
      onPointerUp={() => (active.current = null)}
      onPointerLeave={() => (active.current = null)}
    >
      {[-12, -6, 0, 6, 12].map((db) => (
        <g key={db}>
          <line x1={0} x2={W} y1={yOf(db)} y2={yOf(db)} stroke="var(--line)" strokeWidth={db === 0 ? 1.4 : 0.6} />
          <text x={4} y={yOf(db) - 3} fontSize={9} fill="var(--muted)">{db > 0 ? `+${db}` : db}</text>
        </g>
      ))}
      {gridF.map((f) => (
        <g key={f}>
          <line x1={xOf(f)} x2={xOf(f)} y1={0} y2={H} stroke="var(--line)" strokeWidth={0.6} />
          <text x={xOf(f) + 3} y={H - 4} fontSize={9} fill="var(--muted)">{f >= 1000 ? `${f / 1000}k` : f}</text>
        </g>
      ))}
      <polyline points={pts.join(" ")} fill="none" stroke="var(--accent)" strokeWidth={2.2} strokeLinejoin="round" />
      {bands.map((b, i) => (
        <circle
          key={i}
          cx={xOf(b.freq)}
          cy={yOf(clamp(b.gainDb, -DB_RANGE, DB_RANGE))}
          r={7}
          fill="var(--panel)"
          stroke="var(--accent2)"
          strokeWidth={2.5}
          className="cursor-grab"
          onPointerDown={(e) => {
            active.current = i;
            (e.currentTarget as Element).setPointerCapture(e.pointerId);
          }}
          onWheel={(e) => onChange(i, { q: clamp(Math.round((b.q + (e.deltaY < 0 ? 0.1 : -0.1)) * 10) / 10, 0.2, 8) })}
        >
          <title>{`${b.kind} ${Math.round(b.freq)} Hz, ${b.gainDb} dB, Q ${b.q}`}</title>
        </circle>
      ))}
    </svg>
  );
}
