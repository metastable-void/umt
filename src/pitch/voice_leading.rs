//! Voice leading (UMT-3.2 section 4.4, prompt section 22).
//!
//! Four things that are routinely conflated are four types here, because
//! UMT-3.2 section 4.4.5 says they answer different questions:
//!
//! 1. [`VoiceLeading`] - the *declared relation*, a span
//!    `V_1 <-alpha- E -beta-> V_2`. Purely structural: no costs, no metric,
//!    and no assumption that the relation is a permutation.
//! 2. [`SpanCostModel::declared_cost`] - the cost *of that declared relation*,
//!    the additive `C = C_move + C_split + C_merge + C_birth + C_death` of
//!    section 4.4.2. Section 4.4.2 is explicit that this "is not automatically
//!    a metric on chords", so nothing here calls it one.
//! 3. [`SpanCostModel::minimum_over_assignments`] - the *minimum* over a
//!    stated admissible family, which is a different number answering a
//!    different question. Every [`SpanCost`] records which of the two it is,
//!    in [`CostQuestion`], so a reported total cannot be read as the other.
//! 4. [`ChordDistance`] - a profile that may claim distance laws, and that has
//!    to say which state space it claims them on.
//!
//! # Unequal voice counts
//!
//! Counting one unit of mass per voice gives different total masses when voice
//! counts differ, and classical balanced transport does not solve that case
//! (section 4.4.4). So [`ChordDistance`] under [`MassProfile::PerVoice`]
//! *refuses* to compare chords of different sizes rather than quietly
//! renormalizing, and the normalization that would make them comparable is a
//! separate variant a caller has to name.
//!
//! That is fixture F8. One C and two doubled Cs must not become
//! indistinguishable because an undocumented normalization turned both into
//! the same probability measure. Here the normalization is documented, it is
//! opt-in, the [`TransportProfile::Edit`] profile handles the unequal case
//! without it, and the chords stay distinguishable as chords either way.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::error::PitchError;
use crate::pitch::chord::{Chord, VoiceId, VoiceSet};
use crate::pitch::point::{IntervalGroupElement, PitchPoint};
use crate::pitch::tuning::{L2IntervalGroup, RegularTuning};
use crate::realization::optimization::{ApproximationGuarantee, OptimizationOutcome};

/// An edge of a voice-leading span: one source voice related to one target
/// voice.
///
/// UMT layer: structural. An edge asserts a relation, not a distance;
/// displacement is derived from the two pitch points when it is wanted
/// (UMT-3.2 section 4.4.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Edge {
    /// The source voice, `alpha(e)`.
    pub source: VoiceId,
    /// The target voice, `beta(e)`.
    pub target: VoiceId,
}

impl Edge {
    /// Relates a source voice to a target voice.
    #[must_use]
    pub fn new(source: VoiceId, target: VoiceId) -> Self {
        Self { source, target }
    }
}

/// How many events of each kind a span contains.
///
/// The accounting convention, which UMT-3.2 leaves to the implementation and
/// which this crate therefore states rather than assumes:
///
/// - `moves` is the number of edges: every edge is a movement;
/// - `splits` is the sum over source voices of `max(0, outdegree - 1)`, so a
///   voice fanning out to three targets is two splits;
/// - `merges` is the same over target voices and their indegree;
/// - `entries` counts target voices with no incoming edge;
/// - `exits` counts source voices with no outgoing edge;
/// - `continuations` counts edges whose source has outdegree 1 and whose
///   target has indegree 1, that is, plain one-to-one continuation.
///
/// `continuations` is a subset of `moves`, not an extra term. The cost model
/// charges per edge, per excess branch, and per unmatched voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanShape {
    /// Total number of edges.
    pub moves: usize,
    /// Edges that are plain one-to-one continuations.
    pub continuations: usize,
    /// Excess outgoing branches, summed over source voices.
    pub splits: usize,
    /// Excess incoming branches, summed over target voices.
    pub merges: usize,
    /// Target voices with no incoming edge.
    pub entries: usize,
    /// Source voices with no outgoing edge.
    pub exits: usize,
}

/// A voice-leading relation, represented as the span
/// `V_1 <-alpha- E -beta-> V_2` (UMT-3.2 section 4.4.1).
///
/// UMT layer: structural, and independent of any metric.
///
/// The span form is what makes one-to-one continuation, splits, merges,
/// repeated relations, entries, and exits representable without special cases:
/// an entry is a target voice no edge reaches, an exit is a source voice no
/// edge leaves, and a split is simply a source voice with two edges. Nothing
/// here assumes the relation is a permutation, or even a function.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawVoiceLeading", into = "RawVoiceLeading")
)]
pub struct VoiceLeading {
    source: VoiceSet,
    target: VoiceSet,
    edges: Vec<Edge>,
}

/// A voice-leading span in wire form, validated on the way in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawVoiceLeading {
    /// The source voice set.
    pub source: VoiceSet,
    /// The target voice set.
    pub target: VoiceSet,
    /// The edges, which may repeat.
    pub edges: Vec<Edge>,
}

impl VoiceLeading {
    /// Builds a span, checking that every edge lands inside the declared voice
    /// sets.
    ///
    /// Repeated edges are permitted: UMT-3.2 section 4.4.1 lists "repeated
    /// relations" among the cases the span form must support, so `E` is a
    /// multiset of edges rather than a set.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::UnknownVoice`] if an edge names a voice outside
    /// the corresponding set.
    pub fn new<I>(source: VoiceSet, target: VoiceSet, edges: I) -> Result<Self, PitchError>
    where
        I: IntoIterator<Item = Edge>,
    {
        let edges: Vec<Edge> = edges.into_iter().collect();
        for edge in &edges {
            if !source.contains(&edge.source) {
                return Err(PitchError::UnknownVoice {
                    voice: edge.source.clone(),
                });
            }
            if !target.contains(&edge.target) {
                return Err(PitchError::UnknownVoice {
                    voice: edge.target.clone(),
                });
            }
        }
        Ok(Self {
            source,
            target,
            edges,
        })
    }

    /// The span that holds every voice in place.
    #[must_use]
    pub fn identity(voices: &VoiceSet) -> Self {
        Self {
            source: voices.clone(),
            target: voices.clone(),
            edges: voices
                .iter()
                .map(|voice| Edge::new(voice.clone(), voice.clone()))
                .collect(),
        }
    }

    /// The source voice set `V_1`.
    #[must_use]
    pub fn source(&self) -> &VoiceSet {
        &self.source
    }

    /// The target voice set `V_2`.
    #[must_use]
    pub fn target(&self) -> &VoiceSet {
        &self.target
    }

    /// The edges `E`, in the order they were declared.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// How many edges leave a source voice.
    #[must_use]
    pub fn out_degree(&self, voice: &VoiceId) -> usize {
        self.edges.iter().filter(|e| &e.source == voice).count()
    }

    /// How many edges enter a target voice.
    #[must_use]
    pub fn in_degree(&self, voice: &VoiceId) -> usize {
        self.edges.iter().filter(|e| &e.target == voice).count()
    }

