import { useEffect, useMemo, useState } from "react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { toast } from "../stores/toast";
import { useT } from "../i18n";
import { Banner, Card, Chip, PageHeader } from "../components/ui";
import { eventToAccelerator } from "../utils/format";
import type { HotkeyResult } from "../types";

export function Hotkeys() {
  const t = useT();
  const settings = useApp((s) => s.settings)!;
  const actions = useApp((s) => s.hotkeyActions);
  const updateSettings = useApp((s) => s.updateSettings);
  const [map, setMap] = useState<Record<string, string>>(() => ({ ...settings.hotkeys }));
  const [recording, setRecording] = useState<string | null>(null);
  const [result, setResult] = useState<HotkeyResult | null>(null);
  const [busy, setBusy] = useState(false);

  const accel = (id: string) => map[id] ?? "";
  const dirty = useMemo(
    () => actions.some(([id]) => (map[id] ?? "") !== (settings.hotkeys[id] ?? "")),
    [map, settings.hotkeys, actions],
  );

  // Local conflict detection so the user sees duplicates before applying.
  const localConflicts = useMemo(() => {
    const by: Record<string, string[]> = {};
    for (const [id] of actions) {
      const a = accel(id);
      if (a) (by[a] ??= []).push(id);
    }
    return Object.entries(by).filter(([, ids]) => ids.length > 1);
  }, [map, actions]);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return setRecording(null);
      const a = eventToAccelerator(e);
      if (!a) return;
      if (!/^(Ctrl|Alt|Shift|Super)\+/.test(a) && !/^F\d{1,2}$/.test(a)) {
        toast.warn(t("hotkeys.needModifier"));
        return;
      }
      setMap((m) => ({ ...m, [recording]: a }));
      setRecording(null);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, t]);

  const apply = async () => {
    if (localConflicts.length) return toast.error(t("hotkeys.resolveConflicts"));
    setBusy(true);
    try {
      const cleaned: Record<string, string> = {};
      for (const [id] of actions) if (accel(id)) cleaned[id] = accel(id);
      const r = await api.setHotkeys(cleaned);
      setResult(r);
      await updateSettings({ hotkeys: cleaned });
      const failed = r.results.filter((x) => !x.registered).length;
      if (failed) toast.warn(t("hotkeys.someFailed", { n: failed }));
      else toast.success(t("hotkeys.applied"));
    } catch (e) {
      toast.error(toAppError(e).message);
    } finally {
      setBusy(false);
    }
  };

  const resetDefaults = () => setMap(Object.fromEntries(actions.filter(([, d]) => d).map(([id, d]) => [id, d])));
  const errFor = (id: string) => result?.results.find((r) => r.action === id && !r.registered)?.error ?? null;
  const okFor = (id: string) => result?.results.some((r) => r.action === id && r.registered) ?? false;

  return (
    <>
      <PageHeader
        title={t("nav.hotkeys")}
        subtitle={t("hotkeys.subtitle")}
        actions={
          <>
            <button className="btn btn-sm" onClick={resetDefaults}>{t("hotkeys.defaults")}</button>
            <button className="btn btn-sm btn-primary" disabled={!dirty || busy} onClick={() => void apply()}>{t("hotkeys.apply")}</button>
          </>
        }
      />
      {localConflicts.map(([a, ids]) => (
        <Banner key={a} kind="warn">
          {t("hotkeys.conflict", { accel: a, actions: ids.map((i) => t(`hotkey.${i}`)).join(", ") })}
        </Banner>
      ))}
      {result?.conflicts.map(([a, ids]) => (
        <Banner key={`r${a}`} kind="warn">
          {t("hotkeys.conflict", { accel: a, actions: ids.map((i) => t(`hotkey.${i}`)).join(", ") })}
        </Banner>
      ))}
      <Card>
        <ul className="m-0 list-none p-0">
          {actions.map(([id]) => {
            const err = errFor(id);
            return (
              <li key={id} className="flex items-center gap-3 border-b border-line py-2.5 last:border-b-0">
                <div className="min-w-0 flex-1">
                  <div className="font-medium">{t(`hotkey.${id}`)}</div>
                  {err && <div className="mt-0.5 text-[12px]" style={{ color: "var(--danger)" }}>{err}</div>}
                </div>
                {okFor(id) && !dirty && <Chip kind="ok">{t("hotkeys.registered")}</Chip>}
                <button
                  className="btn btn-sm min-w-[150px] justify-center font-mono"
                  aria-label={`${t(`hotkey.${id}`)}: ${accel(id) || t("hotkeys.none")}`}
                  onClick={() => setRecording(recording === id ? null : id)}
                >
                  {recording === id ? t("hotkeys.pressKeys") : accel(id) || t("hotkeys.none")}
                </button>
                <button className="btn btn-sm" disabled={!accel(id)} onClick={() => setMap((m) => { const n = { ...m }; delete n[id]; return n; })}>
                  {t("hotkeys.clear")}
                </button>
              </li>
            );
          })}
        </ul>
      </Card>
      <p className="mt-3 text-[12px] text-muted">{t("hotkeys.note")}</p>
    </>
  );
}
