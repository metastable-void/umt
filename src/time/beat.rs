//! Exact structural time (UMT-3.2 section 5.1, prompt section 23).
//!
//! The structural beat timeline `T_b` is an affine torsor over an exact
//! ordered duration group `D_b`. This crate implements the default notated
//! profile, `D_b = Q`, which represents ordinary rational subdivisions and
//! nested tuplets exactly. Section 5.1 permits a different exact ordered
//! additive group provided it is declared; [`BEAT_DURATION_GROUP`] is this
//! crate's declaration.
//!
//! **The declared beat unit is one quarter note.** Nothing forces that choice
//! mathematically - the group is the same whatever the unit means - but a unit
//! has to be fixed for `Beats::new(1)` to have a meaning, and the quarter note
//! is the unit that PPQN device grids are already expressed in (section 5.7).
//!
//! Three types, because three things are different:
//!
//! - [`BeatTime`] is a *position*. Positions subtract to durations and do not
//!   add, exactly as at L3 (section 1.10).
//! - [`Beats`] is a signed *difference* between positions. It is the group
//!   `D_b` itself.
//! - [`BeatDuration`] is a strictly positive duration: the value a notated
//!   note or a rhythm-tree weight can take. Zero is excluded because a
//!   zero-duration note is not a short note, and section 5.8.4 requires
//!   zero-structural-duration delays to be modelled explicitly rather than
//!   smuggled in as a degenerate span.
//!
//! Everything here is exact. No structural time value is ever a `f64`.

use num_traits::{One, Signed, Zero};

use crate::algebra::{Q, Z};
use crate::error::TimeError;

/// The declared exact ordered additive group of structural durations
/// (UMT-3.2 section 5.1).
pub const BEAT_DURATION_GROUP: &str = "Q";

/// The declared structural beat unit.
pub const BEAT_UNIT: &str = "quarter note";

/// A signed exact structural duration: an element of `D_b = Q`, in beats.
///
/// UMT layer: L1/L2, exact. This is a *difference* between two [`BeatTime`]
/// positions, so it may be negative, and it forms a group under addition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Beats(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))] Q);

impl Beats {
    /// The zero duration.
    #[must_use]
    pub fn zero() -> Self {
        Self(Q::zero())
    }

    /// Wraps an exact number of beats.
    #[must_use]
    pub fn new(beats: Q) -> Self {
        Self(beats)
    }

    /// `numerator / denominator` beats.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ZeroDenominator`] for a zero denominator.
    pub fn ratio(numerator: i64, denominator: i64) -> Result<Self, TimeError> {
        if denominator == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        Ok(Self(Q::new(Z::from(numerator), Z::from(denominator))))
    }

    /// A whole note, that is, four beats.
    #[must_use]
    pub fn whole() -> Self {
        Self(Q::from(Z::from(4)))
    }

    /// A half note.
    #[must_use]
    pub fn half() -> Self {
        Self(Q::from(Z::from(2)))
    }

    /// A quarter note: one beat, the declared unit.
    #[must_use]
    pub fn quarter() -> Self {
        Self(Q::one())
    }

    /// An eighth note.
    #[must_use]
    pub fn eighth() -> Self {
        Self(Q::new(Z::from(1), Z::from(2)))
    }

    /// A sixteenth note.
    #[must_use]
    pub fn sixteenth() -> Self {
        Self(Q::new(Z::from(1), Z::from(4)))
    }

    /// The exact value in beats.
    #[must_use]
    pub fn get(&self) -> &Q {
        &self.0
    }

    /// Whether this duration is strictly positive.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.0.is_positive()
    }

    /// Whether this duration is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Scales by an exact factor.
    #[must_use]
    pub fn scale(&self, factor: &Q) -> Self {
        Self(&self.0 * factor)
    }

    /// The magnitude, discarding direction.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// Promotes to a strictly positive duration.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveDuration`] for zero or negative values.
    pub fn to_duration(&self) -> Result<BeatDuration, TimeError> {
        BeatDuration::new(self.0.clone())
    }
}

impl core::ops::Add for &Beats {
    type Output = Beats;

    fn add(self, other: &Beats) -> Beats {
        Beats(&self.0 + &other.0)
    }
}

impl core::ops::Sub for &Beats {
    type Output = Beats;

    fn sub(self, other: &Beats) -> Beats {
        Beats(&self.0 - &other.0)
    }
}

impl core::ops::Neg for &Beats {
    type Output = Beats;

    fn neg(self) -> Beats {
        Beats(-&self.0)
    }
}

impl core::iter::Sum for Beats {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), |total, next| &total + &next)
    }
}

impl From<BeatDuration> for Beats {
    fn from(value: BeatDuration) -> Self {
        Self(value.0)
    }
}

impl core::fmt::Display for Beats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} beats", self.0)
    }
}

/// A strictly positive exact structural duration.
///
/// UMT layer: L1/L2, exact. Notated note values and rhythm-tree weights take
/// this type, so a zero or negative "duration" cannot be constructed by
/// accident and then propagate silently through a division.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "Beats", into = "Beats"))]
pub struct BeatDuration(Q);

