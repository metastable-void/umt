# umt

An implementation of **UMT-3.2** (Unified Music Theory, Third Design, revision 3.2):
exact symbolic proportion, structural quotient, metric realization, and device
approximation kept as *different* semantic layers, with every loss and policy
choice made explicit.

## Status

**Every part of UMT-3.2 is implemented, and all thirty-five of its mandatory
adversarial fixtures pass.**

Two things are absent on purpose rather than pending: L0 pitch spelling
(section 4.5), which prompt section 55 defers out of a first release, and
external adapters other than Scala `.scl` (sections 8.4 to 8.7), which prompt
section 40 puts behind native serialization. `docs/conformance.md` states
precisely what the fixture results do and do not license.

Implemented: the proportion lattice; exact integer matrices with Smith and
canonical Hermite normal forms; free lattices and quotients; regular
temperament mappings with their image and kernel lattices; comma-subgroup
saturation validation; equal divisions; homomorphic splittings; representative
policies including a provably bounded minimum-complexity search; the structural
quotient lens; unit equivalence; declared complexity profiles; first-class
optimization outcomes; pitch units, point torsors, regular tuning, and
contextual realization; chords with voice identity; voice leading as a span
with declared costs and honestly-scoped distance claims; continuous pitch
trajectories with derived sampling bounds; exact structural time with rhythm
trees, meter, and grouping; the rate/duration orientation rule; grid
quantization that returns its residuals; tempo maps in the homeomorphism
profile; three separate temporal-constraint solver profiles; an event-indexed
score with scoped events, ties that are relations rather than merges, and
transformations that claim compositionality only when they have it; an
immutable theory context with reference-based serialization; a typed residual
taxonomy with structured provenance in an arena; a compiled, bounded
performance plan whose realtime contract is a checkable value; directly
measured L3 scales that need no lattice explanation; modular generated sets
with the three-gap hypotheses recorded and Euclidean rhythms whose evenness is
verified; the native document container; and a Scala `.scl` adapter that keeps
each entry in its own layer.

All six of the implementation prompt's mandatory examples run, plus two
supplementary ones. See `docs/architecture.md` for the module map and
`docs/conformance.md` for the fixture matrix and law coverage.

## Example

The 5-limit patent val for 6-EDO reaches only half of the ambient step lattice.
Ambient `Gamma = Z` and image `H = 2Z` stay distinct, so an odd step is not
silently detempered (UMT-3.2 section 1.6.1, fixture F4):

```rust
use umt::{Basis, PatentVal, RoundingConvention, Z};

let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
let val = PatentVal::new(&basis, 6, RoundingConvention::NearestHalfAwayFromZero)?;

assert_eq!(val.to_string(), "<6 10 14]");
assert_eq!(val.image_generator(), Z::from(2));
assert!(!val.is_surjective());

// One ambient step exists, but nothing maps onto it.
assert!(!val.contains_ambient(&Z::from(1)));
assert!(val.image_coordinate(&Z::from(1)).is_err());

// The syntonic comma vanishes in 12-EDO, exactly.
let twelve = PatentVal::new(&basis, 12, RoundingConvention::NearestHalfAwayFromZero)?;
let syntonic = basis.monzo([-4, 4, -1])?;
assert_eq!(syntonic.exact_ratio()?.to_string(), "81/80");
assert_eq!(twelve.apply(&syntonic)?, Z::from(0));

// The tempered-out commas are a lattice, not a list: rank 2 here, and
// saturated as a theorem rather than as a validation step.
let kernel = twelve.map().kernel();
assert_eq!(kernel.rank(), 2);
assert!(kernel.is_saturated());
assert!(kernel.contains(&syntonic)?);
# Ok::<(), Box<dyn core::error::Error>>(())
```

The same sounding class, lifted differently in different contexts, with the
difference reported as an exact comma rather than a floating-point deviation:

