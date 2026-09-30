import { X } from "lucide-react";
import { useToasts } from "../stores/toast";

const COLOR = { info: "var(--accent2)", success: "var(--ok)", warn: "var(--warn)", error: "var(--danger)" } as const;

export function Toasts() {
  const toasts = useToasts((s) => s.toasts);
  const dismiss = useToasts((s) => s.dismiss);
  return (
    <div aria-live="polite" className="pointer-events-none fixed bottom-4 end-4 z-50 flex w-[340px] flex-col gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          role={t.kind === "error" ? "alert" : "status"}
          className="page-enter pointer-events-auto flex items-start gap-2 rounded-xl border bg-panel px-3.5 py-2.5 text-[13px] shadow-lg"
          style={{ borderColor: `color-mix(in srgb, ${COLOR[t.kind]} 60%, var(--line))` }}
        >
          <span className="mt-1.5 h-2 w-2 flex-none rounded-full" style={{ background: COLOR[t.kind] }} />
          <span className="flex-1 break-words">{t.text}</span>
          <button
            type="button"
            aria-label="Dismiss"
            className="cursor-pointer border-0 bg-transparent p-0 text-muted hover:text-fg"
            onClick={() => dismiss(t.id)}
          >
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
