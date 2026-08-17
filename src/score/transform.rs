//! Score transformations (UMT-3.2 sections 6.5 and 6.6).
//!
//! A score transformation is four things at once: a relation between source
//! and destination event identities, a transformation of the attached pitch
//! structures, a transformation of the attached temporal structures, and rules
//! for notation and provenance. [`ScoreTransform`] carries all four, because
//! carrying fewer would make "transpose everything after bar 8" inexpressible.
//!
//! # The word "functorial" has a price
//!
//! Section 6.6 is unusually blunt: a profile claiming compositional score
//! morphisms MUST define identity transformations, composition of event
//! relations, composition of pitch components, composition of temporal
//! components, and how residual and provenance records compose - and "a label
//! such as 'functorial' MUST NOT be used without these operations and their
//! laws".
//!
//! So the claim is a value. [`ScoreTransform::compose`] returns `None` exactly
//! when one of the components cannot compose, which is precisely when the
//! component is [`PitchTransform::Declared`] or [`TimeTransform::Declared`] -
//! an application-supplied transformation this crate knows nothing about.
//! [`ScoreTransform::claims_compositional`] answers the question directly.

use alloc::string::String;
use alloc::vec::Vec;
use num_traits::{One, Signed};

use crate::algebra::Q;
use crate::error::ScoreError;
use crate::realization::provenance::ProvenanceId;
use crate::score::id::EventId;
use crate::time::beat::{BeatTime, Beats};

/// A relation between source and destination event identities
/// (UMT-3.2 section 6.5).
///
/// UMT layer: structural. A span rather than a function, because splits,
/// merges, insertions, and deletions are ordinary score edits and a function
/// cannot express any of them. Where the relation happens to be one-to-one,
/// [`EventRelation::is_bijective`] says so.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventRelation {
    edges: Vec<(EventId, EventId)>,
}

impl EventRelation {
    /// An empty relation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A relation from explicit pairs.
    #[must_use]
    pub fn from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (EventId, EventId)>,
    {
        Self {
            edges: pairs.into_iter().collect(),
        }
    }

    /// The identity relation on a set of events.
    #[must_use]
    pub fn identity<'a, I>(events: I) -> Self
    where
        I: IntoIterator<Item = &'a EventId>,
    {
        Self {
            edges: events
                .into_iter()
                .map(|event| (event.clone(), event.clone()))
                .collect(),
        }
    }

    /// The pairs, in declaration order.
    #[must_use]
    pub fn edges(&self) -> &[(EventId, EventId)] {
        &self.edges
    }

    /// The events this relation maps *to* a destination.
    #[must_use]
    pub fn images(&self, source: &EventId) -> Vec<&EventId> {
        self.edges
            .iter()
            .filter(|(from, _)| from == source)
            .map(|(_, to)| to)
            .collect()
    }

    /// The events a destination came *from*.
    #[must_use]
    pub fn preimages(&self, target: &EventId) -> Vec<&EventId> {
        self.edges
            .iter()
            .filter(|(_, to)| to == target)
            .map(|(from, _)| from)
            .collect()
    }

    /// Whether every source has exactly one image and every destination
    /// exactly one preimage.
    ///
    /// Worth asking, because a transformation that is a bijection can be
    /// described by a function, and section 6.5 says a simpler function *may*
    /// be used there - but only there.
    #[must_use]
    pub fn is_bijective(&self) -> bool {
        let sources: alloc::collections::BTreeSet<&EventId> =
            self.edges.iter().map(|(from, _)| from).collect();
        let targets: alloc::collections::BTreeSet<&EventId> =
            self.edges.iter().map(|(_, to)| to).collect();
        sources.len() == self.edges.len() && targets.len() == self.edges.len()
    }

    /// Composition by pullback: `(e, g)` for every `e -> m` and `m -> g`.
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        let mut edges = Vec::new();
        for (from, middle) in &self.edges {
            for (other_middle, to) in &other.edges {
                if middle == other_middle {
                    edges.push((from.clone(), to.clone()));
                }
            }
        }
        Self { edges }
    }
}

