//! Malformed-input robustness (prompt section 48).
//!
//! Prompt section 48 asks for fuzz targets "if practical, or at least
//! fuzz-ready parsers", and states one hard requirement: "Never panic on
//! malformed untrusted serialized input unless the process is explicitly
//! configured to treat invalid data as fatal."
//!
//! This crate is not configured that way, so every entry point that accepts
//! untrusted bytes must return an error rather than panic. These tests are a
//! deterministic stand-in for a fuzzer: a small reproducible pseudo-random
//! generator drives each parser over tens of thousands of malformed inputs,
//! alongside hand-written adversarial cases for the shapes a random generator
//! is unlikely to find.
//!
//! Determinism matters here. A failing case must be reproducible from the
//! test name alone, so the generator is seeded from a constant and never from
//! a clock.

use umt::algebra::{IntMatrix, Q, RoundingConvention, Sublattice, Z};
use umt::time::{AllocationPolicy, BeatSpan, BeatTime, Beats, RhythmTree, TickGrid};

/// A small xorshift generator, so failures reproduce exactly.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 { 0 } else { self.next() % bound }
    }

    /// A string of bytes drawn from a character set that makes near-misses
    /// likely: digits, separators, and the characters the parsers care about.
    #[cfg(any(feature = "scala", feature = "serde"))]
    fn soup(&mut self, max_len: usize) -> String {
        const ALPHABET: &[u8] = b"0123456789/.-+ \t\n!eE_,:{}[]\"abc";
        let len = self.below(max_len as u64 + 1) as usize;
        (0..len)
            .map(|_| ALPHABET[self.below(ALPHABET.len() as u64) as usize] as char)
            .collect()
    }
}

