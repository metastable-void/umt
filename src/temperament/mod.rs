//! Regular temperament structure (UMT-3.2 part I, sections 1.4 to 1.7).
//!
//! A regular temperament mapping is an integer homomorphism
//! `V: Lambda_B -> Gamma` into a declared ambient free abelian group. The
//! mapping need not be surjective, so the ambient group `Gamma` and the
//! reachable image `H = im(V)` are always distinguished (section 1.4).
//!
//! Only the equal-division case is implemented so far, where `Gamma = Z` and
//! the mapping is a single row. The general matrix-valued `TemperamentMap`,
//! with Smith and Hermite normal forms, kernel bases, image lattices, and
//! representative policies, is the next stage; see `docs/architecture.md`.

pub mod edo;

#[doc(inline)]
pub use crate::temperament::edo::{Exactness, PatentVal};
