//! Generated structures (UMT-3.2 part III).
//!
//! Two families that share modular arithmetic and are not the same object:
//!
//! - [`GeneratedSet`] is a modular generated set on a pitch circle, with its
//!   period and generator stored as *designated data* because a rank-2
//!   temperament does not determine which is which (section 3.1).
//! - [`EuclideanRhythm`] is a maximally even distribution of onsets among
//!   pulses, with its rotation convention declared and its evenness verified
//!   rather than assumed (sections 3.5 and 9.11).
//!
//! Section 3.5 permits common algorithms for modular distribution and then
//! declines to identify the two constructions with each other. So they are
//! separate types with no conversion between them, and each computes what its
//! own section is about.

pub mod euclidean;
pub mod scale;

#[doc(inline)]
pub use crate::generated::euclidean::{EuclideanRhythm, EvennessReport, RotationConvention};
#[doc(inline)]
pub use crate::generated::scale::{
    DEFAULT_GAP_TOLERANCE, GapReport, GeneratedSet, GeneratorRatio, MosProfile, MosVerdict,
    quarter_comma_meantone_generator,
};
