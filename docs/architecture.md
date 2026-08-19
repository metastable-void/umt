# Architecture

One crate, `umt`, organized by module rather than by workspace member. The
module boundaries follow the UMT-3.2 layer model, not an arbitrary layering of
convenience.

## Layer map

| UMT layer | Content | Modules |
|---|---|---|
| L0 notation | spelled symbols, ties, tuplet brackets | `score` (ties and event structure); spelling deferred |
| L1 exact structure | monzos, exact ratios, rational durations | `algebra`, `proportion`, `time::beat` |
| L2 structural quotient | tempered classes, image lattices, meter | `temperament`, `pitch::{chord, voice_leading}`, `time::{rhythm, meter}` |
| L3 metric realization | log-frequency, tuning curves, tempo maps | `pitch::{units, tuning, trajectory, empirical}`, `time`, `realization`, `generated` |
| L4 device realization | ticks, MIDI, control words | `time::quantize`, `realization::plan`, `pitch::trajectory::SampledTrajectory` |

## Present modules

```text
src/
  lib.rs                  crate root, no_std attribute, re-exports
  error.rs                typed error enums
  context.rs              TheoryContext, wire-form references
  quantity.rs             validated real-quantity newtype macros (private)
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
    units.rs              FrequencyHz, LogFrequency, Octaves, Cents, Radians
    point.rs              PitchOrigin, PitchPoint torsor, PitchPointRef
    tuning.rs             RegularTuning, PitchRealization, PitchRealizer
    chord.rs              VoiceId, VoiceSet, Chord, PitchMultiset, annotations
    empirical.rs          EmpiricalScale, LatticeFit - measured L3 scales
    voice_leading.rs      VoiceLeading span, span cost, chord distance profiles
    trajectory.rs         Deviation, PitchTrajectory, sampling and its record
  generated/
    scale.rs              GeneratedSet, three-gap report, MOS predicate
    euclidean.rs          EuclideanRhythm, verified maximal evenness
  score/
    id.rs                 EventId, StaffId, PartId, EventScope
    event.rs              TemporalPlacement, EventContent, ScoreEvent
    container.rs          Score, ScoreBuilder, ties, gestures, projections
    transform.rs          EventRelation, ScoreTransform and its composition
  time/
    units.rs              ClockTime, Seconds - the physical timeline
    span.rs               TimeSpan, the closed domain [t0, t1]
    beat.rs               BeatTime, Beats, BeatDuration, BeatSpan - all exact
    rate.rs               SecondsPerBeat, BeatsPerSecond, OrientedRatio
    rhythm.rs             RhythmTree, CyclicRhythm, exact flattening
    meter.rs              TimeSignature, Meter, Grouping, MetricLayering
    quantize.rs           TickGrid, Quantized, allocation and its evidence
    tempo.rs              TempoMap in the homeomorphism profile
    constraint.rs         STP, linear-ratio, and external-predicate profiles
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
    residual.rs           the seven-kind residual taxonomy of section 7.9
    provenance.rs         ProvenanceRecord, ProvenanceArena, CanonicalValue
    record.rs             RealizationRecord, DeviceAdapterProfile
    plan.rs               PerformancePlan, the realtime boundary
  io/
    text.rs               canonical exact-value text codec
    version.rs            UmtSchemaVersion, compatibility rule
    document.rs           UmtDocument, the native container of section 8.8
    scala.rs              the `.scl` adapter (feature `scala`)
    serde_exact.rs        serde adapters (feature `serde`)
```

`PatentVal` wraps a `TemperamentMap` rather than reimplementing it. Its scalar
accessors (`image_generator`, `image_coordinate`, `embed_image`) are views of
the general machinery that are meaningful only because the ambient rank is 1;
`PatentVal::map` reaches the general object.

## What is deliberately absent

Every part of UMT-3.2 is implemented. Two things are absent on purpose rather
than pending:

- **L0 pitch spelling** (section 4.5). Prompt section 55 says not to overbuild
  notation in v1, and a spelling system is a large orthographic model whose
  shape depends on which notation traditions it must serve. Ties, event
  structure, and the enharmonic *mechanism* - a spelling's comma residue
  against a canonical lift - are all present; only the letter-and-accidental
  layer is not.
- **External adapters other than Scala `.scl`** (sections 8.4 to 8.7). Prompt
  section 40 says the first release does not need them and that native
  serialization is the higher priority. The internal model is shaped so each
  can be added behind a feature, and `io::scala::AdapterProfile` is the
  declaration shape section 8.1 requires of every one of them.

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
`TheoryContext` of prompt section 8.

## Why voice leading is four types instead of one function

`chord_distance(a, b) -> f64` would be the obvious API and it would be wrong,
because UMT-3.2 section 4.4.5 says the plausible readings of that call answer
different questions. So the module has four objects instead:

- `VoiceLeading` is the declared relation, a span, with no costs attached. It
  is the only one of the four that is exact and metric-free.
