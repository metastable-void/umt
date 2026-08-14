//! Realization records (UMT-3.2 part VII).
//!
//! Two pieces exist so far. [`optimization`] holds the common optimization
//! interface of section 7.5, which every layer that minimizes anything reports
//! through - adaptive lift selection, voice-leading search, and later grid
//! allocation. [`provenance`] holds the identifier that section 0.6.1 requires
//! every real-valued observation to carry, defined early because objects that
//! need it exist already.
//!
//! Residual taxonomy, realization records, and the compiled performance plan
//! belong to a later stage.

pub mod optimization;
pub mod provenance;