/// A transformation of the attached pitch structures
/// (UMT-3.2 section 6.5).
///
/// UMT layer: as the pitch attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum PitchTransform<E> {
    /// Leaves pitches untouched.
    Identity,
    /// Adds a fixed interval to every pitch: transposition.
    Transpose(E),
    /// An application-declared transformation this crate cannot compose.
    ///
    /// Legitimate and often necessary - a pitch change conditioned on metric
    /// position is exactly the dependent case section 6.5 mentions. It simply
    /// forfeits the compositional claim, which is the honest trade.
    Declared {
        /// A declared identifier naming the transformation.
        name: String,
    },
}

impl<E: Clone + PartialEq> PitchTransform<E> {
    /// Composes two pitch transformations, where both can compose.
    ///
    /// Returns `None` when either is [`PitchTransform::Declared`].
    ///
    /// # Errors
    ///
    /// Propagates interval-group mismatch from adding two transpositions.
    pub fn compose<F>(&self, other: &Self, add: F) -> Option<Result<Self, ScoreError>>
    where
        F: FnOnce(&E, &E) -> Result<E, ScoreError>,
    {
        match (self, other) {
            (Self::Declared { .. }, _) | (_, Self::Declared { .. }) => None,
            (Self::Identity, transform) | (transform, Self::Identity) => {
                Some(Ok(transform.clone()))
            }
            (Self::Transpose(first), Self::Transpose(second)) => {
                Some(add(first, second).map(Self::Transpose))
            }
        }
    }

    /// Whether this component composes.
    #[must_use]
    pub fn is_composable(&self) -> bool {
        !matches!(self, Self::Declared { .. })
    }
}

/// A transformation of the attached temporal structures
/// (UMT-3.2 section 6.5).
///
/// UMT layer: L1, exact.
///
/// The affine family is closed under composition, which is what lets a
/// transposition followed by an augmentation still make a compositional
/// claim. A `scale` must be strictly positive: a non-positive one would
/// reverse or collapse the timeline, which is not a score transformation but a
/// different piece of music.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TimeTransform {
    /// `t -> scale * t + shift`.
    Affine {
        /// The strictly positive scale factor.
        #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
        scale: Q,
        /// The shift, applied after scaling.
        shift: Beats,
    },
    /// An application-declared transformation this crate cannot compose.
    Declared {
        /// A declared identifier naming the transformation.
        name: String,
    },
}

impl TimeTransform {
    /// The identity transformation.
    #[must_use]
    pub fn identity() -> Self {
        Self::Affine {
            scale: Q::one(),
            shift: Beats::zero(),
        }
    }

    /// A pure shift.
    #[must_use]
    pub fn shift(by: Beats) -> Self {
        Self::Affine {
            scale: Q::one(),
            shift: by,
        }
    }

    /// A pure scaling about the origin: augmentation or diminution.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::NonPositiveTimeScale`] for a factor that is not
    /// strictly positive.
    pub fn scale(factor: Q) -> Result<Self, ScoreError> {
        if !factor.is_positive() {
            return Err(ScoreError::NonPositiveTimeScale);
        }
        Ok(Self::Affine {
            scale: factor,
            shift: Beats::zero(),
        })
    }

    /// Applies this transformation to a structural position.
    ///
    /// # Errors
    ///
    /// Returns [`ScoreError::UncomposableTransform`] for a declared
    /// transformation, which this crate cannot evaluate.
    pub fn apply(&self, at: &BeatTime) -> Result<BeatTime, ScoreError> {
        match self {
            Self::Affine { scale, shift } => Ok(BeatTime::new(at.get() * scale + shift.get())),
            Self::Declared { name } => Err(ScoreError::UncomposableTransform {
                component: name.clone(),
            }),
        }
    }

