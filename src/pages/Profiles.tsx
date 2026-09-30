import { useState } from "react";
import { Check, Copy, Download, Pencil, Plus, Trash2, Upload } from "lucide-react";
import { api, toAppError } from "../services/api";
import { useApp } from "../stores/app";
import { toast } from "../stores/toast";
import { useT } from "../i18n";
import { Banner, Card, Chip, Empty, PageHeader, SectionTitle } from "../components/ui";
import type { Profile } from "../types";

export function Profiles() {
  const t = useT();
  const profiles = useApp((s) => s.profiles);
  const settings = useApp((s) => s.settings)!;
  const setProfiles = useApp((s) => s.setProfiles);
  const updateSettings = useApp((s) => s.updateSettings);
  const applyVoiceState = useApp((s) => s.applyVoiceState);
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [confirmDel, setConfirmDel] = useState<string | null>(null);

  const fail = (e: unknown) => toast.error(toAppError(e).message);
  const reload = async () => setProfiles(await api.listProfiles());

  const create = async () => {
    const n = name.trim();
    if (!n) return;
    try {
      await api.createProfileFromCurrent(n);
      setName("");
      await reload();
      toast.success(t("profiles.created"));
    } catch (e) {
      fail(e);
    }
  };

  const activate = async (p: Profile) => {
    try {
      await api.activateProfile(p.id);
      const v = await api.getVoice();
      useApp.setState({ voice: v });
      applyVoiceState(v);
      await reload();
      useApp.setState((s) => ({ settings: s.settings ? { ...s.settings, activeProfile: p.id } : s.settings }));
      toast.success(t("profiles.activated", { name: p.name }));
    } catch (e) {
      fail(e);
    }
  };

  const rename = async (p: Profile) => {
    const n = editName.trim();
    setEditing(null);
    if (!n || n === p.name) return;
    try {
      await api.saveProfile({ ...p, name: n });
      await reload();
    } catch (e) {
      fail(e);
    }
  };

  const remove = async (p: Profile) => {
    setConfirmDel(null);
    try {
      await api.deleteProfile(p.id);
      if (settings.activeProfile === p.id) await updateSettings({ activeProfile: null });
      await reload();
    } catch (e) {
      fail(e);
    }
  };

  const dup = (p: Profile) => api.duplicateProfile(p.id).then(reload).catch(fail);
  const exp = (p: Profile) =>
    api.exportProfile(p.id).then((path) => path && toast.success(t("profiles.exported", { path }))).catch(fail);
  const imp = () =>
    api.importProfile().then(async (p) => {
      if (p) {
        await reload();
        toast.success(t("profiles.imported", { name: p.name }));
      }
    }).catch(fail);

  return (
    <>
      <PageHeader
        title={t("nav.profiles")}
        subtitle={t("profiles.subtitle")}
        actions={<button className="btn btn-sm" onClick={() => void imp()}><Upload size={13} /> {t("profiles.import")}</button>}
      />
      <Card className="mb-4">
        <SectionTitle>{t("profiles.saveCurrent")}</SectionTitle>
        <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); void create(); }}>
          <input
            className="input flex-1"
            value={name}
            maxLength={60}
            placeholder={t("profiles.namePlaceholder")}
            aria-label={t("profiles.namePlaceholder")}
            onChange={(e) => setName(e.target.value)}
          />
          <button className="btn btn-primary" type="submit" disabled={!name.trim()}><Plus size={14} /> {t("profiles.create")}</button>
        </form>
        <p className="mb-0 mt-2 text-[12px] text-muted">{t("profiles.includes")}</p>
      </Card>

      <Card>
        <SectionTitle>{t("profiles.saved")}</SectionTitle>
        {profiles.length === 0 ? (
          <Empty title={t("profiles.empty")} text={t("profiles.emptyHint")} />
        ) : (
          <ul className="m-0 list-none p-0">
            {profiles.map((p) => {
              const active = settings.activeProfile === p.id;
              return (
                <li key={p.id} className="flex items-center gap-3 border-b border-line py-2.5 last:border-b-0">
                  <div className="min-w-0 flex-1">
                    {editing === p.id ? (
                      <input
                        className="input w-full"
                        autoFocus
                        value={editName}
                        maxLength={60}
                        aria-label={t("profiles.rename")}
                        onChange={(e) => setEditName(e.target.value)}
                        onBlur={() => void rename(p)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") void rename(p);
                          if (e.key === "Escape") setEditing(null);
                        }}
                      />
                    ) : (
                      <div className="flex items-center gap-2">
                        <span className="truncate font-medium">{p.name}</span>
                        {active && <Chip kind="ok">{t("profiles.active")}</Chip>}
                      </div>
                    )}
                    <div className="mt-0.5 text-[12px] text-muted">
                      {p.voicePreset ? t("profiles.basedOn", { preset: p.voicePreset }) : t("profiles.custom")}
                      {` · ${new Date(p.createdMs).toLocaleDateString()}`}
                    </div>
                  </div>
                  {confirmDel === p.id ? (
                    <>
                      <span className="text-[12.5px] text-muted">{t("profiles.confirmDelete")}</span>
                      <button className="btn btn-sm btn-danger" onClick={() => void remove(p)}>{t("common.delete")}</button>
                      <button className="btn btn-sm" onClick={() => setConfirmDel(null)}>{t("common.cancel")}</button>
                    </>
                  ) : (
                    <>
                      <button className="btn btn-sm btn-primary" disabled={active} onClick={() => void activate(p)}><Check size={13} /> {t("profiles.activate")}</button>
                      <button className="btn btn-sm" aria-label={t("profiles.rename")} title={t("profiles.rename")} onClick={() => { setEditing(p.id); setEditName(p.name); }}><Pencil size={13} /></button>
                      <button className="btn btn-sm" aria-label={t("profiles.duplicate")} title={t("profiles.duplicate")} onClick={() => void dup(p)}><Copy size={13} /></button>
                      <button className="btn btn-sm" aria-label={t("profiles.export")} title={t("profiles.export")} onClick={() => void exp(p)}><Download size={13} /></button>
                      <button className="btn btn-sm" aria-label={t("common.delete")} title={t("common.delete")} onClick={() => setConfirmDel(p.id)}><Trash2 size={13} /></button>
                    </>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </Card>
      <div className="mt-3"><Banner kind="info">{t("profiles.importNote")}</Banner></div>
    </>
  );
}
