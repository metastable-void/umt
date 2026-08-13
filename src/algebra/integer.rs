//! Exact arbitrary-precision integers, plus the exact `round(n * log2(p/q))`
//! primitive used to build equal-division mappings without floating point.

use core::cmp::Ordering;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::algebra::rounding::RoundingConvention;

/// Exact arbitrary-precision integer.
///
/// UMT layer: L1/L2 storage. This is the canonical storage type for monzo
/// coordinates, mapping-matrix entries, kernel coordinates, and ambient/image
/// lattice coordinates. Bounded integer types are introduced only at the
/// validated device/performance boundary (prompt section 5.2).
pub type Z = BigInt;

/// Compares `p / q` with `2^e` exactly, for `p > 0` and `q > 0`.
///
/// Returns `None` if either argument is not positive or if `e` is so extreme
/// that the required shift cannot be expressed.
fn cmp_with_pow2(p: &Z, q: &Z, e: i64) -> Option<Ordering> {
    if !p.is_positive() || !q.is_positive() {
        return None;
    }
    if e >= 0 {
        let shift = u64::try_from(e).ok()?;
        Some(p.cmp(&(q << shift)))
    } else {
        let shift = u64::try_from(e.checked_neg()?).ok()?;
        Some((p << shift).cmp(q))
    }
}

/// Exact `floor(log2(p / q))` for `p > 0` and `q > 0`.
///
/// This is exact integer arithmetic: no logarithm is evaluated.
fn floor_log2_ratio(p: &Z, q: &Z) -> Option<i64> {
    let bits_p = i64::try_from(p.bits()).ok()?;
    let bits_q = i64::try_from(q.bits()).ok()?;

    // For a positive integer `v`, `bits(v) - 1 <= log2(v) < bits(v)`, so
    // `floor(log2(p/q))` lies in `[bits_p - bits_q - 1, bits_p - bits_q]`.
    // Each loop below therefore runs at most twice.
    let mut k = bits_p.checked_sub(bits_q)?;
    while cmp_with_pow2(p, q, k)? == Ordering::Less {
        k = k.checked_sub(1)?;
    }
    while cmp_with_pow2(p, q, k.checked_add(1)?)? != Ordering::Less {
        k = k.checked_add(1)?;
    }
    Some(k)
}

