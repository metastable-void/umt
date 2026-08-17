//! Property-based tests for the algebraic laws of UMT-3.2 part IX
//! (prompt section 47).
//!
//! Each test names the law it exercises. Laws for structures that are not
//! implemented yet - quantization, rhythm trees, tempo maps, temporal
//! constraints - are absent rather than stubbed.

use std::sync::Arc;

use proptest::prelude::*;
use umt::algebra::integer::round_n_log2;
use umt::algebra::normal_form::{HermiteNormalForm, SmithNormalForm};
use umt::pitch::{
    AdmissibleFamily, Chord, ChordDistance, CostQuestion, Edge, LogFrequency, LogPitchDistance,
    MetricClaim, Octaves, PitchOrigin, PitchPoint, RegularTuning, SpanCostModel, SpanPenalties,
    TransportProfile, VoiceId, VoiceLeading, VoiceSet,
};
use umt::realization::provenance::ProvenanceId;
use umt::score::{
    EventContent, EventId, EventRelation, EventScope, PitchTransform, ProvenanceChain, Score,
    ScoreEvent, ScoreTransform, TemporalPlacement, Tie, TimeTransform,
};
use umt::temperament::{
    AmbientElem, AmbientLattice, HomomorphicSplit, KernelElem, LinearSplit, OffsetPolicy,
    RepresentativePolicy, SplitPolicy, StructuralLens, TemperamentMap,
};
use umt::time::{
    AllocationPolicy, BeatDuration, BeatSpan, BeatTime, Beats, ClockTime, DifferenceConstraint,
    LinearConstraint, LinearTemporalProblem, OrientedRatio, RatioOrientation, RhythmTree,
    StpProblem, TempoBreakpoint, TempoMap, TickGrid, TimeVarId,
};
use umt::{Basis, IntMatrix, PatentVal, Q, RoundingConvention, Sublattice, Z};

fn five_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).expect("valid prime basis")
}

/// The 12-EDO step lattice, used as the interval group for the pitch-layer
/// laws.
fn twelve_edo() -> Arc<AmbientLattice> {
    AmbientLattice::new("umt:edo:12", 1)
}

fn ground_cost() -> LogPitchDistance<AmbientLattice> {
    LogPitchDistance::new(RegularTuning::equal_divisions(&twelve_edo(), 12).expect("rank 1"))
}

/// A chord whose voices are named `v0, v1, ...` in the order given.
///
/// The target side of a voice leading needs its own names, so it is built with
/// [`named_chord`] and the prefix `w`.
fn chord_of(steps: &[i64]) -> Chord<AmbientElem> {
    named_chord("v", steps)
}

fn named_chord(prefix: &str, steps: &[i64]) -> Chord<AmbientElem> {
    let lattice = twelve_edo();
    let origin = PitchOrigin::new("umt:origin:c4");
    Chord::from_voices(steps.iter().enumerate().map(|(index, step)| {
        (
            VoiceId::new(&format!("{prefix}{index}")),
            PitchPoint::new(
                origin.clone(),
                lattice.element([*step]).expect("rank 1 lattice"),
            ),
        )
    }))
    .expect("distinct voice names, one shared origin")
}

/// Edges in a canonical order, so two spans that differ only in edge order
/// compare equal.
fn sorted_edges(span: &VoiceLeading) -> Vec<Edge> {
    let mut edges = span.edges().to_vec();
    edges.sort();
    edges
}

/// Ordered trees of positive integer weights, up to three levels deep.
fn rhythm_trees() -> impl Strategy<Value = RhythmTree> {
    let leaf = (1i64..=8).prop_map(|weight| RhythmTree::leaf(weight).expect("positive"));
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop::collection::vec(inner, 1..=4)
            .prop_map(|children| RhythmTree::division(children).expect("non-empty"))
    })
}

fn seven_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5.7", &[2, 3, 5, 7]).expect("valid prime basis")
}

fn exponents() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(-64i64..=64, 3)
}

/// Small integer matrices of a given shape.
fn matrices(rows: usize, cols: usize) -> impl Strategy<Value = IntMatrix> {
    prop::collection::vec(-12i64..=12, rows * cols).prop_map(move |data| {
        IntMatrix::new(rows, cols, data.into_iter().map(Z::from).collect())
            .expect("shape matches the generated length")
    })
}

/// A 5-limit mapping into a rank-1 ambient lattice, from three entries.
fn five_limit_map(entries: [i64; 3]) -> TemperamentMap {
    TemperamentMap::from_rows(
        &five_limit(),
        &AmbientLattice::new("umt:property-ambient", 1),
        [entries],
    )
    .expect("shape matches the basis rank")
}

/// A spread of 5-limit mappings: surjective, non-surjective, rank 2, and the
/// zero map, so the policy laws are exercised on every degenerate shape too.
fn sample_maps() -> Vec<TemperamentMap> {
    let basis = five_limit();
    vec![
        five_limit_map([12, 19, 28]),
        five_limit_map([6, 10, 14]),
        five_limit_map([0, 0, 0]),
        TemperamentMap::from_rows(
            &basis,
            &AmbientLattice::new("umt:property-rank2", 2),
            [[1i64, 0, -4], [0, 1, 4]],
        )
        .expect("shape matches the basis rank"),
    ]
}

/// A class of `map` built from as many of `coordinates` as its image needs.
fn class_of(map: &TemperamentMap, coordinates: &[i64]) -> umt::temperament::ImageElem {
    map.image()
        .element(coordinates[..map.image().rank()].to_vec())
        .expect("coordinate count matches the image rank")
}

/// A policy that shifts the lift of every odd-summed class by the first kernel
/// generator, which is a right inverse but not additive.
fn shifting_policy(
    map: &TemperamentMap,
) -> OffsetPolicy<
    SplitPolicy<LinearSplit>,
    impl Fn(&umt::temperament::ImageElem, &()) -> Option<KernelElem>,
> {
    let kernel = map.kernel().clone();
    OffsetPolicy::new(
        SplitPolicy::new(LinearSplit::of(map).expect("a validated mapping splits")),
        move |class: &umt::temperament::ImageElem, _: &()| {
            if kernel.rank() == 0 {
                return None;
            }
            let parity: Z = class.coordinates().iter().sum();
            if (&parity % 2i32) == Z::from(0) {
                return None;
            }
            let mut offset = vec![Z::from(0); kernel.rank()];
            offset[0] = Z::from(1);
            kernel.element(offset).ok()
        },
    )
}

/// Compares `numer / denom` with `2^exponent` using plain integer arithmetic,
/// independently of the implementation under test.
fn cmp_pow2(numer: &Z, denom: &Z, exponent: i64) -> core::cmp::Ordering {
    let (left_shift, right_shift) = if exponent >= 0 {
        (0u64, exponent as u64)
    } else {
        (exponent.unsigned_abs(), 0u64)
    };
    (numer << left_shift).cmp(&(denom << right_shift))
}

