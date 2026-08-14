//! Physical and metric pitch quantities (UMT-3.2 section 1.2, prompt section
//! 18).
//!
//! These are L3. Every one is a newtype over a validated `f64`, so a frequency
//! cannot be passed where a log-frequency is wanted, and an interval cannot be
//! passed where a point is wanted. That is not ceremony: the specification's
//! whole L2/L3 discipline rests on those distinctions being visible.
//!
//! Equality and hashing are presentation equality on the bit pattern, which is
//! total here because NaN is excluded at construction and `-0.0` is normalized
//! to `0.0`.

use crate::error::PitchError;

/// Cents per octave.
pub const CENTS_PER_OCTAVE: f64 = 1200.0;

macro_rules! finite_newtype {
    ($name:ident, $doc:expr) => {
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
            /// Returns [`PitchError::NonFiniteQuantity`] for infinities and
            /// NaN.
            pub fn new(value: f64) -> Result<Self, PitchError> {
                if value.is_finite() {
                    Ok(Self(if value == 0.0 { 0.0 } else { value }))
                } else {
                    Err(PitchError::NonFiniteQuantity)
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
            type Error = PitchError;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

macro_rules! interval_arithmetic {
    ($name:ident) => {
        impl $name {
            /// Zero: the unison.
            pub const ZERO: Self = Self(0.0);

            /// Scales an interval by a real factor.
            ///
            /// # Errors
            ///
            /// Returns [`PitchError::NonFiniteQuantity`] if the factor or the
            /// result is not finite.
            pub fn scale(self, factor: f64) -> Result<Self, PitchError> {
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

        // Intervals form a group, so they add. Points do not, which is why no
        // such implementation exists for `LogFrequency`.
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

finite_newtype!(
    Octaves,
    "An interval measured in octaves, that is, in `log2` of a frequency ratio.\n\nUMT layer: L3. This is an *interval*, not a pitch: it is the difference\nbetween two [`LogFrequency`] points, and intervals add while points do not."
);
finite_newtype!(
    Cents,
    "An interval measured in cents, 1200 to the octave.\n\nUMT layer: L3. Conversion to and from [`Octaves`] is exact scaling by 1200,\nso it is lossless and available as `From`."
);
finite_newtype!(
    LogFrequency,
    "A pitch position as `log2` of a frequency in hertz.\n\nUMT layer: L3. This is a *point*, not an interval. It has a canonical origin\nat 1 Hz, so unlike the structural pitch torsor it needs no declared origin,\nbut point plus point is still not an operation."
);

interval_arithmetic!(Octaves);
interval_arithmetic!(Cents);

impl From<Octaves> for Cents {
    fn from(value: Octaves) -> Self {
        Self(value.0 * CENTS_PER_OCTAVE)
    }
}

impl From<Cents> for Octaves {
    fn from(value: Cents) -> Self {
        Self(value.0 / CENTS_PER_OCTAVE)
    }
}

impl core::fmt::Display for Cents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.4} cents", self.0)
    }
}

impl core::fmt::Display for Octaves {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.6} oct", self.0)
    }
}

/// A frequency in hertz.
///
/// UMT layer: L3. Strictly positive: a frequency of zero or below has no
/// logarithm, and the whole pitch model is logarithmic.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
pub struct FrequencyHz(f64);

impl FrequencyHz {
    /// Accepts a positive finite frequency.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NonPositiveFrequency`] for zero, negative,
    /// infinite, and NaN inputs.
    pub fn new(value: f64) -> Result<Self, PitchError> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(PitchError::NonPositiveFrequency)
        }
    }

    /// The value in hertz.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }

    /// The corresponding log-frequency.
    #[must_use]
    pub fn log_frequency(self) -> LogFrequency {
        LogFrequency(libm::log2(self.0))
    }
}

impl LogFrequency {
    /// The corresponding frequency in hertz.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NonPositiveFrequency`] if the exponential
    /// overflows to infinity.
    pub fn frequency(self) -> Result<FrequencyHz, PitchError> {
        FrequencyHz::new(libm::exp2(self.0))
    }

    /// Moves this pitch by an interval.
    ///
    /// This is the torsor action: point plus interval is a point.
    #[must_use]
    pub fn translate(self, interval: Octaves) -> Self {
        Self(self.0 + interval.0)
    }

    /// The interval from this pitch to another.
    ///
    /// This is `int(p, q)`: point minus point is an interval. There is
    /// deliberately no point-plus-point operation (UMT-3.2 section 1.10).
    #[must_use]
    pub fn interval_to(self, other: Self) -> Octaves {
        Octaves(other.0 - self.0)
    }
}

impl PartialEq for FrequencyHz {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FrequencyHz {}

impl core::hash::Hash for FrequencyHz {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl From<FrequencyHz> for f64 {
    fn from(value: FrequencyHz) -> Self {
        value.0
    }
}

impl TryFrom<f64> for FrequencyHz {
    type Error = PitchError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl core::fmt::Display for FrequencyHz {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.4} Hz", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cents, FrequencyHz, LogFrequency, Octaves};
    use crate::error::PitchError;

    #[test]
    fn validation_rejects_what_it_must() {
        assert!(Octaves::new(f64::NAN).is_err());
        assert!(Octaves::new(f64::INFINITY).is_err());
        assert!(Octaves::new(-2.5).is_ok(), "intervals may be negative");
        assert_eq!(FrequencyHz::new(0.0), Err(PitchError::NonPositiveFrequency));
        assert_eq!(
            FrequencyHz::new(-440.0),
            Err(PitchError::NonPositiveFrequency)
        );
        assert!(FrequencyHz::new(440.0).is_ok());
    }

    #[test]
    fn negative_zero_is_normalized() {
        assert_eq!(Octaves::new(-0.0).unwrap(), Octaves::new(0.0).unwrap());
        assert_eq!((-Octaves::ZERO).get().to_bits(), 0.0f64.to_bits());
    }

    #[test]
    fn octaves_and_cents_convert_losslessly() {
        let octave = Octaves::new(1.0).unwrap();
        assert_eq!(Cents::from(octave), Cents::new(1200.0).unwrap());
        assert_eq!(Octaves::from(Cents::from(octave)), octave);

        let comma = Cents::new(21.506_289_596_7).unwrap();
        assert!((Octaves::from(comma).get() - 0.017_921_908_0).abs() < 1e-9);
    }

    #[test]
    fn intervals_add_and_points_do_not() {
        let fifth = Octaves::new(0.584_962_500_721_156).unwrap();
        let fourth = Octaves::new(0.415_037_499_278_844).unwrap();
        assert!(((fifth + fourth).get() - 1.0).abs() < 1e-12);
        assert!(((fifth - fifth).get()).abs() < 1e-15);
        assert!((fifth.scale(2.0).unwrap().get() - 2.0 * fifth.get()).abs() < 1e-12);
        assert_eq!((-fifth + fifth), Octaves::ZERO);
    }

    #[test]
    fn the_pitch_torsor_acts_as_a_torsor() {
        let a440 = FrequencyHz::new(440.0).unwrap().log_frequency();
        let octave = Octaves::new(1.0).unwrap();
        let a880 = a440.translate(octave);

        // p + int(p, q) = q
        assert_eq!(a440.translate(a440.interval_to(a880)), a880);
        // int(p, q) + int(q, r) = int(p, r)
        let a1760 = a880.translate(octave);
        assert!(
            ((a440.interval_to(a880) + a880.interval_to(a1760)).get()
                - a440.interval_to(a1760).get())
            .abs()
                < 1e-12
        );
        // p + 0 = p
        assert_eq!(a440.translate(Octaves::ZERO), a440);

        assert!((a880.frequency().unwrap().get() - 880.0).abs() < 1e-9);
    }

    #[test]
    fn frequency_and_log_frequency_round_trip() {
        for hz in [1.0f64, 27.5, 261.625_565_3, 440.0, 20_000.0] {
            let frequency = FrequencyHz::new(hz).unwrap();
            let back = frequency.log_frequency().frequency().unwrap();
            assert!((back.get() - hz).abs() < 1e-9 * hz, "{hz}");
        }
        assert_eq!(
            FrequencyHz::new(1.0).unwrap().log_frequency(),
            LogFrequency::new(0.0).unwrap()
        );
    }

    #[test]
    fn ordering_is_available_for_sorting() {
        let mut values = [
            Cents::new(700.0).unwrap(),
            Cents::new(-50.0).unwrap(),
            Cents::new(0.0).unwrap(),
        ];
        values.sort();
        assert_eq!(values[0], Cents::new(-50.0).unwrap());
        assert_eq!(values[2], Cents::new(700.0).unwrap());
    }
}
