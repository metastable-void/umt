//! Exact arithmetic primitives (UMT-3.2 L1/L2 storage).
//!
//! Everything in this module is exact. No value produced here depends on binary
//! floating point, in accordance with UMT-3.2 section 0.6.1, which forbids
//! floating point for identity, equality, quotient membership, and conformance
//! decisions at L0-L2.
//!
//! The two aliases [`Z`] and [`Q`] are the crate's stability boundary over its
//! arbitrary-precision arithmetic dependency (prompt section 5.1). Public APIs
//! are expressed in terms of them so the underlying implementation can be
//! replaced without a breaking change to UMT semantics.
//!
//! Real-valued (L3) helpers live in [`crate::algebra::integer`] as well, but
//! they are explicitly named `*_f64` or documented as L3 approximations, and
//! never feed an exact decision.

pub mod integer;
pub mod lattice;
pub mod matrix;
pub mod normal_form;
pub mod quotient;
pub mod rational;
pub mod rounding;

#[doc(inline)]
pub use crate::algebra::integer::Z;
#[doc(inline)]
pub use crate::algebra::lattice::Sublattice;
#[doc(inline)]
pub use crate::algebra::matrix::IntMatrix;
#[doc(inline)]
pub use crate::algebra::normal_form::{HermiteNormalForm, SmithNormalForm};
#[doc(inline)]
pub use crate::algebra::quotient::QuotientGroup;
#[doc(inline)]
pub use crate::algebra::rational::Q;
#[doc(inline)]
pub use crate::algebra::rounding::RoundingConvention;