    /// The event counts of this span, under the convention documented on
    /// [`SpanShape`].
    #[must_use]
    pub fn shape(&self) -> SpanShape {
        let mut out: BTreeMap<&VoiceId, usize> = BTreeMap::new();
        let mut incoming: BTreeMap<&VoiceId, usize> = BTreeMap::new();
        for voice in self.source.iter() {
            out.insert(voice, 0);
        }
        for voice in self.target.iter() {
            incoming.insert(voice, 0);
        }
        for edge in &self.edges {
            *out.entry(&edge.source).or_insert(0) += 1;
            *incoming.entry(&edge.target).or_insert(0) += 1;
        }

        let continuations = self
            .edges
            .iter()
            .filter(|e| out[&e.source] == 1 && incoming[&e.target] == 1)
            .count();

        SpanShape {
            moves: self.edges.len(),
            continuations,
            splits: out.values().map(|d| d.saturating_sub(1)).sum(),
            merges: incoming.values().map(|d| d.saturating_sub(1)).sum(),
            entries: incoming.values().filter(|d| **d == 0).count(),
            exits: out.values().filter(|d| **d == 0).count(),
        }
    }

    /// Whether this span is a bijection between the two voice sets.
    ///
    /// Worth asking explicitly, because a great deal of voice-leading writing
    /// assumes it silently.
    #[must_use]
    pub fn is_bijective(&self) -> bool {
        let shape = self.shape();
        shape.splits == 0 && shape.merges == 0 && shape.entries == 0 && shape.exits == 0
    }

    /// The displacement along each edge, derived from the two pitch points
    /// (UMT-3.2 section 4.4.1).
    ///
    /// Returned in edge order. Nothing is stored on the span itself, because
    /// the observed endpoints already determine it; an implementation that
    /// needs to record an *intended* displacement differing from the observed
    /// one must keep that alongside, as a separate declared claim.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::VoiceSetMismatch`] if the chords do not carry
    /// exactly this span's voice sets, and [`PitchError::OriginMismatch`] if
    /// the two chords are measured from different origins.
    pub fn displacements<E: IntervalGroupElement>(
        &self,
        from: &Chord<E>,
        to: &Chord<E>,
    ) -> Result<Vec<E>, PitchError> {
        self.check_endpoints(from, to)?;
        self.edges
            .iter()
            .map(|edge| {
                from.require(&edge.source)?
                    .interval_to(to.require(&edge.target)?)
            })
            .collect()
    }

    /// Composition by pullback (UMT-3.2 section 4.4.1).
    ///
    /// The composite of `V_1 <- E -> V_2` and `V_2 <- F -> V_3` has one edge
    /// for each pair `(e, f)` with `beta(e) = alpha(f)`, relating `alpha(e)` to
    /// `beta(f)`. That is the categorical composition of relations, and it is
    /// why splits and merges compose correctly instead of being lost.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::VoiceSetMismatch`] if this span's target is not
    /// the other's source.
    pub fn compose(&self, other: &Self) -> Result<Self, PitchError> {
        if self.target != other.source {
            return Err(PitchError::VoiceSetMismatch);
        }
        let mut edges = Vec::new();
        for first in &self.edges {
            for second in &other.edges {
                if first.target == second.source {
                    edges.push(Edge::new(first.source.clone(), second.target.clone()));
                }
            }
        }
        Ok(Self {
            source: self.source.clone(),
            target: other.target.clone(),
            edges,
        })
    }

    fn check_endpoints<E: IntervalGroupElement>(
        &self,
        from: &Chord<E>,
        to: &Chord<E>,
    ) -> Result<(), PitchError> {
        if from.voice_set() != self.source || to.voice_set() != self.target {
            return Err(PitchError::VoiceSetMismatch);
        }
        Ok(())
    }

    fn from_assignment(
        sources: &[VoiceId],
        targets: &[VoiceId],
        source_set: &VoiceSet,
        target_set: &VoiceSet,
        assignment: &[Option<usize>],
    ) -> Self {
        let edges = assignment
            .iter()
            .enumerate()
            .filter_map(|(i, choice)| {
                choice.map(|j| Edge::new(sources[i].clone(), targets[j].clone()))
            })
            .collect();
        Self {
            source: source_set.clone(),
            target: target_set.clone(),
            edges,
        }
    }
}

impl TryFrom<RawVoiceLeading> for VoiceLeading {
    type Error = PitchError;

    fn try_from(value: RawVoiceLeading) -> Result<Self, Self::Error> {
        Self::new(value.source, value.target, value.edges)
    }
}

impl From<VoiceLeading> for RawVoiceLeading {
    fn from(value: VoiceLeading) -> Self {
        Self {
            source: value.source,
            target: value.target,
            edges: value.edges,
        }
    }
}

impl core::fmt::Display for VoiceLeading {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("{")?;
        for (index, edge) in self.edges.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{} -> {}", edge.source, edge.target)?;
        }
        f.write_str("}")
    }
}

/// A declared ground cost between two pitch points (UMT-3.2 section 4.4.3).
///
/// Section 4.4.3 requires the ground metric to be declared, "for example
/// registered log-pitch distance or a pitch-class quotient metric". Making it
/// a trait parameter rather than a built-in means the declaration cannot be
/// omitted, and it also means the laws a distance profile claims are inherited
/// from a stated ground cost rather than assumed into existence.
pub trait GroundCost {
    /// The kind of pitch point this cost measures between.
    ///
    /// An associated type rather than a parameter: a declared ground cost
    /// measures one kind of pitch, and a cost declared on 12-EDO steps is not
    /// a cost on 5-limit monzos.
    type Point: IntervalGroupElement;

    /// The ground distance between two points.
    ///
    /// # Errors
    ///
    /// Implementation-defined; typically an origin or interval-group mismatch.
    fn distance(
        &self,
        from: &PitchPoint<Self::Point>,
        to: &PitchPoint<Self::Point>,
    ) -> Result<f64, PitchError>;
}

/// Registered log-pitch distance, in octaves, under a declared regular tuning.
///
/// UMT layer: L3 values over an L2 domain. This is `|tau(int(p, q))|`. It is a
/// metric on points exactly when the tuning is injective on the interval
/// group: for an equal division of the octave it is, and for a tuning with a
/// zero-size generator it is not. That is a property of the declared tuning,
/// not of this type, and it propagates to every claim built on top.
#[derive(Debug, Clone)]
pub struct LogPitchDistance<G> {
    tuning: RegularTuning<G>,
}

impl<G: L2IntervalGroup> LogPitchDistance<G> {
    /// Measures distance with this tuning.
    #[must_use]
    pub fn new(tuning: RegularTuning<G>) -> Self {
        Self { tuning }
    }

    /// The tuning that defines the distance.
    #[must_use]
    pub fn tuning(&self) -> &RegularTuning<G> {
        &self.tuning
    }
}

impl<G: L2IntervalGroup> GroundCost for LogPitchDistance<G> {
    type Point = G::Element;

    fn distance(
        &self,
        from: &PitchPoint<G::Element>,
        to: &PitchPoint<G::Element>,
    ) -> Result<f64, PitchError> {
        let interval = from.interval_to(to)?;
        Ok(self.tuning.size(&interval)?.get().abs())
    }
}

/// The per-event penalties of UMT-3.2 section 4.4.2.
///
/// Every one is a *declared* number. None is derived from the pitches, and
/// none has a canonical value: what a split costs relative to a semitone of
/// motion is a modelling choice, and section 4.4.2 requires it to be made
/// explicitly rather than inherited from a default nobody chose.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanPenalties {
    /// Charged per excess outgoing branch.
    pub split: f64,
    /// Charged per excess incoming branch.
    pub merge: f64,
    /// Charged per target voice with no incoming edge.
    pub birth: f64,
    /// Charged per source voice with no outgoing edge.
    pub death: f64,
}

