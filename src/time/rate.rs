//! Tempo rates and the rate/duration orientation rule (UMT-3.2 sections 2.1
//! and 5.6, prompt sections 18, 29, and 30).
//!
//! # Two dimensions that are not interchangeable
//!
//! The derivative of clock time with respect to beat time has dimension
//! *clock time per beat*; its reciprocal is *beats per clock time*. Prompt
//! section 18 requires that "the library must prevent accidental use of
//! seconds-per-beat as beats-per-second", so [`SecondsPerBeat`] and
//! [`BeatsPerSecond`] are different types with no arithmetic between them,
//! only an explicit [`SecondsPerBeat::reciprocal`].
//!
//! # The orientation rule
//!
//! Section 2.1 is emphatic: "A system MUST NOT silently reuse a rate ratio as
//! a duration ratio without accounting for this inversion." A bare `3/2` does
//! not say whether it means "half again as fast" or "half again as long", and
//! those are opposite instructions.
//!
//! So a bare ratio is not applicable here. [`OrientedRatio`] pairs the exact
//! proportion with the quantity it multiplies, and applying it to the
//! reciprocal quantity inverts it - visibly, in the type, and without the
//! caller having to remember. That is fixture F32.
//!
//! # Tempo proportions are not grid quantization
//!
//! Section 5.6 puts exact tempo ratios in the multiplicative proportion domain
//! of part I, so a metric modulation is a [`crate::Monzo`] and a chain of them
//! is a path in a proportion lattice - complete with kernels and residues if a
//! composer means it that way. Rounding an onset to a PPQN grid is a different
//! operation with a different residual type, and it lives in
//! [`crate::time::quantize`]. Prompt section 29: do not share one generic
//! "temper" function between them.

use num_traits::{Signed, Zero};

use crate::algebra::Q;
use crate::error::TimeError;
use crate::quantity::finite_newtype;
use crate::time::beat::{BeatDuration, Beats};

/// Seconds of clock time per structural beat.
///
/// UMT layer: L3. This is the dimension of `theta'`, the derivative of a tempo
/// map (UMT-3.2 section 5.8.2). Strictly positive: a tempo map is strictly
/// increasing, so its derivative cannot be zero or negative where it exists.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
pub struct SecondsPerBeat(f64);

/// Structural beats per second of clock time.
///
/// UMT layer: L3. The reciprocal of [`SecondsPerBeat`], and deliberately a
/// different type: the two are numerically distinct and confusing them
/// silently inverts every tempo in a score.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
pub struct BeatsPerSecond(f64);

macro_rules! positive_rate {
    ($name:ident) => {
        impl $name {
            /// Accepts a strictly positive finite rate.
            ///
            /// # Errors
            ///
            /// Returns [`TimeError::NonPositiveRate`] for zero, negative,
            /// infinite, and NaN inputs.
            pub fn new(value: f64) -> Result<Self, TimeError> {
                if value.is_finite() && value > 0.0 {
                    Ok(Self(value))
                } else {
                    Err(TimeError::NonPositiveRate { rate: value })
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
            type Error = TimeError;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

positive_rate!(SecondsPerBeat);
positive_rate!(BeatsPerSecond);

impl SecondsPerBeat {
    /// The corresponding beat rate.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRate`] if the reciprocal underflows to
    /// zero or overflows.
    pub fn reciprocal(self) -> Result<BeatsPerSecond, TimeError> {
        BeatsPerSecond::new(1.0 / self.0)
    }

    /// From a tempo marking in beats per minute.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRate`] for a non-positive marking.
    pub fn from_bpm(beats_per_minute: f64) -> Result<Self, TimeError> {
        if !beats_per_minute.is_finite() || beats_per_minute <= 0.0 {
            return Err(TimeError::NonPositiveRate {
                rate: beats_per_minute,
            });
        }
        Self::new(60.0 / beats_per_minute)
    }

    /// As a tempo marking in beats per minute.
    #[must_use]
    pub fn to_bpm(self) -> f64 {
        60.0 / self.0
    }
}

impl BeatsPerSecond {
    /// The corresponding clock-time-per-beat.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRate`] if the reciprocal underflows to
    /// zero or overflows.
    pub fn reciprocal(self) -> Result<SecondsPerBeat, TimeError> {
        SecondsPerBeat::new(1.0 / self.0)
    }
}

impl core::fmt::Display for SecondsPerBeat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.6} s/beat", self.0)
    }
}

impl core::fmt::Display for BeatsPerSecond {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.6} beats/s", self.0)
    }
}

