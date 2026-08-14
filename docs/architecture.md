# Architecture

One crate, `umt`, organized by module rather than by workspace member. The
module boundaries follow the UMT-3.2 layer model, not an arbitrary layering of
convenience.

## Layer map

| UMT layer | Content | Modules |
|---|---|---|
| L0 notation | spelled symbols, ties, tuplet brackets | not implemented |
| L1 exact structure | monzos, exact ratios, rational durations | `algebra`, `proportion` |
| L2 structural quotient | tempered classes, image lattices, meter | `temperament` |
| L3 metric realization | log-frequency, tuning curves, tempo maps | `*_f64` accessors; `realization` (identifier only) |
| L4 device realization | ticks, MIDI, control words | not implemented |

## Present modules

```text
src/
  lib.rs                  crate root, no_std attribute, re-exports
  error.rs                typed error enums
  context.rs              TheoryContext, wire-form references
  algebra/
    quotient.rs           QuotientGroup: Z^n / L by structure theorem
    integer.rs            Z, exact round(n log2 p/q), L3 log helpers
    rational.rs           Q
    rounding.rs           RoundingConvention
    matrix.rs             IntMatrix
    normal_form.rs        SmithNormalForm, canonical HermiteNormalForm
    lattice.rs            Sublattice: membership, coordinates, saturation
  proportion/
    basis.rs              BasisId, GeneratorId, Basis, RawBasis, BasisBuilder
    monzo.rs              Monzo with basis-checked arithmetic
    valuation.rs          PositiveQ, PositiveFinite, RealValuation
    complexity.rs         declared complexity profiles, weighted l1, Tenney
  pitch/
    units.rs              FrequencyHz, LogFrequency, Octaves, Cents
    point.rs              PitchOrigin, PitchPoint torsor over any interval type
    tuning.rs             RegularTuning, PitchRealization, PitchRealizer
  temperament/
    map.rs                RawTemperamentMap, TemperamentMap, exact preimages
    image.rs              LatticeId, AmbientLattice/Elem, ImageLattice/Elem
    kernel.rs             KernelLattice/Elem, saturation policy and report
    splitting.rs          HomomorphicSplit, LinearSplit
    representative.rs     RepresentativePolicy, LiftDecision, StructuralLens
    unit.rs               UnitEquivalence on the ambient or reachable group
    minimum_complexity.rs provably bounded minimum-complexity lift search
    edo.rs                PatentVal, Exactness
  realization/
    optimization.rs       OptimizationOutcome, ApproximationGuarantee
    provenance.rs         ProvenanceId
  io/
    text.rs               canonical exact-value text codec
    version.rs            UmtSchemaVersion, compatibility rule
    serde_exact.rs        serde adapters (feature `serde`)
```

`PatentVal` wraps a `TemperamentMap` rather than reimplementing it. Its scalar
accessors (`image_generator`, `image_coordinate`, `embed_image`) are views of
the general machinery that are meaningful only because the ambient rank is 1;
`PatentVal::map` reaches the general object.

## Planned modules

Following the staging of the implementation prompt, in order:

1. The rest of `pitch/`: trajectories (section 4.7, F20), chords with voice
   identity (4.3), and voice leading as a span with declared costs (4.4, F8).
2. `time/` - exact durations, rhythm trees, meter and grouping, tempo maps,
   quantizers, temporal constraint networks. The largest single stage, and the
   one that closes the most fixtures: F10 to F18, F25, F27, F32.
3. `score/` - scoped events, temporal placement, ties, event relations (F9,
   F23, F24).
4. The rest of `realization/` - residual taxonomy, realization records, the
   compiled performance-plan boundary.
5. The native container in `io/` (F29), then external adapters (F19, F21).

## Why the normal forms are what they are

Hermite is canonical here, Smith is not. That asymmetry is deliberate:

- A lattice's *identity* must be canonical, so `Sublattice`, `ImageLattice`,
  and `KernelLattice` all store a canonical Hermite basis. Two sublattices are
  then equal as values exactly when they are equal as subgroups, and
  serialized output is reproducible.
- Smith normal form is used only for things that do not depend on the choice
  of transforms: invariant factors, rank, a kernel basis (which is immediately
  canonicalized), and a saturation basis (likewise). Nothing downstream
  observes the particular `U` and `V`.

Derived structure is computed eagerly in `TemperamentMap::new`, not cached
lazily. That keeps the type free of interior mutability, keeps it `Send +
Sync`, avoids a `std`-only `OnceLock`, and guarantees equality can never depend
on cache state.

## Why `Arc<Basis>` inside `Monzo`

Basis identity is semantic (UMT-3.2 section 1.1, prompt section 7), so a monzo
must know which basis it is over. The handle is shared and immutable, so it is
cheap to clone and `Send + Sync`. Compatibility is checked as a pointer
comparison first and a full structural comparison second, which keeps the
guarantee sound for handles rebuilt from serialized data.

The registry that maps a serialized `BasisId` back to a shared handle is the
`TheoryContext` of prompt section 8; it arrives with the general temperament
map, and until then serialization is limited to self-contained definition
objects.

## Checks

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --no-default-features --target x86_64-unknown-none
cargo clippy --no-default-features --target x86_64-unknown-none -- -D warnings
```

The bare-metal build is not decoration: it is the only check that catches a
dependency feature silently pulling `std` back in through feature unification.
A host build cannot detect that.

## Realtime

Not yet documented, because nothing here makes a realtime claim. The semantic
core allocates and uses arbitrary-precision arithmetic by design. `docs/realtime.md`
arrives with the compiled performance-plan boundary; until then, no type in
this crate is realtime-safe and none claims to be.
