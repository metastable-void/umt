//! The score as an event-indexed object (UMT-3.2 part VI).
//!
//! Section 6.1 is a data-modelling argument, not a theorem: a bare pair of a
//! pitch aggregate and a time aggregate cannot say which pitch belongs to
//! which event, which voice produced it, or which duration goes with which
//! notehead. UMT-3.2 therefore makes event identity primary, and so does
//! [`Score`]: it is a map from [`crate::score::EventId`] to
//! [`crate::score::ScoreEvent`], and every other structure refers to events by
//! identity.
//!
//! # Ties are relations, not merges
//!
//! Section 5.2.2 and prompt section 26 both insist: a tie relates two
//! *distinct* notated noteheads, and UMT-3.2 does not merge them at L0. So
//! [`ScoreBuilder::tie`] records a relation and changes nothing else, and
//! [`Score::sounding_gestures`] is a separate derived view that combines a tie
//! chain into one sustained gesture while the score keeps both noteheads. That
//! is fixture F9.
//!
//! # Rests, inactivity, and silence are three predicates
//!
//! Section 5.2.4 separates them and this module keeps them separate:
//! [`Score::rests`] finds notated rests, [`Score::voice_is_inactive`] asks
//! whether a voice has anything sounding at a moment, and acoustic silence is
//! not derivable from a score at all - it is a property of a realization, and
//! no method here claims otherwise.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::context::{MappingId, TheoryContext};
use crate::error::ScoreError;
use crate::pitch::{PitchPoint, PitchPointRef, VoiceId};
use crate::proportion::BasisId;
use crate::realization::provenance::ProvenanceId;
use crate::score::event::{ScoreEvent, TemporalPlacement};
use crate::score::id::EventId;
use crate::temperament::image::AmbientElem;
use crate::time::beat::{BeatSpan, BeatTime};
use crate::time::constraint::StpProblem;
use crate::time::meter::Meter;
use crate::time::tempo::TempoMap;

/// A score whose pitches are structural points over an interval group.
pub type StructuralScore<E> = Score<PitchPoint<E>>;

/// A score in wire form, whose pitches are context references.
pub type ScoreRef = Score<PitchPointRef>;

/// A tie between two distinct notated noteheads (UMT-3.2 section 5.2.2).
///
/// UMT layer: L0 relation. The tie is data *about* two events; it does not
/// replace them, and nothing in this crate merges them at L0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tie {
    /// The earlier notehead.
    pub from: EventId,
    /// The later notehead.
    pub to: EventId,
}

impl Tie {
    /// Relates two noteheads.
    #[must_use]
    pub fn new(from: EventId, to: EventId) -> Self {
        Self { from, to }
    }
}

/// Context shared by many events (UMT-3.2 section 6.3).
///
/// UMT layer: mixed. Section 6.3 requires shared context to be *referenced*
/// rather than copied inconsistently into every event, so the theory objects
/// appear here as identifiers resolved against a
/// [`crate::context::TheoryContext`], while the self-contained structures -
/// meter, tempo map, temporal network - are held directly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScoreContext {
    /// Identifier of the proportion basis these pitches are over.
    #[cfg_attr(feature = "serde", serde(default))]
    pub basis: Option<BasisId>,
    /// Identifier of the temperament mapping in force.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mapping: Option<MappingId>,
    /// The metrical structure, where the music is measured.
    #[cfg_attr(feature = "serde", serde(default))]
    pub meter: Option<Meter>,
    /// The tempo map, where one is declared.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tempo: Option<TempoMap>,
    /// The temporal constraint network that constrained placements refer to.
    #[cfg_attr(feature = "serde", serde(default))]
    pub temporal: Option<StpProblem>,
    /// Where this score came from.
    #[cfg_attr(feature = "serde", serde(default))]
    pub provenance: Option<ProvenanceId>,
}

/// A projection of a score, with its loss set declared
/// (UMT-3.2 section 6.4).
///
/// UMT layer: as the projection. Section 6.4 requires a projection used for
/// interchange or round trip to declare what it forgets, so the loss travels
/// with the value rather than in a comment beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection<T> {
    value: T,
    discarded: Vec<&'static str>,
}

