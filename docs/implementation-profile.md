# Implementation profile

Concrete decisions this crate makes where UMT-3.2 leaves the choice open, and
the reasoning behind each. Decisions that affect serialized semantics or public
API stability are marked **binding**: changing one later is a breaking change.

## D1. `no_std + alloc` is the baseline **binding**

The crate is `#![cfg_attr(not(feature = "std"), no_std)]` with `extern crate
alloc`. The default `std` feature is purely additive and must never change a
computed value.

The specification's exact core needs no floating point at all (section 0.6.1
forbids it for L0 to L2 decisions), and none of the exact machinery needs an
operating system, so the constraint costs nothing here. It does constrain API
shape, and those constraints are cheap now and expensive later:

- no `std::io::Read`/`Write` in any serialization signature; encode to and from
  slices and `Vec<u8>`, with `std::io` adapters behind the feature;
- no `HashMap`/`HashSet` in the semantic core; ordered maps or index vectors,
  which the determinism requirement wants anyway;
- no `OnceLock`, `Mutex`, or `RwLock`; derived data is computed eagerly in
  validating constructors;
- no clock and no random number generator inside the crate; seeds and
  timestamps are supplied by the caller and recorded in provenance.

Verified against `x86_64-unknown-none`, a target with no `std` at all, so a
dependency that pulls `std` back in through feature unification fails the build
rather than passing unnoticed.

Consequence for dependencies: every one is declared `default-features = false`,
and `num-rational`'s default `num-bigint-std` feature in particular must not be
enabled outside the `std` feature.

## D2. `Z = BigInt`, `Q = BigRational` **binding**

Exposed as crate-level aliases, which are the stability boundary over the
arithmetic dependency (prompt section 5.1). Public signatures use the aliases,
so the underlying library can be replaced without changing UMT semantics.

Rejected: fixed-width integers for exact storage (overflow), `f64` (identity
errors), and any dependency requiring a C toolchain or a Python build step.

## D3. Transcendental functions come from `libm`, always **binding**

Including in `std` builds. `f64::log2` and friends resolve to the host math
library, which is not required to be bit-identical across platforms or libc
versions. UMT-3.2 requires the same input and parameters to produce the same
result, and reproducible composition means the answer cannot depend on which
Cargo features a downstream crate happened to enable.

Cost: `libm` is a mandatory dependency, and results may differ in the last unit
in the last place from what a host libm would produce. That is the intended
trade.

## D4. Patent-val entries are computed exactly **binding**

UMT-3.2 section 1.6 defines the entry as `round(N * log2(nu_3(beta_i)))`. For a
rational-profile generator the L3 valuation is the exact embedding of the
rational value (section 1.1.2), so the real number being rounded is pinned
exactly by the rational, and `algebra::integer::round_n_log2` decides the
rounding by comparing `x^N` and `x^(2N)` with powers of two in integer
arithmetic.

This computes the same integer the ideal real formula gives, while keeping an
L2 structural object free of floating point as section 0.6.1 requires. See
`spec-issues.md` issue S1.

Two consequences fall out rather than being special-cased:

- a generator with exact valuation 2 receives the entry `N` under every
  rounding convention, as section 1.6 requires;
- exact nearest-rounding ties are impossible in the rational profile, since
  `x^(2N) = 2^(2k+1)` would require an odd power of two to be a perfect
  `2N`-th power. Both nearest conventions therefore agree on every rational
  input. The convention is still recorded, because it is part of the result's
  provenance and it does change results for symbolic-real generators and for
  the floor and ceiling profiles.

Known performance limitation: the method materializes `x^(2N)`, which is
roughly `2N log2(x)` bits. Fine to several thousand divisions, wasteful beyond
that. A continued-fraction refinement can replace it without changing any
result.

A symbolic-real generator has no exact valuation, so its entry is computed in
`f64` and the resulting `PatentVal` reports `Exactness::RealValued`.

## D5. Equality is presentation equality **binding**

`PartialEq`, `Eq`, and `Hash` mean "same canonical presentation", never
mathematical isomorphism. Two monzos are equal when their bases are the same
declared object and their exponent vectors agree. Monzos over different bases
are unequal rather than incomparable, because inequality is the correct
presentation-level answer.

Structural and quotient-aware equivalences get explicitly named methods as they
arrive (`same_temperament_kernel`, `same_subgroup`, and so on).

