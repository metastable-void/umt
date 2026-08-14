//! Shared machinery for validated real-valued quantity newtypes (prompt
//! section 18).
//!
//! L3 quantities are not interchangeable `f64`s. Seconds are not octaves, a
//! duration is not a position, and seconds-per-beat is not beats-per-second.
//! Every such type wants the same wrapper - validated at construction, totally
//! ordered, hashable - so it is written once here and instantiated per
//! semantic type rather than copied.
//!
//! Two macros, because the two shapes differ. Everything gets
//! [`finite_newtype`]; only *intervals* additionally get
//! [`interval_arithmetic`], because intervals form a group and points do not
//! (UMT-3.2 section 1.10).

/// Defines a newtype over a validated finite `f64`.
///
/// Rejects infinities and NaN at construction and normalizes `-0.0` to `0.0`,
/// which makes bitwise equality total and consistent with numeric equality.
macro_rules! finite_newtype {
    ($name:ident, $error:ty, $nonfinite:expr, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
        pub struct $name(f64);

        impl $name {
            /// Accepts a finite value.
            ///
            /// # Errors
            ///
            /// Rejects infinities and NaN.
            pub fn new(value: f64) -> Result<Self, $error> {
                if value.is_finite() {
                    Ok(Self(if value == 0.0 { 0.0 } else { value }))
                } else {
                    Err($nonfinite)
                }
            }

            /// The underlying value.
            #[must_use]
            pub fn get(self) -> f64 {
                self.0
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.to_bits() == other.0.to_bits()
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.0
                    .partial_cmp(&other.0)
                    .expect("invariant: the value is finite")
            }
        }

        impl core::hash::Hash for $name {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.0.to_bits().hash(state);
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl TryFrom<f64> for $name {
            type Error = $error;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

/// Gives an interval newtype its group structure.
///
/// Applied only to differences, never to positions: `Add` between two points
/// is not an operation UMT-3.2 admits.
macro_rules! interval_arithmetic {
    ($name:ident, $error:ty) => {
        impl $name {
            /// Zero: the neutral element of the interval group.
            pub const ZERO: Self = Self(0.0);

            /// Scales an interval by a real factor.
            ///
            /// # Errors
            ///
            /// Fails if the factor or the result is not finite.
            pub fn scale(self, factor: f64) -> Result<Self, $error> {
                Self::new(self.0 * factor)
            }

            /// The magnitude, discarding direction.
            #[must_use]
            pub fn abs(self) -> Self {
                Self(self.0.abs())
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                Self(if self.0 == 0.0 { 0.0 } else { -self.0 })
            }
        }

        impl core::ops::Add for $name {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                Self(self.0 + other.0)
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                Self(self.0 - other.0)
            }
        }
    };
}

pub(crate) use finite_newtype;
pub(crate) use interval_arithmetic;