impl SpanPenalties {
    /// Penalties that are all zero.
    pub const FREE: Self = Self {
        split: 0.0,
        merge: 0.0,
        birth: 0.0,
        death: 0.0,
    };

    /// The same penalty for every event kind.
    #[must_use]
    pub fn uniform(cost: f64) -> Self {
        Self {
            split: cost,
            merge: cost,
            birth: cost,
            death: cost,
        }
    }

    fn validate(&self) -> Result<(), PitchError> {
        for value in [self.split, self.merge, self.birth, self.death] {
            if !value.is_finite() || value < 0.0 {
                return Err(PitchError::InvalidCostParameter);
            }
        }
        Ok(())
    }
}

/// Which question a [`SpanCost`] answers (UMT-3.2 section 4.4.5).
///
/// Section 4.4.5 requires an implementation to state whether its output is the
/// cost of the declared voice leading or the minimum over an admissible
/// family. Carrying the answer inside the result makes it impossible to report
/// one and be understood as the other.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CostQuestion {
    /// The cost of the span that was declared, exactly as declared.
    DeclaredSpan,
    /// The minimum cost over a stated admissible family of spans.
    MinimumOverFamily(AdmissibleFamily),
}

/// A family of spans an optimizer searched.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AdmissibleFamily {
    /// Every span in which each source voice has at most one outgoing edge and
    /// each target voice at most one incoming edge.
    ///
    /// Entries and exits are inside this family; splits and merges are not. A
    /// minimum over it is therefore *not* a minimum over all spans, which is
    /// why the family is named in the result rather than left implicit.
    PartialAssignment,
}

/// The cost of a voice leading, broken into the terms of UMT-3.2 section
/// 4.4.2.
///
/// UMT layer: L3 policy values.
///
/// The breakdown is kept rather than summed away, so a caller can see *why* a
/// leading is expensive - four births and no motion is a different musical
/// situation from one large leap - and so a reported total can never be
/// mistaken for a distance between chords.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanCost {
    question: CostQuestion,
    shape: SpanShape,
    movement: f64,
    split: f64,
    merge: f64,
    birth: f64,
    death: f64,
}

impl SpanCost {
    /// Which question this cost answers.
    #[must_use]
    pub fn question(&self) -> &CostQuestion {
        &self.question
    }

    /// The event counts it was computed from.
    #[must_use]
    pub fn shape(&self) -> SpanShape {
        self.shape
    }

    /// `C_move`: the summed displacement cost over all edges.
    #[must_use]
    pub fn movement(&self) -> f64 {
        self.movement
    }

    /// `C_split`.
    #[must_use]
    pub fn split(&self) -> f64 {
        self.split
    }

    /// `C_merge`.
    #[must_use]
    pub fn merge(&self) -> f64 {
        self.merge
    }

    /// `C_birth`.
    #[must_use]
    pub fn birth(&self) -> f64 {
        self.birth
    }

    /// `C_death`.
    #[must_use]
    pub fn death(&self) -> f64 {
        self.death
    }

    /// `C = C_move + C_split + C_merge + C_birth + C_death`.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.movement + self.split + self.merge + self.birth + self.death
    }
}

/// How much of the search space an exhaustive optimizer may visit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOptions {
    /// Maximum number of candidate spans to examine.
    ///
    /// Exceeding it downgrades the answer to
    /// [`OptimizationOutcome::Approximate`] with the region recorded, rather
    /// than passing an unproved minimum off as exact.
    pub budget: usize,
    /// Relative tolerance below which two costs count as tied.
    ///
    /// Cost comparison here is floating point, so exact equality would report
    /// spurious uniqueness whenever two genuinely equal costs happened to be
    /// summed in a different order.
    pub tie_tolerance: f64,
}

impl Default for SearchOptions {
    /// A budget of 200000 candidates - which covers every partial assignment
    /// between two seven-voice chords, and every permutation of eight voices -
    /// and a relative tie tolerance of `1e-12`.
    fn default() -> Self {
        Self {
            budget: 200_000,
            tie_tolerance: 1e-12,
        }
    }
}

fn ties(left: f64, right: f64, tolerance: f64) -> bool {
    let scale = 1.0 + left.abs().max(right.abs());
    (left - right).abs() <= tolerance * scale
}

struct Walk<'a, C, F> {
    used: Vec<bool>,
    choice: Vec<C>,
    examined: usize,
    truncated: bool,
    budget: usize,
    visit: &'a mut F,
}

impl<C, F> Walk<'_, C, F> {
    /// Records a completed candidate; reports whether the walk may continue.
    fn admit(&mut self) -> bool {
        if self.examined == self.budget {
            self.truncated = true;
            return false;
        }
        self.examined += 1;
        true
    }
}

/// Visits every partial assignment of `sources` onto `targets`.
///
/// Returns how many were examined and whether the budget cut the walk short.
fn for_each_partial_assignment<F>(
    sources: usize,
    targets: usize,
    budget: usize,
    mut visit: F,
) -> (usize, bool)
where
    F: FnMut(&[Option<usize>]),
{
    fn descend<F: FnMut(&[Option<usize>])>(
        walk: &mut Walk<'_, Option<usize>, F>,
        index: usize,
        sources: usize,
    ) {
        if walk.truncated {
            return;
        }
        if index == sources {
            if walk.admit() {
                (walk.visit)(&walk.choice);
            }
            return;
        }
        walk.choice[index] = None;
        descend(walk, index + 1, sources);
        for target in 0..walk.used.len() {
            if walk.used[target] {
                continue;
            }
            walk.used[target] = true;
            walk.choice[index] = Some(target);
            descend(walk, index + 1, sources);
            walk.used[target] = false;
        }
        walk.choice[index] = None;
    }

    let mut walk = Walk {
        used: alloc::vec![false; targets],
        choice: alloc::vec![None; sources],
        examined: 0,
        truncated: false,
        budget,
        visit: &mut visit,
    };
    descend(&mut walk, 0, sources);
    (walk.examined, walk.truncated)
}

/// Visits every permutation of `count` elements.
fn for_each_permutation<F>(count: usize, budget: usize, mut visit: F) -> (usize, bool)
where
    F: FnMut(&[usize]),
{
    fn descend<F: FnMut(&[usize])>(walk: &mut Walk<'_, usize, F>, index: usize, count: usize) {
        if walk.truncated {
            return;
        }
        if index == count {
            if walk.admit() {
                (walk.visit)(&walk.choice);
            }
            return;
        }
        for target in 0..count {
            if walk.used[target] {
                continue;
            }
            walk.used[target] = true;
            walk.choice[index] = target;
            descend(walk, index + 1, count);
            walk.used[target] = false;
        }
    }

    let mut walk = Walk {
        used: alloc::vec![false; count],
        choice: alloc::vec![0usize; count],
        examined: 0,
        truncated: false,
        budget,
        visit: &mut visit,
    };
    descend(&mut walk, 0, count);
    (walk.examined, walk.truncated)
}

