//! Time (UMT-3.2 part V).
//!
//! Two timelines, kept apart because section 5.1 keeps them apart:
//!
//! - the **structural beat timeline** `T_b` is exact. [`BeatTime`] is a
//!   position, [`Beats`] a signed difference, [`BeatDuration`] a strictly
//!   positive one, and all three are arbitrary-precision rationals. Nothing
//!   notated is ever a `f64`.
//! - the **performance clock timeline** `T_c` is measured. [`ClockTime`] and
//!   [`Seconds`] are real-valued, and so is everything derived from them.
//!
//! A tempo map is the map between them, and it is emphatically not the same
//! kind of object as a pitch tuning (section 5.8.3): a tuning is a group
//! homomorphism on intervals, a tempo map is a monotone map between affine
//! ordered timelines. One shared timeline type would erase the distinction the
//! map exists to express.
//!
//! On top of the structural timeline sit the notated structures:
//! [`RhythmTree`] for hierarchical and tupleted rhythm, [`CyclicRhythm`] for
//! pattern-based rhythm, [`Meter`] for nested periodic pulse sets, and
//! [`Grouping`] for the segmentation that need not agree with any of them.
//!
//! [`rate`] holds the rate/duration orientation rule of part II, which is what
//! stops a tempo ratio being silently reused as a duration ratio.

pub mod beat;
pub mod constraint;
pub mod meter;
pub mod quantize;
pub mod rate;
pub mod rhythm;
pub mod span;
pub mod tempo;
pub mod units;

#[doc(inline)]
pub use crate::time::beat::{
    BEAT_DURATION_GROUP, BEAT_UNIT, BeatDuration, BeatSpan, BeatTime, Beats,
};
#[doc(inline)]
pub use crate::time::constraint::{
    DifferenceConstraint, Exactness, ExternalPredicate, HybridTemporalProblem, LinearConstraint,
    LinearTemporalProblem, PositivityHandling, PredicateEvaluator, RatioConstraint, SolverProfile,
    StpProblem, TemporalOutcome, TimeVarId,
};
#[doc(inline)]
pub use crate::time::meter::{
    Grouping, LayerRelation, LevelNumbering, Meter, MetricLayering, TimeSignature,
};
#[doc(inline)]
pub use crate::time::quantize::{
    AllocatedChild, AllocationInfeasibility, AllocationOutcome, AllocationPolicy, CollisionPolicy,
    GridAllocation, Quantized, QuantizedNode, TickGrid,
};
#[doc(inline)]
pub use crate::time::rate::{
    BeatsPerMinute, BeatsPerSecond, OrientedRatio, RatioOrientation, SecondsPerBeat,
};
#[doc(inline)]
pub use crate::time::rhythm::{CyclicRhythm, FlatLeaf, RhythmTree};
#[doc(inline)]
pub use crate::time::span::TimeSpan;
#[doc(inline)]
pub use crate::time::tempo::{PauseRepresentation, TempoBreakpoint, TempoMap};
#[doc(inline)]
pub use crate::time::units::{ClockTime, Seconds};