```rust
use umt::temperament::{
    AmbientLattice, CanonicalLiftPolicy, OffsetPolicy, RepresentativePolicy, TemperamentMap,
};
use umt::Basis;

let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
let steps = AmbientLattice::new("umt:edo:12", 1);
let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;

let syntonic = map.kernel().coordinates(&basis.monzo([-4, 4, -1])?)?.unwrap();
let policy = OffsetPolicy::new(
    CanonicalLiftPolicy::new(map.clone()),
    move |_class: &_, wide: &bool| if *wide { Some(syntonic.clone()) } else { None },
);

let class = map.apply_to_image(&basis.monzo([-1, 1, 0])?)?;
assert_eq!(policy.choose(&class, &false)?.lift.exact_ratio()?.to_string(), "3/2");
assert_eq!(policy.choose(&class, &true)?.lift.exact_ratio()?.to_string(), "243/160");

// An adaptive policy is not additive, and never claims to be.
assert!(!policy.claims_homomorphic());
# Ok::<(), Box<dyn core::error::Error>>(())
```

A regular tuning evaluates intervals. Realizing an absolute pitch needs
reference data on top of it, and the type system says so (fixture F31):

```rust
use umt::pitch::{Cents, FrequencyHz, PitchOrigin, PitchPoint, PitchRealization, RegularTuning};
use umt::temperament::AmbientLattice;

let steps = AmbientLattice::new("umt:edo:12", 1);
let tuning = RegularTuning::equal_divisions(&steps, 12)?;

// Interval sizes come from the tuning alone.
let fifth = steps.element([7i64])?;
assert!((Cents::from(tuning.size(&fifth)?).get() - 700.0).abs() < 1e-9);

// Pitches need a structural reference and a realized one.
let origin = PitchOrigin::new("umt:origin:a4");
let reference = PitchPoint::new(origin, steps.zero());
let realization =
    PitchRealization::new(tuning, reference.clone(), FrequencyHz::new(440.0)?.log_frequency());

let e5 = realization.realize_point(&reference.translate(&fifth)?)?;
assert!((e5.frequency()?.get() - 659.2551138).abs() < 1e-6);
# Ok::<(), Box<dyn core::error::Error>>(())
```

A chord is a function from voice identities to points, so a doubling is a
multiplicity rather than a duplicate. Classical balanced transport cannot
compare two chords of different sizes, and rather than renormalizing behind
your back, it says so (fixture F8):

```rust
use umt::pitch::{
    Chord, ChordDistance, LogPitchDistance, MassProfile, PitchOrigin, PitchPoint, RegularTuning,
    TransportProfile, VoiceId,
};
use umt::temperament::AmbientLattice;

let steps = AmbientLattice::new("umt:edo:12", 1);
let middle_c = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), steps.zero());

let single = Chord::from_voices([(VoiceId::new("soprano"), middle_c.clone())])?;
let doubled = Chord::from_voices([
    (VoiceId::new("soprano"), middle_c.clone()),
    (VoiceId::new("alto"), middle_c.clone()),
])?;
assert_eq!(doubled.forget_voice_labels().multiplicity(&middle_c), 2);

let ground = LogPitchDistance::new(RegularTuning::equal_divisions(&steps, 12)?);
let balanced = ChordDistance::new(
    ground.clone(),
    2.0,
    TransportProfile::Balanced { mass: MassProfile::PerVoice },
)?;
assert!(balanced.distance(&single, &doubled).is_err());

// An edit profile handles the unequal case without discarding multiplicity,
// and charges exactly the birth cost it was configured with.
let edit = ChordDistance::new(ground, 1.0, TransportProfile::Edit { boundary: 0.75 })?;
assert!((edit.distance(&single, &doubled)? - 0.75).abs() < 1e-12);
# Ok::<(), Box<dyn core::error::Error>>(())
```

Structural time is exact, so a quintuplet inside a triplet closes exactly.
Quantizing it to a device grid returns the residual alongside the value, and
the method that preserves the parent endpoint is a different method from the
one that does not (fixtures F12 and F13):

