//! Problems the page shows the reader, as translatable messages.
//!
//! Rust never sends a sentence for the reader to see: it sends a message
//! key, such as `problem.notAModule`, with values to fill in, and the page
//! looks the key up in its locale files (`src/locales`). `detail` holds the
//! technical reason in English, shown smaller, for diagnosing the problem.

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// A message key in the page's locale files.
    pub key: &'static str,
    /// Values for the message's placeholders, such as `{file}`.
    pub values: BTreeMap<&'static str, String>,
    /// The technical reason, in English, if there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Problem {
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            values: BTreeMap::new(),
            detail: None,
        }
    }

    /// Fills in the placeholder `name`.
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.values.insert(name, value.into());
        self
    }

    /// Adds the technical reason.
    pub fn detail(mut self, detail: impl ToString) -> Self {
        self.detail = Some(detail.to_string());
        self
    }
}
