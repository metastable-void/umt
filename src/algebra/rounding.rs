//! Declared rounding conventions.
//!
//! UMT-3.2 section 1.6 requires a patent val to declare its rounding
//! convention, and section 5.7.2 requires every quantizer to declare its
//! tie-breaking policy. The convention is part of the result and of its
//! provenance; it is never an implementation detail.

/// A declared rounding convention for real-to-integer maps.
///
/// UMT layer: policy declaration, applicable at L2 (patent-val construction)
/// and L4 (device quantization).
///
/// Equality is presentation equality on the declared policy.
///
/// Note that this type declares *which* real number is selected, not how it is
/// computed. [`crate::algebra::integer::round_n_log2`] applies these
/// conventions to `n * log2(p/q)` using exact integer arithmetic only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RoundingConvention {
    /// Nearest integer; exact halves round away from zero.
    NearestHalfAwayFromZero,
    /// Nearest integer; exact halves round to the even neighbour.
    NearestHalfToEven,
    /// Greatest integer not greater than the value.
    Floor,
    /// Least integer not less than the value.
    Ceiling,
}

impl RoundingConvention {
    /// Applies this convention to an exact rational, exactly.
    ///
    /// This is the path that matters above L3: a structural onset quantized to
    /// a device grid has an exact rational coordinate, and rounding it through
    /// `f64` would make the result depend on a representation the source data
    /// never had (UMT-3.2 section 0.6.1).
    #[must_use]
    pub fn apply_q(self, value: &crate::algebra::Q) -> crate::algebra::Z {
        use num_integer::Integer;
        use num_traits::{One, Zero};

        // `BigRational` keeps the denominator positive, so flooring the
        // numerator against it is the ordinary floor.
        let floor = value.numer().div_floor(value.denom());
        let fractional = value - crate::algebra::Q::from(floor.clone());
        let half = crate::algebra::Q::new(crate::algebra::Z::one(), crate::algebra::Z::from(2));

        match self {
            Self::Floor => floor,
            Self::Ceiling => {
                if fractional.numer().is_zero() {
                    floor
                } else {
                    floor + crate::algebra::Z::one()
                }
            }
            Self::NearestHalfAwayFromZero => match fractional.cmp(&half) {
                core::cmp::Ordering::Less => floor,
                core::cmp::Ordering::Greater => floor + crate::algebra::Z::one(),
                // An exact half: away from zero means up for a positive value
                // and down for a negative one, and `floor` is already the
                // lower neighbour.
                core::cmp::Ordering::Equal => {
                    if value.numer().sign() == num_bigint::Sign::Minus {
                        floor
                    } else {
                        floor + crate::algebra::Z::one()
                    }
                }
            },
            Self::NearestHalfToEven => match fractional.cmp(&half) {
                core::cmp::Ordering::Less => floor,
                core::cmp::Ordering::Greater => floor + crate::algebra::Z::one(),
                core::cmp::Ordering::Equal => {
                    if floor.is_even() {
                        floor
                    } else {
                        floor + crate::algebra::Z::one()
                    }
                }
            },
        }
    }

    /// Applies this convention to an L3 real value.
    ///
    /// This is the *metric* path, used when the source quantity is only
    /// available as a real observation. It is implemented with [`libm`] rather
    /// than platform math so results do not vary with the `std` feature or the
    /// host libm.
    ///
    /// Returns the input unchanged if it is not finite; callers that require
    /// finiteness must validate before calling.
    #[must_use]
    pub fn apply_f64(self, x: f64) -> f64 {
        if !x.is_finite() {
            return x;
        }
        match self {
            Self::Floor => libm::floor(x),
            Self::Ceiling => libm::ceil(x),
            Self::NearestHalfAwayFromZero => {
                // libm::round is defined as half-away-from-zero.
                libm::round(x)
            }
            Self::NearestHalfToEven => {
                let lo = libm::floor(x);
                let frac = x - lo;
                let round_up = if frac == 0.5 {
                    // Exact tie: take the even neighbour.
                    libm::fmod(lo, 2.0) != 0.0
                } else {
                    frac > 0.5
                };
                if round_up { lo + 1.0 } else { lo }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoundingConvention::{Ceiling, Floor, NearestHalfAwayFromZero, NearestHalfToEven};

    #[test]
    fn conventions_differ_where_they_should() {
        assert_eq!(NearestHalfAwayFromZero.apply_f64(2.5), 3.0);
        assert_eq!(NearestHalfToEven.apply_f64(2.5), 2.0);
        assert_eq!(NearestHalfAwayFromZero.apply_f64(3.5), 4.0);
        assert_eq!(NearestHalfToEven.apply_f64(3.5), 4.0);
        assert_eq!(NearestHalfAwayFromZero.apply_f64(-2.5), -3.0);
        assert_eq!(NearestHalfToEven.apply_f64(-2.5), -2.0);
        assert_eq!(Floor.apply_f64(-2.5), -3.0);
        assert_eq!(Ceiling.apply_f64(-2.5), -2.0);
        assert_eq!(Floor.apply_f64(2.0), 2.0);
        assert_eq!(Ceiling.apply_f64(2.0), 2.0);
    }
}