`PositiveFinite` and `NonNegativeFinite` implement `Eq` and `Hash` over the bit
pattern. That is total because NaN is excluded at construction and `-0.0` is
either impossible or normalized.

## D6. Basis compatibility is structural, not nominal **binding**

`Basis::same_identity` compares the identifier, the ordered generators, and the
independence contract. A shared identifier alone is not enough: a document that
reuses an identifier for different generators must not pass a compatibility
check. Monzo arithmetic tries `Arc::ptr_eq` first as a fast path.

## D7. Exact values are serialized as canonical text **binding**

`Z` as a decimal string, `Q` as `"numerator/denominator"` reduced with an
explicit denominator. Never as a JSON number, and never through the arithmetic
library's internal digit representation, which is not a stable wire contract.

Loading goes through the same validating constructors as in-memory
construction: `RawBasis` is the unvalidated wire form and `Basis` is reachable
from it only through `TryFrom`.

Objects that reference shared context serialize as references resolved against
a `TheoryContext`; see D17.

## D8. Lattice identity is the canonical Hermite basis **binding**

Every sublattice - image, kernel, or plain `Sublattice` - stores the canonical
column Hermite normal form of its generators. Consequences:

- two sublattices are equal as values exactly when they are equal as
  subgroups, so "do these mappings temper out the same commas" is a `==`;
- serialized bases are reproducible, which matters for diffing scores;
- the basis vectors have a positive leading coordinate, which is a
  presentation choice rather than a musical one. The subgroup generated by the
  syntonic comma `81/80` is presented by the monzo of `80/81`, since both
  generate the same subgroup and only one of them is canonical. Callers that
  want conventional comma orientation negate as they see fit.

Smith normal form is used only where the result does not depend on the choice
of transforms: invariant factors, rank, and kernel and saturation bases that
are immediately canonicalized.

## D9. Derived structure is eager, never lazily cached **binding**

`TemperamentMap::new` computes the image, kernel, and invariant factors at
construction. This follows from D1 - `OnceLock` is `std`-only - but it is also
the better design: the type stays free of interior mutability, stays
`Send + Sync`, and equality can never depend on cache state.

The cost is that constructing a mapping always pays for one Smith normal form
and one Hermite normal form, even if the caller only wanted to apply it. That
is the right trade for a type whose whole purpose is to expose that structure.

## D10. Two kernel constructors, because the specification has two cases

`KernelLattice::of_map` performs no saturation validation, because the kernel
of a homomorphism into a torsion-free group is saturated as a theorem (UMT-3.2
section 1.4.1). `KernelLattice::from_direct_commas` does validate, because a
directly supplied comma subgroup can have torsion (section 1.5), and it takes
an explicit `SaturationPolicy` because the specification permits either
rejecting or saturating-and-reporting.

Collapsing these into one function would either reject valid map-derived
kernels or silently accept unsaturated user input. The separation is the
point.

## D11. Ambient lattices carry an identity **binding**

`Gamma` is a *declared* group, not just a rank, so `AmbientLattice` has a
stable identifier and elements of different declared ambient lattices are
neither equal nor addable, exactly as with bases. Equal-division mappings
declare `umt:edo:<N>`, so two patent vals for the same division count share
one ambient step lattice even over different bases - which is correct, since
`Gamma_N = Z` is the same declared object.

## D12. Operations report `TemperamentError`; construction reports its own

`PatentValError` covers only what can go wrong while deriving entries from
valuations. Everything about applying a mapping, converting coordinates, or
validating a comma subgroup reports `TemperamentError`, so callers match one
error type across the layer rather than translating between parallel ones.

## D13. Canonical identity and readable presentation are separate methods

The canonical Hermite basis pivots on the first coordinate, which for a prime
basis is the octave. That drives the higher-prime exponents up: the 12-EDO
kernel comes out as `[1 20 -14>` and `[0 28 -19>` - correct, canonical, and
unrecognizable to a musician.

So each of these objects exposes two views, and the docs say which is which:

- `KernelLattice::basis` and `basis_monzos` - canonical. Identity, equality,
  serialization.
- `KernelLattice::comma_basis` and `comma_basis_monzos` - presentation. The
  same normal form with the coordinate order reversed, so pivots eliminate the
  *highest* generator first. For 12-EDO this yields the Pythagorean comma and
  the schisma, both under 25 cents.

