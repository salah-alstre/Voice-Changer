import { useEffect, useState } from "react";
import { Circle, OctagonX, Play, RotateCcw, SkipBack, SkipForward, Square, Trash2, Download } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { tOr, useT } from "../i18n";
import { Banner, Card, Chip, DeviceSelect, PageHeader, SectionTitle, Segmented, Slider, Toggle } from "../components/ui";
import { Meter } from "../components/Meter";
import { Waveform } from "../components/Waveform";
import { fmtDuration, fmtMs } from "../utils/format";

const fmtGain = (v: number) => `${v > 0 ? "+" : ""}${v} dB`;

export function VoiceTest() {
  const t = useT();
  const { capture, render, telemetry, take } = useLive();
  const settings = useApp((s) => s.settings)!;
  const update = useApp((s) => s.updateSettings);
  const voice = useApp((s) => s.voice);
  const presets = useApp((s) => s.presets);
  const setControls = useApp((s) => s.setControls);
  const [risk, setRisk] = useState<string | null>(null);
  const [seconds, setSeconds] = useState(8);
  const [which, setWhich] = useState<"original" | "processed">("processed");
  const [presetIdx, setPresetIdx] = useState(-1); // -1 = current voice settings
  const [busy, setBusy] = useState(false);

  const snap = telemetry?.monitor;
  const running = snap?.running ?? false;
  const rec = telemetry?.recorder;
  const player = telemetry?.player;
  const recording = rec?.recording ?? false;
  const inId = settings.monitorInputDevice ?? null;
  const outId = settings.monitorOutputDevice ?? null;
  const fail = (e: unknown) => toast.error(toAppError(e).message);

  // Pre-start feedback check (same input/output on open speakers is the classic howl).
  useEffect(() => {
    if (!settings.feedbackWarning) return setRisk(null);
    let live = true;
    api.feedbackRisk(inId, outId).then((r) => live && setRisk(r)).catch(() => live && setRisk(null));
    return () => { live = false; };
  }, [inId, outId, settings.feedbackWarning, capture, render]);

  const start = () => api.startMonitor(inId, outId).catch(fail);
  const stop = () => api.stopMonitor().catch(fail);

  const process = async (idx: number) => {
    setBusy(true);
    try {
      setPresetIdx(idx);
      const tk = await api.processTake(idx < 0 ? null : presets[idx].id);
      useLive.getState().setTake(tk);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const step = (d: number) => {
    if (presets.length === 0) return;
    const next = presetIdx < 0 ? (d > 0 ? 0 : presets.length - 1) : (presetIdx + d + presets.length) % presets.length;
    void process(next);
  };

  const duration = take?.durationMs ?? 0;
  const position = player?.playing ? player.positionMs / Math.max(1, player.totalMs) : undefined;
  const c = voice?.controls;

  return (
    <>
      <PageHeader
        title={t("voiceTest.title")}
        subtitle={t("voiceTest.subtitle")}
        actions={
          <>
            {running && <Chip kind="live" pulse>{t("voiceTest.liveMonitoring")}</Chip>}
            {recording && <Chip kind="live" pulse>{t("voiceTest.recording")}</Chip>}
            <button className="btn btn-danger" onClick={() => void api.emergencyStop().catch(fail)} title={t("voiceTest.emergencyHint")}>
              <OctagonX size={15} /> {t("voiceTest.emergency")}
            </button>
          </>
        }
      />

      {snap?.error && <Banner kind="error">{snap.error}</Banner>}
      {(snap?.feedbackSuspected) && <Banner kind="error">{t("voiceTest.feedbackDetected")}</Banner>}
      {!running && risk && <Banner kind="warn">{risk}</Banner>}

      <div className="grid grid-cols-[1fr_320px] gap-4">
        <div className="flex min-w-0 flex-col gap-4">
          <Card>
            <SectionTitle>{t("voiceTest.live")}</SectionTitle>
            <div className="mb-3 grid grid-cols-2 gap-3">
              <label className="text-[12px] text-muted">
                {t("voiceTest.input")}
                <DeviceSelect devices={capture} value={inId} onChange={(id) => void update({ monitorInputDevice: id })} label={t("voiceTest.input")} allowDefault defaultLabel={t("common.systemDefault")} disabled={running} className="mt-1 w-full" />
              </label>
              <label className="text-[12px] text-muted">
                {t("voiceTest.output")}
                <DeviceSelect devices={render} value={outId} onChange={(id) => void update({ monitorOutputDevice: id })} label={t("voiceTest.output")} allowDefault defaultLabel={t("common.systemDefault")} disabled={running} className="mt-1 w-full" />
              </label>
            </div>
            <div className="flex items-center gap-3">
              {running ? (
                <button className="btn btn-danger" onClick={() => void stop()}><Square size={14} /> {t("voiceTest.stop")}</button>
              ) : (
                <button className="btn btn-primary" onClick={() => void start()} disabled={recording}><Play size={14} /> {t("voiceTest.start")}</button>
              )}
              {c && (
                <Segmented<"raw" | "processed">
                  label={t("voiceTest.ab")}
                  value={c.processed ? "processed" : "raw"}
                  onChange={(v) => setControls({ processed: v === "processed" })}
                  options={[{ value: "raw", label: t("voiceTest.raw") }, { value: "processed", label: t("voiceTest.processed") }]}
                />
              )}
              <span className="flex-1" />
              {running && snap && snap.latency.totalMs > 0 && (
                <span className="text-[12.5px] text-muted">
                  {t("voiceTest.latency")}: <b className="font-mono text-fg">{fmtMs(snap.latency.totalMs)}</b>
                </span>
              )}
            </div>
            <p className="mb-0 mt-2 text-[12px] text-muted">{t("voiceTest.headphones")}</p>
          </Card>

          <Card>
            <SectionTitle right={<button className="btn btn-sm btn-ghost" onClick={() => void api.resetClips()}><RotateCcw size={13} /> {t("voiceTest.resetClips")}</button>}>
              {t("voiceTest.meters")}
            </SectionTitle>
            <div className="grid grid-cols-2 gap-x-6 gap-y-3">
              <Meter reading={snap?.micIn} label={t("voiceTest.m.micIn")} />
              <Meter reading={snap?.dspIn} label={t("voiceTest.m.dspIn")} />
              <Meter reading={snap?.dspOut} label={t("voiceTest.m.dspOut")} />
              <Meter reading={snap?.monitorOut} label={t("voiceTest.m.monitorOut")} />
            </div>
            <div className="mt-4 grid grid-cols-2 gap-3">
              <Waveform data={snap?.waveIn ?? []} label={t("voiceTest.waveIn")} height={70} live />
              <Waveform data={snap?.waveOut ?? []} label={t("voiceTest.waveOut")} height={70} live color="var(--accent2)" />
            </div>
            {running && snap && (
              <div className="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-[12px] text-muted">
                <span>{t("voiceTest.lat.capture")}: {fmtMs(snap.latency.captureMs)}</span>
                <span>{t("voiceTest.lat.ring")}: {fmtMs(snap.latency.ringMs)}</span>
                <span>{t("voiceTest.lat.dsp")}: {fmtMs(snap.latency.dspMs)}</span>
                <span>{t("voiceTest.lat.render")}: {fmtMs(snap.latency.renderMs)}</span>
                <span>{t("voiceTest.underruns")}: {snap.underruns}</span>
                <span>{t("voiceTest.overruns")}: {snap.overruns}</span>
                <span>{t("voiceTest.drift")}: {snap.driftCorrections}</span>
              </div>
            )}
            {running && snap && <p className="mb-0 mt-1 text-[11.5px] text-muted">{snap.latency.method}</p>}
          </Card>

          <Card>
            <SectionTitle>{t("voiceTest.recordTest")}</SectionTitle>
            <p className="mt-0 text-[12.5px] text-muted">{t("voiceTest.recordPrivacy")}</p>
            <div className="flex items-end gap-4">
              <Slider className="w-56" label={t("voiceTest.length")} value={seconds} min={5} max={30} format={(v) => `${v} s`} onChange={setSeconds} disabled={recording} defaultValue={8} />
              {recording ? (
                <>
                  <button className="btn btn-danger" onClick={() => void api.stopRecording().then((tk) => useLive.getState().setTake(tk)).catch(fail)}>
                    <Square size={14} /> {t("voiceTest.stopRec")}
                  </button>
                  <button className="btn btn-ghost" onClick={() => void api.cancelRecording().catch(fail)}>{t("common.cancel")}</button>
                  <span className="font-mono text-[13px]">{fmtDuration(rec?.elapsedMs ?? 0)} / {fmtDuration(rec?.targetMs ?? 0)}</span>
                </>
              ) : (
                <button className="btn btn-primary" disabled={running} title={running ? t("voiceTest.stopFirst") : undefined} onClick={() => void api.startRecording(seconds, inId).catch(fail)}>
                  <Circle size={14} /> {t("voiceTest.record")}
                </button>
              )}
            </div>
            {recording && <div className="mt-3"><Meter reading={{ peak: rec?.level ?? 0 }} label={t("voiceTest.m.micIn")} peakOnly /></div>}
            {rec?.error && <Banner kind="error">{rec.error}</Banner>}

            {take && !recording && (
              <div className="mt-4 flex flex-col gap-3">
                <Waveform
                  data={which === "processed" && take.processed ? take.processed : take.original}
                  label={which === "processed" ? t("voiceTest.processed") : t("voiceTest.original")}
                  height={90}
                  playhead={position}
                  color={which === "processed" ? "var(--accent2)" : undefined}
                />
                <div className="flex flex-wrap items-center gap-2">
                  <Segmented<"original" | "processed">
                    label={t("voiceTest.ab")}
                    value={which}
                    onChange={setWhich}
                    options={[{ value: "original", label: t("voiceTest.original") }, { value: "processed", label: t("voiceTest.processed") }]}
                  />
                  {player?.playing ? (
                    <button className="btn" onClick={() => void api.stopPlayback().catch(fail)}><Square size={14} /> {t("voiceTest.stopPlay")}</button>
                  ) : (
                    <button className="btn btn-primary" disabled={which === "processed" && !take.processed} onClick={() => void api.playTake(which, outId, c?.outputGainDb ?? 0).catch(fail)}>
                      <Play size={14} /> {which === "original" ? t("voiceTest.playOriginal") : t("voiceTest.playProcessed")}
                    </button>
                  )}
                  <span className="text-[12px] text-muted">{fmtDuration(duration)}</span>
                  <span className="flex-1" />
                  <button className="btn btn-sm" onClick={() => void api.exportTakeWav(which).then((p) => p && toast.success(t("voiceTest.saved", { path: p }))).catch(fail)}>
                    <Download size={13} /> {t("voiceTest.exportWav")}
                  </button>
                  <button className="btn btn-sm btn-danger" onClick={() => void api.discardTake().then(() => { useLive.getState().setTake(null); setPresetIdx(-1); }).catch(fail)}>
                    <Trash2 size={13} /> {t("voiceTest.delete")}
                  </button>
                </div>

                <div className="rounded-xl border border-line p-3">
                  <div className="mb-2 text-[12px] font-semibold uppercase tracking-wide text-muted">{t("voiceTest.tryPresets")}</div>
                  <div className="flex items-center gap-2">
                    <button className="btn btn-sm" aria-label={t("voiceTest.prevPreset")} disabled={busy} onClick={() => step(-1)}><SkipBack size={14} /></button>
                    <div className="min-w-0 flex-1 truncate text-center font-medium" aria-live="polite">
                      {presetIdx < 0 ? t("voiceTest.currentSettings") : tOr(t, `preset.${presets[presetIdx]?.id}.name`, presets[presetIdx]?.name ?? "")}
                    </div>
                    <button className="btn btn-sm" aria-label={t("voiceTest.nextPreset")} disabled={busy} onClick={() => step(1)}><SkipForward size={14} /></button>
                    <button className="btn btn-sm" disabled={busy} onClick={() => void process(-1)}>{t("voiceTest.useCurrent")}</button>
                  </div>
                  <p className="mb-0 mt-2 text-[12px] text-muted">{t("voiceTest.tryPresetsHint")}</p>
                </div>
              </div>
            )}
          </Card>
        </div>

        <Card className="h-fit">
          <SectionTitle>{t("voiceTest.gains")}</SectionTitle>
          {c && (
            <div className="flex flex-col gap-4">
              <Slider label={t("voiceTest.inputGain")} value={c.inputGainDb} min={-24} max={24} step={0.5} defaultValue={0} format={fmtGain} onChange={(v) => setControls({ inputGainDb: v })} />
              <Slider label={t("voiceTest.outputGain")} value={c.outputGainDb} min={-24} max={12} step={0.5} defaultValue={0} format={fmtGain} onChange={(v) => setControls({ outputGainDb: v })} />
              <Slider label={t("voiceTest.monitorGain")} value={c.monitorGainDb} min={-60} max={6} step={0.5} defaultValue={-12} format={fmtGain} onChange={(v) => setControls({ monitorGainDb: v })} />
              <div className="flex items-center justify-between">
                <span>{t("voiceTest.monitorMute")}</span>
                <Toggle label={t("voiceTest.monitorMute")} checked={c.monitorMute} onChange={(v) => setControls({ monitorMute: v })} />
              </div>
            </div>
          )}
          <div className="mt-4 border-t border-line pt-3 text-[12px] text-muted">{t("voiceTest.dspNote")}</div>
        </Card>
      </div>
    </>
  );
}
