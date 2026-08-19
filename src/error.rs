//! Typed errors.
//!
//! Errors preserve the semantic distinction that failed (prompt section 43).
//! No public API returns a bare string, and every enum is `#[non_exhaustive]`
//! so that new failure modes from later UMT layers are not breaking changes.
//!
//! With the `std` feature these implement `std::error::Error`; without it they
//! implement `core::error::Error`, which is the same trait.

use alloc::string::String;

use crate::algebra::Z;
use crate::proportion::basis::{BasisId, GeneratorId};
use crate::temperament::image::LatticeId;

/// A generator valuation could not be accepted or applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValuationError {
    /// An exact generator valuation was not in `Q_{>0}` (UMT-3.2 section 1.1.1).
    #[error("exact generator valuation must be strictly positive")]
    NonPositiveRational,

    /// An L3 real valuation was not a positive finite number.
    #[error("real valuation must be positive and finite")]
    NonPositiveReal,

    /// An uncertainty was negative or not finite.
    #[error("uncertainty must be non-negative and finite")]
    InvalidUncertainty,

    /// An exact rational value was requested from a generator that has only a
    /// symbolic-real valuation (UMT-3.2 section 1.1.2).
    #[error("generator {index} has no exact rational valuation")]
    NotRationalProfile {
        /// Index of the offending generator within the basis.
        index: usize,
    },

    /// Text did not parse as an exact rational.
    #[error("malformed exact rational: `{text}`")]
    MalformedRational {
        /// The rejected text.
        text: String,
    },

    /// A monzo exponent was too large to evaluate as an exact power.
    ///
    /// The exact lattice arithmetic itself has no such bound; only evaluating
    /// `r(m)` as a materialized rational does.
    #[error("exponent at generator {index} is too large to evaluate exactly")]
    ExponentOutOfRange {
        /// Index of the offending generator within the basis.
        index: usize,
    },
}

/// A basis could not be constructed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BasisError {
    /// Two generators declared the same identity.
    ///
    /// Generator identity is semantic, not positional, so duplicates would make
    /// serialized references ambiguous.
    #[error("duplicate generator identity `{id}`")]
    DuplicateGeneratorId {
        /// The repeated identity.
        id: GeneratorId,
    },

    /// A prime-basis constructor was given a value that is not prime.
    ///
    /// Multiplicative independence of a prime basis rests on unique
    /// factorization (UMT-3.2 section 1.1.1); it cannot be claimed for
    /// composite or unit entries.
    #[error(
        "`{value}` is not prime, so a prime-factorization independence contract cannot be claimed"
    )]
    NotPrime {
        /// The offending value.
        value: u32,
    },

    /// A generator valuation was invalid.
    #[error(transparent)]
    Valuation(#[from] ValuationError),
}

/// An integer-matrix or lattice operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MatrixError {
    /// Row-major data did not match the declared shape.
    #[error("expected {expected} entries for this shape, found {found}")]
    DataLength {
        /// Entries required by the declared shape.
        expected: usize,
        /// Entries supplied.
        found: usize,
    },

    /// Rows of differing length were supplied.
    #[error("expected rows of width {expected}, found one of width {found}")]
    RaggedRows {
        /// Width established by the first row.
        expected: usize,
        /// Width of the offending row.
        found: usize,
    },

    /// Two operands had incompatible dimensions.
    #[error("dimension mismatch: {left} versus {right}")]
    DimensionMismatch {
        /// Dimension required by the left operand.
        left: usize,
        /// Dimension offered by the right operand.
        right: usize,
    },

    /// An index was outside the matrix.
    #[error("index ({row}, {col}) is out of bounds")]
    IndexOutOfBounds {
        /// Row index.
        row: usize,
        /// Column index.
        col: usize,
    },
}