The same reasoning drives `Sublattice::reduce` versus `reduce_reversed`. Both
give one canonical representative per coset; the reversed one keeps the
high-generator exponents small, which is what makes
`TemperamentMap::preimage` return `3/2` for the 12-EDO class of seven steps
instead of a twenty-digit member of the same fiber.

Neither presentation form is a claim of minimal complexity. That would need a
declared complexity function and a reduction against it, which is a later
stage.

## D14. A linear splitting cannot be a spelling policy, and the API shows it

A homomorphic splitting is forced to send the class `n x` to `n` times the lift
of `x`. For 12-EDO that means the lift of seven steps is seven times the lift
of one step, which is enormous however the lift of one step is chosen. No
amount of reduction fixes this: it is what additivity *means* here.

That is exactly why UMT-3.2 section 1.7 splits the concept in two, and why this
crate has both `LinearSplit` (additive, useful for direct-sum decompositions)
and `CanonicalLiftPolicy` (reduces every class independently, small, and
correctly declining to claim homomorphism). `examples/temperament_12edo.rs`
prints both side by side, because the contrast is the lesson.

`OffsetPolicy` composes over any policy rather than only over a splitting, so
an adaptive policy can be layered on either base.

## D15. A complexity declares its laws, and the declaration is checked

UMT-3.2 section 9.2 refuses to use the bare word *norm*, so neither does this
crate. `ComplexityProfile` is a field of every complexity function, and the
conformance suite checks the claim rather than trusting it - fixture F34 is
precisely a function that looks like a norm and is not one.

Two consequences in the API:

- `WeightedL1` derives its profile from its weights. All positive gives
  `LatticeNorm`; any zero gives `LatticeSeminorm`, because a zero weight puts
  a nonzero element in the null set. That is how section 1.3.3 models
  octave-equivalent complexity, and it means the weaker claim is made
  automatically rather than by the caller remembering to.
- Weights are never derived from logarithms without validation.
  `LogWeightedL1::from_log2_valuations` rejects a generator whose valuation is
  at most 1, since `log2` of it is not a usable norm weight. That rejection is
  fixture F5.

An exact complexity reports an exact value; Tenney height reports `None` from
`exact_value`, because its value really is an L3 real observation and pretending
otherwise would be the layer violation the whole design is built to avoid.

## D16. The context registry rejects redefinition **binding**

`TheoryContextBuilder` accepts the identical definition twice and rejects a
different definition under an identifier already in use. Taking the last
writer would silently change the meaning of every monzo that referred to the
earlier one.

Registering a mapping also registers its domain basis and ambient lattice, so
a context that can resolve a mapping can always resolve what the mapping
refers to.

## D17. Context-dependent objects serialize as references **binding**

A monzo on the wire is `MonzoRef`: a basis identifier plus exact exponents. A
mapping is `TemperamentMapRef`: two identifiers plus an exact matrix. Neither
inlines a definition, as UMT-3.2 section 6.3 requires, and neither is
meaningful without the context that resolves it - which is the honest
representation of a value whose identity depends on shared context.

Round-tripping therefore takes two steps: deserialize the reference, then
resolve it. There is no single-step path, because a one-step path would have
to invent a basis.

## D18. Schema compatibility is one-directional **binding**

A reader accepts a document whose major version matches and whose minor
version does not exceed its own. A higher minor version means the document may
carry fields this build does not know, and prompt section 54 forbids
interpreting unknown fields as current semantics, so such a document is
rejected rather than guessed at.

## D19. A minimum-complexity search proves its own bound

`MinimumComplexityPolicy` does not search "a reasonable region and hope". For
a lattice norm, the triangle inequality gives `h(m0 + k) >= h(k) - h(m0)`, so
any improving kernel element satisfies `h(k) <= 2 h(m0)`; the echelon
structure of the canonical kernel basis turns that into per-coordinate bounds.

Two consequences the API makes visible:

- The bound needs per-coordinate weights, so the policy requires
  `CoordinateWeighted` rather than bare `Complexity`, and a seminorm is
  rejected outright: its minimizer set over a coset can be infinite.
- The search *region* is computed in floating point and rounded outward, so it
  can only be too large. The *selection* inside it uses the complexity's own
  comparison, which is exact for `WeightedL1`. Approximation never creeps into
  the choice, only into the size of the box.