finite_newtype!(
    BeatsPerMinute,
    TimeError,
    TimeError::NonFiniteQuantity,
    "A tempo marking in beats per minute.\n\nUMT layer: L3. Present because it is the unit musicians write, not because\nit is a third dimension: it is [`BeatsPerSecond`] times sixty, and converts\nto [`SecondsPerBeat`] only through the explicit reciprocal."
);

/// Which quantity an exact proportion multiplies (UMT-3.2 section 2.1).
///
/// The same abstract proportion may be read as a frequency ratio, a tempo
/// ratio, or a subdivision ratio, but when it crosses between a rate and its
/// reciprocal duration it inverts. Recording which side it was declared on is
/// what makes the crossing safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RatioOrientation {
    /// The ratio multiplies a *rate*: `f -> rho f`. Larger means faster.
    Rate,
    /// The ratio multiplies a *duration*: `d -> rho d`. Larger means longer.
    Duration,
}

impl RatioOrientation {
    /// The opposite orientation.
    #[must_use]
    pub fn flipped(self) -> Self {
        match self {
            Self::Rate => Self::Duration,
            Self::Duration => Self::Rate,
        }
    }
}

/// An exact positive proportion together with the quantity it multiplies
/// (UMT-3.2 section 2.1, fixture F32).
///
/// UMT layer: L1, exact.
///
/// There is deliberately no way to apply a bare [`Q`] to both a rate and a
/// duration. Section 2.1 forbids reusing a rate ratio as a duration ratio
/// without accounting for the inversion, and the only reliable way to enforce
/// that is to refuse to accept a proportion that has not said which it is.
///
/// # Examples
///
/// ```
/// use umt::algebra::{Q, Z};
/// use umt::time::{BeatDuration, BeatsPerSecond, OrientedRatio, RatioOrientation};
///
/// // "Three halves as fast."
/// let faster = OrientedRatio::new(Q::new(Z::from(3), Z::from(2)), RatioOrientation::Rate)?;
///
/// let tempo = BeatsPerSecond::new(2.0)?;
/// assert_eq!(faster.scale_rate(tempo)?.get(), 3.0);
///
/// // The same proportion, applied to the reciprocal quantity, inverts.
/// let beat = BeatDuration::one();
/// assert_eq!(*faster.scale_duration(&beat)?.get(), Q::new(Z::from(2), Z::from(3)));
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrientedRatio {
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
    ratio: Q,
    orientation: RatioOrientation,
}

impl OrientedRatio {
    /// Declares a positive exact proportion and what it multiplies.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRatio`] for a ratio that is not
    /// strictly positive. A proportion acts on positive quantities, and a
    /// non-positive one has no meaning here.
    pub fn new(ratio: Q, orientation: RatioOrientation) -> Result<Self, TimeError> {
        if !ratio.is_positive() {
            return Err(TimeError::NonPositiveRatio);
        }
        Ok(Self { ratio, orientation })
    }

    /// The exact proportion, as declared.
    #[must_use]
    pub fn ratio(&self) -> &Q {
        &self.ratio
    }

    /// What the proportion was declared to multiply.
    #[must_use]
    pub fn orientation(&self) -> RatioOrientation {
        self.orientation
    }

