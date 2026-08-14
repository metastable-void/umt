//! Time (UMT-3.2 part V).
//!
//! Only the physical timeline exists so far, because UMT-3.2 section 4.7 needs
//! a domain for pitch trajectories before the rhythm layer is built. What is
//! here is deliberately the *inexact* half:
//!
//! - [`ClockTime`] is a measured position and [`Seconds`] a measured interval,
//!   both real-valued;
//! - [`TimeSpan`] is the closed domain `[t0, t1]` a trajectory is defined on.
//!
//! Structural beat time is a different thing entirely - exact, rational, and
//! notated - and arrives with the rhythm layer as its own type. Section 5.8.3
//! is explicit that a tempo map is not the same kind of object as a pitch
//! tuning, and keeping the two timelines apart is what makes that statable.

pub mod span;
pub mod units;

#[doc(inline)]
pub use crate::time::span::TimeSpan;
#[doc(inline)]
pub use crate::time::units::{ClockTime, Seconds};
