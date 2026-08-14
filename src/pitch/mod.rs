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
//! Still to come in this layer: pitch trajectories (section 4.7), chords and
//! voice identity (4.3), and voice leading (4.4).

pub mod point;
pub mod tuning;
pub mod units;

#[doc(inline)]
pub use crate::pitch::point::{IntervalGroupElement, PitchOrigin, PitchPoint};
#[doc(inline)]
pub use crate::pitch::tuning::{L2IntervalGroup, PitchRealization, PitchRealizer, RegularTuning};
#[doc(inline)]
pub use crate::pitch::units::{CENTS_PER_OCTAVE, Cents, FrequencyHz, LogFrequency, Octaves};
