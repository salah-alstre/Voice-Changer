import { Mic, Play, Square, Volume2 } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { activeDevices, defaultOf, useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { tOr, useT } from "../i18n";
import { Banner, Card, Chip, PageHeader, SectionTitle } from "../components/ui";
import { Meter } from "../components/Meter";
import { fmtMs } from "../utils/format";

export function Dashboard() {
  const t = useT();
  const { telemetry, render, capture, sessions, virtualMic, diagnostics } = useLive();
  const presets = useApp((s) => s.presets);
  const voice = useApp((s) => s.voice);
  const profiles = useApp((s) => s.profiles);
  const settings = useApp((s) => s.settings)!;
  const hotkeyActions = useApp((s) => s.hotkeyActions);
  const setPage = useApp((s) => s.setPage);

  const running = telemetry?.monitor?.running ?? false;
  const out = defaultOf(activeDevices(render));
  const inp = defaultOf(activeDevices(capture));
  const activeProfile = profiles.find((p) => p.id === settings.activeProfile);
  const preset = presets.find((p) => p.id === voice?.presetId);
  const playing = sessions.filter((s) => s.state === "active");

  const run = (action: string) =>
    api.runAction(action).then((m) => toast.info(m)).catch((e) => toast.error(toAppError(e).message));

  return (
    <>
      <PageHeader
        title={t("nav.dashboard")}
        subtitle={t("dashboard.subtitle")}
        actions={
          running ? (
            <button className="btn btn-danger" onClick={() => void api.stopMonitor()}>
              <Square size={14} /> {t("voiceTest.stop")}
            </button>
          ) : (
            <button className="btn btn-primary" onClick={() => setPage("voiceTest")}>
              <Play size={14} /> {t("dashboard.openVoiceTest")}
            </button>
          )
        }
      />
      {diagnostics?.monitorError && <Banner kind="error">{diagnostics.monitorError}</Banner>}
      <div className="grid grid-cols-3 gap-4">
        <Card>
          <SectionTitle right={<Volume2 size={16} className="text-muted" />}>{t("dashboard.output")}</SectionTitle>
          <div className="mb-2 truncate font-medium">{out?.name ?? t("common.none")}</div>
          <Meter reading={{ peak: telemetry?.master?.peak ?? 0 }} label={t("dashboard.output")} peakOnly />
          <div className="mt-2 text-[12px] text-muted">
            {t("mixer.volume")}: {Math.round((telemetry?.master?.volume ?? out?.volume ?? 0) * 100)}%
            {(telemetry?.master?.muted ?? out?.muted) ? ` · ${t("common.muted")}` : ""}
          </div>
        </Card>
        <Card>
          <SectionTitle right={<Mic size={16} className="text-muted" />}>{t("dashboard.input")}</SectionTitle>
          <div className="mb-2 truncate font-medium">{inp?.name ?? t("common.none")}</div>
          <Meter reading={{ peak: telemetry?.mic?.peak ?? 0 }} label={t("dashboard.input")} peakOnly />
          <div className="mt-2 text-[12px] text-muted">
            {t("mixer.volume")}: {Math.round((telemetry?.mic?.volume ?? inp?.volume ?? 0) * 100)}%
            {(telemetry?.mic?.muted ?? inp?.muted) ? ` · ${t("common.muted")}` : ""}
          </div>
        </Card>
        <Card>
          <SectionTitle>{t("dashboard.status")}</SectionTitle>
          <div className="flex flex-col gap-2 text-[13px]">
            <div className="flex items-center justify-between">
              <span className="text-muted">{t("dashboard.monitoring")}</span>
              {running ? <Chip kind="live" pulse>{t("common.live")}</Chip> : <Chip>{t("common.off")}</Chip>}
            </div>
            {running && telemetry && (
              <div className="flex justify-between">
                <span className="text-muted">{t("voiceTest.latency")}</span>
                <span className="font-mono">{fmtMs(telemetry.monitor?.latency.totalMs ?? 0)}</span>
              </div>
            )}
            <div className="flex items-center justify-between">
              <span className="text-muted">{t("dashboard.virtualMic")}</span>
              <Chip kind={virtualMic?.state === "detected" ? "ok" : "warn"}>
                {virtualMic?.state === "detected" ? t("virtual.detected") : t("virtual.notDetected")}
              </Chip>
            </div>
            <div className="flex justify-between">
              <span className="text-muted">{t("dashboard.profile")}</span>
              <span>{activeProfile?.name ?? t("common.none")}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted">{t("dashboard.preset")}</span>
              <span>{preset ? tOr(t, `preset.${preset.id}.name`, preset.name) : t("voiceChanger.custom")}</span>
            </div>
          </div>
        </Card>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-4">
        <Card>
          <SectionTitle>{t("dashboard.quick")}</SectionTitle>
          <div className="flex flex-wrap gap-2">
            {hotkeyActions
              .filter(([id]) => id !== "showWindow")
              .map(([id]) => (
                <button key={id} className={id === "emergencyStop" ? "btn btn-danger btn-sm" : "btn btn-sm"} onClick={() => void run(id)}>
                  {t(`hotkey.${id}`)}
                </button>
              ))}
          </div>
        </Card>
        <Card>
          <SectionTitle>{t("dashboard.playingNow")}</SectionTitle>
          {playing.length === 0 ? (
            <div className="text-muted">{t("dashboard.nothingPlaying")}</div>
          ) : (
            <ul className="m-0 list-none p-0">
              {playing.slice(0, 6).map((s) => (
                <li key={s.id} className="flex items-center justify-between gap-3 border-b border-line py-1.5 last:border-b-0">
                  <span className="truncate">{s.name}</span>
                  <span className="w-28 flex-none"><Meter reading={{ peak: s.peak }} compact showValue={false} peakOnly label={s.name} /></span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  );
}
