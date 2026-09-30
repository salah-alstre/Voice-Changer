import { useEffect, useState, type ComponentType } from "react";
import { api, on, toAppError } from "./services/api";
import { useApp } from "./stores/app";
import { useLive } from "./stores/live";
import { toast } from "./stores/toast";
import { hotkeyMessage, isRtl, useT } from "./i18n";
import { Sidebar } from "./components/Sidebar";
import { Toasts } from "./components/Toasts";
import { Onboarding } from "./components/Onboarding";
import { Dashboard } from "./pages/Dashboard";
import { Mixer } from "./pages/Mixer";
import { Microphone } from "./pages/Microphone";
import { VoiceTest } from "./pages/VoiceTest";
import { VoiceChanger } from "./pages/VoiceChanger";
import { Equalizer } from "./pages/Equalizer";
import { Devices } from "./pages/Devices";
import { Routing } from "./pages/Routing";
import { Profiles } from "./pages/Profiles";
import { Hotkeys } from "./pages/Hotkeys";
import { DiagnosticsPage as Diagnostics } from "./pages/Diagnostics";
import { SettingsPage } from "./pages/Settings";
import { About } from "./pages/About";

const PAGES: Record<string, ComponentType> = {
  dashboard: Dashboard,
  mixer: Mixer,
  microphone: Microphone,
  voiceTest: VoiceTest,
  voiceChanger: VoiceChanger,
  equalizer: Equalizer,
  devices: Devices,
  routing: Routing,
  profiles: Profiles,
  hotkeys: Hotkeys,
  diagnostics: Diagnostics,
  settings: SettingsPage,
  about: About,
};

function useSystemDark(): boolean {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const h = (e: MediaQueryListEvent) => setDark(e.matches);
    mq.addEventListener("change", h);
    return () => mq.removeEventListener("change", h);
  }, []);
  return dark;
}

export function App() {
  const t = useT();
  const ready = useApp((s) => s.ready);
  const loadError = useApp((s) => s.loadError);
  const settings = useApp((s) => s.settings);
  const page = useApp((s) => s.page);
  const systemDark = useSystemDark();

  // Bootstrap once.
  useEffect(() => {
    void useApp.getState().init();
  }, []);

  // Theme / direction / language / motion on <html>.
  useEffect(() => {
    if (!settings) return;
    const theme = settings.theme === "system" ? (systemDark ? "dark" : "light") : settings.theme;
    const el = document.documentElement;
    el.dataset.theme = theme;
    el.lang = settings.language;
    el.dir = isRtl(settings.language) ? "rtl" : "ltr";
    el.dataset.reduceMotion = String(settings.reduceMotion);
    document.title = "Auralis";
  }, [settings?.theme, settings?.language, settings?.reduceMotion, systemDark, settings]);

  // Backend events.
  useEffect(() => {
    if (!ready) return;
    const live = useLive.getState();
    void live.refreshDevices();
    void live.refreshSessions();
    void live.refreshVirtualMic();
    void live.refreshDiagnostics();
    const offs: Promise<() => void>[] = [
      on("telemetry", (p) => useLive.getState().setTelemetry(p)),
      on("sessions", (p) => useLive.getState().setSessions(p)),
      on("devices-changed", () => {
        void useLive.getState().refreshDevices();
        void useLive.getState().refreshVirtualMic();
      }),
      on("voice-changed", (v) => useApp.getState().applyVoiceState(v)),
      on("hotkey-action", (e) => {
        toast.info(hotkeyMessage(useApp.getState().settings?.language ?? "en", e.message));
        void api.listProfiles().then((p) => useApp.getState().setProfiles(p)).catch(() => {});
      }),
      on("take-ready", () => {
        void api.takeInfo().then((tk) => useLive.getState().setTake(tk)).catch((e) => toast.error(toAppError(e).message));
      }),
      on("take-error", (m) => toast.error(m)),
    ];
    // Tell the backend the first frame is painted so it can show the window.
    requestAnimationFrame(() => void api.appReady().catch(() => {}));
    return () => {
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, [ready]);

  if (loadError) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center" role="alert">
        <h1 className="m-0 text-xl font-semibold">Auralis</h1>
        <p className="max-w-lg text-muted">{loadError}</p>
        <button className="btn btn-primary" onClick={() => void useApp.getState().init()}>
          Retry
        </button>
      </div>
    );
  }
  if (!ready || !settings) {
    return <div className="flex h-full items-center justify-center text-muted">…</div>;
  }
  if (!settings.onboardingDone) {
    return (
      <>
        <Onboarding />
        <Toasts />
      </>
    );
  }

  const Page = PAGES[page] ?? Dashboard;
  return (
    <div className="flex h-full">
      <a href="#main" className="skip-link btn btn-primary">{t("common.skip")}</a>
      <Sidebar />
      <main id="main" className="min-w-0 flex-1 overflow-y-auto p-6">
        <div key={page} className="page-enter mx-auto max-w-[1100px]">
          <Page />
        </div>
      </main>
      <Toasts />
    </div>
  );
}
