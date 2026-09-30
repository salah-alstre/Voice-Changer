import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppError, Bootstrap, DeviceInfo, Diagnostics, Flow, HotkeyActionEvent, HotkeyResult,
  MonitorControls, Profile, SessionInfo, Settings, TakeInfo, Telemetry, VirtualMicStatus,
  VoiceParams, VoiceState,
} from "../types";

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Normalise whatever invoke rejects with into an AppError. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "message" in e) {
    const o = e as { code?: string; message: unknown };
    return { code: (o.code as AppError["code"]) ?? "Internal", message: String(o.message) };
  }
  return { code: "Internal", message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    // There is deliberately no fake backend: outside the desktop shell every call fails honestly.
    const err: AppError = {
      code: "Unsupported",
      message: "The audio backend is only available inside the Auralis desktop app.",
    };
    throw err;
  }
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

export const api = {
  bootstrap: () => call<Bootstrap>("get_bootstrap"),
  appReady: () => call<void>("app_ready"),
  updateSettings: (settings: Settings) => call<Settings>("update_settings", { settings }),
  setHotkeys: (hotkeysMap: Record<string, string>) => call<HotkeyResult>("set_hotkeys", { hotkeysMap }),
  runAction: (action: string) => call<string>("run_action", { action }),

  listDevices: (flow: Flow) => call<DeviceInfo[]>("list_devices", { flow }),
  setDefaultDevice: (deviceId: string) => call<void>("set_default_device", { deviceId }),
  setDeviceVolume: (deviceId: string, volume: number) => call<void>("set_device_volume", { deviceId, volume }),
  setDeviceMute: (deviceId: string, muted: boolean) => call<void>("set_device_mute", { deviceId, muted }),
  virtualMicStatus: () => call<VirtualMicStatus>("virtual_mic_status"),

  getSessions: () => call<SessionInfo[]>("get_sessions"),
  refreshSessions: () => call<void>("refresh_sessions"),
  setSessionVolume: (sessionId: string, volume: number) => call<void>("set_session_volume", { sessionId, volume }),
  setSessionMute: (sessionId: string, muted: boolean) => call<void>("set_session_mute", { sessionId, muted }),
  getAppIcon: (exePath: string) => call<string | null>("get_app_icon", { exePath }),
  routingSupported: () => call<boolean>("routing_supported"),
  getAppRoute: (pid: number, flow: Flow) => call<string | null>("get_app_route", { pid, flow }),
  setAppRoute: (pid: number, flow: Flow, deviceId: string | null) =>
    call<void>("set_app_route", { pid, flow, deviceId }),

  getVoice: () => call<VoiceState>("get_voice"),
  setVoiceParams: (params: VoiceParams, presetId: string | null) =>
    call<VoiceState>("set_voice_params", { params, presetId }),
  applyPreset: (presetId: string) => call<VoiceState>("apply_preset", { presetId }),
  setMonitorControls: (controls: MonitorControls) => call<VoiceState>("set_monitor_controls", { controls }),

  startMonitor: (inputId: string | null, outputId: string | null) =>
    call<void>("start_monitor", { inputId, outputId }),
  stopMonitor: () => call<void>("stop_monitor"),
  emergencyStop: () => call<void>("emergency_stop"),
  resetClips: () => call<void>("reset_clips"),
  feedbackRisk: (inputId: string | null, outputId: string | null) =>
    call<string | null>("feedback_risk", { inputId, outputId }),

  startRecording: (seconds: number, inputId: string | null) =>
    call<void>("start_recording", { seconds, inputId }),
  stopRecording: () => call<TakeInfo | null>("stop_recording"),
  cancelRecording: () => call<void>("cancel_recording"),
  discardTake: () => call<void>("discard_take"),
  takeInfo: () => call<TakeInfo | null>("take_info"),
  processTake: (presetId: string | null) => call<TakeInfo | null>("process_take", { presetId }),
  playTake: (which: "original" | "processed", outputId: string | null, gainDb: number) =>
    call<void>("play_take", { which, outputId, gainDb }),
  stopPlayback: () => call<void>("stop_playback"),
  exportTakeWav: (which: "original" | "processed") => call<string | null>("export_take_wav", { which }),

  listProfiles: () => call<Profile[]>("list_profiles"),
  saveProfile: (profile: Profile) => call<Profile>("save_profile", { profile }),
  createProfileFromCurrent: (name: string) => call<Profile>("create_profile_from_current", { name }),
  deleteProfile: (id: string) => call<void>("delete_profile", { id }),
  duplicateProfile: (id: string) => call<Profile>("duplicate_profile", { id }),
  activateProfile: (id: string) => call<Profile>("activate_profile", { id }),
  exportProfile: (id: string) => call<string | null>("export_profile", { id }),
  importProfile: () => call<Profile | null>("import_profile"),

  diagnostics: () => call<Diagnostics>("get_diagnostics"),
  logs: (lines?: number) => call<string[]>("get_logs", { lines }),
  diagnosticsReport: () => call<string>("diagnostics_report"),
  exportDiagnostics: () => call<string | null>("export_diagnostics"),
  restartEngine: () => call<void>("restart_engine"),
  quit: () => call<void>("quit_app"),
};

export interface EventMap {
  sessions: SessionInfo[];
  telemetry: Telemetry;
  "devices-changed": unknown;
  "voice-changed": VoiceState;
  "hotkey-action": HotkeyActionEvent;
  "take-ready": null;
  "take-error": string;
}

export async function on<K extends keyof EventMap>(
  name: K,
  cb: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<EventMap[K]>(name, (e) => cb(e.payload));
}
