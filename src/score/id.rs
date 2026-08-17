//! Score identities and scopes (UMT-3.2 section 6.2, prompt section 24).
//!
//! Event identity is primary in UMT-3.2, because a bare pair of a pitch
//! aggregate and a time aggregate cannot say which pitch belongs to which
//! event (section 6.1). So identity comes first here too: an event *is* its
//! identity plus what is attached to it.
//!
//! Identities are stable strings rather than process-local counters, so they
//! survive serialization and remain meaningful across a document lineage. They
//! are not, however, evidence of musical ancestry: prompt section 24 is
//! explicit that ancestry must come from an explicit source-target relation,
//! which is [`crate::score::EventRelation`].

use alloc::string::String;
use alloc::sync::Arc;

use crate::pitch::VoiceId;

macro_rules! score_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// Wraps a stable identity.
            #[must_use]
            pub fn new(id: &str) -> Self {
                Self(Arc::from(id))
            }

            /// The identity text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(Arc::from(value))
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.as_str().into()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

score_id!(
    EventId,
    "Stable identity of a score event.\n\nUMT layer: metadata. Stable within a score or document lineage.\nEquality of identities is *not* evidence of musical ancestry; a\ntransformation that splits, merges, inserts, or deletes events carries an\nexplicit relation instead (prompt section 24)."
);
score_id!(
    StaffId,
    "Stable identity of a staff.\n\nUMT layer: metadata."
);
score_id!(PartId, "Stable identity of a part.\n\nUMT layer: metadata.");

/// What an event belongs to (prompt section 24).
///
/// UMT layer: metadata.
///
/// A voice-local event *necessarily* carries a voice identity, because the
/// identity lives inside the variant rather than in an `Option` beside it.
/// That is prompt section 24's requirement, and it also rules out the
/// combination it warns against - an event that claims to be voice-local while
/// its voice field is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum EventScope {
    /// Belongs to one voice.
    VoiceLocal(VoiceId),
    /// Belongs to a staff, without a distinguished voice.
    StaffLocal(StaffId),
    /// Belongs to a part.
    PartLocal(PartId),
    /// Genuinely global: a tempo mark, a rehearsal letter, a structural
    /// marker. Such an event has no voice, and is not given a fabricated one
    /// (fixture F24).
    Global,
}

impl EventScope {
    /// The voice this scope names, if it names one.
    ///
    /// `None` for every non-voice scope. There is deliberately no way to get a
    /// voice out of a global event.
    #[must_use]
    pub fn voice(&self) -> Option<&VoiceId> {
        match self {
            Self::VoiceLocal(voice) => Some(voice),
            _ => None,
        }
    }

    /// Whether this scope is global.
    #[must_use]
    pub fn is_global(&self) -> bool {
        matches!(self, Self::Global)
    }

    /// Whether this scope attaches the event to some performing context - a
    /// voice, a staff, or a part - rather than to the score as a whole.
    #[must_use]
    pub fn is_local(&self) -> bool {
        !self.is_global()
    }
}

impl core::fmt::Display for EventScope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::VoiceLocal(voice) => write!(f, "voice:{voice}"),
            Self::StaffLocal(staff) => write!(f, "staff:{staff}"),
            Self::PartLocal(part) => write!(f, "part:{part}"),
            Self::Global => f.write_str("global"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventId, EventScope, PartId, StaffId};
    use crate::pitch::VoiceId;

    #[test]
    fn a_voice_local_scope_necessarily_carries_a_voice() {
        let scope = EventScope::VoiceLocal(VoiceId::new("soprano"));
        assert_eq!(scope.voice(), Some(&VoiceId::new("soprano")));
        assert!(scope.is_local());
        assert!(!scope.is_global());
        assert_eq!(scope.to_string(), "voice:soprano");
    }

    #[test]
    fn a_global_scope_has_no_voice_to_offer() {
        assert_eq!(EventScope::Global.voice(), None);
        assert!(EventScope::Global.is_global());
        assert_eq!(
            EventScope::StaffLocal(StaffId::new("upper")).voice(),
            None,
            "a staff is not a voice"
        );
        assert_eq!(EventScope::PartLocal(PartId::new("vln1")).voice(), None);
    }

    #[test]
    fn identities_are_stable_text() {
        let id = EventId::new("umt:event:1");
        assert_eq!(id.as_str(), "umt:event:1");
        assert_eq!(id.to_string(), "umt:event:1");
        assert_eq!(id, EventId::new("umt:event:1"));
        assert_ne!(id, EventId::new("umt:event:2"));
    }
}