    /// The same proportion, re-expressed for the reciprocal quantity.
    ///
    /// The ratio inverts and the orientation flips, so the two forms describe
    /// the same physical change.
    #[must_use]
    pub fn reoriented(&self) -> Self {
        Self {
            ratio: self.ratio.recip(),
            orientation: self.orientation.flipped(),
        }
    }

    /// The factor this proportion applies to a rate.
    #[must_use]
    pub fn rate_factor(&self) -> Q {
        match self.orientation {
            RatioOrientation::Rate => self.ratio.clone(),
            RatioOrientation::Duration => self.ratio.recip(),
        }
    }

    /// The factor this proportion applies to a duration.
    #[must_use]
    pub fn duration_factor(&self) -> Q {
        match self.orientation {
            RatioOrientation::Rate => self.ratio.recip(),
            RatioOrientation::Duration => self.ratio.clone(),
        }
    }

    /// Applies this proportion to a beat rate.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRate`] if the result is not a positive
    /// finite rate.
    pub fn scale_rate(&self, rate: BeatsPerSecond) -> Result<BeatsPerSecond, TimeError> {
        let factor = ratio_to_f64(&self.rate_factor())
            .ok_or(TimeError::NonPositiveRate { rate: rate.get() })?;
        BeatsPerSecond::new(rate.get() * factor)
    }

    /// Applies this proportion to a clock-time-per-beat.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveRate`] if the result is not a positive
    /// finite rate.
    pub fn scale_seconds_per_beat(
        &self,
        rate: SecondsPerBeat,
    ) -> Result<SecondsPerBeat, TimeError> {
        // Seconds per beat is a duration per unit structure, so it follows the
        // duration factor, not the rate factor.
        let factor = ratio_to_f64(&self.duration_factor())
            .ok_or(TimeError::NonPositiveRate { rate: rate.get() })?;
        SecondsPerBeat::new(rate.get() * factor)
    }

    /// Applies this proportion to an exact structural duration.
    ///
    /// Exact throughout: no floating point is involved, and none is needed.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveDuration`] only if the input was
    /// somehow non-positive, which its type prevents.
    pub fn scale_duration(&self, duration: &BeatDuration) -> Result<BeatDuration, TimeError> {
        duration.scale(&self.duration_factor())
    }

    /// Applies this proportion to a signed exact duration.
    #[must_use]
    pub fn scale_signed_duration(&self, duration: &Beats) -> Beats {
        duration.scale(&self.duration_factor())
    }
}

impl core::fmt::Display for OrientedRatio {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.orientation {
            RatioOrientation::Rate => write!(f, "{} (rate)", self.ratio),
            RatioOrientation::Duration => write!(f, "{} (duration)", self.ratio),
        }
    }
}

fn ratio_to_f64(value: &Q) -> Option<f64> {
    if value.is_zero() {
        return Some(0.0);
    }
    crate::algebra::integer::ratio_to_f64(value.numer(), value.denom())
}

#[cfg(test)]
mod tests {
    use super::{BeatsPerMinute, BeatsPerSecond, OrientedRatio, RatioOrientation, SecondsPerBeat};
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;
    use crate::time::beat::BeatDuration;

    fn three_halves(orientation: RatioOrientation) -> OrientedRatio {
        OrientedRatio::new(Q::new(Z::from(3), Z::from(2)), orientation).unwrap()
    }

    #[test]
    fn the_two_tempo_dimensions_are_different_types() {
        let spb = SecondsPerBeat::from_bpm(120.0).unwrap();
        assert!((spb.get() - 0.5).abs() < 1e-12);
        assert!((spb.to_bpm() - 120.0).abs() < 1e-12);

        let bps = spb.reciprocal().unwrap();
        assert!((bps.get() - 2.0).abs() < 1e-12);
        assert_eq!(bps.reciprocal().unwrap(), spb);

        // The values differ, which is exactly why the types must.
        assert!((spb.get() - bps.get()).abs() > 1.0);
    }

