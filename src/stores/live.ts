import { create } from "zustand";
import { api, toAppError } from "../services/api";
import type {
  DeviceInfo, Diagnostics, SessionInfo, TakeInfo, Telemetry, VirtualMicStatus,
} from "../types";
import { toast } from "./toast";

interface LiveState {
  telemetry: Telemetry | null;
  sessions: SessionInfo[];
  render: DeviceInfo[];
  capture: DeviceInfo[];
  icons: Record<string, string | null>;
  take: TakeInfo | null;
  virtualMic: VirtualMicStatus | null;
  routingSupported: boolean;
  diagnostics: Diagnostics | null;

  setTelemetry: (t: Telemetry) => void;
  setSessions: (s: SessionInfo[]) => void;
  setTake: (t: TakeInfo | null) => void;
  refreshDevices: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  refreshVirtualMic: () => Promise<void>;
  refreshDiagnostics: () => Promise<void>;
  loadIcon: (exePath: string) => void;
  patchDevice: (id: string, patch: Partial<DeviceInfo>) => void;
}

const iconRequested = new Set<string>();

export const useLive = create<LiveState>((set) => ({
  telemetry: null,
  sessions: [],
  render: [],
  capture: [],
  icons: {},
  take: null,
  virtualMic: null,
  routingSupported: false,
  diagnostics: null,

  setTelemetry: (telemetry) => set({ telemetry }),
  setSessions: (sessions) => set({ sessions }),
  setTake: (take) => set({ take }),

  refreshDevices: async () => {
    try {
      const [render, capture] = await Promise.all([api.listDevices("render"), api.listDevices("capture")]);
      set({ render, capture });
    } catch (e) {
      toast.error(toAppError(e).message);
    }
  },

  refreshSessions: async () => {
    try {
      set({ sessions: await api.getSessions() });
    } catch {
      /* the periodic event keeps the list fresh; a single failed read is not user-facing */
    }
  },

  refreshVirtualMic: async () => {
    try {
      set({ virtualMic: await api.virtualMicStatus() });
    } catch {
      set({ virtualMic: null });
    }
  },

  refreshDiagnostics: async () => {
    try {
      const [diagnostics, routingSupported] = await Promise.all([api.diagnostics(), api.routingSupported()]);
      set({ diagnostics, routingSupported });
    } catch (e) {
      toast.error(toAppError(e).message);
    }
  },

  loadIcon: (exePath) => {
    if (!exePath || iconRequested.has(exePath)) return;
    iconRequested.add(exePath);
    api
      .getAppIcon(exePath)
      .then((icon) => set((s) => ({ icons: { ...s.icons, [exePath]: icon } })))
      .catch(() => set((s) => ({ icons: { ...s.icons, [exePath]: null } })));
  },

  patchDevice: (id, patch) =>
    set((s) => ({
      render: s.render.map((d) => (d.id === id ? { ...d, ...patch } : d)),
      capture: s.capture.map((d) => (d.id === id ? { ...d, ...patch } : d)),
    })),
}));

export const activeDevices = (list: DeviceInfo[]) => list.filter((d) => d.state === "active");
export const defaultOf = (list: DeviceInfo[]) => list.find((d) => d.isDefault) ?? list[0] ?? null;
