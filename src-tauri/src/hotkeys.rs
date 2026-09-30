//! Hotkey actions, accelerator validation and conflict detection. Registration
//! with the OS is done in `app` via the global-shortcut plugin.

use std::collections::BTreeMap;

pub const ACTIONS: &[(&str, &str)] = &[
    ("toggleMicMute", "Mute / unmute microphone"),
    ("toggleMonitor", "Start / stop voice monitoring"),
    ("toggleVoiceFx", "Toggle voice effects (A/B)"),
    ("nextPreset", "Next voice preset"),
    ("prevPreset", "Previous voice preset"),
    ("emergencyStop", "Emergency stop monitoring"),
    ("masterMute", "Mute / unmute master output"),
    ("showWindow", "Show / hide window"),
];

const MODIFIERS: &[&str] = &[
    "ctrl",
    "control",
    "alt",
    "shift",
    "super",
    "win",
    "meta",
    "cmdorctrl",
    "commandorcontrol",
];

/// Normalise an accelerator like "ctrl+alt+m" -> "Ctrl+Alt+M". Err(reason) if malformed.
pub fn normalize(acc: &str) -> Result<String, String> {
    let parts: Vec<&str> = acc
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err("a hotkey needs at least one modifier and a key".into());
    }
    let (key, mods) = parts.split_last().unwrap();
    if MODIFIERS.contains(&key.to_lowercase().as_str()) {
        return Err("the last part must be a key, not a modifier".into());
    }
    let mut seen: Vec<String> = Vec::new();
    for m in mods {
        let l = m.to_lowercase();
        if !MODIFIERS.contains(&l.as_str()) {
            return Err(format!("'{m}' is not a modifier"));
        }
        let canon = match l.as_str() {
            "ctrl" | "control" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            "super" | "win" | "meta" => "Super",
            _ => "CommandOrControl",
        };
        if seen.iter().any(|s| s == canon) {
            return Err("duplicate modifier".into());
        }
        seen.push(canon.to_string());
    }
    let key_ok = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric())
        || (key.len() <= 12 && key.chars().all(|c| c.is_ascii_alphanumeric()))
        || [
            "Space",
            "Enter",
            "Tab",
            "Up",
            "Down",
            "Left",
            "Right",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Insert",
            "Delete",
            "Backspace",
        ]
        .contains(key);
    if !key_ok {
        return Err(format!("unsupported key '{key}'"));
    }
    let key = if key.len() == 1 {
        key.to_uppercase()
    } else {
        capitalize(key)
    };
    seen.push(key);
    Ok(seen.join("+"))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// Accelerators bound to more than one action: `(accelerator, actions)`.
pub fn conflicts(map: &BTreeMap<String, String>) -> Vec<(String, Vec<String>)> {
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (action, acc) in map {
        if let Ok(n) = normalize(acc) {
            by.entry(n.to_lowercase()).or_default().push(action.clone());
        }
    }
    by.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

pub fn is_known_action(a: &str) -> bool {
    ACTIONS.iter().any(|(id, _)| *id == a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_rejects() {
        assert_eq!(normalize("ctrl+alt+m").unwrap(), "Ctrl+Alt+M");
        assert_eq!(normalize("Control + Shift + F5").unwrap(), "Ctrl+Shift+F5");
        assert!(normalize("m").is_err());
        assert!(normalize("ctrl+shift").is_err());
        assert!(normalize("ctrl+ctrl+m").is_err());
        assert!(normalize("ctrl+foo+m").is_err());
        assert!(normalize("ctrl+!").is_err());
    }

    #[test]
    fn detects_conflicts_case_insensitively() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), "ctrl+alt+m".to_string());
        m.insert("b".to_string(), "Ctrl+Alt+M".to_string());
        m.insert("c".to_string(), "Ctrl+Alt+N".to_string());
        let c = conflicts(&m);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1.len(), 2);
    }
}
