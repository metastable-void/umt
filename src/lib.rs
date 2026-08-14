#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub mod algebra;
pub mod context;
pub mod error;
pub mod io;
pub mod proportion;
pub mod realization;
pub mod temperament;

#[doc(inline)]
pub use crate::algebra::{IntMatrix, Q, RoundingConvention, Sublattice, Z};
#[doc(inline)]
pub use crate::proportion::{Basis, BasisId, GeneratorId, GeneratorValuation, Monzo, PositiveQ};
#[doc(inline)]
pub use crate::temperament::{AmbientLattice, PatentVal, TemperamentMap};

/// Version of the UMT specification whose semantics this crate implements.
///
/// This is deliberately independent of the Cargo package version and of the
/// native serialization schema version (UMT-3.2 section 8.8, prompt section 54):
/// a patch release of the crate does not imply a new semantic profile, and a
/// future UMT revision is not assumed to be backward compatible.
pub const UMT_SPEC_VERSION: &str = "UMT-3.2";