When the provable region exceeds the budget, the outcome is
`Approximate { guarantee: SearchedRegion { .. } }`, never `Exact`. Downgrading
the claim is the whole point of having the claim be a value.

## D20. Points carry an origin identity, and the L3 torsor does not

A structural pitch point is an origin identity plus an offset, because "a
fifth above C" and "a fifth above D" are different pitches and an exponent
vector cannot tell them apart. `interval_to` across different origins is an
error, not a number.

`LogFrequency` needs no origin: the real line has a canonical one at 1 Hz. It
is still a torsor - point plus interval, point minus point - and there is
still no point-plus-point operation on either.

## D21. The declared L2 interval group is a type, not a field

UMT-3.2 section 1.8.2 lets a tuning live on the reachable image `H` or on the
ambient `Gamma`, and section 1.9 requires the choice to be recorded.
`RegularTuning<G>` is generic over that choice through `L2IntervalGroup`, so
the record cannot be omitted and a tuning of one group cannot be handed an
element of the other. For 6-EDO that is the difference between a generator of
one step and a generator of two.

## D22. A chord's points share one origin **binding**

UMT-3.2 section 4.3 writes a chord as `c: V_c -> P` for a single pitch-point
space `P`. This crate enforces that: `Chord::assign` rejects a point measured
from a different origin. Without it, the interval between two of a chord's own
voices would be undefined, and so would every voice-leading displacement.

An emptied chord forgets its origin and can be reused, which keeps the empty
chord a genuine neutral element for juxtaposition rather than one that has
silently committed to an origin.

## D23. The span cost accounting convention is stated, because 4.4.2 does not

Section 4.4.2 gives `C = C_move + C_split + C_merge + C_birth + C_death` and
leaves the counts to the implementation. This crate charges:

- one movement term per *edge*, `d(e)^p` for the declared ground cost;
- `max(0, outdegree - 1)` splits per source voice, so a three-way fan is two
  splits;
- `max(0, indegree - 1)` merges per target voice;
- one birth per target voice with no incoming edge;
- one death per source voice with no outgoing edge.

`SpanShape` reports all of these plus a `continuations` count, which is a
*subset* of `moves` rather than a sixth term. Any other convention is
defensible; the point is that this one is written down and observable rather
than inferred from the total.

## D24. Optimizers over voice leadings are exhaustive and budgeted

Prompt section 22 says to implement the structural span model first and not to
prioritize sophisticated optimal transport, so `minimum_over_assignments` and
`ChordDistance` enumerate candidates rather than running an assignment solver.
Two consequences, both deliberate:

- Enumeration is tie-aware for free, so a plateau is reported as
  `OptimizationOutcome::Multiple` rather than resolved silently. An
  `O(n^3)` solver would have to work to recover that.
- Where the search cannot be completed, an *optimizer* downgrades to
  `Approximate { SearchedRegion }`, but a *distance* that claims metric laws
  returns `SearchBudgetExceeded` instead. A metric has to be the true minimum,
  so refusing is the only honest answer.

The default budget of 200000 candidates covers every partial assignment
between two seven-voice chords and every permutation of eight voices. A
Jonker-Volgenant or Kuhn-Munkres solver is the obvious later replacement for
the non-tie-aware paths.

## D25. Balanced transport is exact by Birkhoff, not by a linear program

For equal-cardinality uniform measures the optimal coupling is attained at a
permutation, so minimizing over `S_n` gives classical `W_p` exactly. Under
`MassProfile::NormalizedProbability` with `n` and `m` voices, both measures
are refined to `lcm(n, m)` atoms of equal mass first, which reduces the
unequal case to the same permutation search. That is why no transportation
linear program appears anywhere in this crate, and also why the search budget
binds sooner on that path.

## D26. The edit profile's metric claim is unconditional, and says why

An earlier reading of section 9.5 suggested the assignment/edit distance is a
metric only where the ground distance stays below twice the boundary cost.
That condition is unnecessary. In an optimal solution no matched pair ever
costs more than deleting and re-creating it, so the distance is unchanged if
the ground cost is replaced by the truncation `min(d, 2^(1/p) * boundary)` -
and a truncated metric is still a metric. The claim reported by
`metric_claim` therefore states the effective truncated ground cost rather
than a side condition.

The claim is withdrawn for a zero boundary cost, where identity of
indiscernibles genuinely fails: a chord is then at distance zero from every
chord containing it. `tests/properties.rs` exercises symmetry, identity of
indiscernibles, and the triangle inequality across three different
cardinalities, which is what section 9.5 asks for.

