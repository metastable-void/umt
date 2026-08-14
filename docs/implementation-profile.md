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

## D28. Physical time arrives before structural time

`src/time/` currently holds only `ClockTime`, `Seconds`, and `TimeSpan`,
because section 4.7 needs a trajectory domain and the rhythm layer is not
built. Structural beat time will be a *different* exact type, not a
constructor of these: section 5.8.3 is explicit that a tempo map is not the
same kind of object as a pitch tuning, and one shared timeline type would
erase the distinction the map exists to express.

## D29. `divisions = 0` is a legal mapping

The zero mapping has image `{0}`, which section 1.6 fixes through the
convention `gcd(0, ..., 0) = 0`. In the general API its image has rank 0 and
an image element has an empty coordinate vector, which is the correct answer.
Only the rank-one scalar convenience API of `PatentVal` has nothing to return,
and it reports `TrivialImage` rather than pretending the answer is zero.

## D30. Rank-0 bases are permitted

A basis with no generators spans the trivial lattice. Nothing breaks, and
rejecting it would be an invented restriction.

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
- `ProvenanceId` exists without the record arena it refers to.
- The native container of UMT-3.2 section 8.8 does not exist yet: the pieces
  it will assemble - the schema version, the exact text codec, the reference
  forms - are in place, but nothing writes a whole document.
- `ProvenanceRecord` and its arena are not implemented, so `ProvenanceId`
  currently points at nothing.
- `TheoryContext` registers bases, ambient lattices, and mappings. Named
  representative policies, tunings, and notation systems join it as those
  layers arrive.
- Smith normal form uses the textbook pivot-and-reduce algorithm, whose
  intermediate entries can grow well beyond the input size. Exactness is never
  at risk, since every entry is a `Z`, but a modular or fraction-free variant
  would be wanted before large matrices become common.
