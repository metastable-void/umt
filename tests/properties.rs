//! Property-based tests for the algebraic laws of UMT-3.2 part IX
//! (prompt section 47).
//!
//! Each test names the law it exercises. Laws for structures that are not
//! implemented yet - representative policies, torsors, quantization, rhythm
//! trees - are absent rather than stubbed.

use std::sync::Arc;

use proptest::prelude::*;
use umt::algebra::integer::round_n_log2;
use umt::algebra::normal_form::{HermiteNormalForm, SmithNormalForm};
use umt::pitch::{LogFrequency, Octaves, PitchOrigin, PitchPoint, RegularTuning};
use umt::temperament::{
    AmbientLattice, HomomorphicSplit, KernelElem, LinearSplit, OffsetPolicy, RepresentativePolicy,
    SplitPolicy, StructuralLens, TemperamentMap,
};
use umt::{Basis, IntMatrix, PatentVal, RoundingConvention, Sublattice, Z};

fn five_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).expect("valid prime basis")
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