impl<T> Projection<T> {
    /// The projected value.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consumes the projection, yielding the value.
    ///
    /// Named for what it does: the loss set is dropped here, and after this
    /// the value no longer knows what it is missing.
    #[must_use]
    pub fn into_value_discarding_loss_set(self) -> T {
        self.value
    }

    /// What this projection forgot.
    #[must_use]
    pub fn discarded(&self) -> &[&'static str] {
        &self.discarded
    }

    /// Whether the projection lost anything at all.
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        self.discarded.is_empty()
    }
}

/// One sustained sounding gesture, derived from a tie chain
/// (UMT-3.2 section 5.2.3).
///
/// UMT layer: L1 structural realization. The source events are retained, in
/// order, so the relation that produced the gesture is never lost - which is
/// what lets a notation export reconstruct the separate noteheads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundingGesture<P> {
    sources: Vec<EventId>,
    span: BeatSpan,
    pitch: Option<P>,
}

impl<P> SoundingGesture<P> {
    /// The notated events this gesture came from, in time order.
    #[must_use]
    pub fn sources(&self) -> &[EventId] {
        &self.sources
    }

    /// The combined structural span.
    #[must_use]
    pub fn span(&self) -> &BeatSpan {
        &self.span
    }

    /// The sounding pitch, where the gesture has one.
    ///
    /// `None` for an unpitched note, which sounds without having a pitch.
    #[must_use]
    pub fn pitch(&self) -> Option<&P> {
        self.pitch.as_ref()
    }

    /// Whether this gesture came from more than one notehead.
    #[must_use]
    pub fn is_tied(&self) -> bool {
        self.sources.len() > 1
    }
}

/// An event-indexed score (UMT-3.2 section 6.2).
///
/// UMT layer: L0/L1, following the pitch attachment `P`.
///
/// Immutable once built, as prompt section 37 requires: a score is the
/// authoring object, and a realization is a separate thing derived from it
/// rather than a mutated version of it. Build one with [`ScoreBuilder`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "P: serde::Serialize",
        deserialize = "P: serde::Deserialize<'de>"
    ))
)]
pub struct Score<P> {
    events: BTreeMap<EventId, ScoreEvent<P>>,
    ties: Vec<Tie>,
    context: ScoreContext,
}

impl<P> Score<P> {
    /// Starts building a score.
    #[must_use]
    pub fn builder() -> ScoreBuilder<P> {
        ScoreBuilder::default()
    }

    /// The event with this identity.
    #[must_use]
    pub fn event(&self, id: &EventId) -> Option<&ScoreEvent<P>> {
        self.events.get(id)
    }

    /// Every event, in identifier order.
    pub fn events(&self) -> impl Iterator<Item = &ScoreEvent<P>> {
        self.events.values()
    }

    /// How many events the score holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the score has no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The declared ties.
    #[must_use]
    pub fn ties(&self) -> &[Tie] {
        &self.ties
    }

    /// The shared context.
    #[must_use]
    pub fn context(&self) -> &ScoreContext {
        &self.context
    }

    /// Every notated rest (UMT-3.2 section 5.2.4).
    ///
    /// A notated rest, not an inferred silence. See
    /// [`Score::voice_is_inactive`] for the other predicate, and note that
    /// acoustic silence is a third thing this crate does not derive from a
    /// score at all.
    pub fn rests(&self) -> impl Iterator<Item = &ScoreEvent<P>> {
        self.events
            .values()
            .filter(|event| event.content().is_rest())
    }

