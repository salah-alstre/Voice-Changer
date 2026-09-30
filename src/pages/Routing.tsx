import { useEffect, useState } from "react";
import { api, toAppError } from "../services/api";
import { useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { useT } from "../i18n";
import { Banner, Card, DeviceSelect, Empty, PageHeader } from "../components/ui";
import type { SessionInfo } from "../types";

function Line({ s }: { s: SessionInfo }) {
  const t = useT();
  const render = useLive((x) => x.render);
  const [route, setRoute] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    api.getAppRoute(s.pid, "render").then((r) => live && setRoute(r)).catch(() => live && setRoute(null));
    return () => { live = false; };
  }, [s.pid]);
  return (
    <li className="flex items-center gap-3 border-b border-line py-2.5 last:border-b-0">
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{s.name}</div>
        <div className="text-[12px] text-muted">PID {s.pid} · {t(`state.${s.state}`)}</div>
      </div>
      <DeviceSelect
        devices={render}
        value={route}
        label={`${t("routing.output")} — ${s.name}`}
        allowDefault
        defaultLabel={t("common.systemDefault")}
        className="w-[280px]"
        onChange={(id) => api.setAppRoute(s.pid, "render", id).then(() => setRoute(id)).catch((e) => toast.error(toAppError(e).message))}
      />
    </li>
  );
}

export function Routing() {
  const t = useT();
  const { sessions, routingSupported } = useLive();
  const list = sessions.filter((s) => !s.isSystem);
  return (
    <>
      <PageHeader title={t("nav.routing")} subtitle={t("routing.subtitle")} />
      {!routingSupported && <Banner kind="warn">{t("routing.unsupported")}</Banner>}
      <Banner kind="info">{t("routing.note")}</Banner>
      <Card>
        {list.length === 0 ? (
          <Empty title={t("mixer.empty")} text={t("mixer.emptyHint")} />
        ) : (
          <ul className="m-0 list-none p-0">{list.map((s) => <Line key={s.id} s={s} />)}</ul>
        )}
      </Card>
    </>
  );
}
