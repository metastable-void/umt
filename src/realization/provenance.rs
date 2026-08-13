//! Provenance references (UMT-3.2 sections 0.6.1 and 7.10).

use alloc::string::String;
use alloc::sync::Arc;

/// A stable reference to a provenance record.
///
/// UMT layer: metadata, applicable at every layer. The record itself -
/// algorithm, version, parameters, seed, tolerance, source measurements,
/// parents - is stored once in an arena and referenced by this identifier
/// rather than copied into every object (prompt section 36).
///
/// The identifier is a stable string, not a process-local counter, so it
/// survives serialization (prompt section 8).
///
/// Equality is presentation equality on the identifier text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct ProvenanceId(Arc<str>);

impl ProvenanceId {
    /// Wraps a stable identifier.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProvenanceId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<ProvenanceId> for String {
    fn from(value: ProvenanceId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for ProvenanceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}
