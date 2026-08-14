# umt

An implementation of **UMT-3.2** (Unified Music Theory, Third Design, revision 3.2):
exact symbolic proportion, structural quotient, metric realization, and device
approximation kept as *different* semantic layers, with every loss and policy
choice made explicit.

## Status

Early. Implemented: the exact proportion core, exact integer matrices with
Smith and canonical Hermite normal forms, free lattices, regular temperament
mappings with their image and kernel lattices, comma-subgroup saturation
validation, equal divisions, homomorphic splittings, representative policies,
and the structural quotient lens. Not implemented: complexity profiles, pitch,
time, score, realization, and the native container. See `docs/architecture.md`
for the staging plan and `docs/conformance.md` for which UMT-3.2 fixtures are
covered.

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
