//! Translation resource loading.
//!
//! Loads the built-in `en` / `zh` resources that are embedded into the binary
//! at compile time via [`include_str!`]. Reserved keys prefixed with `_`
//! (e.g. `_comment_schema`) are filtered out so they never collide with real
//! translations.
//!
//! Stage 3 will extend this module with remote package loading + SHA256
//! verification; for stage 1 only the built-in path is required.

use std::collections::HashMap;

/// Embedded built-in language resources. The suffix selects the locale file.
///
/// Using `include_str!` keeps the JSON as compile-time string constants:
/// zero runtime file I/O, zero new crate dependencies (serde_json is reused).
fn builtin_raw(code: &str) -> Option<&'static str> {
    match code {
        "en" => Some(include_str!("locales/en.json")),
        "zh" => Some(include_str!("locales/zh.json")),
        _ => None,
    }
}

/// Parses a built-in locale into a flat `key -> text` map.
///
/// Returns `None` if the code is unknown or the JSON fails to parse. Callers
/// must treat `None` as "fall back to English" and never panic.
pub fn load_builtin(code: &str) -> Option<HashMap<String, String>> {
    let raw = builtin_raw(code)?;
    let parsed: HashMap<String, String> = serde_json::from_str(raw).ok()?;
    // Drop reserved metadata keys (prefixed with `_`) so they cannot be
    // accidentally rendered as translations.
    Some(
        parsed
            .into_iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .collect(),
    )
}
