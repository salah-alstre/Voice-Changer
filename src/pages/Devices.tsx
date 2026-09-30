import { Mic, Star, Volume2 } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { tOr, useT } from "../i18n";
import { Banner, Card, Chip, Empty, PageHeader, SectionTitle } from "../components/ui";
import type { DeviceInfo } from "../types";

function DeviceRow({ d }: { d: DeviceInfo }) {
  const t = useT();
  const fail = (e: unknown) => toast.error(toAppError(e).message);
  const usable = d.state === "active";
  return (
    <li className="flex items-center gap-3 border-b border-line py-2.5 last:border-b-0">
      <span className="text-muted">{d.flow === "render" ? <Volume2 size={18} /> : <Mic size={18} />}</span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate font-medium">{d.name}</span>
          {d.isDefault && <Chip kind="ok">{t("devices.default")}</Chip>}
          {d.isDefaultCommunications && <Chip>{t("devices.defaultComms")}</Chip>}
          {d.isVirtual && <Chip kind="warn">{t("devices.virtual")}</Chip>}
          {!usable && <Chip kind="warn">{t(`state.${d.state}`)}</Chip>}
        </div>
        <div className="mt-0.5 text-[12px] text-muted">
          {tOr(t, `ff.${d.formFactor}`, d.formFactor)}
          {d.sampleRate ? ` · ${d.sampleRate} Hz` : ""}
          {d.channels ? ` · ${d.channels} ch` : ""}
          {d.bits ? ` · ${d.bits} bit` : ""}
          {usable ? ` · ${Math.round(d.volume * 100)}%${d.muted ? ` · ${t("common.muted")}` : ""}` : ""}
        </div>
      </div>
      {usable && !d.isDefault && (
        <button
          className="btn btn-sm"
          onClick={() => api.setDefaultDevice(d.id).then(() => useLive.getState().refreshDevices()).catch(fail)}
        >
          <Star size={13} /> {t("devices.makeDefault")}
        </button>
      )}
    </li>
  );
}

export function Devices() {
  const t = useT();
  const { render, capture, virtualMic } = useLive();
  return (
    <>
      <PageHeader title={t("nav.devices")} subtitle={t("devices.subtitle")} />
      <Card className="mb-4">
        <SectionTitle>{t("devices.output")}</SectionTitle>
        {render.length === 0 ? <Empty title={t("devices.none")} /> : <ul className="m-0 list-none p-0">{render.map((d) => <DeviceRow key={d.id} d={d} />)}</ul>}
      </Card>
      <Card className="mb-4">
        <SectionTitle>{t("devices.input")}</SectionTitle>
        {capture.length === 0 ? <Empty title={t("devices.none")} /> : <ul className="m-0 list-none p-0">{capture.map((d) => <DeviceRow key={d.id} d={d} />)}</ul>}
      </Card>
      <Card>
        <SectionTitle right={<Chip kind={virtualMic?.state === "detected" ? "ok" : "warn"}>{virtualMic?.state === "detected" ? t("virtual.detected") : t("virtual.notDetected")}</Chip>}>
          {t("devices.virtualMic")}
        </SectionTitle>
        <Banner kind="info">{virtualMic?.message ?? t("common.unknown")}</Banner>
        <p className="m-0 text-[12.5px] text-muted">{t("devices.virtualHint")}</p>
      </Card>
    </>
  );
}