    /// Every event belonging to a voice.
    pub fn voice_events<'a>(
        &'a self,
        voice: &'a VoiceId,
    ) -> impl Iterator<Item = &'a ScoreEvent<P>> {
        self.events
            .values()
            .filter(move |event| event.scope().voice() == Some(voice))
    }

    /// Every event whose placement awaits a temporal solve.
    ///
    /// These have no structural onset, and none is invented for them
    /// (fixture F23).
    pub fn unmeasured_events(&self) -> impl Iterator<Item = &ScoreEvent<P>> {
        self.events
            .values()
            .filter(|event| !event.placement().is_fixed())
    }

    /// Whether a voice has nothing sounding at a structural position.
    ///
    /// This is *voice-local inactivity*, one of the three predicates UMT-3.2
    /// section 5.2.4 keeps apart. It is not the same as a notated rest being
    /// present, and it is certainly not acoustic silence, which depends on a
    /// realization and on the instrument.
    ///
    /// Events without a fixed span cannot be decided here and are ignored, so
    /// a `true` answer means "nothing measured is sounding".
    #[must_use]
    pub fn voice_is_inactive(&self, voice: &VoiceId, at: &BeatTime) -> bool {
        !self.voice_events(voice).any(|event| {
            event.content().is_sounding()
                && event
                    .span()
                    .is_some_and(|span| span.contains(at) && span.end() != at)
        })
    }

    /// **Lossy view.** The pitches alone, indexed by event
    /// (UMT-3.2 section 6.4).
    #[must_use]
    pub fn project_pitches(&self) -> Projection<Vec<(EventId, &P)>>
    where
        P: Clone,
    {
        Projection {
            value: self
                .events
                .values()
                .filter_map(|event| event.pitch().map(|pitch| (event.id().clone(), pitch)))
                .collect(),
            discarded: alloc::vec![
                "temporal placement",
                "event scope and voice identity",
                "ties",
                "rests and control events",
                "shared context",
            ],
        }
    }

    /// **Lossy view.** The temporal placements alone, indexed by event
    /// (UMT-3.2 section 6.4).
    #[must_use]
    pub fn project_times(&self) -> Projection<Vec<(EventId, TemporalPlacement)>> {
        Projection {
            value: self
                .events
                .values()
                .map(|event| (event.id().clone(), event.placement().clone()))
                .collect(),
            discarded: alloc::vec![
                "pitch",
                "event scope and voice identity",
                "ties",
                "shared context",
            ],
        }
    }

    /// The sustained sounding gestures implied by the ties
    /// (UMT-3.2 section 5.2.3).
    ///
    /// Tied noteheads combine into one gesture *here*, in a derived view,
    /// while the score keeps them distinct. Only events with a fixed span
    /// participate; the rest are listed by [`Score::unmeasured_events`].
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::MisorderedTie`] if a tie runs backwards or leaves
    /// a gap that would make the combined span meaningless.
    pub fn sounding_gestures(&self) -> Result<Vec<SoundingGesture<P>>, ScoreError>
    where
        P: Clone + PartialEq,
    {
        let mut next: BTreeMap<&EventId, &EventId> = BTreeMap::new();
        let mut tied_to: BTreeSet<&EventId> = BTreeSet::new();
        for tie in &self.ties {
            next.insert(&tie.from, &tie.to);
            tied_to.insert(&tie.to);
        }

        let mut gestures = Vec::new();
        for event in self.events.values() {
            if !event.content().is_sounding() || tied_to.contains(event.id()) {
                continue;
            }
            let Some(span) = event.span() else {
                continue;
            };
            let Some(pitch) = event.pitch() else {
                // An unpitched note sounds without having a pitch, and is its
                // own gesture: ties relate noteheads of one pitch, so an
                // unpitched note is never in a tie chain.
                gestures.push(SoundingGesture {
                    sources: alloc::vec![event.id().clone()],
                    span,
                    pitch: None,
                });
                continue;
            };

            let mut sources = alloc::vec![event.id().clone()];
            let mut end = span.end().clone();
            let mut cursor = event.id();
            while let Some(following) = next.get(cursor) {
                let next_event =
                    self.events
                        .get(*following)
                        .ok_or_else(|| ScoreError::UnknownEvent {
                            event: (*following).clone(),
                        })?;
                let Some(next_span) = next_event.span() else {
                    return Err(ScoreError::MisorderedTie {
                        from: cursor.clone(),
                        to: (*following).clone(),
                    });
                };
                if next_span.start() != &end {
                    return Err(ScoreError::MisorderedTie {
                        from: cursor.clone(),
                        to: (*following).clone(),
                    });
                }
                end = next_span.end().clone();
                sources.push((*following).clone());
                cursor = *following;
            }

            gestures.push(SoundingGesture {
                sources,
                span: BeatSpan::new(span.start().clone(), end)?,
                pitch: Some(pitch.clone()),
            });
        }

        gestures.sort_by(|left, right| left.span.start().cmp(right.span.start()));
        Ok(gestures)
    }

    /// Rewrites every pitch attachment.
    ///
    /// # Errors
    ///
    /// Propagates whatever the conversion reports.
    pub fn try_map_pitch<Q, F, E>(self, mut convert: F) -> Result<Score<Q>, E>
    where
        F: FnMut(P) -> Result<Q, E>,
    {
        let mut events = BTreeMap::new();
        for (id, event) in self.events {
            events.insert(id, event.try_map_pitch(&mut convert)?);
        }
        Ok(Score {
            events,
            ties: self.ties,
            context: self.context,
        })
    }
}

impl StructuralScore<AmbientElem> {
    /// Produces the wire form of this score.
    ///
    /// # Errors
    ///
    /// Infallible in practice; the signature mirrors
    /// [`ScoreRef::resolve_ambient`].
    pub fn to_ref(&self) -> Result<ScoreRef, ScoreError> {
        self.clone()
            .try_map_pitch(|pitch| Ok(PitchPointRef::of_ambient(&pitch)))
    }
}

impl ScoreRef {
    /// Resolves this score's pitch references against a context.
    ///
    /// # Errors
    ///
    /// Propagates an unresolved lattice or a coordinate-rank mismatch.
    pub fn resolve_ambient(
        &self,
        context: &TheoryContext,
    ) -> Result<StructuralScore<AmbientElem>, ScoreError> {
        self.clone()
            .try_map_pitch(|reference| Ok(reference.resolve_ambient(context)?))
    }
}

/// Builds an immutable [`Score`], validating as it goes (prompt section 52).
#[derive(Debug, Clone)]
pub struct ScoreBuilder<P> {
    events: BTreeMap<EventId, ScoreEvent<P>>,
    ties: Vec<Tie>,
    context: ScoreContext,
}

impl<P> Default for ScoreBuilder<P> {
    fn default() -> Self {
        Self {
            events: BTreeMap::new(),
            ties: Vec::new(),
            context: ScoreContext::default(),
        }
    }
}

impl<P> ScoreBuilder<P> {
    /// Adds an event.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::DuplicateEvent`] if the identity is already used.
    /// Event identity is the score's index, so a repeat is a defect rather
    /// than an update.
    pub fn event(mut self, event: ScoreEvent<P>) -> Result<Self, ScoreError> {
        if self.events.contains_key(event.id()) {
            return Err(ScoreError::DuplicateEvent {
                event: event.id().clone(),
            });
        }
        self.events.insert(event.id().clone(), event);
        Ok(self)
    }

    /// Declares a tie between two noteheads.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::UnknownEvent`] for an unknown endpoint,
    /// [`ScoreError::SelfTie`] for a tie from an event to itself,
    /// [`ScoreError::TieBetweenNonNotes`] if either endpoint is not a
    /// notehead, [`ScoreError::TieAcrossScopes`] if the two belong to
    /// different voices, and [`ScoreError::TiedPitchesDiffer`] if the pitches
    /// disagree - a relation between different pitches is a slur or a
    /// glissando, not a tie.
    pub fn tie(mut self, tie: Tie) -> Result<Self, ScoreError>
    where
        P: PartialEq,
    {
        if tie.from == tie.to {
            return Err(ScoreError::SelfTie { event: tie.from });
        }
        let from = self
            .events
            .get(&tie.from)
            .ok_or_else(|| ScoreError::UnknownEvent {
                event: tie.from.clone(),
            })?;
        let to = self
            .events
            .get(&tie.to)
            .ok_or_else(|| ScoreError::UnknownEvent {
                event: tie.to.clone(),
            })?;

        let (Some(left), Some(right)) = (from.pitch(), to.pitch()) else {
            return Err(ScoreError::TieBetweenNonNotes {
                from: tie.from,
                to: tie.to,
            });
        };
        if from.scope() != to.scope() {
            return Err(ScoreError::TieAcrossScopes {
                from: tie.from,
                to: tie.to,
            });
        }
        if left != right {
            return Err(ScoreError::TiedPitchesDiffer {
                from: tie.from,
                to: tie.to,
            });
        }
        self.ties.push(tie);
        Ok(self)
    }

    /// Sets the shared context.
    #[must_use]
    pub fn context(mut self, context: ScoreContext) -> Self {
        self.context = context;
        self
    }

    /// Freezes the score, validating cross-event references.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::UnknownEvent`] if a grace event anchors to an
    /// event that is not present, [`ScoreError::GraceAnchorIsGrace`] if it
    /// anchors to another grace event, which would leave the chain with
    /// nothing to stand on, and
    /// [`ScoreError::UndeclaredTemporalVariable`] if a constrained placement
    /// refers to a variable the declared network does not contain.
    pub fn build(self) -> Result<Score<P>, ScoreError> {
        for event in self.events.values() {
            match event.placement() {
                TemporalPlacement::Grace { anchor, .. } => {
                    let anchored =
                        self.events
                            .get(anchor)
                            .ok_or_else(|| ScoreError::UnknownEvent {
                                event: anchor.clone(),
                            })?;
                    if matches!(anchored.placement(), TemporalPlacement::Grace { .. }) {
                        return Err(ScoreError::GraceAnchorIsGrace {
                            event: event.id().clone(),
                        });
                    }
                }
                TemporalPlacement::ConstraintPlacement { .. } => {
                    if let Some(network) = &self.context.temporal {
                        for variable in event.placement().variables() {
                            if !network.variables().contains(variable) {
                                return Err(ScoreError::UndeclaredTemporalVariable {
                                    variable: variable.as_str().into(),
                                });
                            }
                        }
                    }
                }
                TemporalPlacement::FixedSpan { .. } => {}
            }
        }

        Ok(Score {
            events: self.events,
            ties: self.ties,
            context: self.context,
        })
    }
}

