//! Regular temperament structure (UMT-3.2 part I, sections 1.4 to 1.7).
//!
//! A regular temperament mapping is an integer homomorphism
//! `V: Lambda_B -> Gamma` into a declared ambient free abelian group. The
//! mapping need not be surjective, so the ambient group `Gamma` and the
//! reachable image `H = im(V)` are always distinguished (section 1.4), and the
//! kernel `K = ker(V)` is a separate object again.
//!
//! Four things that are easy to confuse are separate types here:
//!
//! - [`TemperamentMap`] - the exact structural map;
//! - [`ImageLattice`] and [`KernelLattice`] - the reachable image and the
//!   tempered-out commas, each with its own intrinsic coordinates;
//! - [`PatentVal`] - a constructor for the equal-division case, carrying the
//!   rounding convention and exactness of the entries it derived;
//! - a tuning, which does not live here at all, because it is a real-valued
//!   map of intervals rather than a structural map of lattices.
//!
//! Section 1.7 adds two more that are also not interchangeable:
//!
//! - [`HomomorphicSplit`] - a group homomorphism `s: H -> Lambda_B` with
//!   `V . s = id_H`, which yields a direct-sum decomposition;
//! - [`RepresentativePolicy`] - an arbitrary right inverse, which need not
//!   preserve addition and usually does not.
//!
//! A splitting adapts into a policy through [`SplitPolicy`]. The reverse
//! conversion does not exist, because it is not generally valid.

pub mod edo;
pub mod image;
pub mod kernel;
pub mod map;
pub mod representative;
pub mod splitting;

#[doc(inline)]
pub use crate::temperament::edo::{Exactness, PatentVal};
#[doc(inline)]
pub use crate::temperament::image::{
    AmbientElem, AmbientLattice, ImageElem, ImageLattice, LatticeId,
};
#[doc(inline)]
pub use crate::temperament::kernel::{
    KernelElem, KernelLattice, SaturationPolicy, SaturationReport,
};
#[doc(inline)]
pub use crate::temperament::map::{RawTemperamentMap, TemperamentMap};
#[doc(inline)]
pub use crate::temperament::representative::{
    CanonicalLiftPolicy, LensError, LiftDecision, OffsetPolicy, RepresentativePolicy, SplitPolicy,
    StructuralLens, no_residue,
};
#[doc(inline)]
pub use crate::temperament::splitting::{HomomorphicSplit, LinearSplit};