/// A monzo operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MonzoError {
    /// Two monzos from different bases were combined.
    ///
    /// Exponent vectors of equal length are not interchangeable: `[1,0,0]` over
    /// `(2,3,5)` and `[1,0,0]` over `(2,3,7)` are different semantic objects
    /// (prompt section 7).
    #[error("basis mismatch: `{left}` versus `{right}`")]
    BasisMismatch {
        /// Identity of the left operand's basis.
        left: BasisId,
        /// Identity of the right operand's basis.
        right: BasisId,
    },

    /// An exponent vector did not match the rank of its basis.
    #[error("expected {expected} exponents for this basis, found {found}")]
    RankMismatch {
        /// Rank of the basis.
        expected: usize,
        /// Number of exponents supplied.
        found: usize,
    },

    /// A valuation was required but could not be evaluated.
    #[error(transparent)]
    Valuation(#[from] ValuationError),
}

/// A theory-context registration or lookup failed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ContextError {
    /// A referenced basis is not registered.
    #[error("no basis registered under `{id}`")]
    UnknownBasis {
        /// The unresolved identifier.
        id: BasisId,
    },

    /// A referenced ambient lattice is not registered.
    #[error("no ambient lattice registered under `{id}`")]
    UnknownAmbient {
        /// The unresolved identifier.
        id: LatticeId,
    },

    /// A referenced mapping is not registered.
    #[error("no mapping registered under `{id}`")]
    UnknownMapping {
        /// The unresolved identifier.
        id: crate::context::MappingId,
    },

    /// An identifier was reused for a different basis.
    #[error("`{id}` is already registered as a different basis")]
    ConflictingBasis {
        /// The reused identifier.
        id: BasisId,
    },

    /// An identifier was reused for a different ambient lattice.
    #[error("`{id}` is already registered as a different ambient lattice")]
    ConflictingAmbient {
        /// The reused identifier.
        id: LatticeId,
    },

    /// An identifier was reused for a different mapping.
    #[error("`{id}` is already registered as a different mapping")]
    ConflictingMapping {
        /// The reused identifier.
        id: crate::context::MappingId,
    },

    /// A resolved monzo failed validation.
    #[error(transparent)]
    Monzo(#[from] MonzoError),

    /// A resolved mapping failed validation.
    #[error(transparent)]
    Temperament(#[from] TemperamentError),
}

/// A physical-time quantity or span was rejected.
///
/// Structural beat time is exact and will report its own failures; everything
/// here is about the measured, real-valued timeline of UMT-3.2 section 5.1.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TimeError {
    /// A time quantity was not finite.
    #[error("physical time quantities must be finite")]
    NonFiniteQuantity,

    /// A span was given endpoints in the wrong order.
    ///
    /// A reversed span in a document is a defect, not a direction, so it is
    /// rejected rather than silently normalized.
    #[error("time span runs backwards: [{start}, {end}]")]
    ReversedSpan {
        /// The declared start.
        start: f64,
        /// The declared end.
        end: f64,
    },

    /// A time outside a closed span was supplied where one inside was
    /// required.
    ///
    /// A trajectory is defined on its domain and nowhere else (UMT-3.2 section
    /// 4.7), so extrapolation is refused rather than guessed.
    #[error("time {time} is outside the span [{start}, {end}]")]
    OutsideSpan {
        /// The offending time.
        time: f64,
        /// Start of the span.
        start: f64,
        /// End of the span.
        end: f64,
    },

    /// A rate was not strictly positive.
    ///
    /// A tempo map is strictly increasing, so neither its derivative nor the
    /// reciprocal beat rate can be zero or negative (UMT-3.2 section 5.8).
    #[error("a tempo rate must be positive and finite, found {rate}")]
    NonPositiveRate {
        /// The rejected value.
        rate: f64,
    },

    /// An exact ratio was zero or negative.
    ///
    /// Proportions act on positive quantities (UMT-3.2 section 2.1).
    #[error("a proportion must be strictly positive")]
    NonPositiveRatio,

    /// A zero denominator was supplied for an exact structural value.
    #[error("a rational structural value cannot have a zero denominator")]
    ZeroDenominator,

    /// A structural duration was zero or negative.
    ///
    /// A zero-duration note is not a short note. Section 5.8.4 requires a
    /// zero-structural-duration delay to be represented explicitly, not as a
    /// degenerate span.
    #[error("a structural duration must be strictly positive")]
    NonPositiveDuration,

    /// A structural span was given endpoints in the wrong order.
    #[error("structural span runs backwards")]
    ReversedBeatSpan,

    /// A span was given a negative length in ticks.
    ///
    /// A span of negative length is not a span. Allocating children within one
    /// would produce negative durations that sum correctly and mean nothing.
    #[error("a span cannot have a negative length")]
    NegativeSpan,

    /// A rhythm-tree weight was not strictly positive
    /// (UMT-3.2 section 5.3.1).
    #[error("rhythm-tree child weights must be strictly positive")]
    NonPositiveWeight,

    /// An internal rhythm-tree node was given no children.
    ///
    /// A node with no children is a leaf; an empty division would be a node
    /// that divides its span into nothing.
    #[error("a rhythm-tree division must have at least one child")]
    EmptyDivision,

    /// A cyclic pattern was given a zero-length cycle.
    #[error("a cyclic pattern must have at least one pulse")]
    EmptyCycle,

    /// A pulse index fell outside its cycle.
    #[error("pulse index {index} is outside a cycle of {pulses} pulses")]
    OnsetOutsideCycle {
        /// The offending index.
        index: u32,
        /// The cycle length.
        pulses: u32,
    },

    /// A metrical level was not contained in the finer level below it.
    ///
    /// Section 5.4.1 nests the levels as `L_2 subset L_1 subset L_0`. Levels
    /// need not be *subgroups*, but they must nest.
    #[error("metrical level {level} is not contained in the level below it")]
    LevelNotNested {
        /// Index of the offending level, counting the pulse lattice as zero.
        level: usize,
    },

    /// A time signature was malformed.
    #[error("`{numerator}/{denominator}` is not a usable time signature")]
    InvalidTimeSignature {
        /// The upper number.
        numerator: u32,
        /// The lower number.
        denominator: u32,
    },

    /// A subgroup extended outside its parent group.
    #[error("a subgroup must lie inside its parent span")]
    GroupOutsideParent,

    /// Two sibling groups overlapped.
    #[error("sibling groups must not overlap")]
    OverlappingGroups,

    /// A structural position fell outside a declared span.
    #[error("structural position is outside the declared span")]
    OutsideBeatSpan,

    /// A tempo map had too few breakpoints to span an interval.
    #[error("a tempo map needs at least two breakpoints spanning a positive interval")]
    DegenerateTempoMap,

    /// A tempo map assigned two clock times to one structural position.
    ///
    /// Strictly increasing is not sufficient: the homeomorphism profile also
    /// requires continuity and surjectivity onto an interval, and a jump has
    /// neither (UMT-3.2 section 9.9, fixture F15). Section 5.8.4 lists the
    /// sanctioned ways to represent a pause instead.
    #[error("tempo map is discontinuous at beat {beat}")]
    DiscontinuousTempoMap {
        /// The structural position of the jump.
        beat: String,
    },

    /// A tempo map failed to increase strictly on one of its timelines.
    #[error("a tempo map must increase strictly on both timelines")]
    NonMonotoneTempoMap,

    /// A temporal variable was referenced but not declared.
    #[error("temporal variable `{variable}` is not declared in this problem")]
    UnknownTimeVariable {
        /// The unresolved identifier.
        variable: String,
    },

    /// An exhaustive elimination exceeded its declared budget.
    ///
    /// Reported rather than approximated: a feasibility answer that might be
    /// wrong is worse than no answer.
    #[error("linear elimination exceeded its budget of {budget} constraints")]
    EliminationBudgetExceeded {
        /// The budget that was exceeded.
        budget: usize,
    },
}

