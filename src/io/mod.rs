//! Interchange support (UMT-3.2 part VIII).
//!
//! Only the exact-value text codec and its serde adapters exist so far. The
//! versioned native container of section 8.8 lands with the `TheoryContext`
//! work, because objects that reference shared context - monzos, mappings,
//! events - must serialize as references to a registry rather than inlining
//! their definitions (UMT-3.2 section 6.3).
//!
//! Wire policy fixed here and not expected to change:
//!
//! - exact integers and rationals are encoded as canonical decimal text, never
//!   as floating point and never through a dependency's internal digit
//!   representation;
//! - a rational always carries an explicit denominator, so `"3"` and `"3/1"`
//!   both parse but only `"3/1"` is written.

pub mod text;
pub mod version;

#[cfg(feature = "serde")]
pub mod serde_exact;
