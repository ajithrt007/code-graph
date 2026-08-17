//! Stable, language-independent identifier for a method.
//!
//! IDs are opaque strings produced by analyzers. They must be stable across
//! runs so the frontend can correlate nodes between analyses. The C# analyzer
//! mints tagged strings of the form `csharp:<fully-qualified-signature>`
//! (the signature includes parameter types), which keeps language analyzers
//! independent while disambiguating namespaces, types, and overloads.

use serde::{Deserialize, Serialize};

/// A stable, opaque identifier for a method.
///
/// IDs are produced by language analyzers and compared by value. They are
/// intentionally strings (rather than integer indices) so additional
/// languages can mint IDs without coordinating with this crate.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MethodId(pub String);

impl MethodId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MethodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for MethodId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}
