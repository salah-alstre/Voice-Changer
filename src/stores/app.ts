import { create } from "zustand";
import { api, toAppError } from "../services/api";
import type {
  MonitorControls, Preset, Profile, Settings, VoiceParams, VoiceState,
} from "../types";
import { toast } from "./toast";

interface AppState {
  ready: boolean;
  loadError: string | null;
  version: string;
  settings: Settings | null;
  profiles: Profile[];
  presets: Preset[];
  defaultParams: VoiceParams | null;
  voice: VoiceState | null;
  hotkeyActions: [string, string][];
  page: string;

  init: () => Promise<void>;
  setPage: (p: string) => void;
  updateSettings: (patch: Partial<Settings>) => Promise<void>;
  editVoice: (mut: (p: VoiceParams) => VoiceParams) => void;
  applyPreset: (id: string) => Promise<void>;
  resetVoice: () => void;
  setControls: (patch: Partial<MonitorControls>) => void;
  setProfiles: (p: Profile[]) => void;
  applyVoiceState: (v: VoiceState) => void;
}

let sendTimer: ReturnType<typeof setTimeout> | null = null;
let ctrlTimer: ReturnType<typeof setTimeout> | null = null;
let lastLocalEdit = 0;

export const useApp = create<AppState>((set, get) => ({
  ready: false,
  loadError: null,
  version: "",
  settings: null,
  profiles: [],
  presets: [],
  defaultParams: null,
  voice: null,
  hotkeyActions: [],
  page: "dashboard",

  init: async () => {
    try {
      const b = await api.bootstrap();
      set({
        ready: true,
        loadError: null,
        version: b.version,
        settings: b.settings,
        profiles: b.profiles,
        presets: b.presets,
        defaultParams: b.defaultParams,
        voice: b.voice,
        hotkeyActions: b.hotkeyActions,
      });
    } catch (e) {
      set({ loadError: toAppError(e).message });
    }
  },

  setPage: (page) => set({ page }),

  updateSettings: async (patch) => {
    const cur = get().settings;
    if (!cur) return;
    const next = { ...cur, ...patch };
    set({ settings: next });
    try {
      const saved = await api.updateSettings(next);
      set({ settings: saved });
    } catch (e) {
      set({ settings: cur });
      toast.error(toAppError(e).message);
    }
  },

  editVoice: (mut) => {
    const v = get().voice;
    if (!v) return;
    const params = mut(structuredClone(v.params));
    lastLocalEdit = Date.now();
    set({ voice: { ...v, params, presetId: null } });
    if (sendTimer) clearTimeout(sendTimer);
    sendTimer = setTimeout(() => {
      const cur = get().voice;
      if (!cur) return;
      api.setVoiceParams(cur.params, null).catch((e) => toast.error(toAppError(e).message));
    }, 40);
  },

  applyPreset: async (id) => {
    try {
      lastLocalEdit = Date.now();
      const v = await api.applyPreset(id);
      set({ voice: v });
    } catch (e) {
      toast.error(toAppError(e).message);
    }
  },

  resetVoice: () => {
    const d = get().defaultParams;
    if (!d) return;
    get().editVoice(() => structuredClone(d));
  },

  setControls: (patch) => {
    const v = get().voice;
    if (!v) return;
    const controls = { ...v.controls, ...patch };
    lastLocalEdit = Date.now();
    set({ voice: { ...v, controls } });
    if (ctrlTimer) clearTimeout(ctrlTimer);
    ctrlTimer = setTimeout(() => {
      const cur = get().voice;
      if (cur) api.setMonitorControls(cur.controls).catch((e) => toast.error(toAppError(e).message));
    }, 30);
  },

  setProfiles: (profiles) => set({ profiles }),

  /** Apply a voice state pushed by the backend, ignoring echoes of our own in-flight edits. */
  applyVoiceState: (v) => {
    if (Date.now() - lastLocalEdit < 400) return;
    set({ voice: v });
  },
}));
