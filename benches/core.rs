//! Benchmarks for the operations prompt section 50 names.
//!
//! Run with `cargo bench`. No benchmarking framework is used: section 50 asks
//! for numbers on a specific list of operations, and a plain timing harness
//! gives those without adding a dependency to a crate whose whole dependency
//! policy is "as few as possible, all pure Rust".
//!
//! These are *indicative*, not statistical. They exist to catch a change that
//! makes something an order of magnitude slower, not to compare compilers.
//! Section 50's own guidance applies: "Do not micro-optimize before the
//! invariants are correct."

use std::hint::black_box;
use std::time::Instant;

use umt::algebra::{Q, RoundingConvention, Z};
use umt::pitch::{PitchOrigin, PitchPoint, VoiceId};
use umt::realization::{PerformancePlan, PlannedEvent};
use umt::score::{EventContent, EventId, EventScope, Score, ScoreEvent, TemporalPlacement};
use umt::temperament::{AmbientElem, AmbientLattice};
use umt::time::{AllocationPolicy, BeatDuration, BeatSpan, BeatTime, RhythmTree, TickGrid};
use umt::{Basis, IntMatrix, TemperamentMap};

fn bench(name: &str, iterations: u32, mut body: impl FnMut()) {
    // One warm-up pass, so the first measured run is not paying for lazy
    // initialisation somewhere.
    body();
    let start = Instant::now();
    for _ in 0..iterations {
        body();
    }
    let elapsed = start.elapsed();
    let each = elapsed.as_secs_f64() / f64::from(iterations);
    let (value, unit) = if each < 1e-6 {
        (each * 1e9, "ns")
    } else if each < 1e-3 {
        (each * 1e6, "us")
    } else {
        (each * 1e3, "ms")
    };
    println!("  {name:<44} {value:>10.2} {unit}  x{iterations}");
}

