import { useApp } from "../stores/app";
import { useT } from "../i18n";
import { Card, PageHeader, Row, SectionTitle } from "../components/ui";

const LIBS: [string, string][] = [
  ["Tauri", "MIT / Apache-2.0"],
  ["React", "MIT"],
  ["Zustand", "MIT"],
  ["Tailwind CSS", "MIT"],
  ["Lucide", "ISC"],
  ["windows-rs", "MIT / Apache-2.0"],
  ["nnnoiseless (RNNoise port)", "BSD-3-Clause"],
  ["rustfft", "MIT / Apache-2.0"],
];

export function About() {
  const t = useT();
  const version = useApp((s) => s.version);
  return (
    <>
      <PageHeader title={t("nav.about")} subtitle={t("about.tagline")} />
      <div className="grid grid-cols-2 gap-4">
        <Card>
          <SectionTitle>Auralis</SectionTitle>
          <Row label={t("about.version")}><span className="font-mono">{version}</span></Row>
          <Row label={t("about.license")}><span>MIT</span></Row>
          <p className="mb-0 mt-3 text-[13px] text-muted">{t("about.privacy")}</p>
        </Card>
        <Card>
          <SectionTitle>{t("about.libraries")}</SectionTitle>
          <ul className="m-0 list-none p-0">
            {LIBS.map(([n, l]) => (
              <Row key={n} label={n}><span className="text-[12.5px] text-muted">{l}</span></Row>
            ))}
          </ul>
        </Card>
      </div>
    </>
  );
}