/// Exact value of `round(n * log2(numer / denom))` under a declared rounding
/// convention, computed with integer arithmetic only.
///
/// UMT layer: L2. This is the structural core of an equal-division mapping
/// (UMT-3.2 section 1.6). The specification writes the patent-val entry as
/// `round(N * log2(nu_3(beta_i)))`, a formula over the L3 real valuation. For a
/// rational-profile generator the L3 valuation is the exact embedding of the
/// rational value (section 1.1.2), so the real quantity being rounded is
/// determined exactly by `numer/denom`, and this function returns the same
/// integer the ideal real computation would return - without making an L2
/// structural object depend on binary floating point, which section 0.6.1
/// forbids. See `docs/spec-issues.md`, issue S1.
///
/// Method: `round(n * log2(x))` is decided by comparing `x^n` with powers of
/// two, and the nearest-rounding tie by comparing `x^(2n)` with `2^(2k+1)`.
/// All comparisons are exact.
///
/// Returns `None` if `numer` or `denom` is not positive.
///
/// # Exact ties
///
/// A nearest-rounding tie requires `x^(2n) = 2^(2k+1)` for a rational `x` and
/// `n >= 1`, which forces an odd power of two to equal `p^(2n)` for an integer
/// `p`; that is impossible because `2n` is even. Ties are therefore
/// unreachable in the rational profile, and both nearest conventions agree on
/// every input. The tie branches are still implemented, so correctness does not
/// rest on that argument.
///
/// # Examples
///
/// ```
/// use umt::algebra::integer::{round_n_log2, Z};
/// use umt::RoundingConvention::NearestHalfAwayFromZero;
///
/// // The 12-EDO patent-val entry for the prime 3.
/// let entry = round_n_log2(12, &Z::from(3), &Z::from(1), NearestHalfAwayFromZero);
/// assert_eq!(entry, Some(Z::from(19)));
///
/// // A generator below 1 maps to a negative entry.
/// let entry = round_n_log2(12, &Z::from(1), &Z::from(2), NearestHalfAwayFromZero);
/// assert_eq!(entry, Some(Z::from(-12)));
/// ```
pub fn round_n_log2(n: u32, numer: &Z, denom: &Z, convention: RoundingConvention) -> Option<Z> {
    if !numer.is_positive() || !denom.is_positive() {
        return None;
    }

    let p = numer.pow(n);
    let q = denom.pow(n);
    let floor_k = floor_log2_ratio(&p, &q)?;

    let k = match convention {
        RoundingConvention::Floor => floor_k,
        RoundingConvention::Ceiling => {
            if cmp_with_pow2(&p, &q, floor_k)? == Ordering::Equal {
                floor_k
            } else {
                floor_k.checked_add(1)?
            }
        }
        RoundingConvention::NearestHalfAwayFromZero | RoundingConvention::NearestHalfToEven => {
            // Compare `n*log2(x)` with `floor_k + 1/2`, i.e. `x^(2n)` with
            // `2^(2*floor_k + 1)`.
            let p2 = &p * &p;
            let q2 = &q * &q;
            let half = floor_k.checked_mul(2)?.checked_add(1)?;
            match cmp_with_pow2(&p2, &q2, half)? {
                Ordering::Less => floor_k,
                Ordering::Greater => floor_k.checked_add(1)?,
                Ordering::Equal => match convention {
                    RoundingConvention::NearestHalfToEven => {
                        if floor_k % 2 == 0 {
                            floor_k
                        } else {
                            floor_k.checked_add(1)?
                        }
                    }
                    // Away from zero: the value is `floor_k + 1/2`, so it is
                    // negative exactly when `floor_k` is negative.
                    _ => {
                        if floor_k >= 0 {
                            floor_k.checked_add(1)?
                        } else {
                            floor_k
                        }
                    }
                },
            }
        }
    };

    Some(Z::from(k))
}

/// L3 real approximation of `log2(v)` for a positive integer `v`.
///
/// UMT layer: L3. The result is a real observation and MUST NOT be used for an
/// exact identity, equality, or membership decision (UMT-3.2 section 0.6.1).
///
/// Accurate for arbitrarily large `v`: the top 53 significant bits are taken
/// and the bit position is added back, so no intermediate `f64` overflow
/// occurs. Returns `None` if `v` is not positive.
#[must_use]
pub fn log2_f64(v: &Z) -> Option<f64> {
    if !v.is_positive() {
        return None;
    }
    let bits = i64::try_from(v.bits()).ok()?;
    if bits <= 53 {
        return Some(libm::log2(v.to_u64()? as f64));
    }
    let shift = bits - 53;
    let top = (v >> u64::try_from(shift).ok()?).to_u64()?;
    Some(libm::log2(top as f64) + shift as f64)
}