```rust
use umt::algebra::RoundingConvention;
use umt::time::{AllocationPolicy, BeatSpan, BeatTime, Beats, RhythmTree, TickGrid};
use umt::{Q, Z};

// Five equal notes in one beat: one fifteenth of a beat each after a further
// triplet division, and no binary float represents that.
let quintuplet = RhythmTree::equal_division(5)?;
let beat = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(1, 1)?)?;
let leaves = quintuplet.flatten(&beat)?;
assert_eq!(leaves[0].span().duration(), Beats::ratio(1, 5)?);
assert_eq!(leaves[4].span().end(), beat.end(), "the partition closes exactly");

// On a 96-tick grid each note is 19.2 ticks. Rounding each one independently
// loses a tick; rounding the boundaries does not.
let grid = TickGrid::new(96)?;
let weights = vec![Q::from(Z::from(1)); 5];

let naive = grid.allocate_locally(&weights, &Z::from(96), RoundingConvention::Floor)?;
assert_eq!(naive.total_ticks(), Z::from(95));
assert!(!naive.endpoint_preserved());

let exact = grid
    .allocate_preserving_endpoint(&weights, &Z::from(96), &AllocationPolicy::default())?
    .into_allocation()
    .expect("feasible");
assert_eq!(exact.child_ticks(), [19, 19, 20, 19, 19].map(Z::from));
assert!(exact.endpoint_preserved());
# Ok::<(), Box<dyn core::error::Error>>(())
```

A tie is a relation between two noteheads, not a merge. Both survive, and the
single sustained gesture is a derived view that keeps their identities
(fixture F9):

```rust
use umt::pitch::{PitchOrigin, PitchPoint, VoiceId};
use umt::score::{
    EventContent, EventId, EventScope, Score, ScoreEvent, TemporalPlacement, Tie,
};
use umt::temperament::AmbientLattice;
use umt::time::{BeatDuration, BeatTime, Beats};

let steps = AmbientLattice::new("umt:edo:12", 1);
let soprano = EventScope::VoiceLocal(VoiceId::new("soprano"));
let g4 = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), steps.element([7i64])?);

let notehead = |id: &str, onset: i64| {
    ScoreEvent::new(
        EventId::new(id),
        soprano.clone(),
        TemporalPlacement::fixed(BeatTime::ratio(onset, 1)?, BeatDuration::ratio(2, 1)?),
        EventContent::Note { pitch: g4.clone() },
    )
};

// Two beats before the barline, two after, tied across it.
let score = Score::builder()
    .event(notehead("n1", 2)?)?
    .event(notehead("n2", 4)?)?
    .tie(Tie::new(EventId::new("n1"), EventId::new("n2")))?
    .build()?;

// Both noteheads survive at L0.
assert_eq!(score.len(), 2);
assert_eq!(score.ties().len(), 1);

// And the realization view is one four-beat gesture that remembers both.
let gestures = score.sounding_gestures()?;
assert_eq!(gestures.len(), 1);
assert_eq!(gestures[0].sources(), &[EventId::new("n1"), EventId::new("n2")]);
assert_eq!(gestures[0].span().duration(), Beats::ratio(4, 1)?);
# Ok::<(), Box<dyn core::error::Error>>(())
```

A generated scale stores its period and generator as *designated data*, because
a rank-2 temperament does not say which is which. The MOS predicate is
operational and its hypotheses are recorded (fixture F35):

```rust
use umt::generated::{
    GeneratedSet, GeneratorRatio, MosProfile, quarter_comma_meantone_generator,
};
use umt::pitch::Cents;

let scale = GeneratedSet::from_cents(
    Cents::new(1200.0)?,
    quarter_comma_meantone_generator(),
    3,
    // Whether g/p is rational cannot be decided from two doubles, so it is
    // declared rather than guessed.
    GeneratorRatio::Irrational,
)?;

// Three points, two gap sizes: cardinality 3 is MOS.
let report = scale.gap_report();
assert_eq!(report.distinct(), 3);
assert_eq!(report.distinct_sizes().len(), 2);
assert!(scale.mos_verdict(MosProfile::TwoStepSizes).is_mos());
assert!(report.satisfies_three_gap_bound());

// The MOS cardinalities are computed, not quoted.
assert_eq!(
    scale.mos_cardinalities(31, MosProfile::TwoStepSizes)?,
    [2, 3, 5, 7, 12, 19, 31]
);

// And the intervening cardinalities are still generated scales - they just
// have three gap sizes rather than two.
let four = scale.at_cardinality(4)?;
assert_eq!(four.sorted_distinct_points().len(), 4);
assert!(!four.mos_verdict(MosProfile::TwoStepSizes).is_mos());
# Ok::<(), Box<dyn core::error::Error>>(())
```

