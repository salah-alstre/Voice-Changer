import type { ReactNode } from "react";
import { RotateCcw } from "lucide-react";
import { useApp } from "../stores/app";
import { useLive } from "../stores/live";
import { tOr, useT } from "../i18n";
import { Card, Chip, PageHeader, SectionTitle, Slider, Toggle, cx } from "../components/ui";
import { fmtHz } from "../utils/format";
import type { VoiceParams } from "../types";

const db = (v: number) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`;
const pct = (v: number) => `${Math.round(v * 100)}%`;
const ms = (v: number) => `${Math.round(v)} ms`;

function Module({
  title, enabled, onToggle, children, always,
}: { title: string; enabled?: boolean; onToggle?: (v: boolean) => void; children?: ReactNode; always?: boolean }) {
  const t = useT();
  const off = !always && enabled === false;
  return (
    <Card className={cx(off && "opacity-70")}>
      <SectionTitle
        right={!always && onToggle ? <Toggle checked={!!enabled} onChange={onToggle} label={`${title} — ${t("voiceChanger.bypass")}`} /> : undefined}
      >
        {title}
      </SectionTitle>
      <div className="flex flex-col gap-3">{children}</div>
    </Card>
  );
}

export function VoiceChanger() {
  const t = useT();
  const voice = useApp((s) => s.voice);
  const presets = useApp((s) => s.presets);
  const settings = useApp((s) => s.settings)!;
  const update = useApp((s) => s.updateSettings);
  const edit = useApp((s) => s.editVoice);
  const applyPreset = useApp((s) => s.applyPreset);
  const reset = useApp((s) => s.resetVoice);
  const running = useLive((s) => s.telemetry?.monitor?.running ?? false);

  if (!voice) return null;
  const p = voice.params;
  const set = <K extends keyof VoiceParams>(k: K, patch: Partial<VoiceParams[K]>) =>
    edit((x) => ({ ...x, [k]: { ...(x[k] as object), ...patch } }) as VoiceParams);
  const adv = settings.showAdvanced;

  return (
    <>
      <PageHeader
        title={t("nav.voiceChanger")}
        subtitle={t("voiceChanger.subtitle")}
        actions={
          <>
            {running ? <Chip kind="live" pulse>{t("voiceTest.liveMonitoring")}</Chip> : <Chip>{t("voiceChanger.previewHint")}</Chip>}
            <button className="btn btn-sm" onClick={reset}><RotateCcw size={13} /> {t("common.reset")}</button>
          </>
        }
      />

      <Card className="mb-4">
        <SectionTitle>{t("voiceChanger.presets")}</SectionTitle>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2" role="radiogroup" aria-label={t("voiceChanger.presets")}>
          {presets.map((pr) => {
            const on = voice.presetId === pr.id;
            return (
              <button
                key={pr.id}
                role="radio"
                aria-checked={on}
                title={tOr(t, `preset.${pr.id}.desc`, pr.description)}
                onClick={() => void applyPreset(pr.id)}
                className={cx(
                  "cursor-pointer rounded-xl border p-2.5 text-start transition-colors",
                  on ? "border-accent bg-panel2" : "border-line bg-transparent hover:bg-panel2",
                )}
              >
                <div className="font-medium text-fg">{tOr(t, `preset.${pr.id}.name`, pr.name)}</div>
                <div className="mt-0.5 line-clamp-2 text-[11.5px] text-muted">{tOr(t, `preset.${pr.id}.desc`, pr.description)}</div>
              </button>
            );
          })}
        </div>
        {!voice.presetId && <div className="mt-2 text-[12px] text-muted">{t("voiceChanger.custom")}</div>}
      </Card>

      <div className="mb-3 flex items-center justify-between">
        <span className="text-[12.5px] text-muted">{t("voiceChanger.chain")}</span>
        <label className="flex items-center gap-2 text-[12.5px]">
          {t("settings.showAdvanced")}
          <Toggle checked={adv} onChange={(v) => void update({ showAdvanced: v })} label={t("settings.showAdvanced")} />
        </label>
      </div>

      <div className="grid grid-cols-2 gap-4 xl:grid-cols-3">
        <Module title={t("voiceChanger.m.input")} always>
          <Slider label={t("voiceTest.inputGain")} value={p.inputGainDb} min={-24} max={24} step={0.5} defaultValue={0} format={db} onChange={(v) => edit((x) => ({ ...x, inputGainDb: v }))} />
          <Slider label={t("voiceChanger.mix")} value={p.mix} min={0} max={1} step={0.01} defaultValue={1} format={pct} onChange={(v) => edit((x) => ({ ...x, mix: v }))} />
        </Module>

        <Module title={t("voiceChanger.m.noise")} enabled={p.noiseSuppression.enabled} onToggle={(v) => set("noiseSuppression", { enabled: v })}>
          <Slider label={t("voiceChanger.strength")} value={p.noiseSuppression.strength} min={0} max={1} step={0.01} defaultValue={0.8} format={pct} onChange={(v) => set("noiseSuppression", { strength: v })} />
        </Module>

        <Module title={t("voiceChanger.m.gate")} enabled={p.gate.enabled} onToggle={(v) => set("gate", { enabled: v })}>
          <Slider label={t("voiceChanger.threshold")} value={p.gate.thresholdDb} min={-80} max={0} step={1} format={db} onChange={(v) => set("gate", { thresholdDb: v })} />
          {adv && (
            <>
              <Slider label={t("voiceChanger.attack")} value={p.gate.attackMs} min={0.5} max={50} step={0.5} format={ms} onChange={(v) => set("gate", { attackMs: v })} />
              <Slider label={t("voiceChanger.hold")} value={p.gate.holdMs} min={0} max={500} step={5} format={ms} onChange={(v) => set("gate", { holdMs: v })} />
              <Slider label={t("voiceChanger.release")} value={p.gate.releaseMs} min={5} max={1000} step={5} format={ms} onChange={(v) => set("gate", { releaseMs: v })} />
            </>
          )}
        </Module>

        <Module title={t("voiceChanger.m.filters")} always>
          <div className="flex items-center justify-between text-[12.5px]">
            <span>{t("voiceChanger.highpass")}</span>
            <Toggle checked={p.filters.highpassEnabled} onChange={(v) => set("filters", { highpassEnabled: v })} label={t("voiceChanger.highpass")} />
          </div>
          <Slider label={t("voiceChanger.cutoff")} value={p.filters.highpassHz} min={20} max={600} step={1} format={fmtHz} disabled={!p.filters.highpassEnabled} onChange={(v) => set("filters", { highpassHz: v })} />
          <div className="flex items-center justify-between text-[12.5px]">
            <span>{t("voiceChanger.lowpass")}</span>
            <Toggle checked={p.filters.lowpassEnabled} onChange={(v) => set("filters", { lowpassEnabled: v })} label={t("voiceChanger.lowpass")} />
          </div>
          <Slider label={t("voiceChanger.cutoff")} value={p.filters.lowpassHz} min={1000} max={20000} step={50} format={fmtHz} disabled={!p.filters.lowpassEnabled} onChange={(v) => set("filters", { lowpassHz: v })} />
        </Module>

        <Module title={t("nav.equalizer")} enabled={p.eq.enabled} onToggle={(v) => set("eq", { enabled: v })}>
          <div className="text-[12.5px] text-muted">{t("voiceChanger.eqHint")}</div>
          <button className="btn btn-sm self-start" onClick={() => useApp.getState().setPage("equalizer")}>{t("voiceChanger.openEq")}</button>
        </Module>

        <Module title={t("voiceChanger.m.pitch")} enabled={p.pitch.enabled} onToggle={(v) => set("pitch", { enabled: v })}>
          <Slider label={t("voiceChanger.semitones")} value={p.pitch.semitones} min={-12} max={12} step={1} defaultValue={0} format={(v) => `${v > 0 ? "+" : ""}${v} st`} onChange={(v) => set("pitch", { semitones: v })} />
          <Slider label={t("voiceChanger.cents")} value={p.pitch.cents} min={-100} max={100} step={1} defaultValue={0} format={(v) => `${v > 0 ? "+" : ""}${v} ct`} onChange={(v) => set("pitch", { cents: v })} />
          <div className="text-[11.5px] text-muted">{t("voiceChanger.pitchLatency")}</div>
        </Module>

        <Module title={t("voiceChanger.m.formant")} enabled={p.formant.enabled} onToggle={(v) => set("formant", { enabled: v })}>
          <Slider label={t("voiceChanger.shift")} value={p.formant.shift} min={-1} max={1} step={0.01} defaultValue={0} format={(v) => v.toFixed(2)} onChange={(v) => set("formant", { shift: v })} />
        </Module>

        <Module title={t("voiceChanger.m.effects")} always>
          <div className="flex items-center justify-between text-[12.5px]">
            <span>{t("voiceChanger.ring")}</span>
            <Toggle checked={p.effects.ringEnabled} onChange={(v) => set("effects", { ringEnabled: v })} label={t("voiceChanger.ring")} />
          </div>
          {p.effects.ringEnabled && (
            <>
              <Slider label={t("voiceChanger.freq")} value={p.effects.ringHz} min={20} max={2000} step={1} format={fmtHz} onChange={(v) => set("effects", { ringHz: v })} />
              <Slider label={t("voiceChanger.mix")} value={p.effects.ringMix} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("effects", { ringMix: v })} />
            </>
          )}
          <div className="flex items-center justify-between text-[12.5px]">
            <span>{t("voiceChanger.saturation")}</span>
            <Toggle checked={p.effects.saturationEnabled} onChange={(v) => set("effects", { saturationEnabled: v })} label={t("voiceChanger.saturation")} />
          </div>
          {p.effects.saturationEnabled && (
            <>
              <Slider label={t("voiceChanger.drive")} value={p.effects.saturationDrive} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("effects", { saturationDrive: v })} />
              <Slider label={t("voiceChanger.mix")} value={p.effects.saturationMix} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("effects", { saturationMix: v })} />
            </>
          )}
          <div className="flex items-center justify-between text-[12.5px]">
            <span>{t("voiceChanger.vibrato")}</span>
            <Toggle checked={p.effects.vibratoEnabled} onChange={(v) => set("effects", { vibratoEnabled: v })} label={t("voiceChanger.vibrato")} />
          </div>
          {p.effects.vibratoEnabled && (
            <>
              <Slider label={t("voiceChanger.rate")} value={p.effects.vibratoRateHz} min={0.5} max={12} step={0.1} format={(v) => `${v.toFixed(1)} Hz`} onChange={(v) => set("effects", { vibratoRateHz: v })} />
              <Slider label={t("voiceChanger.depth")} value={p.effects.vibratoDepth} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("effects", { vibratoDepth: v })} />
            </>
          )}
        </Module>

        <Module title={t("voiceChanger.m.delay")} enabled={p.delay.enabled} onToggle={(v) => set("delay", { enabled: v })}>
          <Slider label={t("voiceChanger.time")} value={p.delay.timeMs} min={10} max={1000} step={5} format={ms} onChange={(v) => set("delay", { timeMs: v })} />
          <Slider label={t("voiceChanger.feedback")} value={p.delay.feedback} min={0} max={0.9} step={0.01} format={pct} onChange={(v) => set("delay", { feedback: v })} />
          <Slider label={t("voiceChanger.mix")} value={p.delay.mix} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("delay", { mix: v })} />
        </Module>

        <Module title={t("voiceChanger.m.reverb")} enabled={p.reverb.enabled} onToggle={(v) => set("reverb", { enabled: v })}>
          <Slider label={t("voiceChanger.room")} value={p.reverb.roomSize} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("reverb", { roomSize: v })} />
          <Slider label={t("voiceChanger.decay")} value={p.reverb.decay} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("reverb", { decay: v })} />
          <Slider label={t("voiceChanger.mix")} value={p.reverb.mix} min={0} max={1} step={0.01} format={pct} onChange={(v) => set("reverb", { mix: v })} />
        </Module>

        <Module title={t("voiceChanger.m.compressor")} enabled={p.compressor.enabled} onToggle={(v) => set("compressor", { enabled: v })}>
          <Slider label={t("voiceChanger.threshold")} value={p.compressor.thresholdDb} min={-60} max={0} step={1} format={db} onChange={(v) => set("compressor", { thresholdDb: v })} />
          <Slider label={t("voiceChanger.ratio")} value={p.compressor.ratio} min={1} max={20} step={0.5} format={(v) => `${v}:1`} onChange={(v) => set("compressor", { ratio: v })} />
          {adv && (
            <>
              <Slider label={t("voiceChanger.attack")} value={p.compressor.attackMs} min={0.5} max={100} step={0.5} format={ms} onChange={(v) => set("compressor", { attackMs: v })} />
              <Slider label={t("voiceChanger.release")} value={p.compressor.releaseMs} min={10} max={1000} step={5} format={ms} onChange={(v) => set("compressor", { releaseMs: v })} />
            </>
          )}
          <Slider label={t("voiceChanger.makeup")} value={p.compressor.makeupDb} min={0} max={24} step={0.5} format={db} onChange={(v) => set("compressor", { makeupDb: v })} />
        </Module>

        <Module title={t("voiceChanger.m.limiter")} enabled={p.limiter.enabled} onToggle={(v) => set("limiter", { enabled: v })}>
          <Slider label={t("voiceChanger.ceiling")} value={p.limiter.ceilingDb} min={-12} max={0} step={0.1} format={db} onChange={(v) => set("limiter", { ceilingDb: v })} />
          <div className="text-[11.5px] text-muted">{t("voiceChanger.limiterHint")}</div>
        </Module>

        <Module title={t("voiceChanger.m.output")} always>
          <Slider label={t("voiceTest.outputGain")} value={p.outputGainDb} min={-24} max={12} step={0.5} defaultValue={0} format={db} onChange={(v) => edit((x) => ({ ...x, outputGainDb: v }))} />
        </Module>
      </div>
    </>
  );
}
