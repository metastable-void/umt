//! Native schema versioning (prompt section 54, UMT-3.2 section 8.8).
//!
//! The schema version is deliberately independent of the Cargo package version
//! and of the UMT specification revision. A patch release of this crate does
//! not imply a new wire format, and a future UMT revision is not assumed to be
//! backward compatible with this one.

/// The version of the native serialization schema.
///
/// UMT layer: interchange metadata.
///
/// Compatibility rule: a reader accepts a document whose major version equals
/// its own and whose minor version does not exceed its own. A higher minor
/// version means the document may use fields this reader does not know, and
/// unknown fields MUST NOT be silently interpreted as current semantics
/// (prompt section 54), so such a document is rejected rather than guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmtSchemaVersion {
    /// Incremented when the wire format changes incompatibly.
    pub major: u16,
    /// Incremented when fields are added compatibly.
    pub minor: u16,
}

impl UmtSchemaVersion {
    /// Builds a version.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// The UMT specification profile this schema encodes.
    #[must_use]
    pub const fn spec_profile(self) -> &'static str {
        crate::UMT_SPEC_VERSION
    }

    /// Whether a reader at this version can read a document written at
    /// `written`.
    #[must_use]
    pub const fn can_read(self, written: Self) -> bool {
        self.major == written.major && written.minor <= self.minor
    }
}

impl core::fmt::Display for UmtSchemaVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The schema version this build reads and writes.
///
/// Major version 0 means the wire format is not yet stable; it becomes 1 when
/// the native container of UMT-3.2 section 8.8 is complete.
pub const NATIVE_SCHEMA_VERSION: UmtSchemaVersion = UmtSchemaVersion::new(0, 1);

#[cfg(test)]
mod tests {
    use super::{NATIVE_SCHEMA_VERSION, UmtSchemaVersion};

    #[test]
    fn compatibility_is_directional() {
        let reader = UmtSchemaVersion::new(1, 3);
        assert!(reader.can_read(UmtSchemaVersion::new(1, 3)));
        assert!(reader.can_read(UmtSchemaVersion::new(1, 0)));
        assert!(
            !reader.can_read(UmtSchemaVersion::new(1, 4)),
            "a newer document may use fields this reader would misinterpret"
        );
        assert!(!reader.can_read(UmtSchemaVersion::new(2, 0)));
        assert!(!reader.can_read(UmtSchemaVersion::new(0, 9)));
    }

    #[test]
    fn the_native_version_declares_its_spec_profile() {
        assert_eq!(NATIVE_SCHEMA_VERSION.spec_profile(), "UMT-3.2");
        assert_eq!(NATIVE_SCHEMA_VERSION.to_string(), "0.1");
        assert!(NATIVE_SCHEMA_VERSION.can_read(NATIVE_SCHEMA_VERSION));
    }
}