A Scala file can mix exact ratios and cents values, so it is not uniformly L3.
The importer keeps each entry in the layer it was written in (fixture F21):

```rust,ignore
use umt::io::scala::ScalaScale;

let scale = ScalaScale::parse("\
! mixed.scl
!
A scale with both kinds of entry
 3
!
 9/8
 386.313714
 2/1
")?;

assert!(scale.is_mixed());

// The ratios stayed exact; the cents value did not acquire exactness.
assert!(scale.entries()[0].is_exact());
assert!(!scale.entries()[1].is_exact());
assert_eq!(scale.entries()[1].exact_ratio(), None);

// Flattening to a uniform L3 scale is available, and reports its cost.
let (_, lost) = scale.to_empirical_scale(umt::pitch::ScaleId::new("umt:scale:mixed"))?;
assert_eq!(lost.len(), 2, "one notation residual per flattened exact entry");
# Ok::<(), Box<dyn core::error::Error>>(())
```

Requires the `scala` feature; the verified form of this example is the module
documentation of `umt::io::scala`.

Monzos over different bases are different objects, and the type system says so:

```rust
use umt::Basis;

let five_limit = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
let seven_limit = Basis::primes("umt:prime:2.3.7", &[2, 3, 7])?;

let a = five_limit.monzo([1, 0, 0])?;
let b = seven_limit.monzo([1, 0, 0])?;

assert_ne!(a, b);
assert!(a.checked_add(&b).is_err());
# Ok::<(), Box<dyn core::error::Error>>(())
```

## Design principles

- **Semantic types stay distinct.** An exact structural map, a representative
  policy, a metric realization, and a device quantizer are four different
  things (UMT-3.2 section 0.7). None of them is substitutable for another.
- **Exactness above L3.** Monzo coordinates, mapping entries, kernel data, and
  structural durations are arbitrary-precision integers and rationals. No
  identity, equality, or quotient-membership decision at L0 to L2 depends on
  binary floating point (section 0.6.1).
- **Ambient is not image.** `Gamma` and `H = im(V)` are represented separately,
  because a mapping need not be surjective (section 1.4).
- **Basis identity is semantic.** Equal-length exponent vectors over different
  bases are never silently combined.
- **No hidden loss.** Operations that forget information say so in their type,
  their result, or their name.

## Exactness policy

`Z` and `Q` are arbitrary-precision. Bounded integer and floating
representations appear only after explicit validation, at the device and
performance boundary. Real-valued (L3) results are named `*_f64` and carry
their uncertainty and provenance where the specification requires it.

Transcendental functions come from [`libm`] in every build, including `std`
builds, so an L3 result does not vary with the host math library or with which
Cargo features are enabled.

## Features

| Feature | Default | What it adds |
|---|---|---|
| `std` | yes | host integrations only; it must never change a computed value |
| `serde` | no | the native document container and exact-text serialization |
| `scala` | no | the Scala `.scl` adapter of UMT-3.2 section 8.2 |

Neither optional feature adds a dependency the core does not already have.

## `no_std`

The semantic core is `no_std + alloc`. The default `std` feature is purely
additive and must never change a computed value.

```text
cargo build --no-default-features --target x86_64-unknown-none
```

`no_std` is not a realtime claim. Arbitrary-precision arithmetic allocates, so
the semantic core is not realtime-safe and does not try to be. The boundary is
`realization::PerformancePlan`, whose guarantees are a `RealtimeContract`
value rather than a marker trait; `docs/realtime.md` states what it does and
does not cover.

## Relationship to UMT-3.2

The specification lives in `UMT-3.2.md` at the repository root and is
authoritative. Where this implementation makes a choice the specification
leaves open, that choice is recorded in `docs/implementation-profile.md`. Where
the specification appears to be wrong or self-inconsistent, the finding is
recorded in `docs/spec-issues.md` with a counterexample and the conservative
behaviour implemented in the meantime - never silently coded around.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- Mozilla Public License, Version 2.0 ([LICENSE-MPL](LICENSE-MPL))

at your option.

[`libm`]: https://docs.rs/libm
