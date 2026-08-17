//! Score events (UMT-3.2 sections 5.2 and 6.2, prompt sections 25 and 26).
//!
//! An event is an identity, a scope, a temporal placement, and content. Not
//! every event is pitched and not every event is voice-local: rests, breath
//! marks, global tempo events, and structural markers share one event-indexed
//! framework, and the *content* determines what else is required.
//!
//! # Placement is an enum, not a bag of options
//!
//! Prompt section 25 asks for an enum rather than many optional fields, and
//! forbids "a fixed event to simultaneously carry contradictory unsolved
//! placement variables". [`TemporalPlacement`] has three variants and no way
//! to be two of them at once, so the contradiction is unrepresentable rather
//! than merely discouraged.
//!
//! That is also what makes fixture F23 work. An event whose onset is only
//! constrained relative to another event is
//! [`TemporalPlacement::ConstraintPlacement`], and no rational onset is
//! fabricated to fill a field that would otherwise be empty.

use alloc::string::String;

use crate::error::ScoreError;
use crate::realization::provenance::ProvenanceId;
use crate::score::id::{EventId, EventScope};
use crate::time::beat::{BeatDuration, BeatSpan, BeatTime};
use crate::time::constraint::TimeVarId;

/// How a grace event relates to its anchor.
///
/// UMT layer: L0/L1 notation policy. A grace note has no structural duration
/// of its own; where its time comes from is a declared rule rather than an
/// arithmetic fact, which is why this is an enum and not a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum GraceRule {
    /// Sounds before the anchor's notated onset, taking time from what
    /// precedes it.
    BeforeAnchor,
    /// Sounds at the anchor's onset, taking time from the anchor.
    OnAnchor,
    /// Unmeasured: the realization decides, under its own declared policy.
    Unmeasured,
}

/// Where an event sits on the structural timeline (prompt section 25).
///
/// UMT layer: L1, exact where it is fixed at all.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TemporalPlacement {
    /// A fixed exact span: an onset and a positive structural duration.
    FixedSpan {
        /// The structural onset.
        onset: BeatTime,
        /// The structural duration.
        duration: BeatDuration,
    },
    /// Onset, and optionally offset, as temporal variables to be solved by a
    /// constraint network (UMT-3.2 section 5.10.5).
    ///
    /// This is how unmeasured music is represented. No rational onset is
    /// invented: the event genuinely does not have one until the network is
    /// solved, and it is a valid score event regardless (fixture F23).
    ConstraintPlacement {
        /// The variable standing for the onset.
        onset: TimeVarId,
        /// The variable standing for the offset, where the event has one.
        #[cfg_attr(feature = "serde", serde(default))]
        offset: Option<TimeVarId>,
    },
    /// A grace event, placed relative to an anchor by a declared rule.
    Grace {
        /// The event this one is attached to.
        anchor: EventId,
        /// How it relates to the anchor.
        rule: GraceRule,
    },
}

impl TemporalPlacement {
    /// A fixed span from an onset and a duration.
    #[must_use]
    pub fn fixed(onset: BeatTime, duration: BeatDuration) -> Self {
        Self::FixedSpan { onset, duration }
    }

    /// The exact structural span, where the placement fixes one.
    ///
    /// `None` for constrained and grace placements. That is not a failure: it
    /// is the honest answer, and the reason a caller must handle it is the
    /// reason fixture F23 exists.
    #[must_use]
    pub fn span(&self) -> Option<BeatSpan> {
        match self {
            Self::FixedSpan { onset, duration } => {
                Some(BeatSpan::from_duration(onset.clone(), duration))
            }
            Self::ConstraintPlacement { .. } | Self::Grace { .. } => None,
        }
    }

    /// The exact structural onset, where the placement fixes one.
    #[must_use]
    pub fn onset(&self) -> Option<&BeatTime> {
        match self {
            Self::FixedSpan { onset, .. } => Some(onset),
            Self::ConstraintPlacement { .. } | Self::Grace { .. } => None,
        }
    }

    /// Whether this placement is fixed on the structural timeline.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::FixedSpan { .. })
    }

    /// Whether this placement awaits a temporal solve.
    #[must_use]
    pub fn is_constrained(&self) -> bool {
        matches!(self, Self::ConstraintPlacement { .. })
    }

    /// The temporal variables this placement refers to.
    #[must_use]
    pub fn variables(&self) -> alloc::vec::Vec<&TimeVarId> {
        match self {
            Self::ConstraintPlacement { onset, offset } => {
                let mut out = alloc::vec![onset];
                if let Some(offset) = offset {
                    out.push(offset);
                }
                out
            }
            Self::FixedSpan { .. } | Self::Grace { .. } => alloc::vec::Vec::new(),
        }
    }
}

