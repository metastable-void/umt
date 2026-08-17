//! The score as an event-indexed object (UMT-3.2 part VI).
//!
//! Section 6.1 makes the case: a bare pair of a pitch aggregate and a time
//! aggregate does not say which pitch belongs to which event, which voice
//! produced it, or which duration goes with which notehead. Event identity is
//! therefore primary, and everything else in this module refers to events by
//! identity.
//!
//! What that buys, concretely:
//!
//! - a rest is a notated event in a voice or staff context rather than the
//!   complement of sound, and a global rest cannot be constructed;
//! - a global tempo marker has no voice and is not given a fabricated one;
//! - a tied pair stays two noteheads plus a relation, and the single sustained
//!   gesture is a derived view rather than a destructive merge;
//! - an unmeasured event carries temporal *variables* rather than an invented
//!   rational onset;
//! - a projection declares what it forgot;
//! - a transformation claims compositionality only when every one of its
//!   components actually composes.

pub mod container;
pub mod event;
pub mod id;
pub mod transform;

#[doc(inline)]
pub use crate::score::container::{
    Projection, Score, ScoreBuilder, ScoreContext, ScoreRef, SoundingGesture, StructuralScore, Tie,
    loss_set_description,
};
#[doc(inline)]
pub use crate::score::event::{EventContent, GraceRule, ScoreEvent, TemporalPlacement};
#[doc(inline)]
pub use crate::score::id::{EventId, EventScope, PartId, StaffId};
#[doc(inline)]
pub use crate::score::transform::{
    EventRelation, PitchTransform, ProvenanceChain, ScoreTransform, TimeTransform,
};
