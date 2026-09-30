import { describe, expect, it } from "vitest";
import { dbToPos, eventToAccelerator, fmtDb, fmtDuration, fmtHz, toDb } from "./format";

describe("format", () => {
  it("converts linear to dB", () => {
    expect(toDb(1)).toBeCloseTo(0);
    expect(toDb(0.5)).toBeCloseTo(-6.02, 1);
    expect(toDb(0)).toBe(-90);
  });
  it("maps dB to meter position", () => {
    expect(dbToPos(0)).toBe(1);
    expect(dbToPos(-60)).toBe(0);
    expect(dbToPos(-30)).toBeCloseTo(0.5);
    expect(dbToPos(6)).toBe(1);
  });
  it("formats values", () => {
    expect(fmtDb(-90)).toBe("-∞ dB");
    expect(fmtDb(3)).toBe("+3.0 dB");
    expect(fmtHz(440)).toBe("440 Hz");
    expect(fmtHz(1000)).toBe("1 kHz");
    expect(fmtHz(2500)).toBe("2.5 kHz");
    expect(fmtDuration(65000)).toBe("1:05");
  });
  it("builds accelerators", () => {
    const base = { ctrlKey: false, altKey: false, shiftKey: false, metaKey: false };
    expect(eventToAccelerator({ ...base, code: "KeyM", ctrlKey: true, shiftKey: true })).toBe("Ctrl+Shift+M");
    expect(eventToAccelerator({ ...base, code: "F9" })).toBe("F9");
    expect(eventToAccelerator({ ...base, code: "ShiftLeft", shiftKey: true })).toBeNull();
  });
});