    /// Composes two temporal transformations, where both can compose.
    ///
    /// `self` first, then `other`. Returns `None` when either is
    /// [`TimeTransform::Declared`].
    #[must_use]
    pub fn compose(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Declared { .. }, _) | (_, Self::Declared { .. }) => None,
            (
                Self::Affine {
                    scale: first_scale,
                    shift: first_shift,
                },
                Self::Affine {
                    scale: second_scale,
                    shift: second_shift,
                },
            ) => Some(Self::Affine {
                scale: first_scale * second_scale,
                shift: Beats::new(first_shift.get() * second_scale + second_shift.get()),
            }),
        }
    }

    /// Whether this component composes.
    #[must_use]
    pub fn is_composable(&self) -> bool {
        !matches!(self, Self::Declared { .. })
    }
}

/// How provenance records combine under composition
/// (UMT-3.2 section 6.6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenanceChain {
    steps: Vec<ProvenanceId>,
}

impl ProvenanceChain {
    /// An empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A chain of one step.
    #[must_use]
    pub fn of(step: ProvenanceId) -> Self {
        Self {
            steps: alloc::vec![step],
        }
    }

    /// The steps, oldest first.
    #[must_use]
    pub fn steps(&self) -> &[ProvenanceId] {
        &self.steps
    }

    /// Appends another chain after this one.
    ///
    /// Composition of provenance is concatenation, not replacement: section
    /// 9.12 requires a later re-realization to be able to consult the original
    /// source rather than compound previous rounding, which is only possible
    /// if the earlier steps survive.
    #[must_use]
    pub fn then(&self, other: &Self) -> Self {
        let mut steps = self.steps.clone();
        steps.extend(other.steps.iter().cloned());
        Self { steps }
    }
}

/// A score transformation (UMT-3.2 section 6.5).
///
/// UMT layer: structural, over whatever the score's pitch attachment is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScoreTransform<E> {
    relation: EventRelation,
    pitch: PitchTransform<E>,
    time: TimeTransform,
    provenance: ProvenanceChain,
}

impl<E: Clone + PartialEq> ScoreTransform<E> {
    /// Assembles a transformation from its four components.
    #[must_use]
    pub fn new(
        relation: EventRelation,
        pitch: PitchTransform<E>,
        time: TimeTransform,
        provenance: ProvenanceChain,
    ) -> Self {
        Self {
            relation,
            pitch,
            time,
            provenance,
        }
    }

    /// The identity transformation on a set of events
    /// (UMT-3.2 section 6.6, first requirement).
    #[must_use]
    pub fn identity<'a, I>(events: I) -> Self
    where
        I: IntoIterator<Item = &'a EventId>,
    {
        Self {
            relation: EventRelation::identity(events),
            pitch: PitchTransform::Identity,
            time: TimeTransform::identity(),
            provenance: ProvenanceChain::new(),
        }
    }

    /// The event relation.
    #[must_use]
    pub fn relation(&self) -> &EventRelation {
        &self.relation
    }

    /// The pitch component.
    #[must_use]
    pub fn pitch(&self) -> &PitchTransform<E> {
        &self.pitch
    }

    /// The temporal component.
    #[must_use]
    pub fn time(&self) -> &TimeTransform {
        &self.time
    }

    /// The provenance chain.
    #[must_use]
    pub fn provenance(&self) -> &ProvenanceChain {
        &self.provenance
    }

    /// Whether this transformation may be called compositional
    /// (UMT-3.2 section 6.6).
    ///
    /// True only when every component composes. A transformation with a
    /// declared pitch or temporal component is perfectly usable; it simply
    /// makes no functorial claim, and this method is how it says so.
    #[must_use]
    pub fn claims_compositional(&self) -> bool {
        self.pitch.is_composable() && self.time.is_composable()
    }

