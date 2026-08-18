//! Example 6 of the implementation prompt: compiling an exact score into a
//! bounded performance plan, with device limits validated.
//!
//! This is the boundary between the semantic core and a realtime consumer
//! (prompt section 38). Everything above it is exact and allocates freely;
//! everything below it is bounded integers in a sorted slice. What the
//! crossing costs is recorded rather than discarded, as UMT-3.2 section 7.9
//! requires.
//!
//! Run with `cargo run --example performance_compilation`.

use umt::pitch::{Cents, PitchOrigin, PitchPoint, RegularTuning, VoiceId};
use umt::realization::{
    AlgorithmId, CanonicalValue, Layer, PerformancePlan, PlannedEvent, ProvenanceArena,
    ProvenanceId, ProvenanceRecord, RealizationRecord, Residual, ResidualKind, ResidualRecord,
    ResidualSet, RoundTripBasis,
};
use umt::score::{EventContent, EventId, EventScope, Score, ScoreEvent, TemporalPlacement};
use umt::temperament::{AmbientElem, AmbientLattice, TemperamentMap};
use umt::time::{BeatDuration, BeatTime, TickGrid};
use umt::{Basis, RoundingConvention, Z};

const TICKS_PER_BEAT: u32 = 96;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---- The exact source ------------------------------------------------
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;
    let tuning = RegularTuning::equal_divisions(&steps, 12)?;
    let origin = PitchOrigin::new("umt:origin:c4");

    // A quintuplet: five notes in two beats, so each is 2/5 of a beat. At 96
    // ticks to the beat that is 38.4 ticks, which no device can represent.
    let soprano = EventScope::VoiceLocal(VoiceId::new("soprano"));
    let mut builder = Score::builder();
    for (index, step) in [0i64, 4, 7, 11, 12].into_iter().enumerate() {
        let onset = BeatTime::ratio(2 * index as i64, 5)?;
        builder = builder.event(ScoreEvent::new(
            EventId::new(&format!("n{index}")),
            soprano.clone(),
            TemporalPlacement::fixed(onset, BeatDuration::ratio(2, 5)?),
            EventContent::Note {
                pitch: PitchPoint::new(origin.clone(), steps.element([step])?),
            },
        )?)?;
    }
    let score = builder.build()?;

    println!("== The exact score ==");
    for event in score.events() {
        let span = event.span().expect("every event here is measured");
        println!(
            "  {:>3}: onset {}, duration {} beats",
            event.id(),
            span.start().get(),
            span.duration().get()
        );
    }

    // ---- Provenance for the compilation ---------------------------------
    let mut arena = ProvenanceArena::new();
    let tuning_step = ProvenanceId::new("umt:prov:tuning");
    arena.insert(
        tuning_step.clone(),
        ProvenanceRecord::new(AlgorithmId::new("umt:algo:equal-divisions"), "0.1.0")
            .with_parameter("divisions", CanonicalValue::Integer(Z::from(12))),
    )?;
    let compile_step = ProvenanceId::new("umt:prov:compile");
    arena.insert(
        compile_step.clone(),
        ProvenanceRecord::new(AlgorithmId::new("umt:algo:performance-compile"), "0.1.0")
            .with_parent(tuning_step)
            .with_parameter(
                "ticks_per_beat",
                CanonicalValue::Integer(Z::from(TICKS_PER_BEAT)),
            )
            .with_rounding(RoundingConvention::NearestHalfAwayFromZero),
    )?;
    println!(
        "\n  provenance: {} records, compile step derives from {:?}",
        arena.len(),
        arena
            .ancestors(&compile_step)?
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );

    // ---- Compile ---------------------------------------------------------
    let grid = TickGrid::new(TICKS_PER_BEAT)?;
    let mut residuals = ResidualSet::new();
    let mut plan = PerformancePlan::builder(TICKS_PER_BEAT).reference_millicents(0);

    println!("\n== Compiling to {grid} ==");
    for event in score.events() {
        let span = event.span().expect("measured");
        let pitch: &PitchPoint<AmbientElem> = event.pitch().expect("pitched");

        // Time: exact rational onset to an integer tick, with the residual
        // kept rather than dropped.
        let onset = grid.quantize(span.start(), RoundingConvention::NearestHalfAwayFromZero);
        let duration = grid.quantize_duration(
            &span.duration(),
            RoundingConvention::NearestHalfAwayFromZero,
        );
        for residual in [&onset.residual, &duration.residual] {
            if !residual.is_zero() {
                residuals.push(
                    ResidualRecord::new(Residual::Grid {
                        deviation: residual.clone(),
                    })
                    .with_provenance(compile_step.clone())
                    .with_note(event.id().as_str()),
                );
            }
        }

        // Pitch: an L2 class through a declared tuning to bounded millicents.
        let reference = PitchPoint::new(origin.clone(), steps.zero());
        let interval = reference.interval_to(pitch)?;
        let cents = Cents::from(tuning.size(&interval)?).get();
        let millicents = (cents * 100.0).round() as i32;
        let encoded_cents = f64::from(millicents) / 100.0;
        if (encoded_cents - cents).abs() > 0.0 {
            residuals.push(
                ResidualRecord::new(Residual::device_control(cents, encoded_cents, "cents")?)
                    .with_provenance(compile_step.clone())
                    .with_note(event.id().as_str()),
            );
        }

        let onset_tick: u32 = onset.value.to_string().parse()?;
        let duration_ticks: u32 = duration.value.to_string().parse()?;
        println!(
            "  {:>3}: ticks {onset_tick}..{}, {millicents} millicents",
            event.id(),
            onset_tick + duration_ticks
        );

        plan = plan.event(PlannedEvent {
            onset_tick,
            duration_ticks,
            millicents: Some(millicents),
            voice: 0,
        })?;
    }

    println!(
        "\n  onsets and durations were quantized independently, so consecutive\n  \
         notes need not abut: n1 ends at 76 and n2 begins at 77. That is the\n  \
         drift of section 5.7.4, and the endpoint-preserving allocator of\n  \
         `TickGrid::allocate_preserving_endpoint` is what avoids it."
    );

    // The score notated a just major third for the second note. Twelve-EDO
    // cannot distinguish 5/4 from 81/64, and the difference is exact
    // structural information rather than a rounding error (section 7.3).
    let just_third = basis.monzo([-2, 0, 1])?;
    let pythagorean_third = basis.monzo([-6, 4, 0])?;
    let discarded = pythagorean_third.checked_sub(&just_third)?;
    assert!(map.kills(&discarded)?, "12-EDO tempers this out");
    let comma = map
        .kernel()
        .coordinates(&discarded)?
        .expect("the difference is in the kernel");
    residuals.push(
        ResidualRecord::new(Residual::Structural { comma })
            .with_provenance(compile_step.clone())
            .with_note("5/4 and 81/64 are one class in 12-EDO"),
    );

    let plan = plan
        .residuals(residuals)
        .provenance(compile_step.clone())
        .build()?;

    // ---- What the crossing cost -----------------------------------------
    println!("\n== What the crossing cost (section 7.9) ==");
    if plan.residuals().is_empty() {
        println!("  nothing: every value was exactly representable");
    }
    for kind in plan.residuals().kinds() {
        let count = plan.residuals().of_kind(kind).count();
        println!("  {kind:?}: {count} residual(s)");
    }
    println!(
        "  every residual attributed: {}",
        plan.residuals().is_fully_attributed()
    );
    let total = plan.residuals().total_of_kind(ResidualKind::Grid)?;
    match total {
        Some(Residual::Grid { deviation }) => {
            println!("  total grid residual: {} beats, exactly", deviation.get());
        }
        _ => println!("  no grid residual"),
    }

    // A realization record says where the object entered and what it lost.
    let record = RealizationRecord::new(Layer::L1Exact, Layer::L4Device)?
        .omitting("the exact rhythm tree")
        .with_round_trip(RoundTripBasis::SourceRetained)
        .with_residuals(plan.residuals().clone())
        .with_provenance(compile_step);
    println!(
        "\n  realization: {} -> {}, bypassing {:?}",
        record.entry(),
        record.exit(),
        record
            .bypassed()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );
    println!("  round trip:  {:?}", record.round_trip());
    println!("  lossless:    {}", record.claims_lossless());
    println!("  attributable: {}", record.is_attributable());

    // ---- What the plan guarantees ---------------------------------------
    println!("\n== The realtime boundary (prompt section 38) ==");
    let contract = plan.realtime_contract();
    println!("  contract satisfied:      {}", contract.is_satisfied());
    println!(
        "  no allocation on read:   {}",
        contract.no_allocation_on_read
    );
    println!(
        "  no arbitrary precision:  {}",
        contract.no_arbitrary_precision
    );
    println!(
        "  ranges validated:        {}",
        contract.bounded_ranges_validated
    );
    println!(
        "  device mapping resolved: {}",
        contract.device_mapping_resolved
    );

    let window = plan.events_in(0, TICKS_PER_BEAT);
    println!(
        "\n  the first beat holds {} event(s), read as a borrowed slice",
        window.len()
    );
    println!("  the plan ends at tick {}", plan.end_tick());

    // Device limits really are enforced.
    let refused = PerformancePlan::builder(TICKS_PER_BEAT).event(PlannedEvent {
        onset_tick: 0,
        duration_ticks: 1,
        millicents: Some(i32::MAX),
        voice: 0,
    });
    println!(
        "\n  a pitch outside the validated range is refused at compile time: {}",
        refused.is_err()
    );

    Ok(())
}
