//! Pitch (UMT-3.2 part IV).
//!
//! The layer boundaries this module keeps apart:
//!
//! - **L1/L2 intervals** are exact lattice elements, and they form groups.
//! - **L1/L2 pitch points** are [`PitchPoint`] values: a torsor over an
//!   interval group, with a declared origin. Points do not add.
//! - **L3 quantities** are the newtypes in [`units`]: [`Octaves`] and
//!   [`Cents`] are intervals, [`LogFrequency`] and [`FrequencyHz`] are
//!   positions.
//! - **Realization** is the boundary between them, and it comes in two kinds
//!   that are not interchangeable: [`RegularTuning`] with reference data, and
//!   the contextual [`PitchRealizer`].
//!
//! Above those sit the objects music is actually written in. A [`Chord`] is a
//! function from voice identities to points, so unisons and doublings survive.
//! A [`VoiceLeading`] is a span rather than a permutation, so splits, merges,
//! entries, and exits are representable. A [`PitchTrajectory`] is
//! `Phi(x, c(t)) + v(t)`, so a bend around a nominal pitch never gets confused
//! with a different nominal pitch.
//!
//! Not yet built: pitch notation at L0 (section 4.5, deliberately deferred by
//! prompt section 55) and empirical inharmonic scales (section 4.9, which
//! belongs with the external adapters).

pub mod chord;
pub mod empirical;
pub mod point;
pub mod trajectory;
pub mod tuning;
pub mod units;
pub mod voice_leading;

#[doc(inline)]
pub use crate::pitch::chord::{Chord, ChordAnnotation, PitchMultiset, VoiceId, VoiceSet};
#[doc(inline)]
pub use crate::pitch::empirical::{
    EmpiricalDegree, EmpiricalScale, FitDeclaration, IndependenceClaim, LatticeFit, ScaleId,
};
#[doc(inline)]
pub use crate::pitch::point::{IntervalGroupElement, PitchOrigin, PitchPoint, PitchPointRef};
#[doc(inline)]
pub use crate::pitch::trajectory::{
    Deviation, Interpolation, PitchTrajectory, PitchTrajectoryRef, SampledTrajectory,
    SamplingRecord, TrajectorySample,
};
#[doc(inline)]
pub use crate::pitch::tuning::{L2IntervalGroup, PitchRealization, PitchRealizer, RegularTuning};
#[doc(inline)]
pub use crate::pitch::units::{
    CENTS_PER_OCTAVE, Cents, FrequencyHz, LogFrequency, Octaves, Radians,
};
#[doc(inline)]
pub use crate::pitch::voice_leading::{
    AdmissibleFamily, ChordDistance, CostQuestion, Edge, GroundCost, LogPitchDistance, MassProfile,
    MetricClaim, SearchOptions, SpanCost, SpanCostModel, SpanPenalties, SpanShape,
    TransportProfile, VoiceLeading,
};
