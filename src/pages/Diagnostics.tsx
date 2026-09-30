import { useCallback, useEffect, useState } from "react";
import { Copy, Download, RefreshCw, RotateCw } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { useT } from "../i18n";

const NOTE_KEYS: Record<string, string> = {
  "Audio is processed locally. Nothing is uploaded and raw audio is never logged.": "note.local",
  "Latency shown in the Voice Test is the measured buffer path, not the acoustic round trip.": "note.latency",
  "Per-application output routing is not supported by this Windows build.": "note.routing",
};
import { Banner, Card, Chip, PageHeader, Row, SectionTitle } from "../components/ui";
import { fmtUptime } from "../utils/format";

export function DiagnosticsPage() {
  const t = useT();
  const d = useLive((s) => s.diagnostics);
  const refresh = useLive((s) => s.refreshDiagnostics);
  const [logs, setLogs] = useState<string[]>([]);

  const load = useCallback(async () => {
    await refresh();
    try {
      setLogs(await api.logs(200));
    } catch {
      setLogs([]);
    }
  }, [refresh]);

  useEffect(() => {
    void load();
  }, [load]);

  const fail = (e: unknown) => toast.error(toAppError(e).message);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(await api.diagnosticsReport());
      toast.success(t("diag.copied"));
    } catch (e) {
      fail(e);
    }
  };
  const exp = () => api.exportDiagnostics().then((p) => p && toast.success(t("diag.exported", { path: p }))).catch(fail);
  const restart = () =>
    api.restartEngine().then(() => { toast.success(t("diag.restarted")); return load(); }).catch(fail);

  return (
    <>
      <PageHeader
        title={t("nav.diagnostics")}
        subtitle={t("diag.subtitle")}
        actions={
          <>
            <button className="btn btn-sm" onClick={() => void load()}><RefreshCw size={13} /> {t("common.refresh")}</button>
            <button className="btn btn-sm" onClick={() => void copy()}><Copy size={13} /> {t("diag.copy")}</button>
            <button className="btn btn-sm" onClick={() => void exp()}><Download size={13} /> {t("diag.export")}</button>
            <button className="btn btn-sm" onClick={() => void restart()}><RotateCw size={13} /> {t("diag.restart")}</button>
          </>
        }
      />
      {d?.monitorError && <Banner kind="error">{d.monitorError}</Banner>}
      {d ? (
        <div className="grid grid-cols-2 gap-4">
          <Card>
            <SectionTitle>{t("diag.system")}</SectionTitle>
            <Row label={t("about.version")}><span className="font-mono">{d.version}</span></Row>
            <Row label="OS"><span className="font-mono">{d.os}</span></Row>
            <Row label={t("diag.uptime")}><span className="font-mono">{fmtUptime(d.uptimeS)}</span></Row>
            <Row label={t("diag.dataDir")}><span className="break-all font-mono text-[12px]">{d.dataDir}</span></Row>
            <Row label={t("diag.logDir")}><span className="break-all font-mono text-[12px]">{d.logDir}</span></Row>
          </Card>
          <Card>
            <SectionTitle>{t("diag.audio")}</SectionTitle>
            <Row label={t("diag.renderDevices")}><span className="font-mono">{d.renderDevices}</span></Row>
            <Row label={t("diag.captureDevices")}><span className="font-mono">{d.captureDevices}</span></Row>
            <Row label={t("diag.sessions")}><span className="font-mono">{d.sessionCount}</span></Row>
            <Row label={t("diag.deviceEvents")}><span className="font-mono">{d.deviceChangeEvents}</span></Row>
            <Row label={t("diag.monitor")}><Chip kind={d.monitorRunning ? "live" : undefined}>{d.monitorRunning ? t("diag.running") : t("diag.stopped")}</Chip></Row>
            <Row label={t("nav.routing")}><Chip kind={d.routingSupported ? "ok" : "warn"}>{d.routingSupported ? t("diag.supported") : t("diag.unsupported")}</Chip></Row>
            <Row label={t("devices.virtualMic")}><Chip kind={d.virtualMic === "detected" ? "ok" : "warn"}>{d.virtualMic === "detected" ? t("virtual.detected") : t("virtual.notDetected")}</Chip></Row>
          </Card>
          {d.notes.length > 0 && (
            <Card className="col-span-2">
              <SectionTitle>{t("diag.notes")}</SectionTitle>
              <ul className="m-0 ps-5 text-[13px]">{d.notes.map((n, i) => <li key={i}>{NOTE_KEYS[n] ? t(NOTE_KEYS[n]) : n}</li>)}</ul>
            </Card>
          )}
        </div>
      ) : (
        <Banner kind="info">{t("common.loading")}</Banner>
      )}
      <Card className="mt-4">
        <SectionTitle>{t("diag.logs")}</SectionTitle>
        <pre className="m-0 max-h-[320px] overflow-auto whitespace-pre-wrap font-mono text-[11.5px] leading-relaxed text-muted" tabIndex={0}>
          {logs.length ? logs.join("\n") : t("diag.noLogs")}
        </pre>
        <p className="mb-0 mt-2 text-[12px] text-muted">{t("diag.privacy")}</p>
      </Card>
    </>
  );
}
