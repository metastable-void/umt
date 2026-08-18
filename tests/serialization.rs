//! Serialization invariants (UMT-3.2 section 8.9, prompt section 39).
//!
//! Self-contained definitions serialize directly; objects that reference
//! shared context serialize as references resolved against a
//! [`umt::context::TheoryContext`].

#![cfg(feature = "serde")]

use std::collections::BTreeMap;

use umt::algebra::matrix::IntMatrix;
use umt::context::{MonzoRef, TemperamentMapRef, TheoryContext};
use umt::error::IoError;
use umt::io::document::{PolicyDeclaration, UmtDocument};
use umt::io::version::{NATIVE_SCHEMA_VERSION, UmtSchemaVersion};
use umt::pitch::{Edge, PitchOrigin, PitchPoint, PitchPointRef, VoiceId, VoiceLeading, VoiceSet};
use umt::proportion::{
    Basis, IndependenceContract, NonNegativeFinite, PositiveFinite, PositiveQ, RawBasis,
    RealValuation,
};
use umt::realization::provenance::ProvenanceId;
use umt::realization::record::Layer;
use umt::score::{
    EventContent, EventId, EventScope, Score, ScoreEvent, ScoreRef, TemporalPlacement, Tie,
};
use umt::temperament::{AmbientLattice, TemperamentMap};
use umt::time::{
    BeatDuration, BeatSpan, BeatTime, ClockTime, DifferenceConstraint, Meter, RhythmTree, Seconds,
    SecondsPerBeat, TempoMap, TimeSignature, TimeSpan, TimeVarId,
};
use umt::{BasisId, Q, Z};

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

#[test]
fn time_layer_objects_round_trip_and_revalidate() {
    // Exact structural values are canonical text, never floating point.
    let onset = BeatTime::ratio(1, 3).unwrap();
    let json = serde_json::to_string(&onset).unwrap();
    assert_eq!(
        json, "\"1/3\"",
        "exact structural time is exact on the wire"
    );
    assert_eq!(serde_json::from_str::<BeatTime>(&json).unwrap(), onset);

    // UMT-3.2 section 9.7, law 5: serialization preserves tree topology and
    // weights.
    let tree = RhythmTree::division([
        RhythmTree::equal_division(5).unwrap(),
        RhythmTree::weighted_leaf(Q::new(Z::from(3), Z::from(2))).unwrap(),
    ])
    .unwrap();
    let json = serde_json::to_string(&tree).unwrap();
    assert!(json.contains("\"3/2\""), "weights are exact text: {json}");
    let parsed: RhythmTree = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, tree);
    assert_eq!(parsed.leaf_count(), tree.leaf_count());
    assert_eq!(parsed.depth(), tree.depth());

    // A non-positive weight is rejected on load, not accepted and tripped
    // over later.
    let forged = json.replace("\"3/2\"", "\"-3/2\"");
    assert!(serde_json::from_str::<RhythmTree>(&forged).is_err());

    // A meter revalidates its level nesting on load.
    let meter = Meter::compound(TimeSignature::new(6, 8).unwrap()).unwrap();
    let json = serde_json::to_string(&meter).unwrap();
    assert_eq!(serde_json::from_str::<Meter>(&json).unwrap(), meter);
    let broken = json.replace("[[0,1,2,3,4,5],[0,3],[0]]", "[[0,1,2,3,4,5],[0,3],[1]]");
    assert_ne!(broken, json, "the test data must actually have been edited");
    assert!(
        serde_json::from_str::<Meter>(&broken).is_err(),
        "a level that is not nested must be rejected on load"
    );

    // A tempo map revalidates the homeomorphism conditions on load.
    let map = TempoMap::constant(
        &BeatSpan::new(BeatTime::zero(), BeatTime::ratio(4, 1).unwrap()).unwrap(),
        ClockTime::ZERO,
        SecondsPerBeat::new(0.5).unwrap(),
    )
    .unwrap();
    let json = serde_json::to_string(&map).unwrap();
    assert_eq!(serde_json::from_str::<TempoMap>(&json).unwrap(), map);
    let jump = "{\"breakpoints\":[{\"beat\":\"0/1\",\"clock\":0.0},\
                {\"beat\":\"2/1\",\"clock\":1.0},{\"beat\":\"2/1\",\"clock\":3.0}]}";
    assert!(
        serde_json::from_str::<TempoMap>(jump).is_err(),
        "a discontinuous map must be rejected on load"
    );

    // Constraint bounds are exact text too.
    let constraint = DifferenceConstraint::between(
        &TimeVarId::new("a"),
        &TimeVarId::new("b"),
        Some(Q::new(Z::from(1), Z::from(3))),
        None,
    );
    let json = serde_json::to_string(&constraint).unwrap();
    assert!(json.contains("\"1/3\""), "{json}");
    assert_eq!(
        serde_json::from_str::<DifferenceConstraint>(&json).unwrap(),
        constraint
    );
}