/// L3 real approximation of `numer / denom`.
///
/// UMT layer: L3. Returns `None` if `denom` is zero. Saturates to an infinity
/// or to zero if the true value is outside the `f64` exponent range.
#[must_use]
pub fn ratio_to_f64(numer: &Z, denom: &Z) -> Option<f64> {
    if denom.is_zero() {
        return None;
    }
    if numer.is_zero() {
        return Some(0.0);
    }
    let negative = numer.is_negative() != denom.is_negative();
    let a = numer.abs();
    let b = denom.abs();

    let bits_a = i64::try_from(a.bits()).ok()?;
    let bits_b = i64::try_from(b.bits()).ok()?;

    // Scale so the quotient keeps about 64 significant bits.
    let scale = 64i64.checked_sub(bits_a.checked_sub(bits_b)?)?;
    let scaled = if scale >= 0 {
        (&a << u64::try_from(scale).ok()?) / &b
    } else {
        (&a >> u64::try_from(scale.checked_neg()?).ok()?) / &b
    };
    if scaled.is_zero() {
        return Some(if negative { -0.0 } else { 0.0 });
    }

    let bits_s = i64::try_from(scaled.bits()).ok()?;
    let drop = if bits_s > 53 { bits_s - 53 } else { 0 };
    let mantissa = if drop > 0 {
        (&scaled >> u64::try_from(drop).ok()?).to_u64()?
    } else {
        scaled.to_u64()?
    };

    let exponent = drop.checked_sub(scale)?;
    let exponent =
        i32::try_from(exponent).unwrap_or(if exponent > 0 { i32::MAX } else { i32::MIN });
    let value = libm::ldexp(mantissa as f64, exponent);
    Some(if negative { -value } else { value })
}

/// L3 real approximation of `log2(numer / denom)` for positive arguments.
///
/// UMT layer: L3. Near a ratio of 1 - the interesting case for commas - the
/// naive difference of two logarithms cancels catastrophically, so `log1p` is
/// used instead. Returns `None` unless both arguments are positive.
#[must_use]
pub fn log2_ratio_f64(numer: &Z, denom: &Z) -> Option<f64> {
    if !numer.is_positive() || !denom.is_positive() {
        return None;
    }
    let delta = numer - denom;
    let doubled = delta.abs() << 1u64;
    if &doubled <= denom {
        let x = ratio_to_f64(&delta, denom)?;
        Some(libm::log1p(x) / core::f64::consts::LN_2)
    } else {
        Some(log2_f64(numer)? - log2_f64(denom)?)
    }
}

#[cfg(test)]
mod tests {
    use super::{Z, log2_f64, log2_ratio_f64, ratio_to_f64, round_n_log2};
    use crate::algebra::rounding::RoundingConvention::{
        Ceiling, Floor, NearestHalfAwayFromZero, NearestHalfToEven,
    };
    use alloc::string::ToString;
    use core::str::FromStr;

    fn z(v: i64) -> Z {
        Z::from(v)
    }

    #[test]
    fn twelve_edo_patent_entries() {
        for (prime, expected) in [(2, 12), (3, 19), (5, 28), (7, 34), (11, 42)] {
            assert_eq!(
                round_n_log2(12, &z(prime), &z(1), NearestHalfAwayFromZero),
                Some(z(expected))
            );
        }
    }

    #[test]
    fn octave_entry_is_fixed_to_n_under_every_convention() {
        // UMT-3.2 section 1.6: a generator with exact valuation 2 has entry N.
        for n in [0u32, 1, 2, 5, 6, 12, 31, 1201] {
            for convention in [NearestHalfAwayFromZero, NearestHalfToEven, Floor, Ceiling] {
                assert_eq!(
                    round_n_log2(n, &z(2), &z(1), convention),
                    Some(Z::from(n)),
                    "n = {n}"
                );
            }
        }
    }

    #[test]
    fn floor_and_ceiling_bracket_nearest() {
        // log2(3) = 1.58496...; at N = 12 the exact value is 19.019...
        assert_eq!(round_n_log2(12, &z(3), &z(1), Floor), Some(z(19)));
        assert_eq!(round_n_log2(12, &z(3), &z(1), Ceiling), Some(z(20)));
        // 5-EDO: 5 * log2(3) = 7.92..., so floor and nearest differ.
        assert_eq!(round_n_log2(5, &z(3), &z(1), Floor), Some(z(7)));
        assert_eq!(round_n_log2(5, &z(3), &z(1), Ceiling), Some(z(8)));
        assert_eq!(
            round_n_log2(5, &z(3), &z(1), NearestHalfAwayFromZero),
            Some(z(8))
        );
    }

