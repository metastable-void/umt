# umt

An implementation of **UMT-3.2** (Unified Music Theory, Third Design, revision 3.2):
exact symbolic proportion, structural quotient, metric realization, and device
approximation kept as *different* semantic layers, with every loss and policy
choice made explicit.

## Status

The exact structural core, the pitch layer, and the time layer are built. What
remains is the score layer, the device boundary, the native container, and the
external adapters.

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
profile; three separate temporal-constraint solver profiles; and an immutable
theory context with reference-based serialization.

**Twenty-seven of the thirty-five UMT-3.2 fixtures pass.** Every remaining one
depends on a layer that is not built yet - score, device, the native container,
external adapters, or generated sets - except F30, which lints the
specification source rather than the library. See `docs/architecture.md` for
the staging plan and `docs/conformance.md` for the fixture matrix.

This crate does not yet claim UMT-3.2 conformance. Conformance is claimed only
when the applicable mandatory fixture suite passes.

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

## `no_std`

The semantic core is `no_std + alloc`. The default `std` feature is purely
additive and must never change a computed value.

```text
cargo build --no-default-features --target x86_64-unknown-none
```

`no_std` is not a realtime claim. Arbitrary-precision arithmetic allocates; the
realtime story is the bounded, validated performance plan, which is not built
yet.

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
