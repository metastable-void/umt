//! Property-based tests for the algebraic laws of UMT-3.2 part IX
//! (prompt section 47).
//!
//! Each test names the law it exercises. Laws for structures that are not
//! implemented yet - representative policies, torsors, quantization, rhythm
//! trees - are absent rather than stubbed.

use std::sync::Arc;

use proptest::prelude::*;
use umt::algebra::integer::round_n_log2;
use umt::{Basis, PatentVal, RoundingConvention, Z};

fn five_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).expect("valid prime basis")
}

fn seven_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5.7", &[2, 3, 5, 7]).expect("valid prime basis")
}

fn exponents() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(-64i64..=64, 3)
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