proptest! {
    /// Law P1: free-lattice arithmetic is associative.
    #[test]
    fn p1_addition_is_associative(a in exponents(), b in exponents(), c in exponents()) {
        let basis = five_limit();
        let (a, b, c) = (
            basis.monzo(a).unwrap(),
            basis.monzo(b).unwrap(),
            basis.monzo(c).unwrap(),
        );
        let left = a.checked_add(&b).unwrap().checked_add(&c).unwrap();
        let right = a.checked_add(&b.checked_add(&c).unwrap()).unwrap();
        prop_assert_eq!(left, right);
    }

    /// Law P1: zero is neutral and every element has an inverse.
    #[test]
    fn p1_zero_and_inverse(a in exponents()) {
        let basis = five_limit();
        let a = basis.monzo(a).unwrap();
        let zero = basis.zero();
        prop_assert_eq!(a.checked_add(&zero).unwrap(), a.clone());
        prop_assert_eq!(a.checked_add(&-&a).unwrap(), zero);
        prop_assert_eq!(a.checked_sub(&a).unwrap(), basis.zero());
    }

    /// Prompt section 7: monzos over unrelated bases are never combined, and
    /// are never equal, even at equal rank.
    #[test]
    fn basis_mismatch_is_always_rejected(a in exponents(), b in exponents()) {
        let five = five_limit();
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        let a = five.monzo(a).unwrap();
        let b = other.monzo(b).unwrap();
        prop_assert!(a.checked_add(&b).is_err());
        prop_assert!(a.checked_sub(&b).is_err());
        prop_assert!(!a.is_compatible_with(&b));
        prop_assert_ne!(a, b);
    }

    /// Law P2: the exact rational valuation is multiplicative.
    #[test]
    fn p2_valuation_is_multiplicative(a in exponents(), b in exponents()) {
        let basis = five_limit();
        let a = basis.monzo(a).unwrap();
        let b = basis.monzo(b).unwrap();
        let sum = a.checked_add(&b).unwrap();
        prop_assert_eq!(
            sum.exact_ratio().unwrap(),
            a.exact_ratio().unwrap() * b.exact_ratio().unwrap()
        );
    }

    /// Law P3: an equal-division mapping is a homomorphism.
    #[test]
    fn p3_mapping_is_a_homomorphism(
        a in prop::collection::vec(-64i64..=64, 4),
        b in prop::collection::vec(-64i64..=64, 4),
        divisions in 0u32..=200,
    ) {
        let basis = seven_limit();
        let val = PatentVal::new(&basis, divisions, RoundingConvention::NearestHalfAwayFromZero)
            .unwrap();
        let a = basis.monzo(a).unwrap();
        let b = basis.monzo(b).unwrap();
        let sum = a.checked_add(&b).unwrap();
        prop_assert_eq!(
            val.apply(&sum).unwrap(),
            val.apply(&a).unwrap() + val.apply(&b).unwrap()
        );
    }

    /// Law P7: every mapped element lies in the image, and the intrinsic image
    /// coordinate round-trips through the ambient lattice.
    #[test]
    fn p7_image_membership_and_round_trip(
        a in prop::collection::vec(-64i64..=64, 4),
        divisions in 1u32..=200,
    ) {
        let basis = seven_limit();
        let val = PatentVal::new(&basis, divisions, RoundingConvention::NearestHalfAwayFromZero)
            .unwrap();
        let monzo = basis.monzo(a).unwrap();
        let step = val.apply(&monzo).unwrap();
        prop_assert!(val.contains_ambient(&step));
        let coordinate = val.image_coordinate(&step).unwrap();
        prop_assert_eq!(val.embed_image(&coordinate).unwrap(), step);
    }

    /// Law P4: a monzo is in the kernel exactly when it maps to zero.
    #[test]
    fn p4_kernel_membership_iff_mapped_to_zero(
        entries in prop::array::uniform3(-40i64..=40),
        exponents in exponents(),
    ) {
        let map = five_limit_map(entries);
        let monzo = five_limit().monzo(exponents).unwrap();
        prop_assert_eq!(
            map.kills(&monzo).unwrap(),
            map.kernel().contains(&monzo).unwrap()
        );
    }

    /// Law P5: a map-derived kernel is saturated. For every nonzero `n`,
    /// `n m` in `K` implies `m` in `K`.
    ///
    /// This is a theorem for mappings into a free abelian group (UMT-3.2
    /// section 1.4.1), used here as an implementation check. The zero
    /// multiplier is excluded, as fixture F33 requires.
    #[test]
    fn p5_kernel_saturation_for_nonzero_multiples(
        entries in prop::array::uniform3(-40i64..=40),
        exponents in exponents(),
        multiplier in prop::sample::select(vec![-7i64, -3, -2, -1, 1, 2, 3, 5, 12]),
    ) {
        let map = five_limit_map(entries);
        let monzo = five_limit().monzo(exponents).unwrap();
        let multiple = monzo.scale(&Z::from(multiplier));
        if map.kills(&multiple).unwrap() {
            prop_assert!(map.kills(&monzo).unwrap(), "kernel must be saturated");
        }
        prop_assert!(map.kernel().is_saturated());
    }

    /// Law P7: image coordinates round-trip through the ambient lattice for a
    /// general mapping, and every mapped element is reachable.
    #[test]
    fn p7_general_image_round_trip(
        entries in prop::array::uniform3(-40i64..=40),
        exponents in exponents(),
    ) {
        let map = five_limit_map(entries);
        let monzo = five_limit().monzo(exponents).unwrap();
        let ambient = map.apply(&monzo).unwrap();
        prop_assert!(map.image().contains(&ambient).unwrap());
        let image = map.apply_to_image(&monzo).unwrap();
        prop_assert_eq!(map.image().embed(&image).unwrap(), ambient);
    }

    /// Law P8: every representative policy is a right inverse, homomorphic or
    /// not, on every mapping shape.
    #[test]
    fn p8_right_inverse_law(coordinates in prop::collection::vec(-24i64..=24, 2)) {
        for map in sample_maps() {
            let class = class_of(&map, &coordinates);

            let honest = SplitPolicy::new(LinearSplit::of(&map).unwrap());
            let chosen = honest.choose(&class, &()).unwrap();
            prop_assert_eq!(map.apply_to_image(&chosen.lift).unwrap(), class.clone());
            prop_assert!(chosen.residue.is_zero(), "a split is its own reference");

            let shifting = shifting_policy(&map);
            let chosen = shifting.choose(&class, &()).unwrap();
            prop_assert_eq!(map.apply_to_image(&chosen.lift).unwrap(), class);
        }
    }

    /// Law P9: the residue `m - sigma(V(m))` is an exact kernel element.
    #[test]
    fn p9_residue_is_in_the_kernel(exponents in exponents()) {
        for map in sample_maps() {
            let monzo = five_limit().monzo(exponents.clone()).unwrap();
            let lens = StructuralLens::new(shifting_policy(&map));
            let residue = lens.residue(&monzo, &()).unwrap();
            let comma = map.kernel().embed(&residue).unwrap();
            prop_assert!(map.kills(&comma).unwrap());
            prop_assert!(map.kernel().contains(&comma).unwrap());
        }
    }

    /// Law P10: GetPut, PutGet, and PutPut, for a policy that is deliberately
    /// not a homomorphism.
    #[test]
    fn p10_lens_laws(
        exponents in exponents(),
        first in prop::collection::vec(-12i64..=12, 2),
        second in prop::collection::vec(-12i64..=12, 2),
    ) {
        for map in sample_maps() {
            let monzo = five_limit().monzo(exponents.clone()).unwrap();
            let lens = StructuralLens::new(shifting_policy(&map));
            let x = class_of(&map, &first);
            let y = class_of(&map, &second);

            // GetPut.
            let class = lens.get(&monzo).unwrap();
            prop_assert_eq!(lens.put(&monzo, &class, &()).unwrap(), monzo.clone());

            // PutGet.
            let put = lens.put(&monzo, &x, &()).unwrap();
            prop_assert_eq!(lens.get(&put).unwrap(), x.clone());

            // PutPut.
            prop_assert_eq!(
                lens.put(&put, &y, &()).unwrap(),
                lens.put(&monzo, &y, &()).unwrap()
            );
        }
    }

    /// Law P11: a policy that claims homomorphism really is additive, and one
    /// that does not claim it is not assumed to be.
    #[test]
    fn p11_homomorphism_only_when_claimed(
        first in prop::collection::vec(-24i64..=24, 2),
        second in prop::collection::vec(-24i64..=24, 2),
    ) {
        for map in sample_maps() {
            let split = LinearSplit::of(&map).unwrap();
            let x = class_of(&map, &first);
            let y = class_of(&map, &second);
            let sum = x.checked_add(&y).unwrap();

            // The splitting claims additivity, so it must have it.
            prop_assert!(RepresentativePolicy::<()>::claims_homomorphic(
                &SplitPolicy::new(split.clone())
            ));
            prop_assert_eq!(
                split.split(&sum).unwrap(),
                split.split(&x).unwrap().checked_add(&split.split(&y).unwrap()).unwrap()
            );

            // The shifting policy claims nothing, and nothing is assumed.
            prop_assert!(!shifting_policy(&map).claims_homomorphic());
        }
    }

    /// UMT-3.2 section 9.4: the point-space laws, on the exact structural
    /// torsor.
    #[test]
    fn point_space_laws(
        base in exponents(),
        g in exponents(),
        h in exponents(),
    ) {
        let basis = five_limit();
        let origin = PitchOrigin::new("umt:origin:test");
        let p = PitchPoint::new(origin, basis.monzo(base).unwrap());
        let g = basis.monzo(g).unwrap();
        let h = basis.monzo(h).unwrap();

        // (p + g) + h = p + (g + h)
        prop_assert_eq!(
            p.translate(&g).unwrap().translate(&h).unwrap(),
            p.translate(&g.checked_add(&h).unwrap()).unwrap()
        );

        // p + 0 = p
        prop_assert_eq!(p.translate(&basis.zero()).unwrap(), p.clone());

        // p + int(p, q) = q
        let q = p.translate(&g).unwrap();
        prop_assert_eq!(p.translate(&p.interval_to(&q).unwrap()).unwrap(), q.clone());

        // int(p, q) + int(q, r) = int(p, r)
        let r = q.translate(&h).unwrap();
        prop_assert_eq!(
            p.interval_to(&q).unwrap().checked_add(&q.interval_to(&r).unwrap()).unwrap(),
            p.interval_to(&r).unwrap()
        );
    }

    /// The same laws on the L3 log-frequency torsor, where the interval group
    /// is the reals.
    #[test]
    fn realized_point_space_laws(
        base in -6.0f64..6.0,
        g in -6.0f64..6.0,
        h in -6.0f64..6.0,
    ) {
        let p = LogFrequency::new(base).unwrap();
        let g = Octaves::new(g).unwrap();
        let h = Octaves::new(h).unwrap();

        prop_assert!(
            (p.translate(g).translate(h).get() - p.translate(g + h).get()).abs() < 1e-9
        );
        prop_assert_eq!(p.translate(Octaves::ZERO), p);

        let q = p.translate(g);
        prop_assert!((p.translate(p.interval_to(q)).get() - q.get()).abs() < 1e-12);

        let r = q.translate(h);
        prop_assert!(
            ((p.interval_to(q) + q.interval_to(r)).get() - p.interval_to(r).get()).abs() < 1e-9
        );
    }

    /// Law T1: a regular tuning is a homomorphism on its declared interval
    /// group.
    #[test]
    fn t1_regular_tuning_is_a_homomorphism(
        a in -200i64..=200,
        b in -200i64..=200,
        divisions in 1u32..=100,
    ) {
        let steps = AmbientLattice::new("umt:property-edo", 1);
        let tuning = RegularTuning::equal_divisions(&steps, divisions).unwrap();
        let x = steps.element([a]).unwrap();
        let y = steps.element([b]).unwrap();
        let sum = x.checked_add(&y).unwrap();
        prop_assert!(
            (tuning.size(&sum).unwrap().get()
                - (tuning.size(&x).unwrap().get() + tuning.size(&y).unwrap().get()))
            .abs()
                < 1e-9
        );
    }

    /// Law T2: for a comma in the kernel, the tuning error is exactly minus
    /// its just size.
    #[test]
    fn t2_comma_error_is_minus_the_just_size(
        multiplier in -6i64..=6,
    ) {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();

        let comma = basis.monzo([-4, 4, -1]).unwrap().scale(&Z::from(multiplier));
        prop_assume!(map.kills(&comma).unwrap());

        let error = tuning.error(&map, &comma).unwrap().get();
        let just = comma.log2_valuation_f64().unwrap();
        prop_assert!((error + just).abs() < 1e-9, "error {error}, just {just}");
    }

    /// UMT-3.2 section 4.3: forgetting voice labels keeps multiplicity, and
    /// the total is always the voice count.
    ///
    /// This is the law that makes fixture F8 statable: a doubling is a
    /// multiplicity, and a view that discards it has to be asked for.
    #[test]
    fn chord_views_lose_exactly_what_they_say_they_lose(
        pitches in prop::collection::vec(-24i64..=24, 1..=6),
    ) {
        let chord = chord_of(&pitches);
        let multiset = chord.forget_voice_labels();

        prop_assert_eq!(chord.len(), pitches.len());
        prop_assert_eq!(multiset.total_len(), pitches.len());
        prop_assert!(multiset.distinct_len() <= multiset.total_len());
        prop_assert_eq!(multiset.expand().len(), pitches.len());

        // Multiplicity agrees with a direct count over the voices.
        for (_, point) in chord.iter() {
            let counted = chord.iter().filter(|(_, other)| *other == point).count();
            prop_assert_eq!(multiset.multiplicity(point), counted);
        }

        // The second, separately named step is the one that erases it.
        prop_assert_eq!(multiset.forget_multiplicity().len(), multiset.distinct_len());
        prop_assert_eq!(chord.has_doubling(), multiset.distinct_len() < pitches.len());
    }

    /// UMT-3.2 section 4.3: disjoint union is associative, commutative on
    /// disjoint operands, and has the empty voice set as its unit.
    #[test]
    fn voice_sets_form_a_partial_commutative_monoid(
        left in prop::collection::vec(0usize..12, 0..=4),
        right in prop::collection::vec(12usize..24, 0..=4),
        third in prop::collection::vec(24usize..36, 0..=4),
    ) {
        let build = |indices: &[usize]| {
            let mut set = VoiceSet::empty();
            for index in indices {
                set.insert(VoiceId::new(&format!("v{index}")));
            }
            set
        };
        let (a, b, c) = (build(&left), build(&right), build(&third));

        // Unit.
        prop_assert_eq!(a.disjoint_union(&VoiceSet::empty()).unwrap(), a.clone());
        prop_assert_eq!(VoiceSet::empty().disjoint_union(&a).unwrap(), a.clone());

        // Commutative, on operands that are disjoint by construction.
        prop_assert_eq!(
            a.disjoint_union(&b).unwrap(),
            b.disjoint_union(&a).unwrap()
        );

        // Associative.
        prop_assert_eq!(
            a.disjoint_union(&b).unwrap().disjoint_union(&c).unwrap(),
            a.disjoint_union(&b.disjoint_union(&c).unwrap()).unwrap()
        );

        // And it is *partial*: a shared identity is a defect, not a merge.
        if !a.is_empty() {
            prop_assert!(a.disjoint_union(&a).is_err());
        }
    }

    /// UMT-3.2 section 4.4.1: composition of spans by pullback is
    /// associative, and the identity span is neutral.
    #[test]
    fn voice_leading_composition_is_associative(
        first in prop::collection::vec((0usize..3, 0usize..3), 0..=4),
        second in prop::collection::vec((0usize..3, 0usize..3), 0..=4),
        third in prop::collection::vec((0usize..3, 0usize..3), 0..=4),
    ) {
        let names = |prefix: &str| {
            VoiceSet::new((0..3).map(|i| VoiceId::new(&format!("{prefix}{i}")))).unwrap()
        };
        let (v1, v2, v3, v4) = (names("a"), names("b"), names("c"), names("d"));
        let span = |source: &VoiceSet, target: &VoiceSet, pairs: &[(usize, usize)], from: &str, to: &str| {
            VoiceLeading::new(
                source.clone(),
                target.clone(),
                pairs.iter().map(|(i, j)| {
                    Edge::new(VoiceId::new(&format!("{from}{i}")), VoiceId::new(&format!("{to}{j}")))
                }),
            )
            .unwrap()
        };

        let f = span(&v1, &v2, &first, "a", "b");
        let g = span(&v2, &v3, &second, "b", "c");
        let h = span(&v3, &v4, &third, "c", "d");

        // Associativity. Edge order can differ, so compare as multisets.
        let left = f.compose(&g).unwrap().compose(&h).unwrap();
        let right = f.compose(&g.compose(&h).unwrap()).unwrap();
        prop_assert_eq!(left.source(), right.source());
        prop_assert_eq!(left.target(), right.target());
        prop_assert_eq!(sorted_edges(&left), sorted_edges(&right));

        // Identity is neutral on both sides.
        prop_assert_eq!(
            sorted_edges(&VoiceLeading::identity(&v1).compose(&f).unwrap()),
            sorted_edges(&f)
        );
        prop_assert_eq!(
            sorted_edges(&f.compose(&VoiceLeading::identity(&v2)).unwrap()),
            sorted_edges(&f)
        );
    }

    /// UMT-3.2 section 4.4.2: the declared span cost really is the sum of its
    /// five terms, and every term is driven by the span's event counts.
    #[test]
    fn declared_span_cost_is_the_sum_of_its_terms(
        from_pitches in prop::collection::vec(-12i64..=12, 1..=4),
        to_pitches in prop::collection::vec(-12i64..=12, 1..=4),
        pairs in prop::collection::vec((0usize..4, 0usize..4), 0..=5),
    ) {
        let from = chord_of(&from_pitches);
        let to = named_chord("w", &to_pitches);
        let edges: Vec<Edge> = pairs
            .iter()
            .filter(|(i, j)| *i < from_pitches.len() && *j < to_pitches.len())
            .map(|(i, j)| Edge::new(VoiceId::new(&format!("v{i}")), VoiceId::new(&format!("w{j}"))))
            .collect();
        let span = VoiceLeading::new(from.voice_set(), to.voice_set(), edges).unwrap();

        let penalties = SpanPenalties { split: 0.25, merge: 0.5, birth: 1.0, death: 2.0 };
        let model = SpanCostModel::new(ground_cost(), 1.0, penalties).unwrap();
        let cost = model.declared_cost(&span, &from, &to).unwrap();
        let shape = cost.shape();

        prop_assert_eq!(cost.split(), penalties.split * shape.splits as f64);
        prop_assert_eq!(cost.merge(), penalties.merge * shape.merges as f64);
        prop_assert_eq!(cost.birth(), penalties.birth * shape.entries as f64);
        prop_assert_eq!(cost.death(), penalties.death * shape.exits as f64);
        prop_assert!(
            (cost.total()
                - (cost.movement() + cost.split() + cost.merge() + cost.birth() + cost.death()))
            .abs()
                < 1e-12
        );
        prop_assert_eq!(cost.question(), &CostQuestion::DeclaredSpan);

        // The movement term is the summed displacement, edge for edge.
        let expected: f64 = span
            .displacements(&from, &to)
            .unwrap()
            .iter()
            .map(|interval| (interval.coordinates()[0].to_string().parse::<f64>().unwrap() / 12.0).abs())
            .sum();
        prop_assert!((cost.movement() - expected).abs() < 1e-12);
    }

    /// UMT-3.2 section 4.4.5: a minimum over the admissible family is never
    /// dearer than a declared span drawn from that same family.
    #[test]
    fn the_family_minimum_is_no_worse_than_any_member(
        from_pitches in prop::collection::vec(-12i64..=12, 1..=4),
        to_pitches in prop::collection::vec(-12i64..=12, 1..=4),
        matching in prop::collection::vec(0usize..5, 0..=4),
    ) {
        let from = chord_of(&from_pitches);
        let to = named_chord("w", &to_pitches);
        let model = SpanCostModel::new(
            ground_cost(),
            1.0,
            SpanPenalties { split: 0.0, merge: 0.0, birth: 0.4, death: 0.4 },
        )
        .unwrap();

        // An arbitrary member of the family: each source voice takes at most
        // one distinct target.
        let mut used = vec![false; to_pitches.len()];
        let mut edges = Vec::new();
        for (i, choice) in matching.iter().enumerate().take(from_pitches.len()) {
            if *choice < to_pitches.len() && !used[*choice] {
                used[*choice] = true;
                edges.push(Edge::new(
                    VoiceId::new(&format!("v{i}")),
                    VoiceId::new(&format!("w{choice}")),
                ));
            }
        }
        let member = VoiceLeading::new(from.voice_set(), to.voice_set(), edges).unwrap();
        let declared = model.declared_cost(&member, &from, &to).unwrap();

        let outcome = model.minimum_over_assignments(&from, &to).unwrap();
        prop_assert!(outcome.is_optimal());
        let minimum = outcome.cost().unwrap();
        prop_assert_eq!(
            minimum.question(),
            &CostQuestion::MinimumOverFamily(AdmissibleFamily::PartialAssignment)
        );
        prop_assert!(
            minimum.total() <= declared.total() + 1e-12,
            "minimum {} exceeded a member at {}",
            minimum.total(),
            declared.total()
        );

        // And the winner really is in the family it claims.
        let winner = outcome.solution().unwrap();
        let shape = winner.shape();
        prop_assert_eq!(shape.splits, 0);
        prop_assert_eq!(shape.merges, 0);
    }

    /// UMT-3.2 section 9.5: the distance laws a profile claims are tested on
    /// the state space it names, not inherited.
    ///
    /// The edit profile spans several cardinalities, which is exactly the case
    /// section 4.4.4 says classical balanced transport does not cover.
    #[test]
    fn the_edit_profile_obeys_the_metric_laws(
        a in prop::collection::vec(-12i64..=12, 1..=3),
        b in prop::collection::vec(-12i64..=12, 1..=3),
        c in prop::collection::vec(-12i64..=12, 1..=3),
        boundary in 0.05f64..1.5,
        exponent in 1.0f64..3.0,
    ) {
        let distance =
            ChordDistance::new(ground_cost(), exponent, TransportProfile::Edit { boundary })
                .unwrap();
        let claim = distance.metric_claim();
        prop_assert!(
            matches!(claim, MetricClaim::Metric { .. }),
            "a positive boundary cost claims the metric laws, got {claim:?}"
        );

        let (x, y, z) = (chord_of(&a), chord_of(&b), chord_of(&c));
        let scale = 1.0 + boundary * 6.0;

        // Identity of indiscernibles, both directions.
        prop_assert_eq!(distance.distance(&x, &x).unwrap(), 0.0);
        if x.forget_voice_labels() != y.forget_voice_labels() {
            prop_assert!(distance.distance(&x, &y).unwrap() > 0.0);
        }

        // Symmetry.
        let there = distance.distance(&x, &y).unwrap();
        let back = distance.distance(&y, &x).unwrap();
        prop_assert!((there - back).abs() < 1e-9 * scale);

        // Triangle inequality.
        let direct = distance.distance(&x, &z).unwrap();
        let via = there + distance.distance(&y, &z).unwrap();
        prop_assert!(direct <= via + 1e-9 * scale, "{direct} > {via}");
    }

    /// UMT-3.2 section 9.7: flattened child spans exactly partition the parent
    /// span, child order is preserved, and recursive flattening preserves the
    /// root total.
    #[test]
    fn rhythm_tree_flattening_partitions_the_parent(
        tree in rhythm_trees(),
        beats in 1i64..=16,
        denominator in 1i64..=7,
    ) {
        let span = BeatSpan::new(
            BeatTime::zero(),
            BeatTime::ratio(beats, denominator).unwrap(),
        )
        .unwrap();
        let leaves = tree.flatten(&span).unwrap();

        prop_assert_eq!(leaves.len(), tree.leaf_count());
        prop_assert_eq!(leaves[0].span().start(), span.start());
        prop_assert_eq!(leaves[leaves.len() - 1].span().end(), span.end());

        // Each leaf begins exactly where the previous ended: a partition, not
        // an approximation of one.
        for pair in leaves.windows(2) {
            prop_assert_eq!(pair[0].span().end(), pair[1].span().start());
        }

        // And the durations sum to the root total, exactly.
        let total: Beats = leaves.iter().map(|leaf| leaf.span().duration()).sum();
        prop_assert_eq!(total, span.duration());

        // Child order is preserved: the leaf paths are lexicographically
        // ascending.
        for pair in leaves.windows(2) {
            prop_assert!(pair[0].path() < pair[1].path());
        }
    }

    /// UMT-3.2 section 9.8, floor and ceiling profiles: monotone, identity on
    /// grid values, one-sided, with one-signed residuals.
    #[test]
    fn floor_and_ceiling_are_order_adjunctions(
        numerator in -400i64..=400,
        denominator in 1i64..=97,
        ticks_per_beat in 1u32..=192,
    ) {
        let grid = TickGrid::new(ticks_per_beat).unwrap();
        let at = BeatTime::ratio(numerator, denominator).unwrap();

        let floor = grid.quantize(&at, RoundingConvention::Floor);
        let ceiling = grid.quantize(&at, RoundingConvention::Ceiling);

        // i(q_down(x)) <= x <= i(q_up(x)).
        prop_assert!(grid.tick_time(&floor.value) <= at);
        prop_assert!(at <= grid.tick_time(&ceiling.value));

        // Residual signs under the convention e = x - i(q(x)).
        prop_assert!(floor.residual.get() >= &Q::from(Z::from(0)));
        prop_assert!(ceiling.residual.get() <= &Q::from(Z::from(0)));

        // The bracket is at most one step wide.
        prop_assert!(&ceiling.value - &floor.value <= Z::from(1));

        // Identity on represented values.
        let on_grid = grid.tick_time(&floor.value);
        prop_assert_eq!(
            grid.quantize(&on_grid, RoundingConvention::Floor).value,
            floor.value.clone()
        );
        prop_assert_eq!(
            grid.quantize(&on_grid, RoundingConvention::Ceiling).value,
            floor.value.clone()
        );

        // Monotone: a later position never quantizes earlier.
        let later = at.translate(&grid.tick_duration());
        prop_assert!(grid.quantize(&later, RoundingConvention::Floor).value >= floor.value);
        prop_assert!(grid.quantize(&later, RoundingConvention::Ceiling).value >= ceiling.value);
    }

    /// UMT-3.2 section 9.8, nearest profile: identity on grid values and a
    /// residual bounded by half a step, with no universal one-sided
    /// inequality.
    #[test]
    fn nearest_quantization_is_bounded_but_not_one_sided(
        numerator in -400i64..=400,
        denominator in 1i64..=97,
        ticks_per_beat in 1u32..=192,
    ) {
        let grid = TickGrid::new(ticks_per_beat).unwrap();
        let at = BeatTime::ratio(numerator, denominator).unwrap();
        let half_step = grid.tick_duration().scale(&Q::new(Z::from(1), Z::from(2)));

        for convention in [
            RoundingConvention::NearestHalfAwayFromZero,
            RoundingConvention::NearestHalfToEven,
        ] {
            let quantized = grid.quantize(&at, convention);
            prop_assert!(
                quantized.residual.abs() <= half_step,
                "residual {} exceeds half a step",
                quantized.residual
            );
            let on_grid = grid.tick_time(&quantized.value);
            prop_assert_eq!(grid.quantize(&on_grid, convention).value, quantized.value);
        }
    }

    /// UMT-3.2 section 9.8, endpoint-preserving profile: the integer children
    /// sum to the parent, and every residual is reported.
    #[test]
    fn endpoint_preserving_allocation_sums_to_the_parent(
        weights in prop::collection::vec(1i64..=9, 1..=7),
        parent_ticks in 1i64..=480,
        ticks_per_beat in 1u32..=192,
    ) {
        let grid = TickGrid::new(ticks_per_beat).unwrap();
        let weights: Vec<Q> = weights.into_iter().map(|w| Q::from(Z::from(w))).collect();
        let allocation = grid
            .allocate_preserving_endpoint(
                &weights,
                &Z::from(parent_ticks),
                &AllocationPolicy::default(),
            )
            .unwrap()
            .into_allocation()
            .expect("no minimum span was declared, so this is always feasible");

        prop_assert_eq!(allocation.total_ticks(), Z::from(parent_ticks));
        prop_assert!(allocation.endpoint_preserved());
        prop_assert_eq!(allocation.children().len(), weights.len());

        // Residuals are relative to the exact structural child duration, and
        // they cancel: that is what preserving the endpoint amounts to.
        let sum: Beats = allocation
            .children()
            .iter()
            .map(|child| child.residual().clone())
            .sum();
        prop_assert_eq!(sum, Beats::zero());
    }

    /// UMT-3.2 section 9.9: a tempo map in the homeomorphism profile is
    /// strictly increasing and invertible on its declared domain.
    #[test]
    fn a_tempo_map_is_strictly_increasing_and_invertible(
        gaps in prop::collection::vec((1i64..=8, 1u32..=400), 2..=5),
    ) {
        let mut breakpoints = Vec::new();
        let mut beat = 0i64;
        let mut seconds = 0.0f64;
        breakpoints.push(TempoBreakpoint::new(BeatTime::zero(), ClockTime::ZERO));
        for (beats, centiseconds) in &gaps {
            beat += beats;
            seconds += f64::from(*centiseconds) / 100.0;
            breakpoints.push(TempoBreakpoint::new(
                BeatTime::ratio(beat, 1).unwrap(),
                ClockTime::new(seconds).unwrap(),
            ));
        }
        let map = TempoMap::new(breakpoints).unwrap();

        // Strictly increasing on the structural domain.
        let mut previous: Option<ClockTime> = None;
        for step in 0..=(beat * 2) {
            let at = BeatTime::ratio(step, 2).unwrap();
            let clock = map.clock_time(&at).unwrap();
            if let Some(previous) = previous {
                prop_assert!(clock > previous, "{at} went backwards");
            }
            previous = Some(clock);
        }

        // Endpoint-consistent, and a bijection onto the declared range.
        prop_assert_eq!(map.clock_time(map.domain().start()).unwrap(), map.range().start());
        prop_assert_eq!(map.clock_time(map.domain().end()).unwrap(), map.range().end());
        prop_assert!(map.clock_time(&BeatTime::ratio(beat + 1, 1).unwrap()).is_err());
    }

    /// UMT-3.2 section 9.10, STP profile: a network reported consistent really
    /// does admit the assignment the solver returns.
    #[test]
    fn a_consistent_stp_assignment_satisfies_every_constraint(
        edges in prop::collection::vec((0usize..4, 0usize..4, -8i64..=8, 0i64..=8), 1..=8),
    ) {
        let mut problem = StpProblem::new();
        let names: Vec<TimeVarId> = (0..4).map(|index| problem.variable(&format!("t{index}"))).collect();

        let mut declared = Vec::new();
        for (from, to, lower, width) in edges {
            if from == to {
                continue;
            }
            let constraint = DifferenceConstraint::between(
                &names[from],
                &names[to],
                Some(Q::from(Z::from(lower))),
                Some(Q::from(Z::from(lower + width))),
            );
            problem.constrain(constraint.clone()).unwrap();
            declared.push(constraint);
        }

        let outcome = problem.solve();
        if let Some(assignment) = outcome.assignment() {
            for constraint in &declared {
                let gap = &assignment[&constraint.to] - &assignment[&constraint.from];
                prop_assert!(
                    constraint.lower.as_ref().is_none_or(|lower| gap >= *lower),
                    "lower bound violated: {gap}"
                );
                prop_assert!(
                    constraint.upper.as_ref().is_none_or(|upper| gap <= *upper),
                    "upper bound violated: {gap}"
                );
            }
        }
    }

    /// UMT-3.2 section 5.10.2: a feasible linear system yields an assignment
    /// that satisfies it, strict inequalities included.
    #[test]
    fn a_feasible_linear_system_yields_a_satisfying_assignment(
        bounds in prop::collection::vec((-6i64..=6, 0i64..=6, prop::bool::ANY), 1..=5),
    ) {
        let mut problem = LinearTemporalProblem::new();
        let x = problem.variable("x");
        let y = problem.variable("y");

        let mut declared = Vec::new();
        for (offset, width, strict) in bounds {
            // A band on x + y, wide enough to stay satisfiable on its own.
            let terms = [(x.clone(), Q::from(Z::from(1))), (y.clone(), Q::from(Z::from(1)))];
            let constraint = if strict {
                LinearConstraint::less_than(terms, Q::from(Z::from(offset + width + 20)))
            } else {
                LinearConstraint::at_most(terms, Q::from(Z::from(offset + width + 20)))
            };
            problem.constrain(constraint.clone()).unwrap();
            declared.push(constraint);
        }

        let outcome = problem.solve().unwrap();
        let assignment = outcome
            .assignment()
            .expect("a band bounded only above is always feasible");
        for constraint in &declared {
            let value: Q = constraint
                .coefficients
                .iter()
                .map(|(variable, coefficient)| coefficient * &assignment[variable])
                .sum();
            if constraint.strict {
                prop_assert!(value < constraint.bound, "{value} !< {}", constraint.bound);
            } else {
                prop_assert!(value <= constraint.bound, "{value} !<= {}", constraint.bound);
            }
        }
    }

    /// UMT-3.2 section 2.1: applying a proportion to a rate and to the
    /// reciprocal duration uses reciprocal factors.
    #[test]
    fn an_oriented_ratio_inverts_across_the_reciprocal(
        numerator in 1i64..=32,
        denominator in 1i64..=32,
    ) {
        let ratio = Q::new(Z::from(numerator), Z::from(denominator));
        let as_rate = OrientedRatio::new(ratio.clone(), RatioOrientation::Rate).unwrap();
        let as_duration = OrientedRatio::new(ratio.clone(), RatioOrientation::Duration).unwrap();

        // The two factors are reciprocals of one another.
        prop_assert_eq!(as_rate.rate_factor() * as_rate.duration_factor(), Q::from(Z::from(1)));
        prop_assert_eq!(as_rate.rate_factor(), as_duration.duration_factor());

        // Reorienting is an involution that preserves both factors.
        let reoriented = as_rate.reoriented();
        prop_assert_eq!(reoriented.rate_factor(), as_rate.rate_factor());
        prop_assert_eq!(reoriented.duration_factor(), as_rate.duration_factor());
        prop_assert_eq!(reoriented.reoriented(), as_rate.clone());

        // And a round trip through a duration returns exactly.
        let beat = BeatDuration::one();
        let stretched = as_rate.scale_duration(&beat).unwrap();
        let restored = as_duration.scale_duration(&stretched).unwrap();
        prop_assert_eq!(restored, beat);
    }

    /// UMT-3.2 section 6.6: composition of score transformations is
    /// associative and the identity transformation is neutral, which are two
    /// of the five obligations a compositional claim carries.
    #[test]
    fn score_transformations_compose_associatively(
        shifts in prop::collection::vec((-8i64..=8, 1i64..=4), 3),
    ) {
        let make = |(shift, scale): (i64, i64), tag: &str| -> ScoreTransform<i64> {
            ScoreTransform::new(
                EventRelation::identity([&EventId::new("a")]),
                PitchTransform::Transpose(shift),
                TimeTransform::Affine {
                    scale: Q::from(Z::from(scale)),
                    shift: Beats::ratio(shift, 1).unwrap(),
                },
                ProvenanceChain::of(ProvenanceId::new(tag)),
            )
        };
        let f = make(shifts[0], "p1");
        let g = make(shifts[1], "p2");
        let h = make(shifts[2], "p3");
        let add = |left: &i64, right: &i64| Ok(left + right);

        prop_assert!(f.claims_compositional());
        let left = f
            .compose(&g, add)
            .unwrap()
            .unwrap()
            .compose(&h, add)
            .unwrap()
            .unwrap();
        let right = f
            .compose(&g.compose(&h, add).unwrap().unwrap(), add)
            .unwrap()
            .unwrap();
        prop_assert_eq!(&left, &right);

        // Identity is neutral on both sides.
        let identity: ScoreTransform<i64> = ScoreTransform::identity([&EventId::new("a")]);
        prop_assert_eq!(identity.compose(&f, add).unwrap().unwrap(), f.clone());
        prop_assert_eq!(f.compose(&identity, add).unwrap().unwrap(), f);

        // Provenance composes by concatenation, oldest first.
        prop_assert_eq!(left.provenance().steps().len(), 3);
        prop_assert_eq!(&left.provenance().steps()[0], &ProvenanceId::new("p1"));
    }

    /// UMT-3.2 section 6.6: an affine temporal composite agrees with applying
    /// its two parts in order.
    #[test]
    fn composed_time_transformations_agree_with_applying_them_in_turn(
        first_scale in 1i64..=6,
        first_shift in -6i64..=6,
        second_scale in 1i64..=6,
        second_shift in -6i64..=6,
        at in -12i64..=12,
    ) {
        let first = TimeTransform::Affine {
            scale: Q::from(Z::from(first_scale)),
            shift: Beats::ratio(first_shift, 1).unwrap(),
        };
        let second = TimeTransform::Affine {
            scale: Q::from(Z::from(second_scale)),
            shift: Beats::ratio(second_shift, 1).unwrap(),
        };
        let composite = first.compose(&second).unwrap();

        let position = BeatTime::ratio(at, 1).unwrap();
        prop_assert_eq!(
            composite.apply(&position).unwrap(),
            second.apply(&first.apply(&position).unwrap()).unwrap()
        );
    }

    /// UMT-3.2 sections 5.2.2 and 5.2.3: a tie chain becomes exactly one
    /// gesture whose span is the sum of its noteheads, and every notehead
    /// survives in the score.
    #[test]
    fn a_tie_chain_yields_one_gesture_and_loses_no_notehead(
        durations in prop::collection::vec(1i64..=4, 1..=5),
    ) {
        let steps = twelve_edo();
        let voice = EventScope::VoiceLocal(VoiceId::new("soprano"));
        let pitch = PitchPoint::new(
            PitchOrigin::new("umt:origin:c4"),
            steps.element([7i64]).unwrap(),
        );

        let mut builder = Score::builder();
        let mut onset = 0i64;
        let mut ids = Vec::new();
        for (index, duration) in durations.iter().enumerate() {
            let id = EventId::new(&format!("n{index}"));
            builder = builder
                .event(
                    ScoreEvent::new(
                        id.clone(),
                        voice.clone(),
                        TemporalPlacement::fixed(
                            BeatTime::ratio(onset, 1).unwrap(),
                            BeatDuration::ratio(*duration, 1).unwrap(),
                        ),
                        EventContent::Note { pitch: pitch.clone() },
                    )
                    .unwrap(),
                )
                .unwrap();
            onset += duration;
            ids.push(id);
        }
        for pair in ids.windows(2) {
            builder = builder.tie(Tie::new(pair[0].clone(), pair[1].clone())).unwrap();
        }
        let score = builder.build().unwrap();

        // Every notehead survives, distinct.
        prop_assert_eq!(score.len(), durations.len());
        prop_assert_eq!(score.ties().len(), durations.len() - 1);

        // And the chain is exactly one gesture, spanning the whole.
        let gestures = score.sounding_gestures().unwrap();
        prop_assert_eq!(gestures.len(), 1);
        prop_assert_eq!(gestures[0].sources().len(), durations.len());
        prop_assert_eq!(
            gestures[0].span().duration(),
            Beats::ratio(durations.iter().sum::<i64>(), 1).unwrap()
        );
        prop_assert_eq!(gestures[0].is_tied(), durations.len() > 1);
    }

    /// Prompt section 10: the Smith normal form reconstructs its input, its
    /// transforms are unimodular, its invariant factors divide in order, and
    /// its kernel basis really is in the kernel.
    #[test]
    fn smith_normal_form_invariants(matrix in matrices(3, 4)) {
        let form = SmithNormalForm::of(&matrix);

        prop_assert_eq!(
            form.left().multiply(&matrix).unwrap().multiply(form.right()).unwrap(),
            form.diagonal()
        );
        prop_assert_eq!(
            form.left().multiply(form.left_inverse()).unwrap(),
            IntMatrix::identity(3)
        );
        prop_assert_eq!(
            form.right().multiply(form.right_inverse()).unwrap(),
            IntMatrix::identity(4)
        );
        for window in form.invariant_factors().windows(2) {
            prop_assert!((&window[1] % &window[0]) == Z::from(0));
        }
        prop_assert!(matrix.multiply(&form.kernel_basis()).unwrap().is_zero());
        prop_assert_eq!(form.kernel_basis().cols(), 4 - form.rank());
    }

    /// Prompt section 53: the Hermite form is canonical, so two generating
    /// sets of the same lattice produce the same basis.
    #[test]
    fn hermite_normal_form_is_canonical(
        generators in matrices(3, 2),
        combination in matrices(2, 3),
    ) {
        let extra = generators.multiply(&combination).unwrap();
        // `[G | G C]` generates the same lattice as `G`.
        let mut columns: Vec<Vec<Z>> = Vec::new();
        for source in [&generators, &extra] {
            for col in 0..source.cols() {
                columns.push(source.column(col).unwrap());
            }
        }
        let mut data = Vec::new();
        for row in 0..3 {
            for column in &columns {
                data.push(column[row].clone());
            }
        }
        let widened = IntMatrix::new(3, columns.len(), data).unwrap();

        prop_assert_eq!(
            HermiteNormalForm::column_of(&generators).basis(),
            HermiteNormalForm::column_of(&widened).basis()
        );
        prop_assert_eq!(
            Sublattice::from_generators(3, &generators).unwrap(),
            Sublattice::from_generators(3, &widened).unwrap()
        );
    }

    /// Sublattice coordinates round-trip, and membership agrees with them.
    #[test]
    fn sublattice_coordinates_round_trip(
        generators in matrices(3, 2),
        coordinates in prop::collection::vec(-20i64..=20, 2),
    ) {
        let lattice = Sublattice::from_generators(3, &generators).unwrap();
        let coordinates: Vec<Z> = coordinates.into_iter().take(lattice.rank()).map(Z::from).collect();
        if coordinates.len() != lattice.rank() {
            return Ok(());
        }
        let point = lattice.embed(&coordinates).unwrap();
        prop_assert!(lattice.contains(&point).unwrap());
        prop_assert_eq!(lattice.coordinates(&point).unwrap(), Some(coordinates));
    }

    /// UMT-3.2 section 1.6: the exact entry really is `round(N log2 x)`.
    ///
    /// Verified against the defining inequality `2^(2k-1) <= x^(2N) < 2^(2k+1)`
    /// with independent integer arithmetic rather than by re-running the
    /// search.
    #[test]
    fn nearest_entry_satisfies_its_defining_inequality(
        numer in 1i64..=4096,
        denom in 1i64..=4096,
        divisions in 0u32..=64,
    ) {
        let (numer, denom) = (Z::from(numer), Z::from(denom));
        let k = round_n_log2(
            divisions,
            &numer,
            &denom,
            RoundingConvention::NearestHalfAwayFromZero,
        )
        .unwrap();
        let k: i64 = k.to_string().parse().unwrap();

        let squared = 2 * divisions;
        let p2 = numer.pow(squared);
        let q2 = denom.pow(squared);
        prop_assert_ne!(cmp_pow2(&p2, &q2, 2 * k - 1), core::cmp::Ordering::Less);
        prop_assert_eq!(cmp_pow2(&p2, &q2, 2 * k + 1), core::cmp::Ordering::Less);
    }

    /// The floor entry satisfies `2^k <= x^N < 2^(k+1)`.
    #[test]
    fn floor_entry_satisfies_its_defining_inequality(
        numer in 1i64..=4096,
        denom in 1i64..=4096,
        divisions in 0u32..=64,
    ) {
        let (numer, denom) = (Z::from(numer), Z::from(denom));
        let k = round_n_log2(divisions, &numer, &denom, RoundingConvention::Floor).unwrap();
        let k: i64 = k.to_string().parse().unwrap();

        let p = numer.pow(divisions);
        let q = denom.pow(divisions);
        prop_assert_ne!(cmp_pow2(&p, &q, k), core::cmp::Ordering::Less);
        prop_assert_eq!(cmp_pow2(&p, &q, k + 1), core::cmp::Ordering::Less);
    }

    /// Rounding conventions are ordered: floor <= nearest <= ceiling, and the
    /// bracket is at most one step wide.
    #[test]
    fn conventions_are_ordered(
        numer in 1i64..=4096,
        denom in 1i64..=4096,
        divisions in 0u32..=64,
    ) {
        let (numer, denom) = (Z::from(numer), Z::from(denom));
        let entry = |convention| {
            round_n_log2(divisions, &numer, &denom, convention).unwrap()
        };
        let floor = entry(RoundingConvention::Floor);
        let ceiling = entry(RoundingConvention::Ceiling);
        let nearest = entry(RoundingConvention::NearestHalfAwayFromZero);
        let even = entry(RoundingConvention::NearestHalfToEven);

        prop_assert!(floor <= nearest && nearest <= ceiling);
        prop_assert!(floor <= even && even <= ceiling);
        prop_assert!(&ceiling - &floor <= Z::from(1));
        // Exact ties cannot occur in the rational profile, so both nearest
        // conventions must agree everywhere.
        prop_assert_eq!(nearest, even);
    }

    /// UMT-3.2 section 1.6: a generator with exact valuation 2 has entry `N`
    /// under every convention.
    #[test]
    fn octave_entry_is_fixed_to_n(divisions in 0u32..=4096) {
        for convention in [
            RoundingConvention::Floor,
            RoundingConvention::Ceiling,
            RoundingConvention::NearestHalfAwayFromZero,
            RoundingConvention::NearestHalfToEven,
        ] {
            prop_assert_eq!(
                round_n_log2(divisions, &Z::from(2), &Z::from(1), convention).unwrap(),
                Z::from(divisions)
            );
        }
    }

    /// The exact entry never differs from a naive floating-point computation
    /// by more than one step, and agrees exactly on the moderate range where
    /// double precision is not in doubt.
    #[test]
    fn exact_and_floating_entries_agree(divisions in 0u32..=1024) {
        for prime in [2u32, 3, 5, 7, 11, 13, 17, 19] {
            let exact = round_n_log2(
                divisions,
                &Z::from(prime),
                &Z::from(1),
                RoundingConvention::NearestHalfAwayFromZero,
            )
            .unwrap();
            let exact: i64 = exact.to_string().parse().unwrap();
            let floating = (f64::from(divisions) * f64::from(prime).log2()).round() as i64;
            prop_assert!((exact - floating).abs() <= 1);
            if divisions <= 512 {
                prop_assert_eq!(exact, floating);
            }
        }
    }
}
