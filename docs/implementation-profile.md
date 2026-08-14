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

Objects that reference shared context - monzos, mappings, events - are not
serializable yet. Inlining a basis definition into every monzo would violate
section 6.3; referencing it needs the registry that arrives with
`TheoryContext`.

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

## D13. `divisions = 0` is a legal mapping

The zero mapping has image `{0}`, which section 1.6 fixes through the
convention `gcd(0, ..., 0) = 0`. In the general API its image has rank 0 and
an image element has an empty coordinate vector, which is the correct answer.
Only the rank-one scalar convenience API of `PatentVal` has nothing to return,
and it reports `TrivialImage` rather than pretending the answer is zero.

## D14. Rank-0 bases are permitted

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
- Matrices, lattices, and mappings are not serializable yet. The canonical
  integer-matrix encoding is defined once, with the native container (UMT-3.2
  section 10.9), rather than ad hoc per type.
- Smith normal form uses the textbook pivot-and-reduce algorithm, whose
  intermediate entries can grow well beyond the input size. Exactness is never
  at risk, since every entry is a `Z`, but a modular or fraction-free variant
  would be wanted before large matrices become common.