/// A declared cost model for voice-leading spans (UMT-3.2 section 4.4.2).
///
/// UMT layer: L3 policy.
///
/// The movement term is `sum_e d(e)^p` for the declared ground cost `d` and
/// exponent `p`; the remaining four terms are event counts times declared
/// penalties. The total is additive, as section 4.4.2 writes it. It is the
/// cost of a declared transformation, and this type never calls it a chord
/// metric - that is [`ChordDistance`], which has to say what it claims.
#[derive(Debug, Clone)]
pub struct SpanCostModel<D> {
    ground: D,
    exponent: f64,
    penalties: SpanPenalties,
    options: SearchOptions,
}

impl<D> SpanCostModel<D> {
    /// Declares a cost model.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NonMetricExponent`] for an exponent below 1 or
    /// not finite, and [`PitchError::InvalidCostParameter`] for a negative or
    /// non-finite penalty.
    pub fn new(ground: D, exponent: f64, penalties: SpanPenalties) -> Result<Self, PitchError> {
        if !exponent.is_finite() || exponent < 1.0 {
            return Err(PitchError::NonMetricExponent { exponent });
        }
        penalties.validate()?;
        Ok(Self {
            ground,
            exponent,
            penalties,
            options: SearchOptions::default(),
        })
    }

    /// Replaces the search options.
    #[must_use]
    pub fn with_search_options(mut self, options: SearchOptions) -> Self {
        self.options = options;
        self
    }

    /// The declared ground cost.
    #[must_use]
    pub fn ground(&self) -> &D {
        &self.ground
    }

    /// The declared exponent `p`.
    #[must_use]
    pub fn exponent(&self) -> f64 {
        self.exponent
    }

    /// The declared per-event penalties.
    #[must_use]
    pub fn penalties(&self) -> SpanPenalties {
        self.penalties
    }

    fn weigh(&self, distance: f64) -> f64 {
        if self.exponent == 1.0 {
            distance
        } else {
            libm::pow(distance, self.exponent)
        }
    }

    fn assemble(&self, question: CostQuestion, shape: SpanShape, movement: f64) -> SpanCost {
        SpanCost {
            question,
            shape,
            movement,
            split: self.penalties.split * shape.splits as f64,
            merge: self.penalties.merge * shape.merges as f64,
            birth: self.penalties.birth * shape.entries as f64,
            death: self.penalties.death * shape.exits as f64,
        }
    }
}

impl<D: GroundCost> SpanCostModel<D> {
    /// The cost of the span that was declared (UMT-3.2 section 4.4.2).
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::VoiceSetMismatch`] if the chords do not carry the
    /// span's voice sets, and propagates ground-cost failures.
    pub fn declared_cost(
        &self,
        span: &VoiceLeading,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
    ) -> Result<SpanCost, PitchError> {
        span.check_endpoints(from, to)?;
        let mut movement = 0.0;
        for edge in span.edges() {
            let distance = self
                .ground
                .distance(from.require(&edge.source)?, to.require(&edge.target)?)?;
            movement += self.weigh(distance);
        }
        Ok(self.assemble(CostQuestion::DeclaredSpan, span.shape(), movement))
    }

    /// The minimum cost over the partial-assignment family
    /// (UMT-3.2 section 4.4.5).
    ///
    /// This is a *different question* from [`SpanCostModel::declared_cost`],
    /// and the returned [`SpanCost`] says so. The family searched is
    /// [`AdmissibleFamily::PartialAssignment`]: one-to-one continuations plus
    /// entries and exits. Splits and merges lie outside it, so this minimum is
    /// not a minimum over all spans.
    ///
    /// Ties are reported as [`OptimizationOutcome::Multiple`] rather than
    /// resolved silently, and a truncated search as
    /// [`OptimizationOutcome::Approximate`] rather than passed off as optimal.
    ///
    /// # Errors
    ///
    /// Propagates ground-cost failures.
    pub fn minimum_over_assignments(
        &self,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
    ) -> Result<OptimizationOutcome<VoiceLeading, SpanCost>, PitchError> {
        let sources: Vec<VoiceId> = from.iter().map(|(voice, _)| voice.clone()).collect();
        let targets: Vec<VoiceId> = to.iter().map(|(voice, _)| voice.clone()).collect();
        let source_set = from.voice_set();
        let target_set = to.voice_set();

        let mut weighted = alloc::vec![alloc::vec![0.0f64; targets.len()]; sources.len()];
        for (i, source) in sources.iter().enumerate() {
            for (j, target) in targets.iter().enumerate() {
                let distance = self
                    .ground
                    .distance(from.require(source)?, to.require(target)?)?;
                weighted[i][j] = self.weigh(distance);
            }
        }

        let mut best: Option<(f64, f64, Vec<Option<usize>>)> = None;
        let mut tied = 0usize;
        let (examined, truncated) = for_each_partial_assignment(
            sources.len(),
            targets.len(),
            self.options.budget,
            |assignment| {
                let mut movement = 0.0;
                let mut matched = 0usize;
                let mut exits = 0usize;
                for (i, choice) in assignment.iter().enumerate() {
                    match choice {
                        Some(j) => {
                            movement += weighted[i][*j];
                            matched += 1;
                        }
                        None => exits += 1,
                    }
                }
                let entries = targets.len() - matched;
                let total = movement
                    + self.penalties.birth * entries as f64
                    + self.penalties.death * exits as f64;

                match &best {
                    Some((incumbent, _, _))
                        if ties(total, *incumbent, self.options.tie_tolerance) =>
                    {
                        tied += 1;
                    }
                    Some((incumbent, _, _)) if total > *incumbent => {}
                    _ => {
                        best = Some((total, movement, assignment.to_vec()));
                        tied = 0;
                    }
                }
            },
        );

        // The all-unmatched assignment is always admissible, so this is only
        // reachable with a zero budget.
        let Some((_, movement, assignment)) = best else {
            return Ok(OptimizationOutcome::Infeasible);
        };
        let span = VoiceLeading::from_assignment(
            &sources,
            &targets,
            &source_set,
            &target_set,
            &assignment,
        );
        let cost = self.assemble(
            CostQuestion::MinimumOverFamily(AdmissibleFamily::PartialAssignment),
            span.shape(),
            movement,
        );

        Ok(if truncated {
            OptimizationOutcome::Approximate {
                solution: span,
                cost,
                guarantee: ApproximationGuarantee::SearchedRegion {
                    region: alloc::format!(
                        "partial assignments of {} source voices onto {} target voices, in enumeration order",
                        sources.len(),
                        targets.len()
                    ),
                    examined,
                },
            }
        } else if tied > 0 {
            OptimizationOutcome::Multiple {
                selected: span,
                cost,
                others: tied,
            }
        } else {
            OptimizationOutcome::Exact {
                solution: span,
                cost,
            }
        })
    }
}

/// How a chord is turned into mass before transport (UMT-3.2 section 4.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum MassProfile {
    /// One unit of mass per voice. Multiplicity is preserved, and chords of
    /// different voice counts have different total mass - so balanced
    /// transport between them is refused rather than fudged.
    PerVoice,
    /// Each chord normalized to total mass 1.
    ///
    /// **Explicitly lossy, and only ever selected by name.** Under this
    /// profile a chord of one C and a chord of two doubled Cs are the same
    /// measure, at distance zero. UMT-3.2 section 4.4.4 permits that "only
    /// when the application explicitly accepts the resulting loss of absolute
    /// multiplicity information" - which is what naming this variant is.
    NormalizedProbability,
}

/// Which transport or edit profile a [`ChordDistance`] uses.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TransportProfile {
    /// Classical balanced `W_p` transport (UMT-3.2 section 4.4.3), available
    /// only where its preconditions hold.
    Balanced {
        /// How mass is assigned to voices.
        mass: MassProfile,
    },
    /// An assignment/edit distance with an explicit per-voice birth and death
    /// cost (UMT-3.2 section 4.4.4).
    ///
    /// Unmatched voices on either side cost `boundary` each. This handles
    /// unequal voice counts without discarding multiplicity, which is what
    /// makes it the profile fixture F8 asks for.
    Edit {
        /// The cost of leaving one voice unmatched.
        boundary: f64,
    },
}

/// What distance laws a profile actually claims (UMT-3.2 section 9.5).
///
/// Section 9.5 requires a profile claiming a metric to test identity of
/// indiscernibles, symmetry, and the triangle inequality "on the actual
/// represented state space", and forbids unequal-mass profiles from inheriting
/// classical Wasserstein claims. So a claim here is a value that names its
/// state space, and can decline to be a claim at all.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetricClaim {
    /// The metric laws hold on the stated state space.
    Metric {
        /// The state space the laws are claimed on, which is generally *not*
        /// the space of labelled chords.
        state_space: &'static str,
    },
    /// The laws hold only if a stated condition holds of the represented data.
    Conditional {
        /// The state space the laws would be claimed on.
        state_space: &'static str,
        /// What must be true for the claim to hold.
        condition: &'static str,
    },
    /// No distance claim is made.
    NotClaimed {
        /// Why not.
        reason: &'static str,
    },
}

/// A declared distance between chords (UMT-3.2 sections 4.4.3 and 4.4.4).
///
/// UMT layer: L3 policy.
///
/// Unlike [`SpanCostModel`], this is about chords rather than about a declared
/// span, and it may claim distance laws - but only the ones its profile
/// actually supports, reported by [`ChordDistance::metric_claim`], and only
/// relative to a ground cost that is itself a metric.
#[derive(Debug, Clone)]
pub struct ChordDistance<D> {
    ground: D,
    exponent: f64,
    profile: TransportProfile,
    options: SearchOptions,
}

impl<D> ChordDistance<D> {
    /// Declares a chord distance.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NonMetricExponent`] for `p < 1`, which puts the
    /// classical `W_p` metric claims out of reach, and
    /// [`PitchError::InvalidCostParameter`] for a negative or non-finite
    /// boundary cost.
    pub fn new(ground: D, exponent: f64, profile: TransportProfile) -> Result<Self, PitchError> {
        if !exponent.is_finite() || exponent < 1.0 {
            return Err(PitchError::NonMetricExponent { exponent });
        }
        if let TransportProfile::Edit { boundary } = profile
            && (!boundary.is_finite() || boundary < 0.0)
        {
            return Err(PitchError::InvalidCostParameter);
        }
        Ok(Self {
            ground,
            exponent,
            profile,
            options: SearchOptions::default(),
        })
    }

    /// Replaces the search options.
    #[must_use]
    pub fn with_search_options(mut self, options: SearchOptions) -> Self {
        self.options = options;
        self
    }

    /// The declared profile.
    #[must_use]
    pub fn profile(&self) -> TransportProfile {
        self.profile
    }

    /// The declared exponent `p`.
    #[must_use]
    pub fn exponent(&self) -> f64 {
        self.exponent
    }

    /// What laws this profile claims, and on what state space.
    ///
    /// Every claim assumes the declared ground cost is a metric on pitch
    /// points; a ground cost that is only a pseudometric weakens each claim in
    /// the same way, and that is a property of the tuning it came from.
    #[must_use]
    pub fn metric_claim(&self) -> MetricClaim {
        match self.profile {
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            } => MetricClaim::Metric {
                state_space: "multisets of pitch points of one fixed cardinality; on labelled chords it is only a pseudometric, since relabelling a chord does not move it",
            },
            TransportProfile::Balanced {
                mass: MassProfile::NormalizedProbability,
            } => MetricClaim::Metric {
                state_space: "uniform probability measures on a chord's distinct pitch points; absolute multiplicity is not part of this state space at all",
            },
            TransportProfile::Edit { boundary } if boundary > 0.0 => MetricClaim::Metric {
                state_space: "multisets of pitch points of any cardinality, under the truncated ground cost min(d, 2^(1/p) * boundary) - no optimal matching ever pays more for a pair than deleting and re-creating it, and a truncated metric is still a metric",
            },
            TransportProfile::Edit { .. } => MetricClaim::NotClaimed {
                reason: "a zero boundary cost puts every chord at distance zero from every chord containing it, so identity of indiscernibles fails",
            },
        }
    }
}