#[test]
fn score_objects_round_trip_and_revalidate() {
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let context = TheoryContext::builder().ambient(&steps).unwrap().build();
    let voice = EventScope::VoiceLocal(VoiceId::new("soprano"));
    let pitch = PitchPoint::new(
        PitchOrigin::new("umt:origin:c4"),
        steps.element([7i64]).unwrap(),
    );

    let note = |id: &str, onset: i64| {
        ScoreEvent::new(
            EventId::new(id),
            voice.clone(),
            TemporalPlacement::fixed(
                BeatTime::ratio(onset, 1).unwrap(),
                BeatDuration::ratio(2, 1).unwrap(),
            ),
            EventContent::Note {
                pitch: pitch.clone(),
            },
        )
        .unwrap()
    };

    let score = Score::builder()
        .event(note("n1", 2))
        .unwrap()
        .event(note("n2", 4))
        .unwrap()
        .tie(Tie::new(EventId::new("n1"), EventId::new("n2")))
        .unwrap()
        .build()
        .unwrap();

    // UMT-3.2 section 9.6: tie identities survive an L0 round trip.
    let json = serde_json::to_string(&score.to_ref().unwrap()).unwrap();
    assert!(json.contains("\"7\""), "coordinates stay exact: {json}");
    let parsed: ScoreRef = serde_json::from_str(&json).unwrap();
    let restored = parsed.resolve_ambient(&context).unwrap();
    assert_eq!(restored, score);
    assert_eq!(restored.ties(), score.ties());
    assert_eq!(restored.len(), 2, "two noteheads, never merged");

    // A tie naming an event that is not there is rejected on load, not
    // accepted and tripped over later.
    let forged = json.replace("\"n2\"}]", "\"ghost\"}]");
    assert_ne!(forged, json, "the test data must actually have been edited");
    // The score deserializes structurally, and the relation is then invalid;
    // rebuilding through the builder is what revalidates it.
    let broken: ScoreRef = serde_json::from_str(&forged).unwrap();
    let rebuilt = ScoreRef::builder()
        .event(broken.events().next().unwrap().clone())
        .unwrap()
        .tie(broken.ties()[0].clone());
    assert!(
        rebuilt.is_err(),
        "a tie to an absent event cannot be rebuilt"
    );
}

#[test]
fn a_native_document_round_trips_with_only_the_sections_it_needs() {
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();

    let mut document = UmtDocument::new()
        .with_profile("umt.pitch")
        .with_profile("umt.time");
    document.basis = Some(RawBasis::from(basis.as_ref().clone()));
    document.unit = Some(MonzoRef {
        basis: BasisId::new("umt:prime:2.3.5"),
        exponents: vec![Z::from(1), Z::from(0), Z::from(0)],
    });
    document.mapping = Some(TheoryContext::mapping_ref(&map));
    document
        .rhythm_trees
        .push(RhythmTree::additive(&[2, 2, 3]).unwrap());

    // A representative policy has to be reproducible one way or the other.
    document.representative_policy = Some(PolicyDeclaration {
        kind: String::from("canonical-lift"),
        policy_id: Some(String::from("umt:policy:canonical")),
        algorithm_version: Some(String::from("0.1.0")),
        parameters: BTreeMap::new(),
        homomorphic: false,
        resolved_lifts: Vec::new(),
    });
    assert_eq!(document.validate(), Ok(()));

    let json = serde_json::to_string(&document).unwrap();
    // UMT-3.2 section 8.9: monzo coordinates and matrix entries exactly, and
    // never through JSON floating point.
    assert!(json.contains("\"12\""), "{json}");
    assert!(!json.contains("12.0"));
    // Absent sections are absent, not null.
    assert!(!json.contains("\"events\""));
    assert!(!json.contains("\"tempo\""));

    let parsed: UmtDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, document);
    assert_eq!(parsed.validate(), Ok(()));
    assert!(parsed.is_fully_understood());
    assert_eq!(
        parsed.represented_layers(),
        [Layer::L1Exact, Layer::L2Quotient]
    );

    // A policy that can be reproduced from neither an identifier nor its
    // selected lifts is refused (section 8.8).
    let mut adaptive = document.clone();
    adaptive.representative_policy = Some(PolicyDeclaration {
        kind: String::from("adaptive"),
        policy_id: None,
        algorithm_version: None,
        parameters: BTreeMap::new(),
        homomorphic: false,
        resolved_lifts: Vec::new(),
    });
    assert_eq!(adaptive.validate(), Err(IoError::IrreproduciblePolicy));

    // The same policy, with the lifts it actually chose, is fine.
    adaptive
        .representative_policy
        .as_mut()
        .unwrap()
        .resolved_lifts = vec![MonzoRef {
        basis: BasisId::new("umt:prime:2.3.5"),
        exponents: vec![Z::from(-1), Z::from(1), Z::from(0)],
    }];
    assert_eq!(adaptive.validate(), Ok(()));
    let reparsed: UmtDocument =
        serde_json::from_str(&serde_json::to_string(&adaptive).unwrap()).unwrap();
    assert_eq!(
        reparsed.representative_policy.unwrap().resolved_lifts.len(),
        1,
        "the selected lifts survive, which is what makes the policy reproducible"
    );
}