## D27. Sampling error bounds are derived, not declared

`PitchTrajectory::sample_in_fixed_context` computes its
`ApproximationGuarantee` from the deviation's analytic Lipschitz constant: an
`L`-Lipschitz function sampled at spacing `h` is reconstructed within `L h / 2`
by linear interpolation and `L h` by a zero-order hold. Where no Lipschitz
bound exists - a step change over zero time - the guarantee is `Unquantified`
rather than a number nobody proved.

The method name carries the assumption that makes the bound sound: in a fixed
context `Phi(x, c)` is constant, so the deviation's constant bounds the whole
trajectory. A varying context has to be sampled through `evaluate_with`, and
the caller states their own guarantee.

## D28. The two timelines are unrelated types **binding**

`ClockTime` and `Seconds` are measured reals; `BeatTime`, `Beats`, and
`BeatDuration` are exact rationals. There is no `From` in either direction and
no arithmetic between them. The only crossing is a `TempoMap`.

Section 5.1 requires the distinction and section 5.8.3 explains why it matters:
a tempo map is a monotone map between affine ordered timelines, not a group
homomorphism on intervals, and a single shared timeline type would erase
exactly what the map exists to express. The practical payoff is smaller and
more immediate - a quintuplet inside a triplet lands on `1/15` of a beat, which
closes exactly as a rational and does not as a `f64`.

## D29. The declared beat unit is the quarter note **binding**

Section 5.1 puts the structural timeline over `D_b = Q` "in declared beat
units" and requires a different group to be declared. This crate declares
`D_b = Q` (`BEAT_DURATION_GROUP`) with the quarter note as the unit
(`BEAT_UNIT`).

The unit is not arbitrary. PPQN device grids are already expressed in pulses
per quarter note, so with this choice a `TickGrid` of `P` and the grid `G_P` of
section 5.7 are the same object, and fixtures F12 and F13 read directly as
written.

## D30. Three temporal solver profiles, separated by type

`StpProblem` accepts `DifferenceConstraint` and nothing else; `RatioConstraint`
cross-multiplies into `LinearConstraint`, for which `StpProblem` has no method.
So fixture F17's obligation - that a ratio constraint is not silently passed to
a shortest-path solver - is discharged by the absence of an API rather than by
a runtime check.

The linear profile uses exact Fourier-Motzkin elimination over the rationals
rather than a floating-point simplex. Two reasons, both load-bearing:

- Strict inequalities propagate through the arithmetic natively, so section
  5.10.2's positive-denominator condition can be *kept* strict. A solver that
  could not express it would have forced the invented `delta` the specification
  forbids (fixture F25).
- The answer is exact, so a feasibility verdict is a proof rather than a
  numerical opinion.

The cost is that elimination can blow up combinatorially. It is budgeted, and
exhaustion is reported as `EliminationBudgetExceeded` rather than answered.

## D31. A justified delta carries its justification in the type

`PositivityHandling::JustifiedDelta` has a mandatory `justification: String`
beside its `delta`. Section 5.10.2 permits the substitution only "when a
positive lower bound `delta` is justified by the model or source data", so a
`delta` with no stated justification is exactly what the specification forbids
- and it is not a value this crate can construct.

## D32. Endpoint-preserving allocation rounds boundaries, not durations

Section 5.7.5 describes the outcome and leaves the method open.
`allocate_preserving_endpoint` rounds the *cumulative* boundaries and takes
each child's span as the difference of consecutive boundaries, which makes
`sum n_i = N` a theorem rather than a correction step. The final boundary is
taken from the parent rather than recomputed, so the partition closes exactly
whatever the weights.

`allocate_locally` exists alongside it precisely so the failure mode of
section 5.7.4 can be demonstrated rather than described.

## D33. A tie relates two noteheads of one pitch, in one scope **binding**

`ScoreBuilder::tie` rejects a tie to itself, to a non-note, across scopes, or
between different pitches. The last is the debatable one, and it is deliberate:
a relation between noteheads of *different* pitch is a slur or a glissando, and
calling it a tie would make `Score::sounding_gestures` produce a sustained tone
at a pitch neither notehead has.

