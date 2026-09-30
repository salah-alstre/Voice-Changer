import { useApp } from "../stores/app";
import { useT } from "../i18n";
import { Card, PageHeader, SectionTitle, Slider, Toggle } from "../components/ui";
import { EqCurve } from "../components/EqCurve";
import { fmtHz } from "../utils/format";
import type { EqBand } from "../types";

export function Equalizer() {
  const t = useT();
  const voice = useApp((s) => s.voice);
  const edit = useApp((s) => s.editVoice);
  const defaults = useApp((s) => s.defaultParams);
  if (!voice) return null;
  const eq = voice.params.eq;

  const patchBand = (i: number, patch: Partial<EqBand>) =>
    edit((x) => ({ ...x, eq: { ...x.eq, bands: x.eq.bands.map((b, k) => (k === i ? { ...b, ...patch } : b)) } }));

  return (
    <>
      <PageHeader
        title={t("nav.equalizer")}
        subtitle={t("eq.subtitle")}
        actions={
          <>
            <span className="text-[12.5px] text-muted">{t("eq.enabled")}</span>
            <Toggle checked={eq.enabled} onChange={(v) => edit((x) => ({ ...x, eq: { ...x.eq, enabled: v } }))} label={t("eq.enabled")} />
            <button
              className="btn btn-sm"
              disabled={!defaults}
              onClick={() => defaults && edit((x) => ({ ...x, eq: { ...x.eq, bands: defaults.eq.bands.map((b) => ({ ...b })) } }))}
            >
              {t("common.reset")}
            </button>
          </>
        }
      />
      <Card className="mb-4">
        <EqCurve bands={eq.bands} onChange={patchBand} enabled={eq.enabled} label={t("eq.curve")} />
        <p className="mb-0 mt-2 text-[12px] text-muted">{t("eq.hint")}</p>
      </Card>
      <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-4">
        {eq.bands.map((b, i) => (
          <Card key={i}>
            <SectionTitle>{t("eq.band", { n: i + 1 })} · {t(`eq.kind.${b.kind}`)}</SectionTitle>
            <div className="flex flex-col gap-3">
              <Slider label={t("eq.freq")} value={Math.round(b.freq)} min={20} max={20000} step={1} format={fmtHz} onChange={(v) => patchBand(i, { freq: v })} />
              <Slider label={t("eq.gain")} value={b.gainDb} min={-18} max={18} step={0.5} defaultValue={0} format={(v) => `${v > 0 ? "+" : ""}${v} dB`} onChange={(v) => patchBand(i, { gainDb: v })} />
              <Slider label="Q" value={b.q} min={0.3} max={8} step={0.1} format={(v) => v.toFixed(1)} onChange={(v) => patchBand(i, { q: v })} />
            </div>
          </Card>
        ))}
      </div>
    </>
  );
}