impl<D: GroundCost> ChordDistance<D> {
    /// The distance between two chords under the declared profile.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::UnequalMass`] when a balanced per-voice profile
    /// is asked to compare chords of different sizes - the case UMT-3.2
    /// section 4.4.4 says classical transport does not solve - and
    /// [`PitchError::SearchBudgetExceeded`] when the exhaustive search would
    /// exceed its budget. Neither is approximated: a value that claims metric
    /// laws has to be the true minimum.
    pub fn distance(
        &self,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
    ) -> Result<f64, PitchError> {
        match self.profile {
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            } => self.balanced_per_voice(from, to),
            TransportProfile::Balanced {
                mass: MassProfile::NormalizedProbability,
            } => self.balanced_normalized(from, to),
            TransportProfile::Edit { boundary } => self.edit(from, to, boundary),
        }
    }

    fn root(&self, total: f64) -> f64 {
        if self.exponent == 1.0 {
            total
        } else {
            libm::pow(total, 1.0 / self.exponent)
        }
    }

    fn weigh(&self, distance: f64) -> f64 {
        if self.exponent == 1.0 {
            distance
        } else {
            libm::pow(distance, self.exponent)
        }
    }

    fn cost_matrix(
        &self,
        from: &[PitchPoint<D::Point>],
        to: &[PitchPoint<D::Point>],
    ) -> Result<Vec<Vec<f64>>, PitchError> {
        let mut matrix = alloc::vec![alloc::vec![0.0f64; to.len()]; from.len()];
        for (i, source) in from.iter().enumerate() {
            for (j, target) in to.iter().enumerate() {
                matrix[i][j] = self.weigh(self.ground.distance(source, target)?);
            }
        }
        Ok(matrix)
    }

    fn points(chord: &Chord<D::Point>) -> Vec<PitchPoint<D::Point>> {
        chord.iter().map(|(_, point)| point.clone()).collect()
    }

    fn balanced_per_voice(
        &self,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
    ) -> Result<f64, PitchError> {
        if from.len() != to.len() {
            return Err(PitchError::UnequalMass {
                left: from.len(),
                right: to.len(),
            });
        }
        let total = self.minimum_over_permutations(&Self::points(from), &Self::points(to))?;
        Ok(self.root(total))
    }

    fn balanced_normalized(
        &self,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
    ) -> Result<f64, PitchError> {
        if from.is_empty() || to.is_empty() {
            return if from.is_empty() && to.is_empty() {
                Ok(0.0)
            } else {
                Err(PitchError::UnequalMass {
                    left: from.len(),
                    right: to.len(),
                })
            };
        }

        // Refine both uniform measures to a common atom mass 1/N with
        // N = lcm(n, m). Both sides then carry N atoms of equal mass, and by
        // Birkhoff's theorem the optimal coupling between two equal-cardinality
        // uniform measures is attained at a permutation - so the exact answer
        // is a permutation search, with no linear program required.
        let units = lcm(from.len(), to.len());
        let sources = repeat_each(&Self::points(from), units / from.len());
        let targets = repeat_each(&Self::points(to), units / to.len());

        let total = self.minimum_over_permutations(&sources, &targets)?;
        Ok(self.root(total / units as f64))
    }

    fn edit(
        &self,
        from: &Chord<D::Point>,
        to: &Chord<D::Point>,
        boundary: f64,
    ) -> Result<f64, PitchError> {
        let sources = Self::points(from);
        let targets = Self::points(to);
        let matrix = self.cost_matrix(&sources, &targets)?;
        let unmatched = self.weigh(boundary);

        let mut best = f64::INFINITY;
        let (_, truncated) = for_each_partial_assignment(
            sources.len(),
            targets.len(),
            self.options.budget,
            |assignment| {
                let mut total = 0.0;
                let mut matched = 0usize;
                for (i, choice) in assignment.iter().enumerate() {
                    match choice {
                        Some(j) => {
                            total += matrix[i][*j];
                            matched += 1;
                        }
                        None => total += unmatched,
                    }
                }
                total += unmatched * (targets.len() - matched) as f64;
                if total < best {
                    best = total;
                }
            },
        );
        if truncated {
            return Err(PitchError::SearchBudgetExceeded {
                budget: self.options.budget,
            });
        }
        Ok(self.root(best))
    }

    fn minimum_over_permutations(
        &self,
        from: &[PitchPoint<D::Point>],
        to: &[PitchPoint<D::Point>],
    ) -> Result<f64, PitchError> {
        debug_assert_eq!(from.len(), to.len());
        if from.is_empty() {
            return Ok(0.0);
        }
        let matrix = self.cost_matrix(from, to)?;
        let mut best = f64::INFINITY;
        let (_, truncated) = for_each_permutation(from.len(), self.options.budget, |permutation| {
            let mut total = 0.0;
            for (i, j) in permutation.iter().enumerate() {
                total += matrix[i][*j];
            }
            if total < best {
                best = total;
            }
        });
        if truncated {
            return Err(PitchError::SearchBudgetExceeded {
                budget: self.options.budget,
            });
        }
        Ok(best)
    }
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