fn main() {
    println!("umt benchmarks (indicative; see prompt section 50)\n");

    let five_limit = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
    let seven_limit = Basis::primes("umt:prime:2.3.5.7", &[2, 3, 5, 7]).unwrap();
    let steps = AmbientLattice::new("umt:edo:12", 1);

    println!("monzo arithmetic");
    let a = five_limit.monzo([-4, 4, -1]).unwrap();
    let b = five_limit.monzo([-19, 12, 0]).unwrap();
    bench("5-limit monzo addition", 200_000, || {
        black_box(a.checked_add(&b).unwrap());
    });
    let c = seven_limit.monzo([-2, 0, 0, 1]).unwrap();
    let d = seven_limit.monzo([1, -1, 0, 0]).unwrap();
    bench("7-limit monzo addition", 200_000, || {
        black_box(c.checked_add(&d).unwrap());
    });
    bench("5-limit exact ratio", 20_000, || {
        black_box(a.exact_ratio().unwrap());
    });

    println!("\nnormal forms and temperament construction");
    let matrix = IntMatrix::from_rows([[12i64, 19, 28], [7, 11, 16]]).unwrap();
    bench("Smith normal form, 2x3", 20_000, || {
        black_box(umt::algebra::normal_form::SmithNormalForm::of(&matrix));
    });
    let bigger = IntMatrix::from_rows([
        [12i64, 19, 28, 34],
        [7, 11, 16, 20],
        [5, 8, 12, 14],
        [3, 5, 7, 9],
    ])
    .unwrap();
    bench("Smith normal form, 4x4", 5_000, || {
        black_box(umt::algebra::normal_form::SmithNormalForm::of(&bigger));
    });
    bench("TemperamentMap::from_rows, rank 1", 20_000, || {
        black_box(TemperamentMap::from_rows(&five_limit, &steps, [[12i64, 19, 28]]).unwrap());
    });
    let rank_two = AmbientLattice::new("umt:meantone", 2);
    bench("TemperamentMap::from_rows, rank 2", 10_000, || {
        black_box(
            TemperamentMap::from_rows(&five_limit, &rank_two, [[1i64, 0, -4], [0, 1, 4]]).unwrap(),
        );
    });

    println!("\nimage and kernel queries");
    let map = TemperamentMap::from_rows(&five_limit, &steps, [[12i64, 19, 28]]).unwrap();
    bench("image membership", 200_000, || {
        black_box(
            map.image()
                .contains(&steps.element([7i64]).unwrap())
                .unwrap(),
        );
    });
    bench("apply to image", 100_000, || {
        black_box(map.apply_to_image(&a).unwrap());
    });
    bench("kernel membership", 100_000, || {
        black_box(map.kernel().contains(&a).unwrap());
    });
    bench("exact preimage", 20_000, || {
        black_box(map.preimage(&map.apply(&a).unwrap()).unwrap());
    });

    println!("\nrhythm-tree flattening");
    let beat = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(4, 1).unwrap()).unwrap();
    let flat = RhythmTree::equal_division(16).unwrap();
    bench("flatten, 16 equal leaves", 50_000, || {
        black_box(flat.flatten(&beat).unwrap());
    });
    let nested = RhythmTree::division([
        RhythmTree::equal_division(5).unwrap(),
        RhythmTree::division([
            RhythmTree::equal_division(3).unwrap(),
            RhythmTree::equal_division(7).unwrap(),
        ])
        .unwrap(),
        RhythmTree::equal_division(4).unwrap(),
    ])
    .unwrap();
    bench("flatten, nested tuplets (19 leaves)", 50_000, || {
        black_box(nested.flatten(&beat).unwrap());
    });

    println!("\nquantization");
    let grid = TickGrid::new(960).unwrap();
    let at = BeatTime::ratio(7, 15).unwrap();
    bench("quantize one position", 200_000, || {
        black_box(grid.quantize(&at, RoundingConvention::NearestHalfAwayFromZero));
    });
    let weights: Vec<Q> = (1..=16).map(|w| Q::from(Z::from(w))).collect();
    let policy = AllocationPolicy::default();
    bench("endpoint-preserving allocation, 16", 20_000, || {
        black_box(
            grid.allocate_preserving_endpoint(&weights, &Z::from(3840), &policy)
                .unwrap(),
        );
    });
    bench("hierarchical quantization, 19 leaves", 10_000, || {
        black_box(grid.quantize_tree(&nested, &beat, &policy).unwrap());
    });

    println!("\nscore traversal and plan compilation");
    let origin = PitchOrigin::new("umt:origin:c4");
    let voice = EventScope::VoiceLocal(VoiceId::new("soprano"));
    let mut builder = Score::builder();
    for index in 0..2_000i64 {
        let pitch: PitchPoint<AmbientElem> =
            PitchPoint::new(origin.clone(), steps.element([index % 24]).unwrap());
        builder = builder
            .event(
                ScoreEvent::new(
                    EventId::new(&format!("n{index}")),
                    voice.clone(),
                    TemporalPlacement::fixed(
                        BeatTime::ratio(index, 4).unwrap(),
                        BeatDuration::ratio(1, 4).unwrap(),
                    ),
                    EventContent::Note { pitch },
                )
                .unwrap(),
            )
            .unwrap();
    }
    let score = builder.build().unwrap();
    bench("traverse 2000 events", 2_000, || {
        let mut total = 0usize;
        for event in score.events() {
            if event.content().is_sounding() {
                total += 1;
            }
        }
        black_box(total);
    });
    bench("sounding gestures, 2000 events", 500, || {
        black_box(score.sounding_gestures().unwrap());
    });

    bench("compile a 2000-event plan", 500, || {
        let mut plan = PerformancePlan::builder(960);
        for (index, event) in score.events().enumerate() {
            let span = event.span().unwrap();
            let onset = grid.quantize(span.start(), RoundingConvention::NearestHalfAwayFromZero);
            let onset_tick: u32 = onset.value.to_string().parse().unwrap();
            plan = plan
                .event(PlannedEvent {
                    onset_tick,
                    duration_ticks: 240,
                    millicents: Some((index % 24) as i32 * 10_000),
                    voice: 0,
                })
                .unwrap();
        }
        black_box(plan.build().unwrap());
    });

    println!("\nDone. These numbers are indicative; correctness comes first.");
}
