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

/// An equal-division mapping could not be constructed or applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PatentValError {
    /// A monzo from an unrelated basis was supplied to this mapping.
    #[error("basis mismatch: mapping is over `{expected}`, argument is over `{found}`")]
    BasisMismatch {
        /// Identity of the basis this mapping is defined over.
        expected: BasisId,
        /// Identity of the argument's basis.
        found: BasisId,
    },

    /// An ambient step is not in the image `H = im(V)` of this mapping.
    ///
    /// This is the expected outcome for an odd 6-EDO step under `[6,10,14]`
    /// (UMT-3.2 section 1.6.1, fixture F4): detempering is defined on `H`, not
    /// on arbitrary elements of the ambient lattice `Gamma`.
    #[error("ambient step {step} is not in the image of this mapping")]
    NotInImage {
        /// The rejected ambient coordinate.
        step: Z,
    },

    /// The image of this mapping is the trivial group.
    ///
    /// A rank-zero image has no integer coordinate, so ambient-to-image
    /// conversion is undefined rather than zero (UMT-3.2 section 1.6: the zero
    /// mapping has `H_N = {0}`).
    #[error("the image of this mapping is trivial and has no rank-one coordinate")]
    TrivialImage,

    /// A generator's valuation is unusable for an equal-division entry.
    #[error("generator {index} has a valuation that cannot produce an entry: {reason}")]
    UnusableValuation {
        /// Index of the offending generator within the basis.
        index: usize,
        /// Why the valuation could not be used.
        reason: String,
    },
}
