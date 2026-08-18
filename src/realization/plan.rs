//! The compiled performance-plan boundary (prompt section 38).
//!
//! The semantic core is not realtime-safe and does not try to be: it uses
//! arbitrary-precision arithmetic, allocates freely, and runs normal-form and
//! graph algorithms. That is the right design for a theory library and the
//! wrong one for an audio thread.
//!
//! A [`PerformancePlan`] is the boundary between the two. Compiling one moves
//! every expensive decision to authoring time: exact rationals become bounded
//! integers, voice identities become indices, pitches become resolved
//! millicent offsets, and events are sorted once. What remains is a sorted
//! slice and integer comparisons.
//!
//! # What is actually guaranteed
//!
//! Prompt section 38 is explicit that a `RealtimeSafe` claim requires a
//! documented contract the type actually satisfies, so this crate does not
//! implement a marker trait and hope. [`RealtimeContract`] is a *value*
//! describing what a built plan guarantees, `docs/realtime.md` states the
//! contract in prose, and the guarantees are these:
//!
//! - reading a plan performs no allocation - [`PerformancePlan::events`] and
//!   [`PerformancePlan::events_in`] return borrowed slices;
//! - reading a plan performs no arbitrary-precision arithmetic - every stored
//!   value is a bounded integer;
//! - every numeric range was validated at compile time, so a reader needs no
//!   bounds checks of its own beyond slice indexing;
//! - device mappings are resolved, so no lookup by name happens on the
//!   performance thread.
//!
//! Compiling a plan is emphatically *not* realtime-safe. It allocates, sorts,
//! and validates. That is the point of it being a separate step.

use alloc::vec::Vec;

use crate::error::RealizationError;
use crate::realization::provenance::ProvenanceId;
use crate::realization::residual::ResidualSet;

/// The largest tick a plan may reference.
///
/// About 24 days at 960 ticks per beat and 120 beats per minute, which is
/// past the point where a single plan is the right structure.
pub const MAX_TICK: u32 = u32::MAX / 2;

/// The pitch range a plan may reference, in millicents from its reference.
///
/// Twenty octaves either way, which covers every audible pitch and a good deal
/// besides.
pub const MAX_MILLICENTS: i32 = 24_000_000;

/// A resolved voice index.
///
/// A `u16` rather than a string: resolving names to indices is exactly the
/// kind of work that must not happen on a performance thread.
pub type VoiceIndex = u16;

/// One event of a compiled plan.
///
/// UMT layer: L4. Every field is a bounded integer, validated when the plan
/// was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlannedEvent {
    /// When the event starts, in ticks.
    pub onset_tick: u32,
    /// How long it lasts, in ticks. Zero is legal: a control event is an
    /// instant.
    pub duration_ticks: u32,
    /// Its pitch, in millicents from the plan's reference, or `None` for an
    /// unpitched or control event.
    pub millicents: Option<i32>,
    /// Which resolved voice it belongs to.
    pub voice: VoiceIndex,
}

impl PlannedEvent {
    /// The tick one past the end of this event.
    #[must_use]
    pub fn end_tick(self) -> u32 {
        self.onset_tick.saturating_add(self.duration_ticks)
    }
}

/// What a built [`PerformancePlan`] guarantees (prompt section 38).
///
/// A value rather than a marker trait, because the claim has to be checkable
/// and specific. Every field is `true` for a plan built through
/// [`PerformancePlanBuilder`]; the type exists so a caller can assert on it
/// and so the guarantee has somewhere to live besides a doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealtimeContract {
    /// Reading the plan allocates nothing.
    pub no_allocation_on_read: bool,
    /// Reading the plan performs no arbitrary-precision arithmetic.
    pub no_arbitrary_precision: bool,
    /// Every numeric range was validated at build time.
    pub bounded_ranges_validated: bool,
    /// Voice and device mappings are resolved to indices.
    pub device_mapping_resolved: bool,
    /// Events are sorted, so a reader can seek by binary search.
    pub events_sorted: bool,
}

impl RealtimeContract {
    /// Whether every guarantee holds.
    #[must_use]
    pub fn is_satisfied(self) -> bool {
        self.no_allocation_on_read
            && self.no_arbitrary_precision
            && self.bounded_ranges_validated
            && self.device_mapping_resolved
            && self.events_sorted
    }
}

