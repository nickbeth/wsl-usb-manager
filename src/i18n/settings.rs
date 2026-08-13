//! Persistent user preferences (`settings.json`).
//!
//! Lives at `%LOCALAPPDATA%\WSL USB Manager\settings.json`, next to the existing
//! `profiles.json`. Reuses the same storage-root resolution as `main.rs`.
//!
//! Layer: **core** (depends on `shared` data types only; no GUI / system APIs).
//!
//! Iron rule: parsing or disk failures must degrade silently to the default
//! language and **never panic**. A corrupted `settings.json` is rewritten on
//! next save and otherwise ignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// File name of the settings store, relative to the storage root.
const SETTINGS_FILE: &str = "settings.json";

/// On-disk schema. Only `language` exists today; future fields can be added
/// behind `#[serde(default)]` without breaking older files.
#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    language: Option<String>,
}

/// Resolves the shared storage root: `%LOCALAPPDATA%\WSL USB Manager\`.
///
/// Creates the directory if it does not yet exist. Returns `None` when the
/// environment variable is missing or the directory cannot be created — every
/// downstream operation must tolerate that gracefully.
pub fn storage_dir() -> Option<PathBuf> {
    let dir = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|d| PathBuf::from(d).join("WSL USB Manager"))?;
    std::fs::create_dir_all(&dir).ok().map(|_| dir)
}

fn settings_path() -> Option<PathBuf> {
    storage_dir().map(|mut p| {
        p.push(SETTINGS_FILE);
        p
    })
}

/// Reads the persisted language preference, if any.
///
/// Returns `None` when the file is absent, unreadable, unparseable, or the
/// field is missing. Callers fall back to the default language in that case.
pub fn load_language() -> Option<String> {
    let path = settings_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let settings: Settings = serde_json::from_str(&contents).ok()?;
    settings.language.filter(|l| !l.is_empty())
}

/// Persists `code` as the active language preference.
///
/// Failures are swallowed: a preference that cannot be saved must not crash
/// the app or block a language switch. The next launch simply falls back to
/// the default.
pub fn save_language(code: &str) {
    let Some(path) = settings_path() else {
        return;
    };
    let settings = Settings {
        language: Some(code.to_owned()),
    };
    let Ok(json) = serde_json::to_string_pretty(&settings) else {
        return;
    };
    // Best-effort atomic write: write to a sibling temp file then rename.
    let mut tmp = path.clone();
    tmp.set_extension("json.tmp");
    if std::fs::write(&tmp, json).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, &path);
}
