//! UMT-3.2 conformance fixtures (specification section 9.13).
//!
//! `docs/conformance.md` maps every fixture to its test and status. Fixtures
//! whose machinery is not implemented yet are absent from this file rather
//! than stubbed, because a skipped law must never look like a passing one
//! (prompt section 60, item 20).
//!
//! Tests named `*_partial` cover the part of a fixture that the implemented
//! layers can decide. The remaining obligations are listed in the docs table.

use std::sync::Arc;

use umt::error::PatentValError;
use umt::{Basis, PatentVal, RoundingConvention, Z};

const NEAREST: RoundingConvention = RoundingConvention::NearestHalfAwayFromZero;

fn five_limit() -> Arc<Basis> {
    Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).expect("valid prime basis")
}

/// F3 - 12-EDO quotient (partial).
///
/// Covered here: the syntonic and Pythagorean commas are both killed, the
/// mapping is surjective so the reachable image is all of `Gamma = Z`, and the
/// octave maps to 12 steps, giving 12 ambient pitch classes.
///
/// Not covered yet: the quotient `T = Lambda_B / K` as a constructed object,
/// which needs the kernel machinery of the general temperament map.
#[test]
fn f03_twelve_edo_quotient_partial() {
    let basis = five_limit();
    let val = PatentVal::new(&basis, 12, NEAREST).unwrap();

    let syntonic = basis.monzo([-4, 4, -1]).unwrap();
    let pythagorean = basis.monzo([-19, 12, 0]).unwrap();
    assert_eq!(val.apply(&syntonic).unwrap(), Z::from(0));
    assert_eq!(val.apply(&pythagorean).unwrap(), Z::from(0));

    // Reachable structure is free of rank 1 and equals the ambient lattice.
    assert!(val.is_surjective());
    assert_eq!(val.image_rank(), 1);
    assert_eq!(val.image_generator(), Z::from(1));

    // With the octave mapped to 12 steps, ambient unit equivalence gives
    // Z/12Z (UMT-3.2 section 1.9).
    let octave = basis.monzo([1, 0, 0]).unwrap();
    assert_eq!(val.apply(&octave).unwrap(), Z::from(12));
}

/// F4 - 6-EDO image (partial).
///
/// Covered here: the ambient step lattice `Gamma = Z` and the mapped image
/// `H = 2Z` are separately represented, and an odd ambient step has no
/// automatic detempering under this mapping.
///
/// Not covered yet: detempering itself, which needs a representative policy.
/// The fixture's remaining obligation is that no lift exists for an odd step,
/// which is exactly the `NotInImage` rejection asserted below.
#[test]
fn f04_6edo_image_partial() {
    let basis = five_limit();
    let val = PatentVal::new(&basis, 6, NEAREST).unwrap();

    assert_eq!(
        val.entries(),
        &[Z::from(6), Z::from(10), Z::from(14)],
        "the usual 5-limit patent val for 6-EDO"
    );

    // Image is 2Z, a proper subgroup of the ambient Z.
    assert_eq!(val.image_generator(), Z::from(2));
    assert!(!val.is_surjective());

    // Odd ambient steps exist but are not reached.
    for odd in [-5i64, -3, -1, 1, 3, 5, 7] {
        let step = Z::from(odd);
        assert!(!val.contains_ambient(&step), "step {odd} must not be in H");
        assert_eq!(
            val.image_coordinate(&step),
            Err(PatentValError::NotInImage { step: Z::from(odd) }),
            "step {odd} has no automatic L1 detempering"
        );
    }

    // Even steps are reached, and their intrinsic image coordinates differ
    // from their ambient coordinates.
    for even in [-4i64, -2, 0, 2, 4, 6] {
        let step = Z::from(even);
        assert!(val.contains_ambient(&step));
        let coordinate = val.image_coordinate(&step).unwrap();
        assert_eq!(coordinate, Z::from(even / 2));
        assert_eq!(val.embed_image(&coordinate).unwrap(), step);
    }
}

/// F22 - reachable versus ambient octave classes (partial).
///
/// With `H = 2Z`, `Gamma = Z`, and octave image 6, the reachable quotient
/// `H/6Z` has 3 elements while the ambient quotient `Gamma/6Z` has 6. The two
/// counts are derived from the represented image, not asserted as constants,
/// and they must not be identified.
///
/// Not covered yet: the quotient groups as constructed objects.
#[test]
fn f22_reachable_versus_ambient_classes_partial() {
    let basis = five_limit();
    let val = PatentVal::new(&basis, 6, NEAREST).unwrap();

    let octave_image = val.apply(&basis.monzo([1, 0, 0]).unwrap()).unwrap();
    assert_eq!(octave_image, Z::from(6));

    let image_generator = val.image_generator();
    assert_eq!(image_generator, Z::from(2));

    // |Gamma / 6Z| = 6.
    let ambient_classes = octave_image.clone();
    // |H / 6Z| = |2Z / 6Z| = 6 / 2 = 3.
    let reachable_classes = &octave_image / &image_generator;

    assert_eq!(ambient_classes, Z::from(6));
    assert_eq!(reachable_classes, Z::from(3));
    assert_ne!(
        ambient_classes, reachable_classes,
        "reachable and ambient pitch-class counts must not be identified"
    );
}

/// Prompt section 13: mandatory equal-division cases.
#[test]
fn edo_mandatory_cases() {
    let basis = five_limit();

    // Ordinary 12-EDO.
    let twelve = PatentVal::new(&basis, 12, NEAREST).unwrap();
    assert_eq!(twelve.entries(), &[Z::from(12), Z::from(19), Z::from(28)]);

    // 6-EDO.
    let six = PatentVal::new(&basis, 6, NEAREST).unwrap();
    assert_eq!(six.entries(), &[Z::from(6), Z::from(10), Z::from(14)]);

    // Zero map.
    let zero = PatentVal::new(&basis, 0, NEAREST).unwrap();
    assert_eq!(zero.entries(), &[Z::from(0), Z::from(0), Z::from(0)]);
    assert_eq!(zero.image_generator(), Z::from(0));
    assert_eq!(zero.image_rank(), 0);
    assert!(!zero.is_surjective());

    // The octave entry is fixed to N.
    for divisions in [0u32, 1, 5, 6, 12, 19, 31, 41, 53, 72, 311, 1200] {
        let val = PatentVal::new(&basis, divisions, NEAREST).unwrap();
        assert_eq!(val.entries()[0], Z::from(divisions));
    }
}

/// UMT-3.2 section 1.6 places no surjectivity requirement on a patent val, and
/// section 1.4.2 keeps surjectivity separate from kernel saturation.
#[test]
fn patent_vals_are_not_assumed_surjective() {
    let basis = five_limit();
    let non_surjective: Vec<u32> = (1..=48)
        .filter(|divisions| {
            !PatentVal::new(&basis, *divisions, NEAREST)
                .unwrap()
                .is_surjective()
        })
        .collect();
    assert!(
        non_surjective.contains(&6),
        "6-EDO must be among the non-surjective 5-limit patent vals, found {non_surjective:?}"
    );
}