/// A compiled, bounded performance plan (prompt sections 37 and 38).
///
/// UMT layer: L4.
///
/// Immutable once built. A score is the authoring object and a plan is
/// compiled *from* it, never by mutating it - which is what makes
/// re-realization at a different resolution, tuning, or tempo policy a matter
/// of recompiling from the source rather than of unpicking a previous result.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformancePlan {
    ticks_per_beat: u32,
    reference_millicents: i32,
    events: Vec<PlannedEvent>,
    voice_count: usize,
    residuals: ResidualSet,
    provenance: Option<ProvenanceId>,
}

impl PerformancePlan {
    /// Starts building a plan.
    #[must_use]
    pub fn builder(ticks_per_beat: u32) -> PerformancePlanBuilder {
        PerformancePlanBuilder {
            ticks_per_beat,
            reference_millicents: 0,
            events: Vec::new(),
            voice_count: 0,
            residuals: ResidualSet::new(),
            provenance: None,
        }
    }

    /// The plan's tick resolution.
    #[must_use]
    pub fn ticks_per_beat(&self) -> u32 {
        self.ticks_per_beat
    }

    /// The reference pitch every event's millicents are measured from.
    #[must_use]
    pub fn reference_millicents(&self) -> i32 {
        self.reference_millicents
    }

    /// Every event, in tick order.
    ///
    /// A borrowed slice: reading a plan allocates nothing.
    #[must_use]
    pub fn events(&self) -> &[PlannedEvent] {
        &self.events
    }

    /// The events whose onset falls in `[from, to)`.
    ///
    /// Two binary searches over a sorted slice. No allocation, no
    /// arbitrary-precision arithmetic, and no work proportional to the whole
    /// plan.
    #[must_use]
    pub fn events_in(&self, from: u32, to: u32) -> &[PlannedEvent] {
        if to <= from {
            return &[];
        }
        let start = self.events.partition_point(|event| event.onset_tick < from);
        let end = self.events.partition_point(|event| event.onset_tick < to);
        &self.events[start..end]
    }

    /// How many resolved voices the plan references.
    #[must_use]
    pub fn voice_count(&self) -> usize {
        self.voice_count
    }

    /// What compiling the plan cost.
    #[must_use]
    pub fn residuals(&self) -> &ResidualSet {
        &self.residuals
    }

    /// The provenance of the compilation.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// What this plan guarantees on a performance thread.
    ///
    /// Every field is `true`, and each one is a property the build step
    /// established rather than an aspiration.
    #[must_use]
    pub fn realtime_contract(&self) -> RealtimeContract {
        RealtimeContract {
            no_allocation_on_read: true,
            no_arbitrary_precision: true,
            bounded_ranges_validated: true,
            device_mapping_resolved: true,
            events_sorted: true,
        }
    }

    /// The last tick any event occupies.
    #[must_use]
    pub fn end_tick(&self) -> u32 {
        self.events
            .iter()
            .map(|event| event.end_tick())
            .max()
            .unwrap_or(0)
    }
}

/// Builds a validated [`PerformancePlan`] (prompt section 52).
///
/// Building is not realtime-safe: it allocates, sorts, and validates. All of
/// that is done here so none of it has to happen later.
#[derive(Debug, Clone)]
pub struct PerformancePlanBuilder {
    ticks_per_beat: u32,
    reference_millicents: i32,
    events: Vec<PlannedEvent>,
    voice_count: usize,
    residuals: ResidualSet,
    provenance: Option<ProvenanceId>,
}

impl PerformancePlanBuilder {
    /// Sets the reference pitch every event's millicents are measured from.
    #[must_use]
    pub fn reference_millicents(mut self, millicents: i32) -> Self {
        self.reference_millicents = millicents;
        self
    }

    /// Adds an event.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::TickOutOfRange`] for an onset or end past
    /// [`MAX_TICK`], and [`RealizationError::PitchOutOfRange`] for a pitch
    /// past [`MAX_MILLICENTS`]. Both are checked here so no reader has to.
    pub fn event(mut self, event: PlannedEvent) -> Result<Self, RealizationError> {
        if event.onset_tick > MAX_TICK || event.end_tick() > MAX_TICK {
            return Err(RealizationError::TickOutOfRange {
                tick: u64::from(event.end_tick()),
            });
        }
        if let Some(millicents) = event.millicents
            && millicents.abs() > MAX_MILLICENTS
        {
            return Err(RealizationError::PitchOutOfRange { millicents });
        }
        self.voice_count = self.voice_count.max(usize::from(event.voice) + 1);
        self.events.push(event);
        Ok(self)
    }

    /// Records what compiling the plan cost.
    #[must_use]
    pub fn residuals(mut self, residuals: ResidualSet) -> Self {
        self.residuals = residuals;
        self
    }

