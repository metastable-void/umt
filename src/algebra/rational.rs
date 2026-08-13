//! Exact rationals.

use num_rational::BigRational;

use crate::algebra::integer::{Z, log2_ratio_f64};

/// Exact rational number, always kept in lowest terms with a positive
/// denominator by the underlying implementation.
///
/// UMT layer: L1/L2 storage. This is the canonical storage type for exact
/// generator valuations, exact ratios `r(m)`, structural beat durations, and
/// rhythm-tree child weights. Floating point is never the source of truth for
/// any of those (UMT-3.2 section 0.6.1, prompt section 23).
pub type Q = BigRational;

/// L3 real approximation of `log2(value)` for a positive rational.
///
/// UMT layer: L3. Returns `None` unless `value` is strictly positive. Precise
/// near 1, so comma sizes in cents are meaningful.
///
/// # Examples
///
/// ```
/// use umt::algebra::rational::{log2_q_f64, Q};
/// use umt::Z;
///
/// let comma = Q::new(Z::from(81), Z::from(80));
/// let cents = log2_q_f64(&comma).unwrap() * 1200.0;
/// assert!((cents - 21.5062895967).abs() < 1e-9);
/// ```
#[must_use]
pub fn log2_q_f64(value: &Q) -> Option<f64> {
    log2_ratio_f64(value.numer(), value.denom())
}

/// Builds an exact rational from integer parts, rejecting a zero denominator.
///
/// Returns `None` if `denom` is zero. This exists so that no public path can
/// panic on a caller-supplied denominator (prompt section 58).
#[must_use]
pub fn q_checked(numer: Z, denom: Z) -> Option<Q> {
    if denom == Z::from(0) {
        None
    } else {
        Some(Q::new(numer, denom))
    }
}

#[cfg(test)]
mod tests {
    use super::{Q, log2_q_f64, q_checked};
    use crate::algebra::integer::Z;

    #[test]
    fn zero_denominator_is_rejected() {
        assert!(q_checked(Z::from(3), Z::from(0)).is_none());
        assert_eq!(
            q_checked(Z::from(6), Z::from(4)),
            Some(Q::new(Z::from(3), Z::from(2)))
        );
    }

    #[test]
    fn log2_rejects_non_positive() {
        assert!(log2_q_f64(&Q::new(Z::from(-3), Z::from(2))).is_none());
        assert!(log2_q_f64(&Q::new(Z::from(0), Z::from(1))).is_none());
    }
}
