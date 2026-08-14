//! Serialization invariants (UMT-3.2 section 8.9, prompt section 39).
//!
//! Self-contained definitions serialize directly; objects that reference
//! shared context serialize as references resolved against a
//! [`umt::context::TheoryContext`].

#![cfg(feature = "serde")]

use umt::algebra::matrix::IntMatrix;
use umt::context::{MonzoRef, TemperamentMapRef, TheoryContext};
use umt::io::version::{NATIVE_SCHEMA_VERSION, UmtSchemaVersion};
use umt::pitch::{Edge, PitchOrigin, PitchPoint, PitchPointRef, VoiceId, VoiceLeading, VoiceSet};
use umt::proportion::{
    Basis, IndependenceContract, NonNegativeFinite, PositiveFinite, PositiveQ, RawBasis,
    RealValuation,
};
use umt::realization::provenance::ProvenanceId;
use umt::temperament::{AmbientLattice, TemperamentMap};
use umt::time::{Seconds, TimeSpan};
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
fn matrices_round_trip_with_exact_entries() {
    let big = Z::from(10).pow(40);
    let matrix = IntMatrix::new(2, 2, vec![big.clone(), -&big, Z::from(0), Z::from(7)]).unwrap();

    let json = serde_json::to_string(&matrix).unwrap();
    assert!(
        json.contains('"'),
        "entries are text, not JSON numbers: {json}"
    );
    assert!(!json.contains("e+"), "no scientific notation: {json}");

    let back: IntMatrix = serde_json::from_str(&json).unwrap();
    assert_eq!(back, matrix);

    // The shape invariant is revalidated on load.
    let malformed = json.replace("\"rows\":2", "\"rows\":3");
    assert!(serde_json::from_str::<IntMatrix>(&malformed).is_err());
}

#[test]
fn context_dependent_objects_serialize_as_references() {
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
    let context = TheoryContext::builder()
        .mapping("umt:map:12edo-5limit", &map)
        .unwrap()
        .build();

    // A monzo on the wire names its basis rather than inlining it.
    let comma = basis.monzo([-4, 4, -1]).unwrap();
    let reference = TheoryContext::monzo_ref(&comma);
    let json = serde_json::to_string(&reference).unwrap();
    assert!(json.contains("umt:prime:2.3.5"));
    assert!(
        !json.contains("independence"),
        "the basis definition must not be copied into every monzo: {json}"
    );

    let back: MonzoRef = serde_json::from_str(&json).unwrap();
    assert_eq!(context.resolve_monzo(&back).unwrap(), comma);

    // Mappings likewise.
    let reference = TheoryContext::mapping_ref(&map);
    let json = serde_json::to_string(&reference).unwrap();
    let back: TemperamentMapRef = serde_json::from_str(&json).unwrap();
    assert_eq!(context.resolve_mapping(&back).unwrap(), map);
}

#[test]
fn a_reference_without_its_context_cannot_be_resolved() {
    let empty = TheoryContext::builder().build();
    let reference = MonzoRef {
        basis: umt::BasisId::new("umt:prime:2.3.5"),
        exponents: vec![Z::from(1), Z::from(0), Z::from(0)],
    };
    assert!(
        empty.resolve_monzo(&reference).is_err(),
        "a reference is meaningless without the context that defines it"
    );
}

#[test]
fn the_schema_version_travels_with_documents() {
    let json = serde_json::to_string(&NATIVE_SCHEMA_VERSION).unwrap();
    let back: UmtSchemaVersion = serde_json::from_str(&json).unwrap();
    assert_eq!(back, NATIVE_SCHEMA_VERSION);
    assert!(NATIVE_SCHEMA_VERSION.can_read(back));
    assert_eq!(NATIVE_SCHEMA_VERSION.spec_profile(), umt::UMT_SPEC_VERSION);
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

#[test]
fn pitch_layer_objects_round_trip_and_revalidate() {
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let context = TheoryContext::builder().ambient(&steps).unwrap().build();

    // A point carries an origin, a lattice reference, and exact coordinates.
    let point = PitchPoint::new(
        PitchOrigin::new("umt:origin:a4"),
        steps.element([7i64]).unwrap(),
    );
    let json = serde_json::to_string(&PitchPointRef::of_ambient(&point)).unwrap();
    assert!(
        json.contains("\"7\""),
        "coordinates are exact decimal text, not floating point: {json}"
    );
    let parsed: PitchPointRef = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.resolve_ambient(&context).unwrap(), point);

    // A voice-leading span revalidates its edges against its voice sets.
    let span = VoiceLeading::new(
        VoiceSet::new([VoiceId::new("a"), VoiceId::new("b")]).unwrap(),
        VoiceSet::new([VoiceId::new("x")]).unwrap(),
        [Edge::new(VoiceId::new("a"), VoiceId::new("x"))],
    )
    .unwrap();
    let json = serde_json::to_string(&span).unwrap();
    assert_eq!(serde_json::from_str::<VoiceLeading>(&json).unwrap(), span);

    let forged = json.replace("\"a\",\"target\"", "\"ghost\",\"target\"");
    assert_ne!(forged, json, "the test data must actually have been edited");
    assert!(
        serde_json::from_str::<VoiceLeading>(&forged).is_err(),
        "an edge naming a voice outside the source set must be rejected on load"
    );

    // A time span revalidates its ordering.
    let reversed = "{\"start\":2.0,\"end\":1.0}";
    assert!(
        serde_json::from_str::<TimeSpan>(reversed).is_err(),
        "a reversed span must be rejected on load"
    );
    let forwards: TimeSpan = serde_json::from_str("{\"start\":1.0,\"end\":2.0}").unwrap();
    assert_eq!(forwards.duration(), Seconds::new(1.0).unwrap());
}