    #[test]
    fn rates_are_strictly_positive() {
        assert!(matches!(
            SecondsPerBeat::new(0.0),
            Err(TimeError::NonPositiveRate { .. })
        ));
        assert!(BeatsPerSecond::new(-1.0).is_err());
        assert!(SecondsPerBeat::from_bpm(0.0).is_err());
        assert!(BeatsPerMinute::new(f64::NAN).is_err());
    }

    #[test]
    fn f32_a_rate_ratio_inverts_when_it_crosses_to_a_duration() {
        let faster = three_halves(RatioOrientation::Rate);
        assert_eq!(*faster.rate_factor().numer(), Z::from(3));
        assert_eq!(faster.duration_factor(), Q::new(Z::from(2), Z::from(3)));

        let tempo = BeatsPerSecond::new(2.0).unwrap();
        assert!((faster.scale_rate(tempo).unwrap().get() - 3.0).abs() < 1e-12);

        let beat = BeatDuration::one();
        assert_eq!(
            *faster.scale_duration(&beat).unwrap().get(),
            Q::new(Z::from(2), Z::from(3)),
            "three halves as fast is two thirds as long"
        );
    }

    #[test]
    fn the_same_numeral_declared_the_other_way_means_the_opposite() {
        let faster = three_halves(RatioOrientation::Rate);
        let longer = three_halves(RatioOrientation::Duration);
        assert_ne!(faster, longer, "the numeral alone does not determine it");

        let beat = BeatDuration::one();
        assert_eq!(
            *longer.scale_duration(&beat).unwrap().get(),
            Q::new(Z::from(3), Z::from(2))
        );
        assert_eq!(
            *faster.scale_duration(&beat).unwrap().get(),
            Q::new(Z::from(2), Z::from(3))
        );

        // Reorienting is the identity on the physical change.
        let reoriented = faster.reoriented();
        assert_eq!(reoriented.orientation(), RatioOrientation::Duration);
        assert_eq!(reoriented.duration_factor(), faster.duration_factor());
        assert_eq!(reoriented.rate_factor(), faster.rate_factor());
        assert_eq!(reoriented.reoriented(), faster);
    }

    #[test]
    fn seconds_per_beat_follows_the_duration_factor() {
        // Twice as fast halves the seconds per beat, and doubles the beats per
        // second. Getting this backwards is the bug the types exist to stop.
        let twice = OrientedRatio::new(Q::from(Z::from(2)), RatioOrientation::Rate).unwrap();
        let slow = SecondsPerBeat::from_bpm(60.0).unwrap();
        let fast = twice.scale_seconds_per_beat(slow).unwrap();
        assert!((fast.to_bpm() - 120.0).abs() < 1e-9);
        assert!(fast.get() < slow.get());
    }

    #[test]
    fn a_non_positive_proportion_is_rejected() {
        assert_eq!(
            OrientedRatio::new(Q::from(Z::from(0)), RatioOrientation::Rate),
            Err(TimeError::NonPositiveRatio)
        );
        assert!(OrientedRatio::new(Q::from(Z::from(-3)), RatioOrientation::Duration).is_err());
    }

    #[test]
    fn a_metric_modulation_chain_is_exact() {
        // 3:2 then 5:4 then 4:5 then 2:3 returns exactly to the start, which
        // is the point of keeping tempo proportions in the exact domain
        // (UMT-3.2 section 5.6).
        let mut duration = BeatDuration::one();
        for (numer, denom) in [(3, 2), (5, 4), (4, 5), (2, 3)] {
            let step = OrientedRatio::new(
                Q::new(Z::from(numer), Z::from(denom)),
                RatioOrientation::Rate,
            )
            .unwrap();
            duration = step.scale_duration(&duration).unwrap();
        }
        assert_eq!(duration, BeatDuration::one());
    }
}