Nothing is merged by any of this. UMT-3.2 section 5.2.2 forbids merging tied
noteheads at L0, so the tie is a relation stored beside the events, and the
single sustained gesture is a *derived view* that keeps both source identities
inside it.

## D34. The score is generic over its pitch attachment **binding**

`Score<P>` is parameterized by what a note carries, so the in-memory score
(`P = PitchPoint<E>`) and its wire form (`P = PitchPointRef`) are the same
types with one field substituted, rather than two parallel hierarchies.
`Score::try_map_pitch` converts.

## D35. `sounding_gestures` requires a contiguous tie chain

A tie whose second notehead does not begin exactly where the first ends is
reported as `MisorderedTie` rather than being smoothed over. A gap or an
overlap between tied noteheads is a defect in the source, and the combined span
would otherwise silently include time neither notehead occupies.

Events without a fixed span - constrained and grace placements - are skipped by
this view rather than being given one, and are listed by
`Score::unmeasured_events`.

## D36. Compositionality is a value, not a label

`ScoreTransform::claims_compositional` reports whether every component
composes, and `ScoreTransform::compose` returns `None` rather than a plausible
answer when one does not. UMT-3.2 section 6.6 forbids the label "functorial"
without identity, composition of relations, of pitch components, of temporal
components, and a provenance rule; all five exist, and the operation is absent
exactly where the label would be unearned.

The temporal component is the affine family `t -> a t + b` with `a > 0`,
because that family is closed under composition. Anything outside it is
`TimeTransform::Declared`, which is perfectly usable and simply makes no
compositional claim.

## D37. Provenance composes by concatenation

`ProvenanceChain::then` appends rather than replaces. Section 9.12 requires a
later re-realization to be able to consult the original source rather than
compound previous rounding, which is only possible if the earlier steps
survive.

## D40. Residual addition is refused for three of the seven kinds

`Residual::try_add` succeeds within the structural, tuning-deviation,
temporal-realization, and grid kinds, and refuses the other three. Prompt
section 35 forbids "a generic arithmetic `Add` across residual variants", and
section 7.9 permits addition only where it "is mathematically meaningful".

- **Empirical fit** is refused because combining two uncertainties needs a
  declared error model, and picking one silently would fabricate a claim about
  the measurements.
- **Device control** is a pair of values - requested and encoded - not a
  difference; adding two pairs has no meaning.
- **Notation** is symbolic and has no numeric value to add.

There is no `Add` implementation for `Residual` at all, so the refusal is at
the type level and the exception is the named method.

## D41. A provenance record must name its algorithm and version **binding**

`ProvenanceArena::insert` rejects a record whose algorithm or version is empty.
Section 7.10 requires provenance "sufficient to identify the semantic profile,
algorithm/version, and parameters that affect the result", and a record without
those two cannot be sufficient for anything.

Parents must already be present when a record is inserted. That makes the
ancestry graph acyclic by construction rather than by a later check, and it is
why `ProvenanceArena::ancestors` terminates for every input.

## D42. Provenance parameters are a typed tree, not a blob

`CanonicalValue` has exact `Integer` and `Rational` variants alongside `Real`,
so an exact tolerance stays exact through a round trip and a measured one is
visibly a double. `CanonicalValue::is_exact` reports which a record is.

Prompt section 36 rules out "copying arbitrary JSON blobs into every semantic
object"; the deeper reason is that a blob cannot preserve the exactness
distinction this crate spends most of its effort maintaining.

## D43. A losslessness claim has exactly two admissible justifications

`RoundTripBasis` has `InjectiveOnDomain`, `SourceRetained`, and
`NotReversible`, and `RealizationRecord::claims_lossless` is true only for the
first two. Section 7.4 lists those two conditions and nothing else, and names
the second as UMT-3.2's default design.

`InjectiveOnDomain` carries the domain as a mandatory string, because
injectivity has to hold of the *represented* domain rather than of the map in
general, and a claim that does not say which domain is not checkable.

## D44. The realtime contract is a value, not a marker trait **binding**

`PerformancePlan::realtime_contract` returns a `RealtimeContract` with five
named boolean fields. Prompt section 38 forbids claiming `RealtimeSafe` without
a documented contract the type satisfies, and a marker trait cannot be asserted
on in a test.

The five guarantees, each established by the build step: reads return borrowed
slices, every stored value is a bounded integer, ranges were validated at build
time, voices and pitches are resolved to indices and millicents, and events are
sorted once by a total derived ordering. `docs/realtime.md` states the contract
in prose and says plainly what it does *not* cover.

