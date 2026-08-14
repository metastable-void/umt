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

/// A temperament mapping, image, or kernel operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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

    /// An underlying monzo operation failed.
    #[error(transparent)]
    Monzo(#[from] MonzoError),
}

/// An equal-division mapping could not be constructed.
///
/// Operations on a constructed mapping report [`TemperamentError`]; this type
/// covers only what can go wrong while deriving the entries.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
