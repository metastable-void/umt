//! Realization (UMT-3.2 part VII).
//!
//! The layer that turns structure into a result, and records what that cost.
//!
//! - [`optimization`] is the common optimization interface of section 7.5,
//!   which every layer that minimizes anything reports through - adaptive lift
//!   selection, voice-leading search, grid allocation.
//! - [`residual`] is the taxonomy of section 7.9. Seven kinds in genuinely
//!   different spaces, and no undifferentiated `error` field anywhere.
//! - [`provenance`] is section 7.10: structured records in an arena,
//!   referenced by identifier, with typed parameters that keep their
//!   exactness.
//! - [`record`] is the realization record itself, plus the device-adapter
//!   contract of section 7.6.
//! - [`plan`] is the compiled performance-plan boundary of prompt section 38,
//!   which is where the semantic core stops and a realtime consumer begins.

pub mod optimization;
pub mod plan;
pub mod provenance;
pub mod record;
pub mod residual;

#[doc(inline)]
pub use crate::realization::optimization::{ApproximationGuarantee, OptimizationOutcome};
#[doc(inline)]
pub use crate::realization::plan::{
    PerformancePlan, PerformancePlanBuilder, PlannedEvent, RealtimeContract,
};
#[doc(inline)]
pub use crate::realization::provenance::{
    AlgorithmId, CanonicalValue, FormatVersion, ProvenanceArena, ProvenanceId, ProvenanceRecord,
};
#[doc(inline)]
pub use crate::realization::record::{
    DeviceAdapterProfile, Layer, RealizationRecord, RoundTripBasis, SaturationBehaviour,
};
#[doc(inline)]
pub use crate::realization::residual::{Residual, ResidualKind, ResidualRecord, ResidualSet};