impl BeatDuration {
    /// Accepts a strictly positive exact duration.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveDuration`] for zero or negative values.
    pub fn new(beats: Q) -> Result<Self, TimeError> {
        if beats.is_positive() {
            Ok(Self(beats))
        } else {
            Err(TimeError::NonPositiveDuration)
        }
    }

    /// `numerator / denominator` beats.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ZeroDenominator`] or
    /// [`TimeError::NonPositiveDuration`].
    pub fn ratio(numerator: i64, denominator: i64) -> Result<Self, TimeError> {
        Self::new(Beats::ratio(numerator, denominator)?.0)
    }

    /// One beat.
    #[must_use]
    pub fn one() -> Self {
        Self(Q::one())
    }

    /// The exact value in beats.
    #[must_use]
    pub fn get(&self) -> &Q {
        &self.0
    }

    /// As a signed duration.
    #[must_use]
    pub fn to_beats(&self) -> Beats {
        Beats(self.0.clone())
    }

    /// Scales by a strictly positive exact factor.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveDuration`] if the factor is not
    /// strictly positive.
    pub fn scale(&self, factor: &Q) -> Result<Self, TimeError> {
        Self::new(&self.0 * factor)
    }
}

impl TryFrom<Beats> for BeatDuration {
    type Error = TimeError;

    fn try_from(value: Beats) -> Result<Self, Self::Error> {
        Self::new(value.0)
    }
}

impl From<BeatDuration> for Q {
    fn from(value: BeatDuration) -> Self {
        value.0
    }
}

impl core::fmt::Display for BeatDuration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} beats", self.0)
    }
}

/// A position on the structural beat timeline `T_b`.
///
/// UMT layer: L1/L2, exact. A torsor over [`Beats`]: `point + duration` is a
/// point and `point - point` is a duration, and there is deliberately no
/// `point + point` (UMT-3.2 sections 1.10 and 5.1).
///
/// Unlike [`crate::pitch::PitchPoint`] this carries no origin identity. The
/// structural timeline has a canonical zero - the start of the notated
/// material - in a way the pitch lattice does not.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct BeatTime(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))] Q);

impl BeatTime {
    /// The origin of the structural timeline.
    #[must_use]
    pub fn zero() -> Self {
        Self(Q::zero())
    }

    /// A position at an exact number of beats from the origin.
    #[must_use]
    pub fn new(beats: Q) -> Self {
        Self(beats)
    }

    /// `numerator / denominator` beats from the origin.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ZeroDenominator`] for a zero denominator.
    pub fn ratio(numerator: i64, denominator: i64) -> Result<Self, TimeError> {
        Ok(Self(Beats::ratio(numerator, denominator)?.0))
    }

    /// The exact position, in beats from the origin.
    #[must_use]
    pub fn get(&self) -> &Q {
        &self.0
    }

    /// The torsor action `p + g`.
    #[must_use]
    pub fn translate(&self, duration: &Beats) -> Self {
        Self(&self.0 + &duration.0)
    }

    /// The unique duration `int(p, q)` with `p + int(p, q) = q`.
    #[must_use]
    pub fn interval_to(&self, other: &Self) -> Beats {
        Beats(&other.0 - &self.0)
    }
}

impl core::fmt::Display for BeatTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "b={}", self.0)
    }
}

/// A closed span `[start, end]` of structural time, with `start <= end`.
///
/// UMT layer: L1/L2, exact. Instants are legal; reversed spans are rejected
/// at construction, for the same reason [`crate::time::TimeSpan`] rejects
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawBeatSpan", into = "RawBeatSpan")
)]
pub struct BeatSpan {
    start: BeatTime,
    end: BeatTime,
}

/// A structural span in wire form, validated on the way in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawBeatSpan {
    /// The earlier endpoint.
    pub start: BeatTime,
    /// The later endpoint.
    pub end: BeatTime,
}

impl BeatSpan {
    /// Builds `[start, end]`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedBeatSpan`] if `end` precedes `start`.
    pub fn new(start: BeatTime, end: BeatTime) -> Result<Self, TimeError> {
        if end < start {
            return Err(TimeError::ReversedBeatSpan);
        }
        Ok(Self { start, end })
    }

    /// Builds `[start, start + duration]`.
    #[must_use]
    pub fn from_duration(start: BeatTime, duration: &BeatDuration) -> Self {
        let end = start.translate(&duration.to_beats());
        Self { start, end }
    }

    /// The earlier endpoint.
    #[must_use]
    pub fn start(&self) -> &BeatTime {
        &self.start
    }

    /// The later endpoint.
    #[must_use]
    pub fn end(&self) -> &BeatTime {
        &self.end
    }

    /// The exact length, which is never negative.
    #[must_use]
    pub fn duration(&self) -> Beats {
        self.start.interval_to(&self.end)
    }

    /// Whether the span is a single instant.
    #[must_use]
    pub fn is_instant(&self) -> bool {
        self.start == self.end
    }