/// What an event *is* (UMT-3.2 section 6.2).
///
/// UMT layer: L0/L1, following the pitch attachment.
///
/// Generic over the pitch attachment `P` so the same event type serves an
/// in-memory score, where `P` is a [`crate::pitch::PitchPoint`], and a
/// document, where `P` is a [`crate::pitch::PitchPointRef`] resolved against a
/// context. Nothing else has to be duplicated for serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum EventContent<P> {
    /// A pitched notehead.
    Note {
        /// Its structural pitch.
        pitch: P,
    },
    /// A notated note without determinate pitch: unpitched percussion,
    /// spoken text, a rhythmic cue.
    UnpitchedNote,
    /// A notated rest.
    ///
    /// A rest is a notated event in a voice or staff context, *not* the global
    /// complement of sounding intervals (UMT-3.2 section 5.2.4). A rest with
    /// [`EventScope::Global`] is therefore rejected at construction.
    Rest,
    /// A control event: a tempo mark, a dynamic, a pedal change.
    ///
    /// The type is a declared identifier and the value is text, so a document
    /// can carry controls this crate has never heard of without them becoming
    /// executable or untyped.
    Control {
        /// A declared identifier naming the control's semantics.
        declared_type: String,
        /// Its value, in the declared type's own vocabulary.
        value: String,
    },
    /// A structural marker: a rehearsal letter, a section boundary.
    Marker {
        /// The marker's label.
        label: String,
    },
}

impl<P> EventContent<P> {
    /// Whether this content sounds, that is, is a note of some kind.
    #[must_use]
    pub fn is_sounding(&self) -> bool {
        matches!(self, Self::Note { .. } | Self::UnpitchedNote)
    }

    /// Whether this content is a notated rest.
    #[must_use]
    pub fn is_rest(&self) -> bool {
        matches!(self, Self::Rest)
    }

    /// The attached pitch, where there is one.
    #[must_use]
    pub fn pitch(&self) -> Option<&P> {
        match self {
            Self::Note { pitch } => Some(pitch),
            _ => None,
        }
    }

    /// Rewrites the pitch attachment.
    ///
    /// This is what turns an in-memory score into its wire form and back,
    /// without a parallel type hierarchy.
    ///
    /// # Errors
    ///
    /// Propagates whatever the conversion reports.
    pub fn try_map_pitch<Q, F, E>(self, convert: F) -> Result<EventContent<Q>, E>
    where
        F: FnOnce(P) -> Result<Q, E>,
    {
        Ok(match self {
            Self::Note { pitch } => EventContent::Note {
                pitch: convert(pitch)?,
            },
            Self::UnpitchedNote => EventContent::UnpitchedNote,
            Self::Rest => EventContent::Rest,
            Self::Control {
                declared_type,
                value,
            } => EventContent::Control {
                declared_type,
                value,
            },
            Self::Marker { label } => EventContent::Marker { label },
        })
    }
}

/// One event of a score (UMT-3.2 section 6.2).
///
/// UMT layer: L0/L1.
///
/// Built through [`ScoreEvent::new`], which rejects the combinations the
/// specification forbids rather than accepting them and hoping a later stage
/// notices.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "P: serde::Serialize",
        deserialize = "P: serde::Deserialize<'de>"
    ))
)]
pub struct ScoreEvent<P> {
    id: EventId,
    scope: EventScope,
    placement: TemporalPlacement,
    content: EventContent<P>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    provenance: Option<ProvenanceId>,
}

impl<P> ScoreEvent<P> {
    /// Builds an event, validating scope against content.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::SoundingEventWithoutContext`] for a note or rest
    /// with [`EventScope::Global`]. A sounding event or a notated rest belongs
    /// to a voice, a staff, or a part; a "global rest" would be the
    /// set-theoretic complement of sound that UMT-3.2 section 5.2.4 explicitly
    /// rejects.
    pub fn new(
        id: EventId,
        scope: EventScope,
        placement: TemporalPlacement,
        content: EventContent<P>,
    ) -> Result<Self, ScoreError> {
        if scope.is_global() && (content.is_sounding() || content.is_rest()) {
            return Err(ScoreError::SoundingEventWithoutContext { event: id });
        }
        Ok(Self {
            id,
            scope,
            placement,
            content,
            provenance: None,
        })
    }

    /// Attaches a provenance reference.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// The event's identity.
    #[must_use]
    pub fn id(&self) -> &EventId {
        &self.id
    }

    /// What the event belongs to.
    #[must_use]
    pub fn scope(&self) -> &EventScope {
        &self.scope
    }

    /// Where the event sits on the structural timeline.
    #[must_use]
    pub fn placement(&self) -> &TemporalPlacement {
        &self.placement
    }

    /// What the event is.
    #[must_use]
    pub fn content(&self) -> &EventContent<P> {
        &self.content
    }

    /// The provenance of this event, if it has one.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// The exact structural span, where the placement fixes one.
    #[must_use]
    pub fn span(&self) -> Option<BeatSpan> {
        self.placement.span()
    }

    /// The attached pitch, where there is one.
    #[must_use]
    pub fn pitch(&self) -> Option<&P> {
        self.content.pitch()
    }

