//! Atomic JSON persistence with a `.bak` fallback. Files live under the app's
//! data directory; nothing here ever stores audio.

use crate::error::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let d = base.join("Auralis");
    let _ = fs::create_dir_all(&d);
    d
}

pub fn log_dir() -> PathBuf {
    let d = data_dir().join("logs");
    let _ = fs::create_dir_all(&d);
    d
}

/// Write `value` as pretty JSON via temp file + rename; keep the previous version as `.bak`.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let text = serde_json::to_string_pretty(value)?;
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    if path.exists() {
        let _ = fs::copy(path, path.with_extension("json.bak"));
    }
    fs::rename(&tmp, path).map_err(AppError::from)
}

/// Load JSON, falling back to the backup when the main file is missing or corrupt.
/// Returns `None` if neither can be parsed.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    for p in [path.to_path_buf(), path.with_extension("json.bak")] {
        if let Ok(text) = fs::read_to_string(&p) {
            match serde_json::from_str::<T>(&text) {
                Ok(v) => return Some(v),
                Err(e) => tracing::warn!("could not parse {}: {e}", p.display()),
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
    struct T {
        a: i32,
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("auralis-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn round_trip_and_backup_recovery() {
        let d = tmp("rt");
        let f = d.join("x.json");
        save_json(&f, &T { a: 1 }).unwrap();
        save_json(&f, &T { a: 2 }).unwrap();
        assert_eq!(load_json::<T>(&f), Some(T { a: 2 }));
        fs::write(&f, "{ corrupt").unwrap();
        assert_eq!(load_json::<T>(&f), Some(T { a: 1 }), "falls back to .bak");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_file_is_none() {
        assert_eq!(load_json::<T>(&tmp("none").join("nope.json")), None);
    }
}
