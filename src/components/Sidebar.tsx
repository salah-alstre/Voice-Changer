import {
  Activity, AudioLines, Cable, Info, Keyboard, LayoutDashboard, Mic, Route, Settings, SlidersHorizontal,
  Stethoscope, User, Volume2, WandSparkles, type LucideIcon,
} from "lucide-react";
import { useApp } from "../stores/app";
import { useLive } from "../stores/live";
import { useT } from "../i18n";
import { cx } from "./ui";

export const NAV: { id: string; icon: LucideIcon; group: number }[] = [
  { id: "dashboard", icon: LayoutDashboard, group: 0 },
  { id: "mixer", icon: SlidersHorizontal, group: 0 },
  { id: "microphone", icon: Mic, group: 0 },
  { id: "voiceTest", icon: AudioLines, group: 1 },
  { id: "voiceChanger", icon: WandSparkles, group: 1 },
  { id: "equalizer", icon: Activity, group: 1 },
  { id: "devices", icon: Volume2, group: 2 },
  { id: "routing", icon: Route, group: 2 },
  { id: "profiles", icon: User, group: 2 },
  { id: "hotkeys", icon: Keyboard, group: 3 },
  { id: "diagnostics", icon: Stethoscope, group: 3 },
  { id: "settings", icon: Settings, group: 3 },
  { id: "about", icon: Info, group: 3 },
];

export function Sidebar() {
  const t = useT();
  const page = useApp((s) => s.page);
  const setPage = useApp((s) => s.setPage);
  const running = useLive((s) => s.telemetry?.monitor?.running ?? false);
  const recording = useLive((s) => s.telemetry?.recorder?.recording ?? false);

  return (
    <nav aria-label={t("nav.label")} className="flex w-[216px] flex-none flex-col gap-0.5 overflow-y-auto border-e border-line bg-bg2 p-3">
      <div className="mb-3 flex items-center gap-2 px-2 pt-1">
        <Cable size={22} className="text-accent" aria-hidden />
        <span className="text-[17px] font-semibold tracking-tight">Auralis</span>
      </div>
      {NAV.map((n, i) => {
        const Icon = n.icon;
        const active = page === n.id;
        const showLive = n.id === "voiceTest" && (running || recording);
        return (
          <div key={n.id}>
            {i > 0 && NAV[i - 1].group !== n.group && <div className="my-2 h-px bg-line" />}
            <button
              type="button"
              aria-current={active ? "page" : undefined}
              onClick={() => setPage(n.id)}
              className={cx(
                "flex w-full cursor-pointer items-center gap-2.5 rounded-[10px] border-0 px-2.5 py-2 text-start text-[13.5px] font-medium transition-colors",
                active ? "bg-accent/15 text-fg" : "bg-transparent text-muted hover:bg-panel hover:text-fg",
              )}
            >
              <Icon size={17} className={active ? "text-accent" : ""} aria-hidden />
              <span className="flex-1 truncate">{t(`nav.${n.id}`)}</span>
              {showLive && <span className="pulse h-2 w-2 rounded-full bg-danger" aria-label={t("common.live")} />}
            </button>
          </div>
        );
      })}
    </nav>
  );
}
