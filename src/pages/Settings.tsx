import { useApp } from "../stores/app";
import { useT } from "../i18n";
import { Card, PageHeader, Row, SectionTitle, Segmented, Toggle } from "../components/ui";
import type { Settings } from "../types";

export function SettingsPage() {
  const t = useT();
  const s = useApp((x) => x.settings)!;
  const update = useApp((x) => x.updateSettings);
  const tog = (k: keyof Settings) => (v: boolean) => void update({ [k]: v } as Partial<Settings>);

  return (
    <>
      <PageHeader title={t("nav.settings")} subtitle={t("settings.subtitle")} />
      <div className="grid gap-4">
        <Card>
          <SectionTitle>{t("settings.appearance")}</SectionTitle>
          <Row label={t("settings.language")}>
            <Segmented
              label={t("settings.language")}
              value={s.language}
              options={[{ value: "en", label: "English" }, { value: "ar", label: "العربية" }]}
              onChange={(v) => void update({ language: v })}
            />
          </Row>
          <Row label={t("settings.theme")}>
            <Segmented
              label={t("settings.theme")}
              value={s.theme}
              options={(["dark", "light", "oled", "system"] as const).map((v) => ({ value: v, label: t(`theme.${v}`) }))}
              onChange={(v) => void update({ theme: v })}
            />
          </Row>
          <Row label={t("settings.reduceMotion")} hint={t("settings.reduceMotionHint")}>
            <Toggle checked={s.reduceMotion} onChange={tog("reduceMotion")} label={t("settings.reduceMotion")} />
          </Row>
          <Row label={t("settings.meterRate")} hint={t("settings.meterRateHint")}>
            <Segmented
              label={t("settings.meterRate")}
              value={String(s.meterRateHz)}
              options={[{ value: "15", label: "15 Hz" }, { value: "30", label: "30 Hz" }, { value: "60", label: "60 Hz" }]}
              onChange={(v) => void update({ meterRateHz: Number(v) })}
            />
          </Row>
        </Card>

        <Card>
          <SectionTitle>{t("settings.startup")}</SectionTitle>
          <Row label={t("settings.startWithWindows")}>
            <Toggle checked={s.startWithWindows} onChange={tog("startWithWindows")} label={t("settings.startWithWindows")} />
          </Row>
          <Row label={t("settings.startMinimized")}>
            <Toggle checked={s.startMinimized} onChange={tog("startMinimized")} label={t("settings.startMinimized")} />
          </Row>
          <Row label={t("settings.minimizeToTray")}>
            <Toggle checked={s.minimizeToTray} onChange={tog("minimizeToTray")} label={t("settings.minimizeToTray")} />
          </Row>
          <Row label={t("settings.closeToTray")} hint={t("settings.closeToTrayHint")}>
            <Toggle checked={s.closeToTray} onChange={tog("closeToTray")} label={t("settings.closeToTray")} />
          </Row>
        </Card>

        <Card>
          <SectionTitle>{t("settings.audio")}</SectionTitle>
          <Row label={t("settings.feedbackWarning")} hint={t("settings.feedbackWarningHint")}>
            <Toggle checked={s.feedbackWarning} onChange={tog("feedbackWarning")} label={t("settings.feedbackWarning")} />
          </Row>
          <Row label={t("settings.showAdvanced")} hint={t("settings.showAdvancedHint")}>
            <Toggle checked={s.showAdvanced} onChange={tog("showAdvanced")} label={t("settings.showAdvanced")} />
          </Row>
        </Card>

        <Card>
          <SectionTitle>{t("settings.setup")}</SectionTitle>
          <Row label={t("settings.replayOnboarding")} hint={t("settings.replayOnboardingHint")}>
            <button className="btn btn-sm" onClick={() => void update({ onboardingDone: false })}>{t("settings.replay")}</button>
          </Row>
        </Card>

        <Card>
          <SectionTitle>{t("settings.privacy")}</SectionTitle>
          <p className="m-0 text-[13px] text-muted">{t("settings.privacyText")}</p>
        </Card>
      </div>
    </>
  );
}