    #[test]
    fn exact_powers_of_two_are_not_rounded_up_by_ceiling() {
        // 8 = 2^3 exactly, so ceiling must not add one.
        assert_eq!(round_n_log2(1, &z(8), &z(1), Ceiling), Some(z(3)));
        assert_eq!(round_n_log2(1, &z(8), &z(1), Floor), Some(z(3)));
        assert_eq!(round_n_log2(4, &z(1), &z(8), Ceiling), Some(z(-12)));
    }

    #[test]
    fn zero_multiplier_is_the_zero_map() {
        for prime in [2, 3, 5, 7] {
            assert_eq!(
                round_n_log2(0, &z(prime), &z(1), NearestHalfAwayFromZero),
                Some(z(0))
            );
        }
    }

    #[test]
    fn rational_generators_below_one_round_negatively() {
        // 3/4 is below 1: 12 * log2(3/4) = -4.98...
        assert_eq!(
            round_n_log2(12, &z(3), &z(4), NearestHalfAwayFromZero),
            Some(z(-5))
        );
        assert_eq!(round_n_log2(12, &z(3), &z(4), Floor), Some(z(-5)));
        assert_eq!(round_n_log2(12, &z(3), &z(4), Ceiling), Some(z(-4)));
    }

    #[test]
    fn non_positive_arguments_are_rejected() {
        assert!(round_n_log2(12, &z(0), &z(1), Floor).is_none());
        assert!(round_n_log2(12, &z(-3), &z(1), Floor).is_none());
        assert!(round_n_log2(12, &z(3), &z(0), Floor).is_none());
    }

    #[test]
    fn huge_generators_do_not_overflow() {
        // 10^200 - 1, just below 2^664.3857.
        let big = Z::from_str(&("9".repeat(200))).unwrap();
        assert_eq!(
            round_n_log2(1, &big, &z(1), Floor).unwrap().to_string(),
            "664"
        );
        // 100 * log2(10^200 - 1) = 66438.56..., which rounds to 66439.
        assert_eq!(
            round_n_log2(100, &big, &z(1), NearestHalfAwayFromZero)
                .unwrap()
                .to_string(),
            "66439"
        );
    }

    #[test]
    fn log2_of_large_integers_is_accurate() {
        assert!((log2_f64(&z(1024)).unwrap() - 10.0).abs() < 1e-12);
        let big = Z::from(1u64) << 400u64;
        assert!((log2_f64(&big).unwrap() - 400.0).abs() < 1e-9);
        assert!(log2_f64(&z(0)).is_none());
    }

    #[test]
    fn log2_of_a_comma_ratio_is_precise() {
        // Syntonic comma 81/80 = 21.5062895967... cents.
        let cents = log2_ratio_f64(&z(81), &z(80)).unwrap() * 1200.0;
        assert!((cents - 21.506_289_596_7).abs() < 1e-9, "{cents}");
        // The pythagorean comma 531441/524288 = 23.4600103... cents.
        let cents = log2_ratio_f64(&z(531_441), &z(524_288)).unwrap() * 1200.0;
        assert!((cents - 23.460_010_384_1).abs() < 1e-9, "{cents}");
        assert!((log2_ratio_f64(&z(2), &z(1)).unwrap() - 1.0).abs() < 1e-15);
        assert!((log2_ratio_f64(&z(1), &z(2)).unwrap() + 1.0).abs() < 1e-15);
    }

    #[test]
    fn ratio_conversion_handles_extremes() {
        assert!((ratio_to_f64(&z(1), &z(3)).unwrap() - 1.0 / 3.0).abs() < 1e-15);
        assert!((ratio_to_f64(&z(-3), &z(2)).unwrap() + 1.5).abs() < 1e-15);
        let big = Z::from(1u64) << 2000u64;
        assert_eq!(ratio_to_f64(&big, &z(1)).unwrap(), f64::INFINITY);
        assert_eq!(ratio_to_f64(&z(1), &big).unwrap(), 0.0);
        assert!(ratio_to_f64(&z(1), &z(0)).is_none());
    }
}
