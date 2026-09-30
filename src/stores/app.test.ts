import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/api", async () => {
  const actual = await vi.importActual<typeof import("../services/api")>("../services/api");
  return {
    ...actual,
    api: {
      bootstrap: vi.fn(),
      updateSettings: vi.fn(),
      setVoiceParams: vi.fn().mockResolvedValue(undefined),
      setMonitorControls: vi.fn().mockResolvedValue(undefined),
      applyPreset: vi.fn(),
    },
  };
});

import { api } from "../services/api";
import { useApp } from "./app";
import { useToasts } from "./toast";

const mocked = api as unknown as Record<string, ReturnType<typeof vi.fn>>;

const settings = { language: "en", theme: "dark" } as never;
const voice = { params: { pitch: { semitones: 0 } }, presetId: "clean", controls: { inputGain: 1 } } as never;

beforeEach(() => {
  vi.useFakeTimers();
  useApp.setState({ settings, voice, ready: false, loadError: null });
  useToasts.setState({ toasts: [] });
  Object.values(mocked).forEach((f) => f.mockClear());
});
afterEach(() => vi.useRealTimers());

describe("app store", () => {
  it("loads the bootstrap payload", async () => {
    mocked.bootstrap.mockResolvedValueOnce({
      version: "1.2.3", settings, profiles: [], presets: [], defaultParams: null, voice, hotkeyActions: [],
    });
    await useApp.getState().init();
    expect(useApp.getState().ready).toBe(true);
    expect(useApp.getState().version).toBe("1.2.3");
  });

  it("reports an honest load error when the backend is unavailable", async () => {
    mocked.bootstrap.mockRejectedValueOnce({ code: "Unsupported", message: "no backend" });
    await useApp.getState().init();
    expect(useApp.getState().ready).toBe(false);
    expect(useApp.getState().loadError).toBe("no backend");
  });

  it("rolls settings back and toasts when saving fails", async () => {
    mocked.updateSettings.mockRejectedValueOnce({ code: "Internal", message: "disk full" });
    await useApp.getState().updateSettings({ theme: "light" });
    expect((useApp.getState().settings as { theme: string }).theme).toBe("dark");
    expect(useToasts.getState().toasts[0]?.text).toBe("disk full");
  });

  it("debounces voice edits into a single backend call and clears the preset id", () => {
    for (let i = 1; i <= 5; i++) {
      useApp.getState().editVoice((p) => ({ ...p, pitch: { ...p.pitch, semitones: i } }));
    }
    expect(useApp.getState().voice?.presetId).toBeNull();
    expect(mocked.setVoiceParams).not.toHaveBeenCalled();
    vi.advanceTimersByTime(50);
    expect(mocked.setVoiceParams).toHaveBeenCalledTimes(1);
  });

  it("ignores backend echoes right after a local edit", () => {
    useApp.getState().editVoice((p) => p);
    const other = { ...(voice as object), presetId: "robot" } as never;
    useApp.getState().applyVoiceState(other);
    expect(useApp.getState().voice?.presetId).toBeNull();
    vi.advanceTimersByTime(500);
    useApp.getState().applyVoiceState(other);
    expect(useApp.getState().voice?.presetId).toBe("robot");
  });
});
