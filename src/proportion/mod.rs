//! The exact proportion core (UMT-3.2 part I, sections 1.1 to 1.3).
//!
//! A formal basis `B = (beta_1, ..., beta_k)` spans the free abelian group
//! `Lambda_B = Z^k`. Its elements are monzos, and addition of monzos
//! corresponds to multiplication of the represented proportions. The same
//! algebra serves pitch intervals and tempo/rhythmic proportions (UMT-3.2
//! section 0.1); what differs is the enrichment each domain adds on top, which
//! lives in other modules.
//!
//! Everything here is exact. Real-valued views are explicitly named `*_f64`
//! and belong to L3.

pub mod basis;
pub mod monzo;
pub mod valuation;

#[doc(inline)]
pub use crate::proportion::basis::{
    Basis, BasisBuilder, BasisGenerator, BasisId, GeneratorId, GeneratorValuation,
    IndependenceContract, RawBasis,
};
#[doc(inline)]
pub use crate::proportion::monzo::Monzo;
#[doc(inline)]
pub use crate::proportion::valuation::{
    NonNegativeFinite, PositiveFinite, PositiveQ, RealValuation,
};