impl<P> core::fmt::Display for Score<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "score of {} events, {} ties",
            self.events.len(),
            self.ties.len()
        )
    }
}

/// A description of what a projection discarded, as text.
#[must_use]
pub fn loss_set_description(discarded: &[&'static str]) -> String {
    discarded.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{Score, ScoreContext, Tie};
    use crate::error::ScoreError;
    use crate::pitch::{PitchOrigin, PitchPoint, VoiceId};
    use crate::score::event::{EventContent, GraceRule, ScoreEvent, TemporalPlacement};
    use crate::score::id::{EventId, EventScope};
    use crate::temperament::image::{AmbientElem, AmbientLattice};
    use crate::time::beat::{BeatDuration, BeatTime, Beats};
    use crate::time::constraint::{DifferenceConstraint, StpProblem, TimeVarId};
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    fn steps() -> Arc<AmbientLattice> {
        AmbientLattice::new("umt:edo:12", 1)
    }

    fn pitch(step: i64) -> PitchPoint<AmbientElem> {
        PitchPoint::new(
            PitchOrigin::new("umt:origin:c4"),
            steps().element([step]).unwrap(),
        )
    }

    fn soprano() -> EventScope {
        EventScope::VoiceLocal(VoiceId::new("soprano"))
    }

    fn note(id: &str, onset: i64, duration: i64, step: i64) -> ScoreEvent<PitchPoint<AmbientElem>> {
        ScoreEvent::new(
            EventId::new(id),
            soprano(),
            TemporalPlacement::fixed(
                BeatTime::ratio(onset, 1).unwrap(),
                BeatDuration::ratio(duration, 1).unwrap(),
            ),
            EventContent::Note { pitch: pitch(step) },
        )
        .unwrap()
    }

    #[test]
    fn a_score_is_indexed_by_event_identity() {
        let score = Score::builder()
            .event(note("n1", 0, 2, 0))
            .unwrap()
            .event(note("n2", 2, 2, 7))
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(score.len(), 2);
        assert!(!score.is_empty());
        assert_eq!(
            score.event(&EventId::new("n1")).unwrap().pitch(),
            Some(&pitch(0))
        );
        assert!(score.event(&EventId::new("n3")).is_none());
        assert_eq!(score.to_string(), "score of 2 events, 0 ties");
    }

    #[test]
    fn duplicate_event_identities_are_rejected() {
        let builder = Score::builder().event(note("n1", 0, 2, 0)).unwrap();
        assert!(matches!(
            builder.event(note("n1", 2, 2, 7)),
            Err(ScoreError::DuplicateEvent { .. })
        ));
    }

    #[test]
    fn a_tie_relates_two_noteheads_and_merges_neither() {
        // Two tied noteheads across a barline: 2 beats then 2 beats.
        let score = Score::builder()
            .event(note("n1", 2, 2, 0))
            .unwrap()
            .event(note("n2", 4, 2, 0))
            .unwrap()
            .tie(Tie::new(EventId::new("n1"), EventId::new("n2")))
            .unwrap()
            .build()
            .unwrap();

        // Both noteheads survive, distinct.
        assert_eq!(score.len(), 2);
        assert_eq!(score.ties().len(), 1);
        assert!(score.event(&EventId::new("n1")).is_some());
        assert!(score.event(&EventId::new("n2")).is_some());

        // And the realization view combines them into one gesture.
        let gestures = score.sounding_gestures().unwrap();
        assert_eq!(gestures.len(), 1);
        assert!(gestures[0].is_tied());
        assert_eq!(
            gestures[0].sources(),
            &[EventId::new("n1"), EventId::new("n2")]
        );
        assert_eq!(gestures[0].span().duration(), Beats::ratio(4, 1).unwrap());
        assert_eq!(*gestures[0].span().start(), BeatTime::ratio(2, 1).unwrap());
    }

    #[test]
    fn ties_are_validated_against_what_a_tie_means() {
        let base = Score::builder()
            .event(note("n1", 0, 2, 0))
            .unwrap()
            .event(note("n2", 2, 2, 0))
            .unwrap()
            .event(note("n3", 4, 2, 7))
            .unwrap()
            .event(
                ScoreEvent::new(
                    EventId::new("r1"),
                    soprano(),
                    TemporalPlacement::fixed(BeatTime::ratio(6, 1).unwrap(), BeatDuration::one()),
                    EventContent::Rest,
                )
                .unwrap(),
            )
            .unwrap()
            .event(
                ScoreEvent::new(
                    EventId::new("n4"),
                    EventScope::VoiceLocal(VoiceId::new("alto")),
                    TemporalPlacement::fixed(
                        BeatTime::ratio(2, 1).unwrap(),
                        BeatDuration::ratio(2, 1).unwrap(),
                    ),
                    EventContent::Note { pitch: pitch(0) },
                )
                .unwrap(),
            )
            .unwrap();

        // A tie to itself is not a relation between distinct noteheads.
        assert!(matches!(
            base.clone()
                .tie(Tie::new(EventId::new("n1"), EventId::new("n1"))),
            Err(ScoreError::SelfTie { .. })
        ));
        // A tie to a rest is not a tie.
        assert!(matches!(
            base.clone()
                .tie(Tie::new(EventId::new("n1"), EventId::new("r1"))),
            Err(ScoreError::TieBetweenNonNotes { .. })
        ));
        // A tie across voices is not a tie.
        assert!(matches!(
            base.clone()
                .tie(Tie::new(EventId::new("n1"), EventId::new("n4"))),
            Err(ScoreError::TieAcrossScopes { .. })
        ));
        // A relation between different pitches is a slur, not a tie.
        assert!(matches!(
            base.clone()
                .tie(Tie::new(EventId::new("n1"), EventId::new("n3"))),
            Err(ScoreError::TiedPitchesDiffer { .. })
        ));
        // An unknown endpoint.
        assert!(matches!(
            base.clone()
                .tie(Tie::new(EventId::new("n1"), EventId::new("ghost"))),
            Err(ScoreError::UnknownEvent { .. })
        ));
        // And the one that is a tie.
        assert!(
            base.tie(Tie::new(EventId::new("n1"), EventId::new("n2")))
                .is_ok()
        );
    }

    #[test]
    fn a_tie_that_leaves_a_gap_is_reported() {
        let score = Score::builder()
            .event(note("n1", 0, 2, 0))
            .unwrap()
            .event(note("n2", 5, 2, 0))
            .unwrap()
            .tie(Tie::new(EventId::new("n1"), EventId::new("n2")))
            .unwrap()
            .build()
            .unwrap();
        assert!(matches!(
            score.sounding_gestures(),
            Err(ScoreError::MisorderedTie { .. })
        ));
    }

    #[test]
    fn a_chain_of_three_ties_becomes_one_gesture() {
        let score = Score::builder()
            .event(note("n1", 0, 1, 0))
            .unwrap()
            .event(note("n2", 1, 1, 0))
            .unwrap()
            .event(note("n3", 2, 1, 0))
            .unwrap()
            .tie(Tie::new(EventId::new("n1"), EventId::new("n2")))
            .unwrap()
            .tie(Tie::new(EventId::new("n2"), EventId::new("n3")))
            .unwrap()
            .build()
            .unwrap();

        let gestures = score.sounding_gestures().unwrap();
        assert_eq!(gestures.len(), 1);
        assert_eq!(gestures[0].sources().len(), 3);
        assert_eq!(gestures[0].span().duration(), Beats::ratio(3, 1).unwrap());
        assert_eq!(score.len(), 3, "and all three noteheads survive");
    }

    #[test]
    fn rests_inactivity_and_silence_are_different_questions() {
        let score = Score::builder()
            .event(note("n1", 0, 2, 0))
            .unwrap()
            .event(
                ScoreEvent::new(
                    EventId::new("r1"),
                    soprano(),
                    TemporalPlacement::fixed(BeatTime::ratio(2, 1).unwrap(), BeatDuration::one()),
                    EventContent::Rest,
                )
                .unwrap(),
            )
            .unwrap()
            .build()
            .unwrap();

        let soprano_voice = VoiceId::new("soprano");
        assert_eq!(score.rests().count(), 1, "one notated rest");

        // Inactivity is a different predicate: the voice is inactive during
        // the rest, and also after beat 3 where nothing is notated at all.
        assert!(!score.voice_is_inactive(&soprano_voice, &BeatTime::ratio(1, 1).unwrap()));
        assert!(score.voice_is_inactive(&soprano_voice, &BeatTime::ratio(2, 1).unwrap()));
        assert!(
            score.voice_is_inactive(&soprano_voice, &BeatTime::ratio(9, 1).unwrap()),
            "inactive where nothing is notated, though no rest is written there"
        );

        // A voice that does not appear at all is inactive everywhere.
        assert!(score.voice_is_inactive(&VoiceId::new("bass"), &BeatTime::zero()));
    }

    #[test]
    fn projections_declare_what_they_forget() {
        let score = Score::builder()
            .event(note("n1", 0, 2, 0))
            .unwrap()
            .event(
                ScoreEvent::new(
                    EventId::new("tempo"),
                    EventScope::Global,
                    TemporalPlacement::fixed(BeatTime::zero(), BeatDuration::one()),
                    EventContent::Control {
                        declared_type: "umt:control:tempo".into(),
                        value: "quarter = 120".into(),
                    },
                )
                .unwrap(),
            )
            .unwrap()
            .build()
            .unwrap();

        let pitches = score.project_pitches();
        assert_eq!(pitches.value().len(), 1, "the control event has no pitch");
        assert!(!pitches.is_lossless());
        assert!(pitches.discarded().contains(&"ties"));
        assert!(pitches.discarded().contains(&"temporal placement"));

        let times = score.project_times();
        assert_eq!(times.value().len(), 2, "both events have a placement");
        assert!(times.discarded().contains(&"pitch"));
        assert_eq!(
            super::loss_set_description(times.discarded()),
            "pitch, event scope and voice identity, ties, shared context"
        );
    }

    #[test]
    fn a_constrained_placement_needs_its_variable_declared() {
        let mut network = StpProblem::new();
        let cue = network.variable("cue");
        let entry = network.variable("entry");
        network
            .constrain(DifferenceConstraint::at_least(
                &cue,
                &entry,
                crate::algebra::Q::from(crate::algebra::Z::from(0)),
            ))
            .unwrap();

        let unmeasured = ScoreEvent::new(
            EventId::new("free"),
            soprano(),
            TemporalPlacement::ConstraintPlacement {
                onset: TimeVarId::new("entry"),
                offset: None,
            },
            EventContent::Note { pitch: pitch(0) },
        )
        .unwrap();

        let context = ScoreContext {
            temporal: Some(network),
            ..ScoreContext::default()
        };
        let score = Score::builder()
            .event(unmeasured.clone())
            .unwrap()
            .context(context.clone())
            .build()
            .unwrap();
        assert_eq!(score.unmeasured_events().count(), 1);
        assert!(score.event(&EventId::new("free")).unwrap().span().is_none());

        // A variable the network does not declare is a defect.
        let stray = ScoreEvent::new(
            EventId::new("stray"),
            soprano(),
            TemporalPlacement::ConstraintPlacement {
                onset: TimeVarId::new("nowhere"),
                offset: None,
            },
            EventContent::Note { pitch: pitch(0) },
        )
        .unwrap();
        assert!(matches!(
            Score::builder()
                .event(stray)
                .unwrap()
                .context(context)
                .build(),
            Err(ScoreError::UndeclaredTemporalVariable { .. })
        ));
    }

    #[test]
    fn grace_anchors_are_validated() {
        let grace = |id: &str, anchor: &str| {
            ScoreEvent::new(
                EventId::new(id),
                soprano(),
                TemporalPlacement::Grace {
                    anchor: EventId::new(anchor),
                    rule: GraceRule::BeforeAnchor,
                },
                EventContent::Note { pitch: pitch(2) },
            )
            .unwrap()
        };

        // An anchor that is not in the score.
        assert!(matches!(
            Score::builder()
                .event(grace("g1", "missing"))
                .unwrap()
                .build(),
            Err(ScoreError::UnknownEvent { .. })
        ));

        // An anchor that is itself a grace event has nothing to stand on.
        assert!(matches!(
            Score::builder()
                .event(note("n1", 0, 1, 0))
                .unwrap()
                .event(grace("g1", "n1"))
                .unwrap()
                .event(grace("g2", "g1"))
                .unwrap()
                .build(),
            Err(ScoreError::GraceAnchorIsGrace { .. })
        ));

        // A grace note on a real note is fine, and has no fixed span.
        let score = Score::builder()
            .event(note("n1", 0, 1, 0))
            .unwrap()
            .event(grace("g1", "n1"))
            .unwrap()
            .build()
            .unwrap();
        assert!(score.event(&EventId::new("g1")).unwrap().span().is_none());
        assert_eq!(score.unmeasured_events().count(), 1);
    }

    #[test]
    fn unmeasured_events_do_not_appear_as_gestures() {
        let score = Score::builder()
            .event(note("n1", 0, 1, 0))
            .unwrap()
            .event(
                ScoreEvent::new(
                    EventId::new("free"),
                    soprano(),
                    TemporalPlacement::ConstraintPlacement {
                        onset: TimeVarId::new("entry"),
                        offset: None,
                    },
                    EventContent::Note { pitch: pitch(4) },
                )
                .unwrap(),
            )
            .unwrap()
            .build()
            .unwrap();

        let gestures: Vec<_> = score.sounding_gestures().unwrap();
        assert_eq!(gestures.len(), 1, "only the measured note has a span");
        assert_eq!(score.unmeasured_events().count(), 1);
    }
}