Bounds: `MAX_TICK` is `u32::MAX / 2` and `MAX_MILLICENTS` is twenty octaves
either way. Both are arbitrary in the sense that some number had to be chosen,
and deliberate in that exceeding either is a signal that a single plan is the
wrong structure.

## D45. `divisions = 0` is a legal mapping

The zero mapping has image `{0}`, which section 1.6 fixes through the
convention `gcd(0, ..., 0) = 0`. In the general API its image has rank 0 and
an image element has an empty coordinate vector, which is the correct answer.
Only the rank-one scalar convenience API of `PatentVal` has nothing to return,
and it reports `TrivialImage` rather than pretending the answer is zero.

## D46. Rank-0 bases are permitted

A basis with no generators spans the trivial lattice. Nothing breaks, and
rejecting it would be an invented restriction.

## D47. An empirical scale needs no basis, no unit, and no fit **binding**

`EmpiricalScale` has no basis field, its `period` is `Option`, and its `fit` is
`Option`. Section 4.9.1 makes a direct empirical scale "the minimum adequate
representation for tunings whose cultural or acoustic basis is not established
by a small-integer model", so requiring any of the three would force a claim
the measurements do not support.

When a fit *is* supplied, `FitDeclaration` has six mandatory fields - one for
each thing section 4.9.3 says an inference MUST declare - and `LatticeFit`
requires one empirical-fit residual per degree. A fit that covers fewer degrees
is rejected rather than accepted as partial.

This crate infers no lattice on its own. Section 4.9.3 warns that "there is no
canonical instruction to take a maximally independent subset of local minima",
and an inference this crate performed would have to declare a candidate-
selection procedure nobody asked for.

## D48. Container sections are optional and their absence is literal **binding**

Every domain section of `UmtDocument` is optional, as section 8.8 requires, and
an absent section is omitted from the encoding entirely rather than written as
a null. A null would assert that the section exists and is empty, which is a
different statement.

The one enforced cross-section rule is the converse: a `unit` without a `basis`
is refused, since a monzo's coordinates are meaningless without the basis they
are over.

## D49. Profiles carry the semantic-compatibility question **binding**

`UmtSchemaVersion` governs whether this build can read the *encoding*;
`UmtDocument::profiles` governs whether it understands the *semantics*. They
are separate fields because they are separate questions, and a document can be
perfectly readable while declaring semantics this build does not implement.

`unsupported_profiles()` names them and `is_fully_understood()` reports the
verdict. Neither is an error: prompt section 39 asks for unknown extensions to
be allowed "without silently treating them as understood", and reporting is how
that is done.

## D50. A `.scl` entry keeps its layer **binding**

`ScalaEntry` is an enum with `Ratio` and `Cents` variants, because section 8.2
says an `.scl` file "is not uniformly `L3 only`" and requires an importer to
retain which each entry was. `exact_ratio()` returns `None` for a cents entry:
turning a decimal into a rational would fabricate exactness the file never
claimed.

Flattening to a uniform L3 scale is available through `to_empirical_scale`, and
it returns a `ResidualSet` with one notation residual per exact entry it
flattened. The loss is reported, which is law 2 of section 9.12.

## D56. No lazy caches, on purpose

Prompt section 50 lists candidate caches - normal forms, kernel and image
bases, coordinate transformations, valuations, fingerprints - and this crate
adds none of them. D9 records why: derived structure is computed eagerly in the
validating constructor, which keeps the types free of interior mutability,
keeps them `Send + Sync` without a `std`-only `OnceLock`, and guarantees
equality can never depend on cache state.

The cost is visible in `cargo bench`: constructing a `TemperamentMap` pays for
one Smith and one Hermite normal form even if the caller only wanted to apply
it. For a type whose whole purpose is to expose that structure, that is the
right trade. A context-owned cache keyed by mapping identity remains available
later without changing any semantics, which is the shape section 50 suggests.

## Known limitations

