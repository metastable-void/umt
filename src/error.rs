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