/// A score event, tie, or transformation was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ScoreError {
    /// An event identity was used twice.
    ///
    /// Event identity is the score's index (UMT-3.2 section 6.2), so a repeat
    /// is a defect rather than an update.
    #[error("event `{event}` is already in this score")]
    DuplicateEvent {
        /// The repeated identity.
        event: crate::score::id::EventId,
    },

    /// A reference named an event the score does not contain.
    #[error("event `{event}` is not in this score")]
    UnknownEvent {
        /// The unresolved identity.
        event: crate::score::id::EventId,
    },

    /// A note or rest was given a global scope.
    ///
    /// A rest is a notated event in a voice or staff context, not the global
    /// set-theoretic complement of sounding intervals (UMT-3.2 section 5.2.4),
    /// and a sounding event without any performing context is not a thing.
    #[error("event `{event}` sounds or rests, so it needs a voice, staff, or part")]
    SoundingEventWithoutContext {
        /// The offending event.
        event: crate::score::id::EventId,
    },

    /// A tie related an event to itself.
    ///
    /// A tie relates two *distinct* noteheads (UMT-3.2 section 5.2.2).
    #[error("event `{event}` cannot be tied to itself")]
    SelfTie {
        /// The offending event.
        event: crate::score::id::EventId,
    },

    /// A tie endpoint was not a notehead.
    #[error("a tie relates two noteheads, and `{from}` or `{to}` is not one")]
    TieBetweenNonNotes {
        /// The earlier endpoint.
        from: crate::score::id::EventId,
        /// The later endpoint.
        to: crate::score::id::EventId,
    },

    /// A tie crossed between different scopes.
    #[error("`{from}` and `{to}` are in different scopes, so a tie between them is not a tie")]
    TieAcrossScopes {
        /// The earlier endpoint.
        from: crate::score::id::EventId,
        /// The later endpoint.
        to: crate::score::id::EventId,
    },

    /// A tie related two different pitches.
    ///
    /// A relation between noteheads of different pitch is a slur or a
    /// glissando; a tie continues one pitch.
    #[error("`{from}` and `{to}` have different pitches, so a tie between them is not a tie")]
    TiedPitchesDiffer {
        /// The earlier endpoint.
        from: crate::score::id::EventId,
        /// The later endpoint.
        to: crate::score::id::EventId,
    },

    /// A tie ran backwards or left a gap.
    #[error("the tie from `{from}` to `{to}` does not continue it")]
    MisorderedTie {
        /// The earlier endpoint.
        from: crate::score::id::EventId,
        /// The later endpoint.
        to: crate::score::id::EventId,
    },

    /// A grace event anchored to another grace event.
    #[error("grace event `{event}` anchors to another grace event, so it has nothing to stand on")]
    GraceAnchorIsGrace {
        /// The offending event.
        event: crate::score::id::EventId,
    },

    /// A constrained placement referred to a variable the network does not
    /// declare.
    #[error("temporal variable `{variable}` is not declared in this score's network")]
    UndeclaredTemporalVariable {
        /// The unresolved identifier.
        variable: String,
    },

    /// A temporal scale factor was zero or negative.
    ///
    /// A non-positive scale reverses or collapses the timeline, which is not a
    /// score transformation.
    #[error("a temporal scale factor must be strictly positive")]
    NonPositiveTimeScale,

    /// A declared transformation component was asked to evaluate or compose.
    ///
    /// UMT-3.2 section 6.6 forbids claiming compositionality without the
    /// operation, so the operation is absent exactly where the claim would be
    /// unearned.
    #[error("transformation component `{component}` is application-declared and does not compose")]
    UncomposableTransform {
        /// The declared component's name.
        component: String,
    },

    /// An underlying structural-time operation failed.
    #[error(transparent)]
    Time(#[from] TimeError),

    /// An underlying pitch operation failed.
    #[error(transparent)]
    Pitch(#[from] PitchError),

    /// An underlying context resolution failed.
    #[error(transparent)]
    Context(#[from] ContextError),
}

/// A pitch quantity, point, or realization was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PitchError {
    /// A metric quantity was not finite.
    #[error("pitch quantities must be finite")]
    NonFiniteQuantity,

    /// A frequency was not strictly positive.
    #[error("a frequency must be positive and finite")]
    NonPositiveFrequency,

    /// Two pitch points were measured from different origins.
    ///
    /// No interval between them is defined: "a fifth above C" and "a fifth
    /// above D" are different pitches, and nothing in an exponent vector says
    /// which origin is meant (UMT-3.2 section 1.10).
    #[error("pitch origin mismatch: `{left}` versus `{right}`")]
    OriginMismatch {
        /// Origin of the left point.
        left: crate::pitch::point::PitchOrigin,
        /// Origin of the right point.
        right: crate::pitch::point::PitchOrigin,
    },

    /// An interval belongs to a different declared L2 interval group.
    ///
    /// A tuning of the reachable image is not a tuning of the ambient group,
    /// and section 1.9 requires the choice to be recorded rather than assumed.
    #[error("this interval belongs to a different declared interval group")]
    IntervalGroupMismatch,

    /// A tuning was given the wrong number of generator sizes.
    #[error("expected {expected} generator sizes, found {found}")]
    SizeCount {
        /// Sizes required.
        expected: usize,
        /// Sizes supplied.
        found: usize,
    },

    /// A voice identity was used twice where each must be distinct.
    ///
    /// Deduplicating silently would turn a two-voice doubling into one voice,
    /// which is exactly the loss UMT-3.2 section 4.4.4 forbids.
    #[error("voice `{voice}` appears more than once")]
    DuplicateVoice {
        /// The repeated identity.
        voice: crate::pitch::chord::VoiceId,
    },

    /// A lookup or an edge named a voice that is not present.
    #[error("voice `{voice}` is not in this voice set")]
    UnknownVoice {
        /// The unresolved identity.
        voice: crate::pitch::chord::VoiceId,
    },

    /// A voice-leading span was applied to chords that are not its endpoints.
    #[error("this voice leading does not connect the voice sets it was given")]
    VoiceSetMismatch,

    /// Balanced transport was asked to compare states of different total mass.
    ///
    /// Not a defect in the input: UMT-3.2 section 4.4.4 says classical
    /// balanced transport simply does not solve this case, so an unbalanced,
    /// partial, or edit profile has to be selected instead.
    #[error("balanced transport requires equal total mass: {left} versus {right} voices")]
    UnequalMass {
        /// Voices on the left.
        left: usize,
        /// Voices on the right.
        right: usize,
    },

    /// A transport exponent below 1 was supplied.
    ///
    /// The classical `W_p` metric claims require `p >= 1` (UMT-3.2 section
    /// 9.5), so a smaller exponent is rejected rather than accepted with the
    /// claims quietly withdrawn.
    #[error("a W_p transport exponent must be at least 1, found {exponent}")]
    NonMetricExponent {
        /// The rejected exponent.
        exponent: f64,
    },

    /// A declared cost parameter was negative or not finite.
    #[error("declared cost parameters must be non-negative and finite")]
    InvalidCostParameter,

    /// An exhaustive search would have exceeded its budget.
    ///
    /// Raised only where an approximate answer would be wrong to return, such
    /// as a distance that claims metric laws and therefore has to be the true
    /// minimum. Optimizers that may approximate report
    /// [`crate::realization::optimization::OptimizationOutcome::Approximate`]
    /// instead.
    #[error("the exhaustive search exceeded its budget of {budget} candidates")]
    SearchBudgetExceeded {
        /// The budget that was exceeded.
        budget: usize,
    },

    /// A reconstruction was requested from an empty sampling.
    #[error("this sampling contains no samples")]
    NoSamples,

    /// A measurement uncertainty was negative.
    #[error("a measurement uncertainty cannot be negative")]
    NegativeUncertainty,

    /// A lattice fit did not cover every measured degree with an
    /// empirical-fit residual.
    ///
    /// UMT-3.2 section 4.9.3 requires "the approximation residual for every
    /// fitted interval", so a fit that reports fewer is incomplete rather than
    /// partial.
    #[error(
        "a lattice fit of {degrees} degrees needs {degrees} empirical-fit residuals, found {residuals}"
    )]
    IncompleteFit {
        /// Degrees the scale has.
        degrees: usize,
        /// Residuals the fit supplied.
        residuals: usize,
    },

    /// An underlying physical-time operation failed.
    #[error(transparent)]
    Time(#[from] TimeError),

    /// An underlying monzo operation failed.
    #[error(transparent)]
    Monzo(#[from] MonzoError),

    /// An underlying valuation failed.
    #[error(transparent)]
    Valuation(#[from] ValuationError),

    /// An underlying temperament operation failed.
    #[error(transparent)]
    Temperament(#[from] TemperamentError),
}

/// A complexity function could not be built or evaluated.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ComplexityError {
    /// The number of weights did not match the basis rank.
    #[error("expected {expected} weights for this basis, found {found}")]
    WeightCount {
        /// Rank of the basis.
        expected: usize,
        /// Number of weights supplied.
        found: usize,
    },

    /// A weight was negative.
    #[error("weight {index} is negative")]
    NegativeWeight {
        /// Index of the offending weight.
        index: usize,
    },

    /// A derived weight was not strictly positive.
    ///
    /// A generator whose valuation is at most 1 has a logarithm that is zero
    /// or negative. Using it as a norm weight would produce a function that is
    /// not a norm, which UMT-3.2 fixture F5 requires be caught rather than
    /// silently accepted.
    #[error("derived weight {weight} at generator {index} is not strictly positive")]
    NonPositiveWeight {
        /// Index of the offending generator.
        index: usize,
        /// The rejected weight.
        weight: f64,
    },

    /// Tenney height was requested for a basis that is not a prime basis.
    ///
    /// The reduced-rational identity `h_T(m) = log2(n d)` is specific to
    /// prime-factor coordinates (UMT-3.2 section 1.3.2).
    #[error("Tenney height requires a basis with a prime-factorization independence contract")]
    NotPrimeBasis,

    /// An exact rational valuation was required and is not available.
    #[error("this complexity requires an exact rational basis profile")]
    NotRationalProfile,

    /// An exponent was too large to evaluate.
    #[error("exponent is too large to evaluate as a real magnitude")]
    ExponentOutOfRange,

    /// A monzo from an unrelated basis was supplied.
    #[error("basis mismatch: expected `{expected}`, found `{found}`")]
    BasisMismatch {
        /// Identity of the expected basis.
        expected: BasisId,
        /// Identity of the supplied basis.
        found: BasisId,
    },
}

/// A temperament mapping, image, or kernel operation was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TemperamentError {
    /// The mapping matrix does not match the declared domain and ambient
    /// ranks.
    #[error(
        "mapping matrix must be {expected_rows}x{expected_cols} for this domain and ambient lattice, found {found_rows}x{found_cols}"
    )]
    ShapeMismatch {
        /// Rows required, that is, the ambient rank.
        expected_rows: usize,
        /// Columns required, that is, the domain rank.
        expected_cols: usize,
        /// Rows supplied.
        found_rows: usize,
        /// Columns supplied.
        found_cols: usize,
    },

    /// A monzo from an unrelated basis was supplied.
    #[error("basis mismatch: expected `{expected}`, found `{found}`")]
    BasisMismatch {
        /// Identity of the expected basis.
        expected: BasisId,
        /// Identity of the supplied basis.
        found: BasisId,
    },

    /// An element of an unrelated ambient lattice was supplied.
    #[error("ambient lattice mismatch: expected `{expected}`, found `{found}`")]
    AmbientMismatch {
        /// Identity of the expected ambient lattice.
        expected: LatticeId,
        /// Identity of the supplied element's lattice.
        found: LatticeId,
    },

    /// An element of a different image lattice was supplied.
    #[error("image lattice mismatch")]
    ImageMismatch,

    /// An element of a different kernel lattice was supplied.
    #[error("kernel lattice mismatch")]
    KernelMismatch,

    /// An ambient element is not in the reachable image `H = im(V)`.
    ///
    /// Not a defect: an ambient coordinate outside the image simply has no
    /// automatic detempering under the mapping (UMT-3.2 section 1.6.1).
    #[error("ambient element is not in the image of this mapping")]
    NotInImage {
        /// The rejected ambient coordinates.
        coordinates: alloc::vec::Vec<Z>,
    },

    /// The image of a mapping is the trivial group, so it has no rank-one
    /// coordinate.
    ///
    /// Raised only by the rank-one convenience API of
    /// [`crate::temperament::PatentVal`]. In the general API a rank-zero image
    /// has an empty coordinate vector, which is the correct answer rather than
    /// an error.
    #[error("the image of this mapping is trivial and has no rank-one coordinate")]
    TrivialImage,

    /// A coordinate vector did not match the rank of its lattice.
    #[error("expected {expected} coordinates, found {found}")]
    CoordinateRank {
        /// Rank of the lattice.
        expected: usize,
        /// Number of coordinates supplied.
        found: usize,
    },

    /// A directly supplied comma subgroup is not saturated.
    ///
    /// Such a subgroup defines a quotient with torsion, which no homomorphism
    /// into a torsion-free real group can realize (UMT-3.2 section 1.5). This
    /// applies to directly specified commas only; a kernel computed from a
    /// mapping is saturated automatically and is never rejected for this.
    #[error("directly supplied comma subgroup is not saturated")]
    UnsaturatedCommaSubgroup {
        /// The invariant factors above 1, that is, the torsion orders of the
        /// resulting quotient.
        torsion_invariants: alloc::vec::Vec<Z>,
    },

    /// A representative policy or splitting violated its own contract.
    ///
    /// The right-inverse law `V(sigma(x)) = x` is not optional (UMT-3.2 law
    /// P8). A policy that returns a lift outside the fiber it was asked about
    /// is reported here rather than silently producing a residue that is not
    /// in the kernel.
    #[error("the representative policy is not a right inverse of this mapping")]
    NotARightInverse,

    /// An underlying matrix or lattice operation failed.
    #[error(transparent)]
    Matrix(#[from] MatrixError),

    /// A complexity function does not bound a search over a fiber.
    ///
    /// A minimum-complexity policy needs a `lattice_norm`: a seminorm has
    /// nonzero elements of zero cost, so a coset can contain infinitely many
    /// minimizers and no finite search region exists.
    #[error("this complexity does not bound the search: a lattice norm is required")]
    UnboundedComplexity,

    /// An underlying monzo operation failed.
    #[error(transparent)]
    Monzo(#[from] MonzoError),

    /// An underlying complexity evaluation failed.
    #[error(transparent)]
    Complexity(#[from] ComplexityError),
}

/// An equal-division mapping could not be constructed.
///
/// Operations on a constructed mapping report [`TemperamentError`]; this type
/// covers only what can go wrong while deriving the entries.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PatentValError {
    /// A generator's valuation is unusable for an equal-division entry.
    #[error("generator {index} has a valuation that cannot produce an entry: {reason}")]
    UnusableValuation {
        /// Index of the offending generator within the basis.
        index: usize,
        /// Why the valuation could not be used.
        reason: String,
    },

    /// Building the underlying structural mapping failed.
    #[error(transparent)]
    Temperament(#[from] TemperamentError),
}

/// A realization record, residual, provenance record, or performance plan was
/// rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RealizationError {
    /// Two residuals of different kinds were added.
    ///
    /// UMT-3.2 section 7.9: residuals "MUST NOT be added numerically unless
    /// they live in compatible spaces and the addition is mathematically
    /// meaningful".
    #[error("residuals of kind {left:?} and {right:?} live in different spaces")]
    IncompatibleResiduals {
        /// The left operand's kind.
        left: crate::realization::residual::ResidualKind,
        /// The right operand's kind.
        right: crate::realization::residual::ResidualKind,
    },

    /// Two residuals of a kind this crate declines to add were added.
    ///
    /// An empirical fit would need a declared model for combining its
    /// uncertainties; a device-control residual is a pair of encoded values;
    /// a notation residual is symbolic. Refusing beats inventing a
    /// convention.
    #[error("residuals of kind {kind:?} are not additive")]
    NonAdditiveResidual {
        /// The offending kind.
        kind: crate::realization::residual::ResidualKind,
    },

    /// A residual value was not finite.
    #[error("a residual must be finite")]
    NonFiniteResidual,

    /// An uncertainty was negative or not finite.
    #[error("an uncertainty must be non-negative and finite")]
    InvalidUncertainty,

    /// A provenance record did not name an algorithm and a version.
    ///
    /// UMT-3.2 section 7.10 requires provenance "sufficient to identify the
    /// semantic profile, algorithm/version, and parameters"; a record without
    /// them cannot.
    #[error("provenance record `{id}` does not identify its algorithm and version")]
    AnonymousProvenance {
        /// The offending identifier.
        id: crate::realization::provenance::ProvenanceId,
    },

    /// A provenance identifier was reused for a different record.
    #[error("`{id}` is already registered as a different provenance record")]
    DuplicateProvenance {
        /// The reused identifier.
        id: crate::realization::provenance::ProvenanceId,
    },

    /// A provenance identifier is not in the arena.
    #[error("no provenance record registered under `{id}`")]
    UnknownProvenance {
        /// The unresolved identifier.
        id: crate::realization::provenance::ProvenanceId,
    },

    /// A realization record ran backwards through the pipeline.
    ///
    /// Backward paths exist but are type-specific: UMT-3.2 section 7.1
    /// explicitly rejects the claim that every adjacent pair is the same kind
    /// of lens.
    #[error("a realization record cannot run backwards, from {entry} to {exit}")]
    BackwardRealization {
        /// The entry layer.
        entry: crate::realization::record::Layer,
        /// The exit layer.
        exit: crate::realization::record::Layer,
    },

    /// A planned tick fell outside the validated range.
    #[error("tick {tick} is outside the range a performance plan may reference")]
    TickOutOfRange {
        /// The offending tick.
        tick: u64,
    },

    /// A planned pitch fell outside the validated range.
    #[error("{millicents} millicents is outside the range a performance plan may reference")]
    PitchOutOfRange {
        /// The offending pitch.
        millicents: i32,
    },

    /// A performance plan was given a zero tick resolution.
    #[error("a performance plan needs a positive tick resolution")]
    ZeroResolution,

    /// An underlying temperament operation failed.
    #[error(transparent)]
    Temperament(#[from] TemperamentError),
}

/// A native document could not be validated.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum IoError {
    /// This build cannot read the document's schema version.
    #[error("cannot read schema version {document}; this build writes {native}")]
    UnreadableSchema {
        /// The document's version.
        document: crate::io::version::UmtSchemaVersion,
        /// This build's version.
        native: crate::io::version::UmtSchemaVersion,
    },

    /// A distinguished unit was present with no basis to interpret it.
    ///
    /// The unit is a monzo, and a monzo's coordinates mean nothing without the
    /// basis they are over (UMT-3.2 section 1.1).
    #[error("a distinguished unit needs a basis to be interpreted against")]
    UnitWithoutBasis,

    /// A representative policy could be reproduced neither from an identifier
    /// and version nor from the lifts it used.
    ///
    /// UMT-3.2 section 8.8 requires one or the other whenever the choices
    /// matter for round trip.
    #[error(
        "a representative policy must be reproducible from an identifier and version, \
         or must serialize the lifts it selected"
    )]
    IrreproduciblePolicy,

    /// A section referenced a provenance record the document does not carry.
    #[error("provenance record `{id}` is referenced but not present in this document")]
    DanglingProvenance {
        /// The unresolved identifier.
        id: crate::realization::provenance::ProvenanceId,
    },

    /// An external format could not be parsed.
    #[error("malformed {format} input at line {line}: {reason}")]
    MalformedInput {
        /// Which format was being read.
        format: String,
        /// The line the problem was found on, counting from 1.
        line: usize,
        /// What was wrong.
        reason: String,
    },
}

/// A generated set or Euclidean rhythm was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GeneratedError {
    /// A period was zero or negative.
    #[error("a generated set needs a strictly positive period")]
    NonPositivePeriod,

    /// A cardinality or pulse count was zero.
    ///
    /// UMT-3.2 section 3.1 defines the construction for `n >= 1`, and section
    /// 3.5 for `0 < k <= n`.
    #[error("a generated structure needs at least one element")]
    EmptyCardinality,

    /// More onsets than pulses were requested.
    #[error("cannot distribute {onsets} onsets among {pulses} pulses")]
    TooManyOnsets {
        /// Onsets requested.
        onsets: u32,
        /// Pulses available.
        pulses: u32,
    },

    /// A mode or rotation index fell outside the pattern.
    #[error("degree {degree} is outside a pattern of {steps} steps")]
    DegreeOutOfRange {
        /// The offending index.
        degree: usize,
        /// How many steps the pattern has.
        steps: usize,
    },

    /// A gap-comparison tolerance was negative or not finite.
    #[error("a gap tolerance must be non-negative and finite")]
    InvalidTolerance,
}
