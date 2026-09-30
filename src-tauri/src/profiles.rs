//! Profiles: a named snapshot of voice settings, hotkey-free mixer state and
//! per-app volumes that can be applied together.

use crate::dsp::VoiceParams;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub voice: VoiceParams,
    pub voice_preset: Option<String>,
    pub master_volume: Option<f32>,
    pub mic_gain_db: f32,
    pub mic_muted: Option<bool>,
    /// Session volumes keyed by process name (lower-case), 0..1.
    pub app_volumes: BTreeMap<String, f32>,
    pub created_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportFile {
    format: String,
    version: u32,
    profile: Profile,
}

const FORMAT: &str = "auralis-profile";

impl Profile {
    pub fn sanitize(&mut self) -> AppResult<()> {
        self.name = self.name.trim().chars().take(60).collect();
        if self.name.is_empty() {
            return Err(AppError::Invalid("profile name cannot be empty".into()));
        }
        self.description = self.description.chars().take(300).collect();
        self.voice = self.voice.sanitized();
        self.mic_gain_db = if self.mic_gain_db.is_finite() {
            self.mic_gain_db.clamp(-30.0, 30.0)
        } else {
            0.0
        };
        self.master_volume = self.master_volume.map(|v| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                1.0
            }
        });
        self.app_volumes.retain(|k, _| k.len() <= 128);
        for v in self.app_volumes.values_mut() {
            *v = if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                1.0
            };
        }
        Ok(())
    }
}

impl ProfileStore {
    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Insert or replace by id. Assigns an id when empty.
    pub fn upsert(
        &mut self,
        mut p: Profile,
        new_id: impl FnOnce() -> String,
        now_ms: u64,
    ) -> AppResult<Profile> {
        p.sanitize()?;
        if p.id.is_empty() {
            p.id = new_id();
            p.created_ms = now_ms;
        }
        if let Some(slot) = self.profiles.iter_mut().find(|x| x.id == p.id) {
            p.created_ms = slot.created_ms;
            *slot = p.clone();
        } else {
            if self.profiles.len() >= 200 {
                return Err(AppError::Invalid("too many profiles".into()));
            }
            self.profiles.push(p.clone());
        }
        Ok(p)
    }

    pub fn delete(&mut self, id: &str) -> AppResult<()> {
        let n = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if self.profiles.len() == n {
            return Err(AppError::NotFound("profile not found".into()));
        }
        Ok(())
    }

    pub fn duplicate(
        &mut self,
        id: &str,
        new_id: impl FnOnce() -> String,
        now_ms: u64,
    ) -> AppResult<Profile> {
        let mut p = self
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("profile not found".into()))?;
        p.id = String::new();
        p.name = format!("{} copy", p.name);
        self.upsert(p, new_id, now_ms)
    }
}

pub fn export_profile(p: &Profile) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(&ExportFile {
        format: FORMAT.into(),
        version: 1,
        profile: p.clone(),
    })?)
}

pub fn import_profile(text: &str) -> AppResult<Profile> {
    if text.len() > 512 * 1024 {
        return Err(AppError::Invalid("profile file is too large".into()));
    }
    let f: ExportFile = serde_json::from_str(text)
        .map_err(|e| AppError::Invalid(format!("not a valid profile file: {e}")))?;
    if f.format != FORMAT {
        return Err(AppError::Invalid("not an Auralis profile file".into()));
    }
    let mut p = f.profile;
    p.id = String::new();
    p.sanitize()?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(n: &str) -> Profile {
        Profile {
            name: n.into(),
            ..Default::default()
        }
    }

    #[test]
    fn crud_flow() {
        let mut s = ProfileStore::default();
        let a = s.upsert(named("Gaming"), || "id1".into(), 10).unwrap();
        assert_eq!(a.id, "id1");
        let mut edited = a.clone();
        edited.name = "Gaming 2".into();
        s.upsert(edited, || unreachable!(), 99).unwrap();
        assert_eq!(s.profiles.len(), 1);
        assert_eq!(s.get("id1").unwrap().created_ms, 10);
        let d = s.duplicate("id1", || "id2".into(), 20).unwrap();
        assert_eq!(d.name, "Gaming 2 copy");
        s.delete("id1").unwrap();
        assert!(s.delete("id1").is_err());
    }

    #[test]
    fn empty_name_rejected() {
        assert!(ProfileStore::default()
            .upsert(named("  "), || "x".into(), 0)
            .is_err());
    }

    #[test]
    fn export_import_round_trip_and_rejects_foreign_json() {
        let mut p = named("Stream");
        p.app_volumes.insert("discord.exe".into(), 0.4);
        let text = export_profile(&p).unwrap();
        let back = import_profile(&text).unwrap();
        assert_eq!(back.name, "Stream");
        assert_eq!(back.app_volumes["discord.exe"], 0.4);
        assert!(back.id.is_empty());
        assert!(import_profile(r#"{"format":"other","version":1,"profile":{}}"#).is_err());
        assert!(import_profile("garbage").is_err());
    }

    #[test]
    fn sanitize_clamps_values() {
        let mut p = named("x");
        p.mic_gain_db = 999.0;
        p.master_volume = Some(f32::NAN);
        p.sanitize().unwrap();
        assert_eq!(p.mic_gain_db, 30.0);
        assert_eq!(p.master_volume, Some(1.0));
    }
}