- `SpanCostModel::declared_cost` prices *that* relation, additively, in the
  five terms section 4.4.2 names.
- `SpanCostModel::minimum_over_assignments` minimizes over a stated family, and
  the `SpanCost` it returns carries `CostQuestion::MinimumOverFamily` so the
  number cannot be mistaken for the previous one.
- `ChordDistance` is the only one permitted to claim distance laws, and
  `metric_claim` names the state space each profile claims them on.

The fourth is where the crate is most careful. Balanced per-voice transport is
a metric on multisets, but only a *pseudometric* on labelled chords - a voice
exchange leaves it at zero. Saying "metric" without saying "on what" is the
error section 9.5 exists to prevent, so the claim is a value with a
`state_space` field rather than a sentence in a doc comment.

## Why there are two timelines

`ClockTime` and `Seconds` are measured reals. `BeatTime`, `Beats`, and
`BeatDuration` are exact rationals. They are separate types with no
`From` between them, and the only way across is a `TempoMap`.

That is section 5.1 taken literally, and it pays for itself immediately.
Nested tuplets land on rationals like `1/15` of a beat that no binary float
represents, so an exact structural timeline is the difference between a
quintuplet inside a triplet closing exactly and closing to within `1e-16`. And
because the crossing is a named object rather than a numeric conversion,
section 5.8.3's point - that a tempo map is not the same construction as a
pitch tuning - is enforced rather than merely stated.

## Three temporal solvers, because there are three problems

Section 5.10 distinguishes solver profiles because not all temporal constraints
reduce to shortest paths, and prompt section 31 forbids a universal solver that
pretends otherwise. So:

- `StpProblem` takes difference bounds and runs Floyd-Warshall. It is the only
  profile with the unconditional consistency claim, and the only one that can
  make it, because it is the only one whose constraints are difference bounds.
- `LinearTemporalProblem` takes general linear inequalities and runs exact
  Fourier-Motzkin elimination over the rationals. A three-variable ratio bound
  cross-multiplies into this profile and cannot reach the previous one: the
  types do not connect.
- `HybridTemporalProblem` adds external predicates, which are typed data with a
  declared contract and never executable code, and reports them as outstanding
  rather than deciding them.

Fourier-Motzkin was chosen over a simplex or an interior-point method for one
reason: it carries strict inequalities through the arithmetic natively. That is
what lets section 5.10.2's positivity condition stay strict instead of being
replaced by an invented `delta`, which is fixture F25. The cost is
combinatorial blow-up, so the elimination is budgeted and reports exhaustion
rather than guessing.

## Realtime

The semantic core is not realtime-safe and does not try to be. The boundary is
`realization::PerformancePlan`, whose contract is a *value* -
`RealtimeContract`, with five named fields - rather than a marker trait that
cannot be checked. `docs/realtime.md` states what a compiled plan guarantees,
what it does not, and why building one is deliberately not realtime-safe.

## Why the score is generic over its pitch attachment

`Score<P>` is parameterized by what hangs off a note, not by an interval group.
In memory `P` is a `PitchPoint<E>`; on the wire it is a `PitchPointRef`, which
names a lattice and coordinates and resolves against a `TheoryContext`.
`Score::try_map_pitch` moves between the two.

The alternative was a parallel `RawScore`/`RawEvent`/`RawContent` hierarchy
duplicating every field. This way the placement, scope, tie, and transformation
types exist once, and the only thing that differs between an authored score and
a document is the one field that genuinely differs.

It also states something true: what a note carries is a parameter of the score,
not a fixed choice. A score of 5-limit monzos and a score of 12-EDO steps are
different types, which is the same discipline `Monzo` and `Chord` already
apply.

## Why residuals have seven types and no `Add`

Section 7.9 opens by ruling something out: "UMT-3.2 never stores one
undifferentiated `error` field." The seven kinds it tabulates are in genuinely
different spaces - an exact kernel element, a real interval, a real value with
an uncertainty, an exact rational duration, a pair of encoded control values, a
symbolic note - and a single `f64` would make every question about them
unanswerable.

So `Residual` is an enum whose variants carry their own units, and there is no
`Add` implementation. `Residual::try_add` succeeds within a kind that is
genuinely additive - structural, tuning, temporal, grid - and refuses
otherwise. An empirical fit is refused because combining its uncertainties
needs a model nobody declared; a device-control residual is a pair rather than
a difference; a notation residual is symbolic.

`examples/performance_compilation.rs` shows all three of the common kinds
arising from one compilation: an exact syntonic comma, an exact rational grid
residual totalling `1/48` of a beat, and a real device-control residual.

## Why the container's sections are all optional

Section 8.8 requires it: "Domain sections are present only when required by the
represented objects; this is necessary because UMT permits, for example, direct
empirical L3 scales with no L1 basis and domains with no distinguished periodic
unit."

