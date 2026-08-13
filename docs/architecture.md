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
  algebra/
    integer.rs            Z, exact round(n log2 p/q), L3 log helpers
    rational.rs           Q
    rounding.rs           RoundingConvention
  proportion/
    basis.rs              BasisId, GeneratorId, Basis, RawBasis, BasisBuilder
    monzo.rs              Monzo with basis-checked arithmetic
    valuation.rs          PositiveQ, PositiveFinite, RealValuation
  temperament/
    edo.rs                PatentVal, Exactness
  realization/
    provenance.rs         ProvenanceId
  io/
    text.rs               canonical exact-value text codec
    serde_exact.rs        serde adapters (feature `serde`)
```

## Planned modules

Following the staging of the implementation prompt, in order:

1. `algebra/vector.rs`, `matrix.rs`, `normal_form.rs`, `lattice.rs` - integer
   matrices, Smith and Hermite normal forms, free lattices and sublattices.
2. `temperament/map.rs`, `image.rs`, `kernel.rs` - the general
   `TemperamentMap` with `RawTemperamentMap` validation, `AmbientLattice`,
   `ImageLattice`, kernel bases. `PatentVal` becomes a constructor for it.
3. `temperament/splitting.rs`, `representative.rs` - `HomomorphicSplit` and
   `RepresentativePolicy`, kept as separate traits, with `LiftDecision` and
   exact kernel residues.
4. `proportion/complexity.rs` - `group_length`, `lattice_seminorm`,
   `lattice_norm`, `cost` as distinct declared profiles.
5. `pitch/`, then `time/`, then `score/`, then `realization/`, then the native
   container in `io/`.

`PatentVal` deliberately exposes a rank-1 preview of the ambient/image
distinction (`image_generator`, `image_coordinate`, `embed_image`). When
`ImageLattice` lands, those become thin wrappers over the general form rather
than a parallel implementation.

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