#[cfg(feature = "scala")]
#[test]
fn the_scala_parser_never_panics_on_untrusted_text() {
    use umt::io::scala::ScalaScale;

    // Hand-written adversarial shapes first.
    for text in [
        "",
        "!",
        "!\n",
        "\n\n\n",
        "desc",
        "desc\n",
        "desc\n 0\n",
        "desc\nnot a number\n",
        "desc\n 1\n",
        "desc\n 1\n 3/0\n",
        "desc\n 1\n -1\n",
        "desc\n 1\n 0\n",
        "desc\n 1\n 0.0\n",
        "desc\n 1\n .\n",
        "desc\n 1\n 1e999\n",
        "desc\n 1\n 99999999999999999999999999/1\n",
        "desc\n 99999999999999999999\n",
        "desc\n 2\n 1/2\n",
        "desc\n 1\n 1/2 3/4\n",
        "desc\n 1\n NaN\n",
        "desc\n 1\n inf\n",
        "desc\n 1\n -0.0\n",
        "\u{0}\u{1}\u{2}",
    ] {
        // The contract is "returns", not "succeeds".
        let _ = ScalaScale::parse(text);
    }

    // Then a deterministic sweep.
    let mut rng = Rng::new(0x5ca1_ab1e);
    for _ in 0..20_000 {
        let text = rng.soup(60);
        if let Ok(scale) = ScalaScale::parse(&text) {
            // Anything that parses must also survive being written and read
            // back, which is a much stronger property than not panicking.
            let round_tripped =
                ScalaScale::parse(&scale.to_scl_text()).expect("output of the writer must parse");
            assert_eq!(round_tripped, scale, "round trip failed for {text:?}");
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn native_deserialization_never_panics_on_untrusted_json() {
    use umt::io::document::UmtDocument;
    use umt::score::ScoreRef;
    use umt::time::{Meter, TempoMap};

    // Structurally plausible but semantically wrong documents.
    for text in [
        "",
        "{}",
        "null",
        "[]",
        "\"\"",
        "0",
        r#"{"umt_version":"UMT-3.2"}"#,
        r#"{"umt_version":"UMT-3.2","schema":{"major":0,"minor":1}}"#,
        r#"{"umt_version":"UMT-3.2","schema":{"major":999,"minor":0},"profiles":[]}"#,
        r#"{"umt_version":"UMT-3.2","schema":{"major":0,"minor":1},"profiles":[],"unit":{"basis":"b","exponents":["1"]}}"#,
        r#"{"umt_version":"x","schema":{"major":0,"minor":1},"profiles":[],"rhythm_trees":[{"weight":"0/1","children":[]}]}"#,
        r#"{"umt_version":"x","schema":{"major":0,"minor":1},"profiles":[],"rhythm_trees":[{"weight":"1/0","children":[]}]}"#,
        r#"{"umt_version":"x","schema":{"major":0,"minor":1},"profiles":[],"extensions":{"k":{"Rational":"1/0"}}}"#,
    ] {
        // If it deserializes, validation must still be reachable without
        // panicking.
        if let Ok(document) = serde_json::from_str::<UmtDocument>(text) {
            let _ = document.validate();
            let _ = document.represented_layers();
            let _ = document.unsupported_profiles();
            let _ = serde_json::to_string(&document);
        }
    }

    // Deeply nested rhythm trees: a recursion-depth probe.
    let mut nested = String::from(r#"{"weight":"1/1","children":["#);
    for _ in 0..200 {
        nested.push_str(r#"{"weight":"1/1","children":["#);
    }
    for _ in 0..200 {
        nested.push_str("]},");
    }
    nested.pop();
    nested.push_str("]}");
    let _ = serde_json::from_str::<RhythmTree>(&nested);

    // A deterministic sweep over each type that has a wire form.
    let mut rng = Rng::new(0x0000_0d0c_u64);
    for _ in 0..8_000 {
        let text = rng.soup(48);
        let _ = serde_json::from_str::<UmtDocument>(&text);
        let _ = serde_json::from_str::<RhythmTree>(&text);
        let _ = serde_json::from_str::<Meter>(&text);
        let _ = serde_json::from_str::<TempoMap>(&text);
        let _ = serde_json::from_str::<ScoreRef>(&text);
        let _ = serde_json::from_str::<IntMatrix>(&text);
    }
}

#[test]
fn malformed_lattice_dimensions_are_rejected_rather_than_trusted() {
    // A shape that disagrees with the data length.
    assert!(IntMatrix::new(2, 3, vec![Z::from(1)]).is_err());
    assert!(
        IntMatrix::new(0, 5, Vec::new()).is_ok(),
        "an empty shape is legal"
    );
    assert!(IntMatrix::from_rows([vec![1i64, 2], vec![3]]).is_err());

    // A sublattice over a mismatched ambient rank.
    let generators = IntMatrix::from_rows([[1i64, 0], [0, 1]]).unwrap();
    let lattice = Sublattice::from_generators(2, &generators).unwrap();
    assert!(lattice.coordinates(&[Z::from(1)]).is_err(), "wrong rank");
    assert!(Sublattice::from_generators(1, &generators).is_err());
    assert!(Sublattice::from_generators(99, &generators).is_err());
}

#[test]
fn extreme_integers_do_not_overflow_or_panic() {
    // Numbers far beyond any machine word, through the exact paths.
    let huge = Z::from(10i64).pow(400);
    let matrix =
        IntMatrix::from_rows([[huge.clone(), Z::from(1)], [Z::from(1), huge.clone()]]).unwrap();
    let lattice = Sublattice::from_generators(2, &matrix).unwrap();
    assert_eq!(lattice.rank(), 2);
    assert!(lattice.contains(&[huge.clone(), Z::from(1)]).unwrap());

    // Exact rounding of a rational with enormous terms.
    let ratio = Q::new(huge.clone() + Z::from(1), huge.clone());
    for convention in [
        RoundingConvention::Floor,
        RoundingConvention::Ceiling,
        RoundingConvention::NearestHalfAwayFromZero,
        RoundingConvention::NearestHalfToEven,
    ] {
        let rounded = convention.apply_q(&ratio);
        assert!(rounded == Z::from(1) || rounded == Z::from(2), "{rounded}");
    }

    // A structural time with an enormous denominator.
    let tiny = Beats::new(Q::new(Z::from(1), huge));
    assert!(tiny.is_positive());
    assert!(!tiny.is_zero());
}

#[test]
fn pathological_rhythm_trees_flatten_without_panicking() {
    // A deeply nested chain: 300 levels, one child each.
    let mut tree = RhythmTree::leaf(1).unwrap();
    for _ in 0..300 {
        tree = RhythmTree::division([tree]).unwrap();
    }
    let span = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(1, 1).unwrap()).unwrap();
    let leaves = tree.flatten(&span).unwrap();
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].depth(), 300);

    // A very wide division with wildly unequal weights.
    let wide =
        RhythmTree::division((1..=200).map(|weight| RhythmTree::leaf(weight).unwrap())).unwrap();
    let leaves = wide.flatten(&span).unwrap();
    assert_eq!(leaves.len(), 200);
    let total: Beats = leaves.iter().map(|leaf| leaf.span().duration()).sum();
    assert_eq!(total, Beats::ratio(1, 1).unwrap(), "still exact");

    // Weights that are enormous rationals.
    let heavy = RhythmTree::division([
        RhythmTree::weighted_leaf(Q::new(Z::from(10i64).pow(60), Z::from(7))).unwrap(),
        RhythmTree::leaf(1).unwrap(),
    ])
    .unwrap();
    assert_eq!(heavy.flatten(&span).unwrap().len(), 2);
}

#[test]
fn quantization_edge_cases_are_answered_rather_than_assumed() {
    let grid = TickGrid::new(1).unwrap();
    let policy = AllocationPolicy::default();

    // A single child, a zero-tick parent, and a huge parent.
    for parent in [0i64, 1, 1_000_000] {
        let outcome = grid
            .allocate_preserving_endpoint(&[Q::from(Z::from(1))], &Z::from(parent), &policy)
            .unwrap();
        let allocation = outcome.into_allocation().expect("no minimum was declared");
        assert_eq!(allocation.total_ticks(), Z::from(parent));
    }

    // A negative parent span is not a span, and is refused rather than
    // allocated into negative child durations that happen to sum correctly.
    assert_eq!(
        grid.allocate_preserving_endpoint(
            &[Q::from(Z::from(1)), Q::from(Z::from(1))],
            &Z::from(-4),
            &policy,
        ),
        Err(umt::error::TimeError::NegativeSpan)
    );
    assert!(
        grid.allocate_locally(
            &[Q::from(Z::from(1))],
            &Z::from(-1),
            RoundingConvention::Floor
        )
        .is_err()
    );

    // Weights spanning many orders of magnitude.
    let weights = [
        Q::new(Z::from(1), Z::from(10i64).pow(30)),
        Q::from(Z::from(1)),
    ];
    let allocation = grid
        .allocate_preserving_endpoint(&weights, &Z::from(96), &policy)
        .unwrap()
        .into_allocation()
        .unwrap();
    assert_eq!(allocation.total_ticks(), Z::from(96));
    assert!(allocation.endpoint_preserved());

    // Quantizing positions with enormous denominators.
    let fine = TickGrid::new(u32::MAX).unwrap();
    let at = BeatTime::new(Q::new(Z::from(1), Z::from(10i64).pow(40)));
    assert_eq!(
        fine.quantize(&at, RoundingConvention::Floor).value,
        Z::from(0)
    );
    assert_eq!(
        fine.quantize(&at, RoundingConvention::Ceiling).value,
        Z::from(1)
    );
}

#[test]
fn temporal_constraint_graphs_terminate_on_adversarial_input() {
    use umt::algebra::Q as Rational;
    use umt::time::{DifferenceConstraint, StpProblem};

    // A dense graph with every pair constrained in both directions.
    let mut problem = StpProblem::new();
    let vars: Vec<_> = (0..12)
        .map(|index| problem.variable(&format!("t{index}")))
        .collect();
    let mut rng = Rng::new(0x00c0_f05e);
    for from in &vars {
        for to in &vars {
            if from == to {
                continue;
            }
            let lower = rng.below(21) as i64 - 10;
            problem
                .constrain(DifferenceConstraint::between(
                    from,
                    to,
                    Some(Rational::from(Z::from(lower))),
                    Some(Rational::from(Z::from(lower + rng.below(11) as i64))),
                ))
                .unwrap();
        }
    }
    // Whatever the verdict, it terminates and is self-consistent.
    let outcome = problem.solve();
    if let Some(assignment) = outcome.assignment() {
        assert_eq!(assignment.len(), vars.len());
    }

    // A long negative cycle, which must be reported rather than looped over.
    let mut cyclic = StpProblem::new();
    let ring: Vec<_> = (0..40)
        .map(|index| cyclic.variable(&format!("r{index}")))
        .collect();
    for index in 0..ring.len() {
        cyclic
            .constrain(DifferenceConstraint::at_most(
                &ring[index],
                &ring[(index + 1) % ring.len()],
                Rational::from(Z::from(-1)),
            ))
            .unwrap();
    }
    assert!(!cyclic.solve().is_consistent(), "a negative cycle");
}

#[test]
fn arbitrary_temperament_matrices_are_accepted_or_refused_without_panicking() {
    use umt::temperament::AmbientLattice;
    use umt::{Basis, TemperamentMap};

    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
    let mut rng = Rng::new(0xdeadbeef);

    for _ in 0..2_000 {
        let rows = rng.below(4) as usize;
        let ambient = AmbientLattice::new("umt:fuzz", rows);
        let entries: Vec<Vec<i64>> = (0..rows)
            .map(|_| {
                (0..3)
                    .map(|_| rng.below(41) as i64 - 20)
                    .collect::<Vec<i64>>()
            })
            .collect();
        let Ok(map) = TemperamentMap::from_rows(&basis, &ambient, entries) else {
            continue;
        };
        // Every derived structure is computed eagerly, so if construction
        // succeeded these are all reachable.
        let _ = map.image().rank();
        let _ = map.kernel().rank();
        let _ = map.is_surjective();
        assert!(
            map.kernel().is_saturated(),
            "map-derived kernels are saturated"
        );
        assert_eq!(
            map.image().rank() + map.kernel().rank(),
            basis.rank(),
            "rank-nullity"
        );
    }

    // A matrix of the wrong shape for its declared lattice.
    let ambient = AmbientLattice::new("umt:fuzz", 2);
    assert!(TemperamentMap::from_rows(&basis, &ambient, [[1i64, 2, 3]]).is_err());
    assert!(TemperamentMap::from_rows(&basis, &ambient, [[1i64, 2]; 2]).is_err());
}
