//! Internationalization (i18n) infrastructure.
//!
//! Layer: **shared** for [`Translator`] / [`keys`] / [`loader`]; **core** for
//! [`settings`] (disk persistence). See `docs/i18n-architecture.md` §7 for the
//! full dependency map.
//!
//! Design summary (stage 1):
//! - Built-in `en` / `zh` resources are embedded via `include_str!` at compile
//!   time and deserialized once into a `HashMap<String, String>`.
//! - A [`Translator`] is an immutable snapshot of one language. Switching
//!   languages builds a new snapshot and swaps the shared `Rc<Translator>` —
//!   no interior mutability of the map, so `get` returns a plain `&str`.
//! - Missing keys fall back to English, then to the key name itself (so gaps
//!   are visible during development but never panic at runtime).

pub mod keys;
pub mod loader;
pub mod settings;

use std::collections::HashMap;

/// A localizable UI unit. Implementations re-apply all control text from the
/// given [`Translator`]. Must be invoked on the GUI thread (NWG controls are
/// `!Send`).
pub trait Localizable {
    fn re_localize(&self, t: &Translator);
}

/// An immutable snapshot of one language's translation table.
///
/// Held behind `Rc<Translator>` in the GUI so that a language switch atomically
/// replaces the shared instance without mutating any existing borrows.
pub struct Translator {
    /// Active language code (`"en"` / `"zh"`).
    code: &'static str,
    /// Active translations.
    map: HashMap<String, String>,
    /// English table kept as the missing-key fallback. For the English
    /// translator itself this is a duplicate of `map`; the memory cost
    /// (~10 KB) is negligible and buys uniform fallback behaviour.
    fallback_en: HashMap<String, String>,
}

/// Maps a runtime code to a known `&'static str`. Only built-in codes that
/// `loader::load_builtin` can resolve ever reach here.
fn static_code(code: &str) -> &'static str {
    match code {
        "zh" => "zh",
        // Unknown / future codes default to English at the call site.
        _ => "en",
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::load("en").unwrap_or_else(|| Self {
            code: "en",
            map: HashMap::new(),
            fallback_en: HashMap::new(),
        })
    }
}

impl Translator {
    /// Loads a built-in language snapshot. Returns `None` for unknown codes or
    /// unparseable resources; callers fall back to [`Translator::default`].
    pub fn load(code: &str) -> Option<Self> {
        let map = loader::load_builtin(code)?;
        let fallback_en = loader::load_builtin("en").unwrap_or_default();
        Some(Self {
            code: static_code(code),
            map,
            fallback_en,
        })
    }

    /// The active language code.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// Looks up `key`. Falls back to English when the active language lacks
    /// the key, then to the key name itself. Never panics.
    ///
    /// The explicit single lifetime ties the returned reference to both the
    /// translator and the key so that the missing-key branch (`key`) type-checks
    /// regardless of the input source.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        if let Some(text) = self.map.get(key) {
            return text.as_str();
        }
        if let Some(text) = self.fallback_en.get(key) {
            return text.as_str();
        }
        key
    }

    /// Like [`get`](Self::get) but substitutes `{placeholder}` tokens with the
    /// provided values. Example: `t.getf(key, &[("bus_id", "1-2")])`.
    pub fn getf(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut out = self.get(key).to_owned();
        for (name, value) in args {
            out = out.replace(&format!("{{{name}}}"), value);
        }
        out
    }
}
