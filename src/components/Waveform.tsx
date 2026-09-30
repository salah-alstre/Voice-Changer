import { useEffect, useRef } from "react";

function cssVar(el: Element, name: string, fallback: string): string {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  return v || fallback;
}

/**
 * Peak-envelope waveform drawn on a canvas. `data` holds magnitudes in 0..1.
 * Optional `secondary` is drawn behind in a muted tone (e.g. the raw input behind the processed output).
 * `playhead` is 0..1 and draws a cursor line.
 */
export function Waveform({
  data, secondary, playhead, height = 96, label, onSeek, color = "--accent", live,
}: {
  data: number[];
  secondary?: number[];
  playhead?: number | null;
  height?: number;
  label: string;
  onSeek?: (pos: number) => void;
  color?: string;
  live?: boolean;
}) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const dpr = window.devicePixelRatio || 1;
    const w = cv.clientWidth;
    const h = cv.clientHeight;
    if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) {
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
    }
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const line = cssVar(cv, "--line", "#333");
    const main = cssVar(cv, color, "#7c6cff");
    const muted = cssVar(cv, "--muted", "#888");
    const mid = h / 2;
    ctx.strokeStyle = line;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, mid + 0.5);
    ctx.lineTo(w, mid + 0.5);
    ctx.stroke();

    const draw = (arr: number[], style: string, alpha: number) => {
      if (!arr.length) return;
      ctx.globalAlpha = alpha;
      ctx.fillStyle = style;
      const n = arr.length;
      const bw = Math.max(1, w / n);
      for (let i = 0; i < n; i++) {
        const a = Math.min(1, Math.max(0, arr[i]));
        // gentle curve so quiet speech is still visible
        const amp = Math.pow(a, 0.6) * (mid - 2);
        const x = (i / n) * w;
        ctx.fillRect(x, mid - amp, Math.max(1, bw - 0.5), Math.max(1, amp * 2));
      }
      ctx.globalAlpha = 1;
    };
    if (secondary) draw(secondary, muted, 0.4);
    draw(data, main, 0.95);

    if (playhead != null && playhead >= 0) {
      ctx.fillStyle = cssVar(cv, "--fg", "#fff");
      ctx.fillRect(Math.min(w - 2, playhead * w), 0, 2, h);
    }
  }, [data, secondary, playhead, color]);

  return (
    <canvas
      ref={ref}
      role="img"
      aria-label={label}
      data-live={live ? "1" : undefined}
      className="block w-full rounded-lg bg-bg2 ring-1 ring-line"
      style={{ height, cursor: onSeek ? "pointer" : "default" }}
      onClick={(e) => {
        if (!onSeek) return;
        const r = e.currentTarget.getBoundingClientRect();
        let p = (e.clientX - r.left) / r.width;
        if (getComputedStyle(e.currentTarget).direction === "rtl") p = 1 - p;
        onSeek(Math.min(1, Math.max(0, p)));
      }}
    />
  );
}
