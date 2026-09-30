import { Mic, MicOff } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { activeDevices, defaultOf, useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { useT } from "../i18n";
import { Banner, Card, DeviceSelect, Empty, PageHeader, SectionTitle, Slider } from "../components/ui";
import { Meter } from "../components/Meter";
import { Fader } from "../components/Fader";

export function Microphone() {
  const t = useT();
  const { capture, telemetry } = useLive();
  const settings = useApp((s) => s.settings)!;
  const voice = useApp((s) => s.voice);
  const setControls = useApp((s) => s.setControls);
  const update = useApp((s) => s.updateSettings);

  const active = activeDevices(capture);
  const selected = active.find((d) => d.id === settings.monitorInputDevice) ?? defaultOf(active);
  const isDefault = selected?.isDefault ?? false;
  const fail = (e: unknown) => toast.error(toAppError(e).message);

  // Endpoint telemetry only covers the default capture device, so show its live level only then.
  const liveLevel = isDefault && telemetry ? telemetry.mic : null;
  const muted = isDefault ? (telemetry?.mic?.muted ?? selected?.muted ?? false) : (selected?.muted ?? false);
  const volume = isDefault ? (telemetry?.mic?.volume ?? selected?.volume ?? 0) : (selected?.volume ?? 0);

  if (!selected) {
    return (
      <>
        <PageHeader title={t("nav.microphone")} />
        <Card><Empty icon={<MicOff size={28} />} title={t("mic.none")} text={t("mic.noneHint")} /></Card>
      </>
    );
  }

  return (
    <>
      <PageHeader title={t("nav.microphone")} subtitle={t("mic.subtitle")} />
      <div className="grid grid-cols-[220px_1fr] gap-4">
        <Card>
          <SectionTitle right={<Mic size={16} className="text-muted" />}>{t("mic.level")}</SectionTitle>
          <div className="flex flex-col items-center gap-3">
            <Fader
              label={t("mic.gain")}
              value={volume}
              meter={{ peak: liveLevel?.peak ?? 0 }}
              disabled={muted}
              onChange={(v) => {
                api.setDeviceVolume(selected.id, v).catch(fail);
                useLive.getState().patchDevice(selected.id, { volume: v });
              }}
            />
            <button
              className={`btn btn-sm w-full ${muted ? "btn-danger" : ""}`}
              aria-pressed={muted}
              onClick={() => {
                api.setDeviceMute(selected.id, !muted).catch(fail);
                useLive.getState().patchDevice(selected.id, { muted: !muted });
              }}
            >
              {muted ? <MicOff size={14} /> : <Mic size={14} />} {muted ? t("common.muted") : t("common.mute")}
            </button>
          </div>
        </Card>

        <div className="flex min-w-0 flex-col gap-4">
          <Card>
            <SectionTitle>{t("mic.device")}</SectionTitle>
            <DeviceSelect
              devices={capture}
              value={settings.monitorInputDevice ?? selected.id}
              label={t("voiceTest.input")}
              onChange={(id) => void update({ monitorInputDevice: id })}
              className="w-full"
            />
            <div className="mt-3 grid grid-cols-3 gap-3 text-[12.5px]">
              <div><div className="text-muted">{t("mic.sampleRate")}</div><div className="font-mono">{selected.sampleRate ? `${selected.sampleRate} Hz` : "—"}</div></div>
              <div><div className="text-muted">{t("mic.channels")}</div><div className="font-mono">{selected.channels || "—"}</div></div>
              <div><div className="text-muted">{t("mic.bits")}</div><div className="font-mono">{selected.bits || "—"}</div></div>
            </div>
            {!isDefault && <Banner kind="info">{t("mic.levelDefaultOnly")}</Banner>}
            {liveLevel && (
              <div className="mt-3"><Meter reading={{ peak: liveLevel.peak }} label={t("mic.level")} peakOnly /></div>
            )}
          </Card>

          <Card>
            <SectionTitle>{t("mic.processingGain")}</SectionTitle>
            <p className="mt-0 text-[12.5px] text-muted">{t("mic.processingHint")}</p>
            {voice && (
              <Slider
                label={t("voiceTest.inputGain")}
                value={voice.controls.inputGainDb}
                min={-24}
                max={24}
                step={0.5}
                defaultValue={0}
                format={(v) => `${v > 0 ? "+" : ""}${v} dB`}
                onChange={(v) => setControls({ inputGainDb: v })}
              />
            )}
          </Card>
        </div>
      </div>
    </>
  );
}