    /// Composes two transformations, `self` first.
    ///
    /// Returns `None` when either transformation declines the compositional
    /// claim. That is deliberate: section 6.6 forbids the label without the
    /// operation, so the operation is absent exactly where the label would be
    /// unearned.
    ///
    /// # Errors
    ///
    /// Propagates interval-group mismatch when composing two transpositions.
    pub fn compose<F>(&self, other: &Self, add: F) -> Option<Result<Self, ScoreError>>
    where
        F: FnOnce(&E, &E) -> Result<E, ScoreError>,
    {
        let pitch = self.pitch.compose(&other.pitch, add)?;
        let time = self.time.compose(&other.time)?;
        Some(pitch.map(|pitch| Self {
            relation: self.relation.compose(&other.relation),
            pitch,
            time,
            provenance: self.provenance.then(&other.provenance),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{EventRelation, PitchTransform, ProvenanceChain, ScoreTransform, TimeTransform};
    use crate::algebra::{Q, Z};
    use crate::error::ScoreError;
    use crate::realization::provenance::ProvenanceId;
    use crate::score::id::EventId;
    use crate::time::beat::{BeatTime, Beats};
    use alloc::string::String;

    fn id(name: &str) -> EventId {
        EventId::new(name)
    }

    fn add_i64(left: &i64, right: &i64) -> Result<i64, ScoreError> {
        Ok(left + right)
    }

    #[test]
    fn a_relation_expresses_splits_and_merges() {
        // One event splits into two; two merge into one.
        let split = EventRelation::from_pairs([
            (id("a"), id("x")),
            (id("a"), id("y")),
            (id("b"), id("z")),
            (id("c"), id("z")),
        ]);
        assert_eq!(split.images(&id("a")).len(), 2, "a split");
        assert_eq!(split.preimages(&id("z")).len(), 2, "a merge");
        assert!(!split.is_bijective());

        let renaming = EventRelation::from_pairs([(id("a"), id("x")), (id("b"), id("y"))]);
        assert!(renaming.is_bijective());
    }

    #[test]
    fn relations_compose_by_pullback_with_identity_neutral() {
        let first = EventRelation::from_pairs([(id("a"), id("m")), (id("a"), id("n"))]);
        let second = EventRelation::from_pairs([(id("m"), id("z")), (id("n"), id("z"))]);
        let composite = first.compose(&second);
        assert_eq!(composite.edges().len(), 2, "two routes from a to z");
        assert_eq!(composite.preimages(&id("z")).len(), 2);

        let identity = EventRelation::identity([&id("a")]);
        assert_eq!(identity.compose(&first), first);
        let middles = EventRelation::identity([&id("m"), &id("n")]);
        assert_eq!(first.compose(&middles), first);
    }

    #[test]
    fn affine_time_transformations_compose() {
        // Double the durations, then shift by one beat.
        let augment = TimeTransform::scale(Q::from(Z::from(2))).unwrap();
        let shift = TimeTransform::shift(Beats::ratio(1, 1).unwrap());
        let both = augment.compose(&shift).unwrap();

        let at = BeatTime::ratio(3, 1).unwrap();
        assert_eq!(
            both.apply(&at).unwrap(),
            shift.apply(&augment.apply(&at).unwrap()).unwrap(),
            "composition agrees with applying the two in order"
        );
        assert_eq!(*both.apply(&at).unwrap().get(), Q::from(Z::from(7)));

        // The other order is a different transformation, as it should be.
        let reversed = shift.compose(&augment).unwrap();
        assert_ne!(reversed, both);
        assert_eq!(*reversed.apply(&at).unwrap().get(), Q::from(Z::from(8)));

        // Identity is neutral on both sides.
        let identity = TimeTransform::identity();
        assert_eq!(identity.compose(&both).unwrap(), both);
        assert_eq!(both.compose(&identity).unwrap(), both);
    }

    #[test]
    fn a_non_positive_time_scale_is_rejected() {
        assert!(matches!(
            TimeTransform::scale(Q::from(Z::from(0))),
            Err(ScoreError::NonPositiveTimeScale)
        ));
        assert!(TimeTransform::scale(Q::from(Z::from(-1))).is_err());
    }

    #[test]
    fn transpositions_compose_by_adding() {
        let up_a_fifth = PitchTransform::Transpose(7i64);
        let up_a_fourth = PitchTransform::Transpose(5i64);
        let both = up_a_fifth.compose(&up_a_fourth, add_i64).unwrap().unwrap();
        assert_eq!(both, PitchTransform::Transpose(12));

        let identity = PitchTransform::Identity;
        assert_eq!(
            identity.compose(&up_a_fifth, add_i64).unwrap().unwrap(),
            up_a_fifth
        );
        assert_eq!(
            up_a_fifth.compose(&identity, add_i64).unwrap().unwrap(),
            up_a_fifth
        );
    }

    #[test]
    fn the_compositional_claim_is_withdrawn_where_it_is_unearned() {
        let plain: ScoreTransform<i64> = ScoreTransform::new(
            EventRelation::identity([&id("a")]),
            PitchTransform::Transpose(2),
            TimeTransform::identity(),
            ProvenanceChain::of(ProvenanceId::new("p1")),
        );
        assert!(plain.claims_compositional());

        // A pitch change conditioned on metric position: legitimate, and not
        // composable by this crate.
        let dependent: ScoreTransform<i64> = ScoreTransform::new(
            EventRelation::identity([&id("a")]),
            PitchTransform::Declared {
                name: String::from("umt:transform:metric-conditioned"),
            },
            TimeTransform::identity(),
            ProvenanceChain::of(ProvenanceId::new("p2")),
        );
        assert!(!dependent.claims_compositional());
        assert!(
            plain.compose(&dependent, add_i64).is_none(),
            "section 6.6 forbids the label without the operation"
        );
        assert!(dependent.compose(&plain, add_i64).is_none());

        // A declared temporal component is evaluated by nobody here.
        let declared_time: ScoreTransform<i64> = ScoreTransform::new(
            EventRelation::new(),
            PitchTransform::Identity,
            TimeTransform::Declared {
                name: String::from("umt:transform:rubato"),
            },
            ProvenanceChain::new(),
        );
        assert!(!declared_time.claims_compositional());
        assert!(matches!(
            declared_time.time().apply(&BeatTime::zero()),
            Err(ScoreError::UncomposableTransform { .. })
        ));
    }

    #[test]
    fn composition_is_associative_and_chains_provenance() {
        let make = |shift: i64, provenance: &str| -> ScoreTransform<i64> {
            ScoreTransform::new(
                EventRelation::identity([&id("a")]),
                PitchTransform::Transpose(shift),
                TimeTransform::shift(Beats::ratio(shift, 1).unwrap()),
                ProvenanceChain::of(ProvenanceId::new(provenance)),
            )
        };
        let (f, g, h) = (make(1, "p1"), make(2, "p2"), make(3, "p3"));

        let left = f
            .compose(&g, add_i64)
            .unwrap()
            .unwrap()
            .compose(&h, add_i64)
            .unwrap()
            .unwrap();
        let right = f
            .compose(&g.compose(&h, add_i64).unwrap().unwrap(), add_i64)
            .unwrap()
            .unwrap();
        assert_eq!(left, right);

        // Provenance composes by concatenation, oldest first.
        assert_eq!(
            left.provenance().steps(),
            &[
                ProvenanceId::new("p1"),
                ProvenanceId::new("p2"),
                ProvenanceId::new("p3")
            ]
        );
        assert_eq!(left.pitch(), &PitchTransform::Transpose(6));
    }

    #[test]
    fn the_identity_transformation_is_neutral() {
        let identity: ScoreTransform<i64> = ScoreTransform::identity([&id("a")]);
        assert!(identity.claims_compositional());
        assert_eq!(identity.pitch(), &PitchTransform::Identity);
        assert_eq!(identity.time(), &TimeTransform::identity());

        let transform: ScoreTransform<i64> = ScoreTransform::new(
            EventRelation::identity([&id("a")]),
            PitchTransform::Transpose(4),
            TimeTransform::shift(Beats::ratio(2, 1).unwrap()),
            ProvenanceChain::new(),
        );
        assert_eq!(
            identity.compose(&transform, add_i64).unwrap().unwrap(),
            transform
        );
        assert_eq!(
            transform.compose(&identity, add_i64).unwrap().unwrap(),
            transform
        );
    }
}
