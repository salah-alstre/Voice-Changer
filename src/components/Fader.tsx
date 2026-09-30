import { useRef } from "react";
import { clamp } from "../utils/format";
import { Meter } from "./Meter";
import type { MeterReading } from "../types";

/** Vertical fader (0..1) with an integrated level meter. Keyboard: arrows, PageUp/Down, Home/End. */
export function Fader({
  value, onChange, label, meter, disabled, height = 150, format,
}: {
  value: number;
  onChange: (v: number) => void;
  label: string;
  meter?: Pick<MeterReading, "peak"> & Partial<MeterReading>;
  disabled?: boolean;
  height?: number;
  format?: (v: number) => string;
}) {
  const track = useRef<HTMLDivElement>(null);
  const drag = useRef(false);

  const setFromY = (clientY: number) => {
    const r = track.current?.getBoundingClientRect();
    if (!r) return;
    onChange(clamp(1 - (clientY - r.top) / r.height, 0, 1));
  };

  return (
    <div className="flex items-stretch gap-2" style={{ height }}>
      <div
        ref={track}
        role="slider"
        tabIndex={disabled ? -1 : 0}
        aria-label={label}
        aria-orientation="vertical"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(value * 100)}
        aria-valuetext={format ? format(value) : `${Math.round(value * 100)}%`}
        aria-disabled={disabled}
        className="fader rounded-lg bg-bg2 ring-1 ring-line"
        style={{ opacity: disabled ? 0.5 : 1 }}
        onPointerDown={(e) => {
          if (disabled) return;
          drag.current = true;
          e.currentTarget.setPointerCapture(e.pointerId);
          setFromY(e.clientY);
        }}
        onPointerMove={(e) => drag.current && setFromY(e.clientY)}
        onPointerUp={() => (drag.current = false)}
        onKeyDown={(e) => {
          if (disabled) return;
          const step = e.shiftKey ? 0.1 : 0.02;
          const map: Record<string, number> = {
            ArrowUp: value + step, ArrowRight: value + step, PageUp: value + 0.1,
            ArrowDown: value - step, ArrowLeft: value - step, PageDown: value - 0.1,
            Home: 0, End: 1,
          };
          if (e.key in map) {
            e.preventDefault();
            onChange(clamp(map[e.key], 0, 1));
          }
        }}
      >
        <div
          className="absolute inset-x-1 bottom-1 rounded-[6px] opacity-30"
          style={{ height: `calc(${value * 100}% - 8px)`, background: "var(--accent)" }}
        />
        <div
          className="absolute inset-x-0.5 h-3.5 rounded-[5px] border-2 border-accent bg-fg shadow"
          style={{ bottom: `calc(${value * 100}% - ${value * 14}px)` }}
        />
      </div>
      <Meter reading={meter} vertical label={label} />
    </div>
  );
}
