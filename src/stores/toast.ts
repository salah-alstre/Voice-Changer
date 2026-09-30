import { create } from "zustand";

export type ToastKind = "info" | "success" | "warn" | "error";
export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: ToastKind, text: string) => void;
  dismiss: (id: number) => void;
}

let nextId = 1;

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (kind, text) => {
    const id = nextId++;
    set((s) => ({ toasts: [...s.toasts.slice(-3), { id, kind, text }] }));
    setTimeout(() => get().dismiss(id), kind === "error" ? 7000 : 3500);
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

export const toast = {
  info: (t: string) => useToasts.getState().push("info", t),
  success: (t: string) => useToasts.getState().push("success", t),
  warn: (t: string) => useToasts.getState().push("warn", t),
  error: (t: string) => useToasts.getState().push("error", t),
};