    /// Whether a position lies within the closed span.
    #[must_use]
    pub fn contains(&self, at: &BeatTime) -> bool {
        &self.start <= at && at <= &self.end
    }

    /// The sub-span from `offset` of length `length`, measured from the start.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedBeatSpan`] if the requested length is
    /// negative.
    pub fn subspan(&self, offset: &Beats, length: &Beats) -> Result<Self, TimeError> {
        let start = self.start.translate(offset);
        let end = start.translate(length);
        Self::new(start, end)
    }
}

impl TryFrom<RawBeatSpan> for BeatSpan {
    type Error = TimeError;

    fn try_from(value: RawBeatSpan) -> Result<Self, Self::Error> {
        Self::new(value.start, value.end)
    }
}

impl From<BeatSpan> for RawBeatSpan {
    fn from(value: BeatSpan) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl core::fmt::Display for BeatSpan {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}, {}] beats", self.start.0, self.end.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{BeatDuration, BeatSpan, BeatTime, Beats};
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;

    #[test]
    fn the_structural_timeline_is_a_torsor() {
        let p = BeatTime::ratio(3, 2).unwrap();
        let g = Beats::ratio(1, 3).unwrap();
        let h = Beats::ratio(-5, 4).unwrap();

        // (p + g) + h = p + (g + h)
        assert_eq!(p.translate(&g).translate(&h), p.translate(&(&g + &h)));
        // p + 0 = p
        assert_eq!(p.translate(&Beats::zero()), p);
        // p + int(p, q) = q
        let q = p.translate(&g);
        assert_eq!(p.translate(&p.interval_to(&q)), q);
        // int(p, q) + int(q, r) = int(p, r)
        let r = q.translate(&h);
        assert_eq!(&p.interval_to(&q) + &q.interval_to(&r), p.interval_to(&r));
    }

    #[test]
    fn nested_tuplets_are_represented_exactly() {
        // A quintuplet sixteenth inside a triplet eighth: 1/3 of a beat split
        // five ways is 1/15 of a beat, which no binary float represents.
        let triplet_eighth = Beats::ratio(1, 3).unwrap();
        let fifth = triplet_eighth.scale(&Q::new(Z::from(1), Z::from(5)));
        assert_eq!(*fifth.get(), Q::new(Z::from(1), Z::from(15)));

        let five: Beats = core::iter::repeat_n(fifth, 5).sum();
        assert_eq!(five, triplet_eighth, "five fifths are exactly the whole");
    }

    #[test]
    fn a_duration_is_strictly_positive() {
        assert_eq!(
            BeatDuration::new(Q::from(Z::from(0))),
            Err(TimeError::NonPositiveDuration)
        );
        assert!(BeatDuration::ratio(-1, 2).is_err());
        assert!(BeatDuration::ratio(1, 0).is_err());
        assert_eq!(
            BeatDuration::ratio(3, 2).unwrap().to_beats(),
            Beats::ratio(3, 2).unwrap()
        );
        assert!(Beats::zero().to_duration().is_err());
    }

    #[test]
    fn note_values_are_what_they_should_be() {
        assert_eq!(Beats::whole(), Beats::ratio(4, 1).unwrap());
        assert_eq!(Beats::half(), Beats::ratio(2, 1).unwrap());
        assert_eq!(Beats::quarter(), Beats::ratio(1, 1).unwrap());
        assert_eq!(Beats::eighth(), Beats::ratio(1, 2).unwrap());
        assert_eq!(Beats::sixteenth(), Beats::ratio(1, 4).unwrap());
        // A dotted quarter is one and a half beats.
        assert_eq!(
            &Beats::quarter() + &Beats::eighth(),
            Beats::ratio(3, 2).unwrap()
        );
    }

    #[test]
    fn spans_are_ordered_and_measurable() {
        let span = BeatSpan::from_duration(BeatTime::zero(), &BeatDuration::ratio(7, 2).unwrap());
        assert_eq!(span.duration(), Beats::ratio(7, 2).unwrap());
        assert!(span.contains(&BeatTime::ratio(7, 2).unwrap()));
        assert!(!span.contains(&BeatTime::ratio(4, 1).unwrap()));
        assert!(!span.is_instant());

        assert!(matches!(
            BeatSpan::new(BeatTime::ratio(2, 1).unwrap(), BeatTime::zero()),
            Err(TimeError::ReversedBeatSpan)
        ));
        assert!(
            BeatSpan::new(BeatTime::zero(), BeatTime::zero()).is_ok(),
            "an instant is legal"
        );
    }

    #[test]
    fn durations_form_a_group() {
        let a = Beats::ratio(2, 3).unwrap();
        let b = Beats::ratio(-1, 6).unwrap();
        assert_eq!(&(&a + &b) - &b, a);
        assert_eq!(&a + &-&a, Beats::zero());
        assert_eq!(a.abs(), a);
        assert_eq!((-&a).abs(), a);
        assert!(a.is_positive());
        assert!(!b.is_positive());
        assert!(Beats::zero().is_zero());
    }
}