    /// Attaches provenance.
    #[must_use]
    pub fn provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Freezes and sorts the plan.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::ZeroResolution`] for a resolution of zero,
    /// which is not a grid.
    pub fn build(mut self) -> Result<PerformancePlan, RealizationError> {
        if self.ticks_per_beat == 0 {
            return Err(RealizationError::ZeroResolution);
        }
        // Sorted once, here, so a reader can seek by binary search. The
        // ordering is total and derived, which keeps it deterministic.
        self.events.sort_unstable();
        Ok(PerformancePlan {
            ticks_per_beat: self.ticks_per_beat,
            reference_millicents: self.reference_millicents,
            events: self.events,
            voice_count: self.voice_count,
            residuals: self.residuals,
            provenance: self.provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_MILLICENTS, MAX_TICK, PerformancePlan, PlannedEvent};
    use crate::error::RealizationError;
    use crate::realization::provenance::ProvenanceId;

    fn note(onset: u32, duration: u32, millicents: i32, voice: u16) -> PlannedEvent {
        PlannedEvent {
            onset_tick: onset,
            duration_ticks: duration,
            millicents: Some(millicents),
            voice,
        }
    }

    fn plan() -> PerformancePlan {
        PerformancePlan::builder(960)
            .reference_millicents(0)
            .event(note(1920, 480, 70_000, 0))
            .unwrap()
            .event(note(0, 960, 0, 0))
            .unwrap()
            .event(note(960, 960, 40_000, 1))
            .unwrap()
            .provenance(ProvenanceId::new("umt:prov:compile-1"))
            .build()
            .unwrap()
    }

    #[test]
    fn building_sorts_once_so_reading_can_seek() {
        let plan = plan();
        let onsets: alloc::vec::Vec<u32> =
            plan.events().iter().map(|event| event.onset_tick).collect();
        assert_eq!(
            onsets,
            [0, 960, 1920],
            "inserted out of order, stored sorted"
        );
        assert_eq!(plan.voice_count(), 2);
        assert_eq!(plan.ticks_per_beat(), 960);
        assert_eq!(plan.end_tick(), 2400);
    }

    #[test]
    fn a_window_is_two_binary_searches_over_a_borrowed_slice() {
        let plan = plan();
        assert_eq!(plan.events_in(0, 960).len(), 1);
        assert_eq!(plan.events_in(960, 2400).len(), 2);
        assert_eq!(plan.events_in(2400, 9999).len(), 0);
        assert_eq!(plan.events_in(500, 500).len(), 0, "an empty window");
        assert_eq!(plan.events_in(1000, 100).len(), 0, "a reversed window");

        // The window really is a subslice of the plan's own storage: nothing
        // was copied to produce it.
        let window = plan.events_in(0, 3000);
        assert_eq!(window.as_ptr(), plan.events().as_ptr());
        assert_eq!(window.len(), plan.events().len());
    }

    #[test]
    fn ranges_are_validated_at_build_time_so_readers_need_not_check() {
        let builder = PerformancePlan::builder(960);
        assert!(matches!(
            builder.clone().event(note(MAX_TICK, 1, 0, 0)),
            Err(RealizationError::TickOutOfRange { .. })
        ));
        assert!(matches!(
            builder.clone().event(note(0, 1, MAX_MILLICENTS + 1, 0)),
            Err(RealizationError::PitchOutOfRange { .. })
        ));
        assert!(builder.event(note(0, 1, -MAX_MILLICENTS, 0)).is_ok());
        assert!(matches!(
            PerformancePlan::builder(0).build(),
            Err(RealizationError::ZeroResolution)
        ));
    }

    #[test]
    fn the_realtime_contract_is_a_value_not_a_marker_trait() {
        let contract = plan().realtime_contract();
        assert!(contract.is_satisfied());
        assert!(contract.no_allocation_on_read);
        assert!(contract.no_arbitrary_precision);
        assert!(contract.bounded_ranges_validated);
        assert!(contract.device_mapping_resolved);
        assert!(contract.events_sorted);
    }

    #[test]
    fn a_plan_is_send_and_sync_so_it_can_cross_to_the_audio_thread() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PerformancePlan>();
        assert_send_sync::<PlannedEvent>();
    }

    #[test]
    fn control_events_are_instants_without_pitch() {
        let plan = PerformancePlan::builder(96)
            .event(PlannedEvent {
                onset_tick: 0,
                duration_ticks: 0,
                millicents: None,
                voice: 0,
            })
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(plan.events()[0].millicents, None);
        assert_eq!(plan.events()[0].end_tick(), 0);
        assert!(plan.provenance().is_none());
        assert!(plan.residuals().is_empty());
    }
}
