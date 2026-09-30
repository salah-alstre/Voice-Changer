import { useEffect, useMemo, useRef, useState } from "react";
import { AppWindow, Headphones, Search, Star, Volume2, VolumeX } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { activeDevices, defaultOf, useLive } from "../stores/live";
import { toast } from "../stores/toast";
import { useT } from "../i18n";
import { Banner, Card, DeviceSelect, Empty, PageHeader, SectionTitle, Slider, cx } from "../components/ui";
import { Meter } from "../components/Meter";
import { Fader } from "../components/Fader";
import type { SessionInfo } from "../types";

function AppIcon({ s }: { s: SessionInfo }) {
  const icon = useLive((x) => x.icons[s.exePath]);
  const loadIcon = useLive((x) => x.loadIcon);
  useEffect(() => loadIcon(s.exePath), [s.exePath, loadIcon]);
  return icon ? (
    <img src={icon} alt="" width={28} height={28} className="rounded" />
  ) : (
    <span className="flex h-7 w-7 items-center justify-center rounded bg-panel2 text-muted">
      <AppWindow size={16} />
    </span>
  );
}

export function Mixer() {
  const t = useT();
  const { sessions, render, telemetry, routingSupported } = useLive();
  const settings = useApp((s) => s.settings)!;
  const update = useApp((s) => s.updateSettings);
  const [filter, setFilter] = useState("");
  const [solo, setSolo] = useState<string | null>(null);
  // Mute state of each session before solo was engaged, so leaving solo restores it exactly.
  const before = useRef<Record<string, boolean>>({});
  // Local overrides while dragging, so sliders feel instant.
  const [local, setLocal] = useState<Record<string, number>>({});
  const pending = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const master = defaultOf(activeDevices(render));
  const masterVol = telemetry?.master?.volume ?? master?.volume ?? 0;
  const masterMuted = telemetry?.master?.muted ?? master?.muted ?? false;

  const fail = (e: unknown) => toast.error(toAppError(e).message);

  const setVol = (s: SessionInfo, v: number) => {
    setLocal((l) => ({ ...l, [s.id]: v }));
    clearTimeout(pending.current[s.id]);
    pending.current[s.id] = setTimeout(() => {
      api.setSessionVolume(s.id, v).catch(fail).finally(() => setLocal((l) => {
        const { [s.id]: _drop, ...rest } = l;
        return rest;
      }));
    }, 25);
  };

  const toggleSolo = (s: SessionInfo) => {
    if (solo === s.id) {
      for (const o of sessions) {
        if (o.id !== s.id && o.id in before.current) void api.setSessionMute(o.id, before.current[o.id]).catch(fail);
      }
      before.current = {};
      setSolo(null);
      return;
    }
    if (solo === null) before.current = Object.fromEntries(sessions.map((o) => [o.id, o.muted]));
    setSolo(s.id);
  };

  // While solo is on, keep every other session muted (new sessions included).
  useEffect(() => {
    if (!solo) return;
    if (!sessions.some((s) => s.id === solo)) {
      setSolo(null);
      before.current = {};
      return;
    }
    for (const o of sessions) {
      if (o.id === solo) {
        if (o.muted && !before.current[o.id]) void api.setSessionMute(o.id, false).catch(fail);
      } else if (!o.muted) {
        if (!(o.id in before.current)) before.current[o.id] = false;
        void api.setSessionMute(o.id, true).catch(fail);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [solo, sessions]);

  const toggleFav = (s: SessionInfo) => {
    const key = s.exeName || s.name;
    const fav = settings.favorites.includes(key);
    void update({ favorites: fav ? settings.favorites.filter((f) => f !== key) : [...settings.favorites, key] });
  };

  const list = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const isFav = (s: SessionInfo) => settings.favorites.includes(s.exeName || s.name);
    return sessions
      .filter((s) => !q || s.name.toLowerCase().includes(q) || s.exeName.toLowerCase().includes(q))
      .sort(
        (a, b) =>
          Number(isFav(b)) - Number(isFav(a)) ||
          Number(b.state === "active") - Number(a.state === "active") ||
          a.name.localeCompare(b.name),
      );
  }, [sessions, filter, settings.favorites]);

  return (
    <>
      <PageHeader title={t("nav.mixer")} subtitle={t("mixer.subtitle")} />
      <div className="grid grid-cols-[220px_1fr] gap-4">
        <Card>
          <SectionTitle right={<Volume2 size={16} className="text-muted" />}>{t("mixer.master")}</SectionTitle>
          <div className="mb-2 truncate text-[13px] font-medium" title={master?.name}>{master?.name ?? t("common.none")}</div>
          {master && (
            <div className="flex flex-col items-center gap-3">
              <Fader
                label={t("mixer.master")}
                value={masterVol}
                meter={{ peak: telemetry?.master?.peak ?? 0 }}
                disabled={masterMuted}
                onChange={(v) => {
                  api.setDeviceVolume(master.id, v).catch(fail);
                  useLive.getState().patchDevice(master.id, { volume: v });
                }}
              />
              <button
                className={cx("btn btn-sm w-full", masterMuted && "btn-danger")}
                aria-pressed={masterMuted}
                onClick={() => {
                  api.setDeviceMute(master.id, !masterMuted).catch(fail);
                  useLive.getState().patchDevice(master.id, { muted: !masterMuted });
                }}
              >
                {masterMuted ? <VolumeX size={14} /> : <Volume2 size={14} />} {masterMuted ? t("common.muted") : t("common.mute")}
              </button>
              <div className="w-full">
                <DeviceSelect
                  devices={render}
                  value={master.id}
                  label={t("mixer.outputDevice")}
                  onChange={(id) => id && api.setDefaultDevice(id).then(() => useLive.getState().refreshDevices()).catch(fail)}
                  className="w-full"
                />
              </div>
            </div>
          )}
        </Card>

        <div className="min-w-0">
          <div className="relative mb-3">
            <Search size={15} className="pointer-events-none absolute start-3 top-1/2 -translate-y-1/2 text-muted" />
            <input
              className="input w-full !ps-9"
              placeholder={t("mixer.search")}
              aria-label={t("mixer.search")}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
          {solo && <Banner kind="info">{t("mixer.soloActive")}</Banner>}
          {list.length === 0 ? (
            <Card><Empty icon={<Headphones size={28} />} title={t("mixer.empty")} text={t("mixer.emptyHint")} /></Card>
          ) : (
            <div className="flex flex-col gap-2">
              {list.map((s) => {
                const fav = settings.favorites.includes(s.exeName || s.name);
                const vol = local[s.id] ?? s.volume;
                return (
                  <Card key={s.id} className="!p-3">
                    <div className="flex items-center gap-3">
                      <AppIcon s={s} />
                      <div className="w-[170px] flex-none min-w-0">
                        <div className="truncate font-medium" title={s.exePath}>{s.name}</div>
                        <div className="truncate text-[11.5px] text-muted">
                          {s.isSystem ? t("mixer.system") : `PID ${s.pid}`} · {t(`state.${s.state}`)}
                        </div>
                      </div>
                      <Slider
                        className="flex-1"
                        label={t("mixer.volume")}
                        value={Math.round(vol * 100)}
                        min={0}
                        max={100}
                        format={(v) => `${v}%`}
                        onChange={(v) => setVol(s, v / 100)}
                        defaultValue={100}
                      />
                      <div className="w-24 flex-none"><Meter reading={{ peak: s.peak }} compact showValue={false} peakOnly label={s.name} /></div>
                      <button
                        className={cx("btn btn-sm", s.muted && "btn-danger")}
                        aria-pressed={s.muted}
                        aria-label={`${t("common.mute")} ${s.name}`}
                        onClick={() => void api.setSessionMute(s.id, !s.muted).catch(fail)}
                      >
                        {s.muted ? <VolumeX size={14} /> : <Volume2 size={14} />}
                      </button>
                      <button
                        className={cx("btn btn-sm", solo === s.id && "btn-primary")}
                        aria-pressed={solo === s.id}
                        aria-label={`${t("mixer.solo")} ${s.name}`}
                        onClick={() => toggleSolo(s)}
                      >
                        S
                      </button>
                      <button
                        className="btn btn-sm btn-ghost"
                        aria-pressed={fav}
                        aria-label={`${t("mixer.favorite")} ${s.name}`}
                        onClick={() => toggleFav(s)}
                      >
                        <Star size={15} fill={fav ? "currentColor" : "none"} className={fav ? "text-warn" : ""} />
                      </button>
                    </div>
                    {routingSupported && !s.isSystem && <RouteSelect s={s} render={render} />}
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function RouteSelect({ s, render }: { s: SessionInfo; render: import("../types").DeviceInfo[] }) {
  const t = useT();
  const [route, setRoute] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    api.getAppRoute(s.pid, "render").then((r) => live && setRoute(r)).catch(() => live && setRoute(null));
    return () => { live = false; };
  }, [s.pid]);
  return (
    <div className="mt-2 flex items-center gap-2 ps-[40px]">
      <span className="text-[12px] text-muted">{t("mixer.routeTo")}</span>
      <DeviceSelect
        devices={render}
        value={route}
        label={`${t("mixer.routeTo")} ${s.name}`}
        allowDefault
        defaultLabel={t("common.systemDefault")}
        className="!py-1 text-[12px]"
        onChange={(id) =>
          api.setAppRoute(s.pid, "render", id).then(() => setRoute(id)).catch((e) => toast.error(toAppError(e).message))
        }
      />
    </div>
  );
}