    /// Rewrites the pitch attachment, keeping everything else.
    ///
    /// # Errors
    ///
    /// Propagates whatever the conversion reports.
    pub fn try_map_pitch<Q, F, E>(self, convert: F) -> Result<ScoreEvent<Q>, E>
    where
        F: FnOnce(P) -> Result<Q, E>,
    {
        Ok(ScoreEvent {
            id: self.id,
            scope: self.scope,
            placement: self.placement,
            content: self.content.try_map_pitch(convert)?,
            provenance: self.provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EventContent, GraceRule, ScoreEvent, TemporalPlacement};
    use crate::error::ScoreError;
    use crate::pitch::VoiceId;
    use crate::score::id::{EventId, EventScope, StaffId};
    use crate::time::beat::{BeatDuration, BeatTime, Beats};
    use crate::time::constraint::TimeVarId;

    fn voice() -> EventScope {
        EventScope::VoiceLocal(VoiceId::new("soprano"))
    }

    #[test]
    fn a_fixed_placement_yields_an_exact_span() {
        let placement = TemporalPlacement::fixed(
            BeatTime::ratio(3, 2).unwrap(),
            BeatDuration::ratio(1, 3).unwrap(),
        );
        let span = placement.span().unwrap();
        assert_eq!(*span.start(), BeatTime::ratio(3, 2).unwrap());
        assert_eq!(span.duration(), Beats::ratio(1, 3).unwrap());
        assert!(placement.is_fixed());
        assert!(!placement.is_constrained());
        assert!(placement.variables().is_empty());
    }

    #[test]
    fn a_constrained_placement_has_no_onset_to_offer() {
        let placement = TemporalPlacement::ConstraintPlacement {
            onset: TimeVarId::new("entry"),
            offset: Some(TimeVarId::new("entry-end")),
        };
        assert_eq!(placement.onset(), None, "and none is fabricated");
        assert_eq!(placement.span(), None);
        assert!(placement.is_constrained());
        assert_eq!(placement.variables().len(), 2);
    }

    #[test]
    fn a_grace_placement_names_its_anchor_and_rule() {
        let placement = TemporalPlacement::Grace {
            anchor: EventId::new("e2"),
            rule: GraceRule::BeforeAnchor,
        };
        assert!(!placement.is_fixed());
        assert_eq!(placement.span(), None);
        assert_ne!(GraceRule::BeforeAnchor, GraceRule::OnAnchor);
    }

    #[test]
    fn a_rest_is_never_global() {
        let placement = TemporalPlacement::fixed(BeatTime::zero(), BeatDuration::one());
        assert!(matches!(
            ScoreEvent::<()>::new(
                EventId::new("r1"),
                EventScope::Global,
                placement.clone(),
                EventContent::Rest,
            ),
            Err(ScoreError::SoundingEventWithoutContext { .. })
        ));
        assert!(
            ScoreEvent::<()>::new(
                EventId::new("r1"),
                EventScope::StaffLocal(StaffId::new("upper")),
                placement.clone(),
                EventContent::Rest,
            )
            .is_ok(),
            "a staff context is enough"
        );
        assert!(
            ScoreEvent::<()>::new(EventId::new("r1"), voice(), placement, EventContent::Rest,)
                .is_ok()
        );
    }

    #[test]
    fn a_note_is_never_global_either() {
        let placement = TemporalPlacement::fixed(BeatTime::zero(), BeatDuration::one());
        assert!(
            ScoreEvent::new(
                EventId::new("n1"),
                EventScope::Global,
                placement.clone(),
                EventContent::Note { pitch: 60u8 },
            )
            .is_err()
        );
        assert!(
            ScoreEvent::<u8>::new(
                EventId::new("n1"),
                EventScope::Global,
                placement,
                EventContent::UnpitchedNote,
            )
            .is_err()
        );
    }

    #[test]
    fn a_global_control_event_needs_no_voice() {
        let event = ScoreEvent::<()>::new(
            EventId::new("tempo-1"),
            EventScope::Global,
            TemporalPlacement::fixed(BeatTime::zero(), BeatDuration::one()),
            EventContent::Control {
                declared_type: "umt:control:tempo".into(),
                value: "quarter = 120".into(),
            },
        )
        .unwrap();
        assert!(event.scope().is_global());
        assert_eq!(event.scope().voice(), None);
        assert!(!event.content().is_sounding());
        assert!(event.pitch().is_none());
    }

    #[test]
    fn the_pitch_attachment_can_be_rewritten_without_touching_anything_else() {
        let event = ScoreEvent::new(
            EventId::new("n1"),
            voice(),
            TemporalPlacement::fixed(BeatTime::zero(), BeatDuration::one()),
            EventContent::Note { pitch: 7i64 },
        )
        .unwrap()
        .with_provenance(crate::realization::provenance::ProvenanceId::new("p1"));

        let rewritten: ScoreEvent<alloc::string::String> = event
            .clone()
            .try_map_pitch(|pitch| Ok::<_, core::convert::Infallible>(alloc::format!("{pitch}")))
            .unwrap();

        assert_eq!(rewritten.id(), event.id());
        assert_eq!(rewritten.scope(), event.scope());
        assert_eq!(rewritten.placement(), event.placement());
        assert_eq!(rewritten.provenance(), event.provenance());
        assert_eq!(rewritten.pitch().unwrap(), "7");
    }
}
