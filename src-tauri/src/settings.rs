//! User settings (persisted). Every field has a serde default so older files keep loading.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub onboarding_done: bool,
    pub reduce_motion: bool,
    pub meter_rate_hz: u32,
    pub monitor_input_device: Option<String>,
    pub monitor_output_device: Option<String>,
    pub favorites: Vec<String>,
    pub hotkeys: BTreeMap<String, String>,
    pub active_profile: Option<String>,
    pub show_advanced: bool,
    pub feedback_warning: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            theme: "dark".into(),
            start_with_windows: false,
            start_minimized: false,
            minimize_to_tray: true,
            close_to_tray: true,
            onboarding_done: false,
            reduce_motion: false,
            meter_rate_hz: 30,
            monitor_input_device: None,
            monitor_output_device: None,
            favorites: vec![],
            hotkeys: BTreeMap::new(),
            active_profile: None,
            show_advanced: false,
            feedback_warning: true,
        }
    }
}

impl Settings {
    pub fn validate(&mut self) -> AppResult<()> {
        if !["en", "ar"].contains(&self.language.as_str()) {
            return Err(AppError::Invalid(format!(
                "unsupported language '{}'",
                self.language
            )));
        }
        if !["dark", "light", "system", "oled"].contains(&self.theme.as_str()) {
            return Err(AppError::Invalid(format!(
                "unsupported theme '{}'",
                self.theme
            )));
        }
        self.meter_rate_hz = self.meter_rate_hz.clamp(10, 60);
        self.favorites.truncate(500);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_language_and_theme() {
        let mut s = Settings {
            language: "xx".into(),
            ..Default::default()
        };
        assert!(s.validate().is_err());
        let mut s = Settings {
            theme: "neon".into(),
            ..Default::default()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn clamps_meter_rate() {
        let mut s = Settings {
            meter_rate_hz: 1000,
            ..Default::default()
        };
        s.validate().unwrap();
        assert_eq!(s.meter_rate_hz, 60);
    }

    #[test]
    fn partial_json_uses_defaults() {
        let s: Settings = serde_json::from_str(r#"{"language":"ar"}"#).unwrap();
        assert_eq!(s.language, "ar");
        assert_eq!(s.theme, "dark");
    }
}
