//! Serialization invariants (UMT-3.2 section 8.9, prompt section 39).
//!
//! Only self-contained definition objects serialize today; see `src/io/mod.rs`
//! for why objects that reference shared context wait for the native
//! container.

#![cfg(feature = "serde")]

use umt::proportion::{
    Basis, IndependenceContract, NonNegativeFinite, PositiveFinite, PositiveQ, RawBasis,
    RealValuation,
};
use umt::realization::provenance::ProvenanceId;
use umt::{Q, Z};

fn huge_rational() -> Q {
    // A value no floating-point encoding could carry.
    Q::new(Z::from(10).pow(60) + Z::from(1), Z::from(3).pow(50))
}

#[test]
fn arbitrary_size_exact_values_survive_a_round_trip() {
    let value = PositiveQ::new(huge_rational()).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    let back: PositiveQ = serde_json::from_str(&json).unwrap();
    assert_eq!(back, value);
    assert_eq!(back.value(), &huge_rational());
}

#[test]
fn exact_values_are_never_written_as_floating_point() {
    let value = PositiveQ::new(huge_rational()).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    // A quoted canonical `numerator/denominator` string, not a JSON number.
    assert!(json.starts_with('"') && json.ends_with('"'), "{json}");
    assert!(json.contains('/'), "{json}");
    assert!(!json.contains('e') && !json.contains('.'), "{json}");
}

#[test]
fn a_basis_round_trips_with_its_identity_and_contract() {
    let basis = Basis::builder("umt:basis:example")
        .rational_generator("2", PositiveQ::integer(2).unwrap())
        .rational_generator("big", PositiveQ::new(huge_rational()).unwrap())
        .symbolic_real_generator(
            "measured",
            RealValuation::new(PositiveFinite::new(1.618_033_988_749_895).unwrap())
                .with_uncertainty(NonNegativeFinite::new(0.000_5).unwrap())
                .with_provenance(ProvenanceId::new("umt:prov:measurement-17")),
        )
        .independence(IndependenceContract::Declared {
            note: "modelling assumption".into(),
        })
        .build()
        .unwrap();

    let json = serde_json::to_string(&*basis).unwrap();
    let back: Basis = serde_json::from_str(&json).unwrap();

    assert!(back.same_identity(&basis));
    assert_eq!(back.rank(), 3);
    assert!(!back.is_rational_profile());
    assert_eq!(
        back.independence(),
        &IndependenceContract::Declared {
            note: "modelling assumption".into()
        }
    );
}

#[test]
fn loading_revalidates_invariants() {
    // A duplicate generator identity must be rejected on load, not accepted
    // because it arrived over the wire.
    let basis = Basis::builder("umt:basis:example")
        .rational_generator("2", PositiveQ::integer(2).unwrap())
        .build()
        .unwrap();

    let mut raw = RawBasis::from((*basis).clone());
    let duplicate = raw.generators[0].clone();
    raw.generators.push(duplicate);

    let json = serde_json::to_string(&raw).unwrap();
    let result: Result<Basis, _> = serde_json::from_str(&json);
    assert!(
        result.is_err(),
        "duplicate generator identity must be rejected"
    );
}

#[test]
fn out_of_range_values_are_rejected_on_load() {
    let result: Result<PositiveQ, _> = serde_json::from_str("\"-3/2\"");
    assert!(
        result.is_err(),
        "a negative exact valuation must be rejected"
    );

    let result: Result<PositiveQ, _> = serde_json::from_str("\"3/0\"");
    assert!(result.is_err(), "a zero denominator must be rejected");

    let result: Result<PositiveFinite, _> = serde_json::from_str("0.0");
    assert!(result.is_err(), "a zero real valuation must be rejected");
}
