import { useState } from "react";
import { useApp } from "../stores/app";
import { activeDevices, defaultOf, useLive } from "../stores/live";
import { useT } from "../i18n";
import { Banner, Card, DeviceSelect, Segmented } from "./ui";
import { Meter } from "./Meter";
import type { Lang, ThemeName } from "../types";

const STEPS = ["welcome", "language", "output", "mic", "safety", "virtual", "hotkeys", "done"] as const;

export function Onboarding() {
  const t = useT();
  const settings = useApp((s) => s.settings)!;
  const update = useApp((s) => s.updateSettings);
  const hotkeyActions = useApp((s) => s.hotkeyActions);
  const { render, capture, telemetry, virtualMic } = useLive();
  const [i, setI] = useState(0);
  const step = STEPS[i];
  const last = i === STEPS.length - 1;

  const finish = () => void update({ onboardingDone: true });

  return (
    <div className="flex h-full items-center justify-center p-6">
      <Card className="w-full max-w-[640px] !p-7" role="dialog" aria-modal="true" aria-label={t("onb.title")}>
        <div className="mb-1 text-[11px] font-semibold uppercase tracking-[0.09em] text-muted">
          {t("onb.step", { n: i + 1, total: STEPS.length })}
        </div>
        <div className="mb-4 flex gap-1" aria-hidden>
          {STEPS.map((s, k) => (
            <span key={s} className={`h-1 flex-1 rounded-full ${k <= i ? "bg-accent" : "bg-line"}`} />
          ))}
        </div>
        <h1 className="m-0 mb-2 text-2xl font-semibold tracking-tight">{t(`onb.${step}.title`)}</h1>
        <p className="mt-0 text-muted">{t(`onb.${step}.text`)}</p>

        <div className="my-5 min-h-[120px]">
          {step === "language" && (
            <div className="flex flex-col gap-4">
              <div>
                <div className="mb-1.5 text-[12px] text-muted">{t("settings.language")}</div>
                <Segmented<Lang>
                  label={t("settings.language")}
                  value={settings.language}
                  onChange={(language) => void update({ language })}
                  options={[{ value: "en", label: "English" }, { value: "ar", label: "العربية" }]}
                />
              </div>
              <div>
                <div className="mb-1.5 text-[12px] text-muted">{t("settings.theme")}</div>
                <Segmented<ThemeName>
                  label={t("settings.theme")}
                  value={settings.theme}
                  onChange={(theme) => void update({ theme })}
                  options={(["dark", "light", "oled", "system"] as ThemeName[]).map((v) => ({ value: v, label: t(`theme.${v}`) }))}
                />
              </div>
            </div>
          )}
          {step === "output" && (
            <div className="flex flex-col gap-2">
              <div className="text-[12px] text-muted">{t("onb.output.current", { name: defaultOf(activeDevices(render))?.name ?? t("common.none") })}</div>
              <DeviceSelect
                devices={render}
                value={settings.monitorOutputDevice}
                onChange={(id) => void update({ monitorOutputDevice: id })}
                label={t("voiceTest.output")}
                allowDefault
                defaultLabel={t("common.systemDefault")}
              />
            </div>
          )}
          {step === "mic" && (
            <div className="flex flex-col gap-3">
              <DeviceSelect
                devices={capture}
                value={settings.monitorInputDevice}
                onChange={(id) => void update({ monitorInputDevice: id })}
                label={t("voiceTest.input")}
                allowDefault
                defaultLabel={t("common.systemDefault")}
              />
              <div className="text-[12px] text-muted">{t("onb.mic.speak")}</div>
              <Meter reading={telemetry ? { peak: telemetry.mic?.peak ?? 0, rms: telemetry.mic?.peak ?? 0, clipped: false, clipCount: 0 } : undefined} label={t("onb.mic.level")} peakOnly />
              {!telemetry && <Banner kind="info">{t("onb.mic.nolevel")}</Banner>}
            </div>
          )}
          {step === "safety" && <Banner kind="warn">{t("onb.safety.warn")}</Banner>}
          {step === "virtual" && (
            <Banner kind={virtualMic?.state === "detected" ? "info" : "warn"}>
              {virtualMic?.message ?? t("common.unknown")}
            </Banner>
          )}
          {step === "hotkeys" && (
            <ul className="m-0 list-none p-0 text-[13px]">
              {hotkeyActions.slice(0, 5).map(([id]) => (
                <li key={id} className="flex justify-between border-b border-line py-1.5">
                  <span>{t(`hotkey.${id}`)}</span>
                  <kbd className="font-mono text-muted">{settings.hotkeys[id] || t("hotkeys.unassigned")}</kbd>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex items-center justify-between">
          <button className="btn btn-ghost" onClick={finish}>{t("onb.skip")}</button>
          <div className="flex gap-2">
            <button className="btn" disabled={i === 0} onClick={() => setI(i - 1)}>{t("common.back")}</button>
            <button className="btn btn-primary" onClick={() => (last ? finish() : setI(i + 1))}>
              {last ? t("onb.start") : t("common.next")}
            </button>
          </div>
        </div>
      </Card>
    </div>
  );
}