So `UmtDocument` has no mandatory domain section, and absence is meaningful
rather than a defect. The serialized form omits an absent section entirely
rather than writing a null, because a null would be a claim that the section
exists and is empty.

The one cross-section rule that *is* enforced runs the other way: a
distinguished `unit` without a `basis` is refused, since the unit is a monzo
and a monzo's coordinates mean nothing without the basis they are over.

## How unknown extensions are handled

Prompt section 39 asks to "allow unknown future extension fields where feasible
without silently treating them as understood". Two mechanisms:

- **Profiles.** A document declares a profile set. This build implements the
  profiles in `SUPPORTED_PROFILES`; a document declaring anything else loads,
  and `unsupported_profiles()` names it, and `is_fully_understood()` is false.
  Nothing pretends the semantics were honoured.
- **Extension data.** `UmtDocument::extensions` holds `CanonicalValue`, so an
  extension's exact rational stays exact rather than becoming a double on the
  way through.

The schema version governs the *encoding* and the profile set governs the
*semantics*; they are separate questions and separate fields.

## Checks, with feature combinations

The crate's own check list, which every change is expected to pass:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo test --all-features
cargo test
cargo build --no-default-features --features serde,scala --target x86_64-unknown-none
cargo clippy --no-default-features --target x86_64-unknown-none -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo bench
```

The second and fifth lines matter more than they look. A test suite that only
compiles with `--all-features` is a test suite nobody can run by typing
`cargo test`, and fixtures whose obligations are specifically about an encoding
- F9, F20, F21, F29 - are gated on the feature that provides it rather than
making the whole suite depend on one.

The bare-metal line is not decoration either: it is the only check that catches
a dependency feature silently pulling `std` back in through feature
unification. A host build cannot detect that.

## Why generated sets and Euclidean rhythms are separate types

They share modular arithmetic, balance properties, and continued-fraction
structure, and section 3.5 explicitly permits common algorithms for modular
distribution. It then says, in bold, that UMT-3.2 "does **not** identify every
MOS construction with every Euclidean-rhythm construction as the same theorem
or the same object".

So `GeneratedSet` and `EuclideanRhythm` are separate types with no conversion
between them. `GeneratedSet` works in a real ordered realization space with a
declared period and generator; `EuclideanRhythm` works in `Z_n` with integer
onsets. What they compute is analogous, and the objects are not the same.

Two smaller decisions follow the same instinct:

- `GeneratorRatio` is **declared**, not inferred. Whether `g/p` is rational
  cannot be decided from two `f64` values, and section 3.2 requires the answer
  to be recorded by anything claiming a three-gap result. An `Undeclared` ratio
  makes no closure claim at all, and `GapReport::closure_matches_declaration`
  reports when a declaration and the arithmetic disagree instead of silently
  preferring one.
- There is **no well-formedness predicate**. Section 3.3 permits one but
  requires the exact definition to be declared, and warns against treating
  "well-formed" and "two gap sizes" as interchangeable. `mos_verdict` is the
  operational two-gap predicate under a named `MosProfile`; anyone needing a
  particular well-formedness definition should attach it to their own type.

## Benchmarks and what they say

`cargo bench` runs `benches/core.rs`, a plain timing harness over the
operations prompt section 50 names. There is no benchmarking framework: the
crate's dependency policy is "as few as possible, all pure Rust", and section
50 asks for numbers on a named list rather than for statistics. The numbers are
indicative, meant to catch an order-of-magnitude regression.

What they show, on a typical host:

- monzo addition is under 100 ns and image membership around 120 ns, so the
  hot path of temperament work is cheap;
- a Smith normal form of a small matrix is a few microseconds, and
  `TemperamentMap::from_rows` is dominated by it - which is the cost of D9's
  decision to compute derived structure eagerly, paid once per mapping;
- rhythm-tree flattening and grid allocation are tens of microseconds for
  scales of tens of leaves, essentially all of it arbitrary-precision rational
  arithmetic and its allocation.

That last line is the whole trade. Exact structural time costs roughly two
orders of magnitude over `f64`, and buys a quintuplet inside a triplet that
closes exactly. Anything needing the speed compiles a `PerformancePlan`, where
every value is a bounded integer and reads are borrowed slices.

Section 50 also lists caches. D9 records the decision *not* to add them: derived
structure is computed eagerly in the validating constructor, which keeps the
types free of interior mutability, keeps them `Send + Sync` without a
`std`-only `OnceLock`, and guarantees equality can never depend on cache state.

## Robustness

`tests/robustness.rs` covers prompt section 48's requirement that untrusted
input never panic. It is a deterministic stand-in for a fuzzer: a seeded
xorshift generator drives each parser over tens of thousands of malformed
inputs, alongside hand-written adversarial cases - empty files, zero
denominators, 400-digit integers, 300-deep rhythm trees, dense constraint
graphs with negative cycles, and grids at `u32::MAX`.

Every failing case is reproducible from the test name alone, because the seeds
are constants and nothing reads a clock.
