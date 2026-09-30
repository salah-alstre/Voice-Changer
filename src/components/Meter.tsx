import { useEffect, useRef, useState } from "react";
import { cx } from "./ui";
import { fmtDb, linToPos, toDb } from "../utils/format";
import type { MeterReading } from "../types";

const FLOOR = -60;

function zoneColor(pos: number): string {
  // pos is a 0..1 position across -60..0 dB: green up to -18 dB, amber to -6 dB, red above.
  const db = FLOOR + pos * -FLOOR;
  return db > -6 ? "var(--danger)" : db > -18 ? "var(--warn)" : "var(--ok)";
}

/** Peak-hold tracker: holds the highest recent value, then falls. */
function usePeakHold(value: number): number {
  const hold = useRef({ v: 0, t: 0 });
  const now = performance.now();
  if (value >= hold.current.v || now - hold.current.t > 1200) {
    hold.current = { v: value, t: now };
  }
  return hold.current.v;
}

export function Meter({
  reading, label, vertical, compact, showValue = true, peakOnly,
}: {
  reading?: Pick<MeterReading, "peak"> & Partial<MeterReading>;
  label?: string;
  vertical?: boolean;
  compact?: boolean;
  showValue?: boolean;
  peakOnly?: boolean;
}) {
  const peak = reading?.peak ?? 0;
  const rms = reading?.rms ?? 0;
  const hold = usePeakHold(peak);
  const [clipLatched, setClipLatched] = useState(false);
  useEffect(() => {
    if (reading?.clipped) setClipLatched(true);
    if (reading && (reading.clipCount ?? 0) === 0 && !reading.clipped) setClipLatched(false);
  }, [reading?.clipped, reading?.clipCount, reading]);

  const pPos = linToPos(peak);
  const rPos = linToPos(rms);
  const hPos = linToPos(hold);
  const db = toDb(peak);

  const fillStyle = (pos: number, extra?: React.CSSProperties): React.CSSProperties => ({
    background: `linear-gradient(${vertical ? "to top" : "to right"}, var(--ok) 0 ${(1 - 18 / 60) * 100 - 20}%, var(--warn) ${(1 - 18 / 60) * 100 - 20}% 90%, var(--danger) 90% 100%)`,
    ...(vertical ? { height: `${pos * 100}%`, width: "100%" } : { width: `${pos * 100}%`, height: "100%" }),
    ...extra,
  });

  return (
    <div
      className={cx("flex min-w-0", vertical ? "h-full flex-col items-center gap-1" : "flex-col gap-1")}
      role="meter"
      aria-label={label ?? "level"}
      aria-valuemin={FLOOR}
      aria-valuemax={0}
      aria-valuenow={Math.round(Math.max(FLOOR, db))}
    >
      {label && !vertical && (
        <div className="flex items-center justify-between text-[12px]">
          <span className="truncate text-muted">{label}</span>
          {showValue && <span className="font-mono tabular-nums text-fg">{fmtDb(db)}</span>}
        </div>
      )}
      <div className={cx("flex items-center gap-1.5", vertical && "h-full min-h-0 flex-1 flex-col-reverse")}>
        <div
          className={cx(
            "relative overflow-hidden rounded-[4px] bg-bg2 ring-1 ring-line",
            vertical ? "h-full w-2.5" : compact ? "h-2 flex-1" : "h-3 flex-1",
          )}
        >
          {!peakOnly && (
            <div
              className="absolute bottom-0 start-0 opacity-45"
              style={{ ...fillStyle(Math.max(rPos, 0)), transition: "none" }}
            />
          )}
          <div className="absolute bottom-0 start-0" style={{ ...fillStyle(pPos), maskImage: "none" }} />
          <div
            className="absolute"
            style={
              vertical
                ? { bottom: `${hPos * 100}%`, left: 0, right: 0, height: 2, background: zoneColor(hPos) }
                : { insetInlineStart: `calc(${hPos * 100}% - 2px)`, top: 0, bottom: 0, width: 2, background: zoneColor(hPos) }
            }
          />
        </div>
        <span
          title={clipLatched ? "Clipped" : "No clipping"}
          className="h-2.5 w-2.5 flex-none rounded-full"
          style={{ background: clipLatched ? "var(--danger)" : "var(--line)" }}
        />
      </div>
    </div>
  );
}