fn lcm(a: usize, b: usize) -> usize {
    a / gcd(a, b) * b
}

fn repeat_each<T: Clone>(items: &[T], times: usize) -> Vec<T> {
    let mut out = Vec::with_capacity(items.len() * times);
    for item in items {
        for _ in 0..times {
            out.push(item.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        ChordDistance, CostQuestion, Edge, GroundCost, LogPitchDistance, MassProfile, MetricClaim,
        SearchOptions, SpanCostModel, SpanPenalties, TransportProfile, VoiceLeading,
    };
    use crate::error::PitchError;
    use crate::pitch::chord::{Chord, VoiceId, VoiceSet};
    use crate::pitch::point::{PitchOrigin, PitchPoint};
    use crate::pitch::tuning::RegularTuning;
    use crate::realization::optimization::OptimizationOutcome;
    use crate::temperament::image::{AmbientElem, AmbientLattice};
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    fn steps() -> Arc<AmbientLattice> {
        AmbientLattice::new("umt:edo:12", 1)
    }

    fn ground() -> LogPitchDistance<AmbientLattice> {
        LogPitchDistance::new(RegularTuning::equal_divisions(&steps(), 12).unwrap())
    }

    fn point(step: i64) -> PitchPoint<AmbientElem> {
        PitchPoint::new(
            PitchOrigin::new("umt:origin:c4"),
            steps().element([step]).unwrap(),
        )
    }

    fn chord(voices: &[(&str, i64)]) -> Chord<AmbientElem> {
        Chord::from_voices(
            voices
                .iter()
                .map(|(name, step)| (VoiceId::new(name), point(*step))),
        )
        .unwrap()
    }

    fn voices(names: &[&str]) -> VoiceSet {
        VoiceSet::new(names.iter().map(|n| VoiceId::new(n))).unwrap()
    }

    #[test]
    fn a_span_is_not_assumed_to_be_a_permutation() {
        // One voice splits into two, one voice exits, one voice is new.
        let span = VoiceLeading::new(
            voices(&["a", "b"]),
            voices(&["x", "y", "z"]),
            [
                Edge::new(VoiceId::new("a"), VoiceId::new("x")),
                Edge::new(VoiceId::new("a"), VoiceId::new("y")),
            ],
        )
        .unwrap();

        let shape = span.shape();
        assert_eq!(shape.moves, 2);
        assert_eq!(shape.splits, 1, "one excess branch out of `a`");
        assert_eq!(shape.merges, 0);
        assert_eq!(shape.entries, 1, "`z` has no incoming edge");
        assert_eq!(shape.exits, 1, "`b` has no outgoing edge");
        assert_eq!(shape.continuations, 0);
        assert!(!span.is_bijective());
        assert_eq!(span.out_degree(&VoiceId::new("a")), 2);
        assert_eq!(span.in_degree(&VoiceId::new("z")), 0);
    }

    #[test]
    fn merges_and_repeated_relations_are_representable() {
        let span = VoiceLeading::new(
            voices(&["a", "b"]),
            voices(&["x"]),
            [
                Edge::new(VoiceId::new("a"), VoiceId::new("x")),
                Edge::new(VoiceId::new("b"), VoiceId::new("x")),
                Edge::new(VoiceId::new("b"), VoiceId::new("x")),
            ],
        )
        .unwrap();
        let shape = span.shape();
        assert_eq!(shape.moves, 3, "a repeated relation is a separate edge");
        assert_eq!(shape.merges, 2);
        assert_eq!(shape.splits, 1);
    }

    #[test]
    fn edges_must_land_in_the_declared_voice_sets() {
        assert!(matches!(
            VoiceLeading::new(
                voices(&["a"]),
                voices(&["x"]),
                [Edge::new(VoiceId::new("ghost"), VoiceId::new("x"))],
            ),
            Err(PitchError::UnknownVoice { .. })
        ));
    }

    #[test]
    fn composition_is_by_pullback() {
        // `a` splits into x and y; x and y both go to p. The composite must
        // relate a to p twice, once through each intermediate voice.
        let first = VoiceLeading::new(
            voices(&["a"]),
            voices(&["x", "y"]),
            [
                Edge::new(VoiceId::new("a"), VoiceId::new("x")),
                Edge::new(VoiceId::new("a"), VoiceId::new("y")),
            ],
        )
        .unwrap();
        let second = VoiceLeading::new(
            voices(&["x", "y"]),
            voices(&["p"]),
            [
                Edge::new(VoiceId::new("x"), VoiceId::new("p")),
                Edge::new(VoiceId::new("y"), VoiceId::new("p")),
            ],
        )
        .unwrap();

        let composite = first.compose(&second).unwrap();
        assert_eq!(composite.edges().len(), 2);
        assert_eq!(composite.source(), &voices(&["a"]));
        assert_eq!(composite.target(), &voices(&["p"]));

        // Identity on either side changes nothing.
        assert_eq!(
            VoiceLeading::identity(&voices(&["a"]))
                .compose(&first)
                .unwrap(),
            first
        );
        assert_eq!(
            first
                .compose(&VoiceLeading::identity(&voices(&["x", "y"])))
                .unwrap(),
            first
        );

        // Mismatched middles do not compose.
        assert!(matches!(
            first.compose(&VoiceLeading::identity(&voices(&["q"]))),
            Err(PitchError::VoiceSetMismatch)
        ));
    }

    #[test]
    fn displacement_comes_from_the_endpoints() {
        let from = chord(&[("a", 0), ("b", 4)]);
        let to = chord(&[("x", 2), ("y", 5)]);
        let span = VoiceLeading::new(
            from.voice_set(),
            to.voice_set(),
            [
                Edge::new(VoiceId::new("a"), VoiceId::new("x")),
                Edge::new(VoiceId::new("b"), VoiceId::new("y")),
            ],
        )
        .unwrap();

        let moves = span.displacements(&from, &to).unwrap();
        assert_eq!(moves[0], steps().element([2i64]).unwrap());
        assert_eq!(moves[1], steps().element([1i64]).unwrap());

        // A span applied to the wrong chords is rejected, not reinterpreted.
        assert!(matches!(
            span.displacements(&to, &from),
            Err(PitchError::VoiceSetMismatch)
        ));
    }

    #[test]
    fn a_declared_cost_reports_every_term_separately() {
        let model = SpanCostModel::new(
            ground(),
            1.0,
            SpanPenalties {
                split: 0.5,
                merge: 0.5,
                birth: 2.0,
                death: 3.0,
            },
        )
        .unwrap();

        let from = chord(&[("a", 0), ("b", 12)]);
        let to = chord(&[("x", 1)]);
        let span = VoiceLeading::new(
            from.voice_set(),
            to.voice_set(),
            [Edge::new(VoiceId::new("a"), VoiceId::new("x"))],
        )
        .unwrap();

        let cost = model.declared_cost(&span, &from, &to).unwrap();
        assert_eq!(cost.question(), &CostQuestion::DeclaredSpan);
        assert!((cost.movement() - 1.0 / 12.0).abs() < 1e-12);
        assert_eq!(cost.split(), 0.0);
        assert_eq!(cost.merge(), 0.0);
        assert_eq!(cost.birth(), 0.0);
        assert_eq!(cost.death(), 3.0, "`b` was left behind");
        assert!((cost.total() - (1.0 / 12.0 + 3.0)).abs() < 1e-12);
    }

    #[test]
    fn a_declared_cost_and_a_family_minimum_are_labelled_differently() {
        let model = SpanCostModel::new(ground(), 1.0, SpanPenalties::uniform(10.0)).unwrap();
        let from = chord(&[("a", 0), ("b", 7)]);
        let to = chord(&[("x", 7), ("y", 0)]);

        let outcome = model.minimum_over_assignments(&from, &to).unwrap();
        assert!(outcome.is_optimal());
        let span = outcome.solution().unwrap();
        let minimum = outcome.cost().unwrap();
        assert_eq!(
            minimum.question(),
            &CostQuestion::MinimumOverFamily(super::AdmissibleFamily::PartialAssignment)
        );
        // Crossing to the nearer target is free, so the optimizer must cross.
        assert_eq!(minimum.total(), 0.0);
        assert!(span.is_bijective());

        // The declared identity-order span costs strictly more, which is why
        // the two numbers answer different questions.
        let naive = VoiceLeading::new(
            from.voice_set(),
            to.voice_set(),
            [
                Edge::new(VoiceId::new("a"), VoiceId::new("x")),
                Edge::new(VoiceId::new("b"), VoiceId::new("y")),
            ],
        )
        .unwrap();
        let declared = model.declared_cost(&naive, &from, &to).unwrap();
        assert_eq!(declared.question(), &CostQuestion::DeclaredSpan);
        assert!(declared.total() > minimum.total());
    }

    #[test]
    fn ties_are_reported_rather_than_hidden() {
        // Two targets equidistant from two identical sources: swapping them
        // costs the same, so the minimum is attained more than once. The
        // penalty has to be positive, or leaving every voice unmatched would
        // be free and uniquely optimal.
        let model = SpanCostModel::new(ground(), 1.0, SpanPenalties::uniform(1.0)).unwrap();
        let from = chord(&[("a", 0), ("b", 0)]);
        let to = chord(&[("x", 3), ("y", 3)]);
        let outcome = model.minimum_over_assignments(&from, &to).unwrap();
        assert!(
            matches!(outcome, OptimizationOutcome::Multiple { others, .. } if others > 0),
            "{outcome:?}"
        );
    }

    #[test]
    fn a_truncated_search_is_reported_as_approximate() {
        let model = SpanCostModel::new(ground(), 1.0, SpanPenalties::uniform(1.0))
            .unwrap()
            .with_search_options(SearchOptions {
                budget: 3,
                ..SearchOptions::default()
            });
        let from = chord(&[("a", 0), ("b", 1), ("c", 2)]);
        let to = chord(&[("x", 3), ("y", 4), ("z", 5)]);
        let outcome = model.minimum_over_assignments(&from, &to).unwrap();
        assert!(!outcome.is_optimal());
        assert!(matches!(outcome, OptimizationOutcome::Approximate { .. }));
    }

    #[test]
    fn balanced_transport_refuses_unequal_voice_counts() {
        let distance = ChordDistance::new(
            ground(),
            2.0,
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            },
        )
        .unwrap();
        let single = chord(&[("a", 0)]);
        let doubled = chord(&[("a", 0), ("b", 0)]);

        assert!(matches!(
            distance.distance(&single, &doubled),
            Err(PitchError::UnequalMass { left: 1, right: 2 })
        ));
        assert_eq!(distance.distance(&single, &single).unwrap(), 0.0);
    }

    #[test]
    fn normalization_loses_multiplicity_and_has_to_be_asked_for() {
        let distance = ChordDistance::new(
            ground(),
            2.0,
            TransportProfile::Balanced {
                mass: MassProfile::NormalizedProbability,
            },
        )
        .unwrap();
        let single = chord(&[("a", 0)]);
        let doubled = chord(&[("a", 0), ("b", 0)]);

        assert_eq!(
            distance.distance(&single, &doubled).unwrap(),
            0.0,
            "this is the documented loss, not an accident"
        );
        // The chords themselves remain different objects.
        assert_ne!(single, doubled);
        assert_ne!(
            single.forget_voice_labels().total_len(),
            doubled.forget_voice_labels().total_len()
        );
    }

    #[test]
    fn the_edit_profile_charges_the_configured_boundary_cost() {
        let distance =
            ChordDistance::new(ground(), 1.0, TransportProfile::Edit { boundary: 0.75 }).unwrap();
        let single = chord(&[("a", 0)]);
        let doubled = chord(&[("a", 0), ("b", 0)]);

        // One voice matches at zero cost; the other has to be born.
        assert!((distance.distance(&single, &doubled).unwrap() - 0.75).abs() < 1e-12);
        assert!((distance.distance(&doubled, &single).unwrap() - 0.75).abs() < 1e-12);
        assert_eq!(distance.distance(&single, &single).unwrap(), 0.0);
    }

    #[test]
    fn the_edit_profile_prefers_deleting_when_moving_is_dearer() {
        // Boundary 0.05 octaves each way: moving a whole octave costs 1.0, so
        // a death plus a birth at 0.10 is cheaper, and the optimizer takes it.
        let thrifty =
            ChordDistance::new(ground(), 1.0, TransportProfile::Edit { boundary: 0.05 }).unwrap();
        let low = chord(&[("a", 0)]);
        let high = chord(&[("x", 12)]);
        assert!((thrifty.distance(&low, &high).unwrap() - 0.10).abs() < 1e-12);

        // With a large boundary the same pair is matched instead.
        let patient =
            ChordDistance::new(ground(), 1.0, TransportProfile::Edit { boundary: 5.0 }).unwrap();
        assert!((patient.distance(&low, &high).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_zero_boundary_cost_withdraws_the_metric_claim() {
        let degenerate =
            ChordDistance::new(ground(), 1.0, TransportProfile::Edit { boundary: 0.0 }).unwrap();
        assert!(matches!(
            degenerate.metric_claim(),
            MetricClaim::NotClaimed { .. }
        ));
        // And the reason is real: a chord and a chord containing it coincide.
        let single = chord(&[("a", 0)]);
        let doubled = chord(&[("a", 0), ("b", 7)]);
        assert_eq!(degenerate.distance(&single, &doubled).unwrap(), 0.0);
        assert_ne!(single, doubled);
    }

    #[test]
    fn the_edit_profile_satisfies_the_laws_it_claims() {
        // Section 9.5: an unequal-mass profile tests the laws of its own
        // metric rather than inheriting the classical Wasserstein ones. The
        // state space here spans three different cardinalities.
        let distance =
            ChordDistance::new(ground(), 1.0, TransportProfile::Edit { boundary: 0.3 }).unwrap();
        assert!(matches!(
            distance.metric_claim(),
            MetricClaim::Metric { .. }
        ));

        let states: Vec<Chord<AmbientElem>> = [
            &[("a", 0)][..],
            &[("a", 7)][..],
            &[("a", 0), ("b", 4)][..],
            &[("a", 0), ("b", 7)][..],
            &[("a", 1), ("b", 5), ("c", 8)][..],
            &[("a", 0), ("b", 4), ("c", 7)][..],
        ]
        .iter()
        .map(|voices| chord(voices))
        .collect();

        for left in &states {
            assert_eq!(
                distance.distance(left, left).unwrap(),
                0.0,
                "identity of indiscernibles, one direction"
            );
            for right in &states {
                let there = distance.distance(left, right).unwrap();
                let back = distance.distance(right, left).unwrap();
                assert!((there - back).abs() < 1e-12, "symmetry");
                if left.forget_voice_labels() != right.forget_voice_labels() {
                    assert!(there > 0.0, "identity of indiscernibles, other direction");
                }
                for middle in &states {
                    let via = distance.distance(left, middle).unwrap()
                        + distance.distance(middle, right).unwrap();
                    assert!(there <= via + 1e-12, "triangle inequality: {there} > {via}");
                }
            }
        }
    }

    #[test]
    fn transport_parameters_are_validated() {
        assert!(matches!(
            ChordDistance::new(
                ground(),
                0.5,
                TransportProfile::Balanced {
                    mass: MassProfile::PerVoice
                }
            ),
            Err(PitchError::NonMetricExponent { .. })
        ));
        assert!(matches!(
            ChordDistance::new(ground(), 2.0, TransportProfile::Edit { boundary: -1.0 }),
            Err(PitchError::InvalidCostParameter)
        ));
        assert!(matches!(
            SpanCostModel::new(ground(), 1.0, SpanPenalties::uniform(-1.0)),
            Err(PitchError::InvalidCostParameter)
        ));
        assert_eq!(SpanPenalties::FREE.birth, 0.0);
    }

    #[test]
    fn balanced_per_voice_transport_matches_the_hand_computed_value() {
        // Two voices moving by 2 and 1 semitones, p = 2:
        // W_2 = sqrt((2/12)^2 + (1/12)^2).
        let distance = ChordDistance::new(
            ground(),
            2.0,
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            },
        )
        .unwrap();
        let from = chord(&[("a", 0), ("b", 4)]);
        let to = chord(&[("x", 2), ("y", 5)]);
        let expected = libm::sqrt((2.0f64 / 12.0) * (2.0 / 12.0) + (1.0f64 / 12.0) * (1.0 / 12.0));
        assert!((distance.distance(&from, &to).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn balanced_transport_satisfies_the_laws_it_claims() {
        let distance = ChordDistance::new(
            ground(),
            2.0,
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            },
        )
        .unwrap();
        let states: Vec<Chord<AmbientElem>> = [
            &[("a", 0), ("b", 4), ("c", 7)][..],
            &[("a", 0), ("b", 3), ("c", 7)][..],
            &[("a", 2), ("b", 5), ("c", 9)][..],
            &[("a", -5), ("b", 0), ("c", 4)][..],
        ]
        .iter()
        .map(|voices| chord(voices))
        .collect();

        for left in &states {
            assert_eq!(distance.distance(left, left).unwrap(), 0.0);
            for right in &states {
                let there = distance.distance(left, right).unwrap();
                let back = distance.distance(right, left).unwrap();
                assert!((there - back).abs() < 1e-12, "symmetry");
                if left.forget_voice_labels() != right.forget_voice_labels() {
                    assert!(there > 0.0);
                }
                for middle in &states {
                    let via = distance.distance(left, middle).unwrap()
                        + distance.distance(middle, right).unwrap();
                    assert!(there <= via + 1e-12, "triangle inequality: {there} > {via}");
                }
            }
        }
    }

    #[test]
    fn a_pseudometric_on_labelled_chords_is_reported_as_such() {
        let distance = ChordDistance::new(
            ground(),
            2.0,
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            },
        )
        .unwrap();
        let one = chord(&[("a", 0), ("b", 7)]);
        let relabelled = chord(&[("b", 0), ("a", 7)]);
        assert_ne!(one, relabelled, "different chords");
        assert_eq!(
            distance.distance(&one, &relabelled).unwrap(),
            0.0,
            "at distance zero, which is why the claim names the multiset space"
        );
        match distance.metric_claim() {
            MetricClaim::Metric { state_space } => {
                assert!(state_space.contains("pseudometric"));
            }
            other => panic!("unexpected claim {other:?}"),
        }
    }

    #[test]
    fn a_search_that_cannot_be_exact_refuses_rather_than_approximates() {
        let distance = ChordDistance::new(
            ground(),
            1.0,
            TransportProfile::Balanced {
                mass: MassProfile::PerVoice,
            },
        )
        .unwrap()
        .with_search_options(SearchOptions {
            budget: 2,
            ..SearchOptions::default()
        });
        let from = chord(&[("a", 0), ("b", 4), ("c", 7)]);
        let to = chord(&[("x", 1), ("y", 5), ("z", 8)]);
        assert!(matches!(
            distance.distance(&from, &to),
            Err(PitchError::SearchBudgetExceeded { budget: 2 })
        ));
    }

    #[test]
    fn ground_cost_rejects_points_from_another_origin() {
        let here = point(0);
        let there = PitchPoint::new(PitchOrigin::new("umt:origin:a4"), steps().zero());
        assert!(matches!(
            ground().distance(&here, &there),
            Err(PitchError::OriginMismatch { .. })
        ));
    }

    #[test]
    fn a_span_displays_its_edges() {
        let span = VoiceLeading::identity(&voices(&["a", "b"]));
        assert_eq!(alloc::format!("{span}"), "{a -> a, b -> b}");
    }
}