- A generator cannot currently carry both an exact rational valuation and a
  distinct L3 override, which section 1.1.2 permits ("unless a different L3
  realization is explicitly selected"). Adding it means turning
  `GeneratorValuation::Rational` into a struct variant with an optional
  override; no serialized data written today would become invalid, since the
  override would be an added optional field.
- `IndependenceContract::Certified` carries a reference to a certificate but
  does not verify one. Verification is out of scope until there is a
  certificate format to verify against; the contract is declared metadata, as
  section 1.1.2 requires, and is never inferred.
- The native container writes and reads every section this crate models, but
  the `events` section carries a `ScoreRef` whose pitches resolve only against
  an ambient lattice. A score over 5-limit monzos has no wire form yet.
- `TheoryContext` registers bases, ambient lattices, and mappings. Named
  representative policies, tunings, and notation systems join it as those
  layers arrive.
- The Scala adapter reads and writes `.scl` only. Keyboard mappings (`.kbm`,
  section 8.3) are a separate object the format keeps separate, and are not
  implemented.
- `GeneratedSet` works in `f64`, because part III is defined in an ordered
  additive realization space rather than an exact one. A caller who wants an
  exact generated set over a rational period and generator would want a
  parallel exact type; nothing about the API shape prevents adding one.
- Smith normal form uses the textbook pivot-and-reduce algorithm, whose
  intermediate entries can grow well beyond the input size. Exactness is never
  at risk, since every entry is a `Z`, but a modular or fraction-free variant
  would be wanted before large matrices become common.
- `PerformancePlan` resolves voices to `u16` indices but does not yet carry the
  `DeviceAdapterProfile` that produced them. Pairing the two is what the device
  stage will do.

## D51. The generator-to-period ratio is declared, never inferred **binding**

`GeneratorRatio` is a stored field with `Rational`, `Irrational`, and
`Undeclared` variants. UMT-3.2 section 3.2 requires anything claiming a
three-gap result to record whether `g/p` is rational, and no finite computation
on two `f64` values can decide that.

`Undeclared` makes no closure claim at all. When a declaration and the
arithmetic disagree - a "rational" ratio whose orbit does not close where the
denominator says, or an "irrational" one that produces duplicates -
`GapReport::closure_matches_declaration` reports the disagreement rather than
silently preferring one of them.

## D52. There is no well-formedness predicate

Section 3.3 permits one and requires the exact definition to be declared,
warning against treating "well-formed" and "two gap sizes" as interchangeable
labels. `GeneratedSet::mos_verdict` is the operational two-gap predicate under
a named `MosProfile`, and nothing here is called well-formed.

Adding one would mean choosing among several incompatible definitions from a
historically specific literature. The honest way to offer it is with the
definition attached, which is work for whoever needs a particular one.

## D53. Maximal evenness is verified, not inferred from the formula

`EuclideanRhythm::onset_positions` generates onsets at `floor(i n / k)`, which
is maximally even by construction. `verify_maximal_evenness` nevertheless
checks the Clough-Douthett characterisation against the produced positions: for
every `m` from 1 to `k - 1`, the circular distances between onsets `m` apart
take at most two values differing by exactly one pulse.

Section 9.11 requires an implementation to "verify maximal evenness under the
selected definition", and a construction that is even by argument is not a
verification. The property test runs it over every `E(k, n)` up to 48 pulses.

## D54. Gap comparison uses a declared tolerance

Generated points are computed in `f64`, so gaps that are equal in exact
arithmetic differ in their low bits. `DEFAULT_GAP_TOLERANCE` is `1e-9` octaves,
about a millionth of a cent: wide enough to absorb that and far too narrow to
merge sizes that are genuinely different. It is configurable per set, and the
value used is recorded on every `GapReport`.

This is the one place in the crate where a structural conclusion - how many
distinct step sizes a scale has - rests on a floating-point comparison. It does
so because part III is defined in "an ordered additive realization space",
which is L3; the tolerance is therefore a declared parameter of the result
rather than an implementation detail.

## D55. A negative parent span is refused, not allocated into

`TickGrid::allocate_locally` and `allocate_preserving_endpoint` reject a
negative `parent_ticks` with `TimeError::NegativeSpan`. A span of negative
length is not a span, and allocating children within one would produce
negative durations that sum correctly and mean nothing.

This was found by `tests/robustness.rs`, which also exposed the related defect
it replaced: with the default policy - minimum span zero - the feasibility
check `required > parent` fired for any negative parent and reported
`MinimumSpan { required_ticks: 0, available_ticks: -4 }`. Nothing required zero
ticks and failed to get them, so the reason was nonsense even though the
refusal happened to be right. The check is now guarded on the minimum actually
being positive, and the input is validated where it should have been.
