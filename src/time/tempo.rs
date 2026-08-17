//! Tempo maps and rubato (UMT-3.2 section 5.8, prompt section 30).
//!
//! A tempo realization maps structural beat time to physical clock time,
//!
//! ```text
//! theta: I_b -> I_c,
//! ```
//!
//! and in the homeomorphism profile it must be continuous, strictly
//! increasing, and bijective onto its target interval (section 9.9).
//!
//! Those are not documentation here, they are construction conditions.
//! [`TempoMap`] has no constructor that skips them, so *holding* a `TempoMap`
//! is the proof that it is a homeomorphism. Fixture F15 supplies a strictly
//! increasing map with a jump; it is rejected, because strictly increasing is
//! not sufficient.
//!
//! # This is not a tuning
//!
//! Section 5.8.3 is worth restating, because the analogy is tempting and
//! wrong. A regular pitch tuning is a group homomorphism on an interval group;
//! a tempo map is a monotone map between affine ordered timelines. They share
//! a realization-optimization interface and nothing else. In this crate they
//! do not even share a supertrait.
//!
//! # Pauses
//!
//! An orientation-preserving homeomorphism cannot insert clock time at a
//! single structural instant, so a fermata cannot be a discontinuity in this
//! profile (section 5.8.4). [`TempoMap::with_structural_pause`] implements the
//! first of the three sanctioned representations: give the pause an explicit
//! structural span, then stretch it. The other two - a temporal-constraint
//! variable, or a declared generalized time relation outside this profile -
//! live in [`crate::time::constraint`] and outside the crate respectively.
//! [`PauseRepresentation`] names all three so a document can say which it used.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::algebra::Q;
use crate::error::TimeError;
use crate::time::beat::{BeatDuration, BeatSpan, BeatTime, Beats};
use crate::time::rate::SecondsPerBeat;
use crate::time::span::TimeSpan;
use crate::time::units::{ClockTime, Seconds};

/// A point where a tempo map's structural and clock timelines are pinned
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TempoBreakpoint {
    /// The structural position.
    pub beat: BeatTime,
    /// The clock position it realizes to.
    pub clock: ClockTime,
}

impl TempoBreakpoint {
    /// Pins a structural position to a clock position.
    #[must_use]
    pub fn new(beat: BeatTime, clock: ClockTime) -> Self {
        Self { beat, clock }
    }
}

/// How a document represents a pause with zero structural duration
/// (UMT-3.2 section 5.8.4).
///
/// A fermata, caesura, or cue wait cannot be a discontinuity in the
/// homeomorphism profile. Section 5.8.4 lists exactly three sanctioned
/// alternatives, and this enum is them; there is no fourth variant meaning
/// "hidden as a jump".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum PauseRepresentation {
    /// An explicit structural span, subsequently stretched. Implemented by
    /// [`TempoMap::with_structural_pause`].
    StructuralSpan,
    /// A temporal-constraint variable or external predicate, solved by
    /// [`crate::time::constraint`].
    ConstraintVariable,
    /// A declared generalized monotone time relation outside the
    /// homeomorphism profile. This crate does not implement one; naming it
    /// lets a document say that is what it means.
    OutsideHomeomorphismProfile,
}

/// A piecewise-linear tempo map in the homeomorphism profile
/// (UMT-3.2 section 5.8).
///
/// UMT layer: L2 domain, L3 codomain.
///
/// Piecewise linear is the simplest family that covers constant tempo, stepped
/// tempo changes, and arbitrarily close approximations of accelerando and
/// ritardando. It is continuous and strictly increasing everywhere, and
/// differentiable except at the breakpoints - which is why
/// [`TempoMap::seconds_per_beat_at`] returns an `Option` rather than
/// pretending a corner has a slope.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawTempoMap", into = "RawTempoMap")
)]
pub struct TempoMap {
    breakpoints: Vec<TempoBreakpoint>,
}

/// A tempo map in wire form, revalidated on the way in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawTempoMap {
    /// The breakpoints, in order.
    pub breakpoints: Vec<TempoBreakpoint>,
}

impl TempoMap {
    /// Builds a tempo map from breakpoints, validating the homeomorphism
    /// conditions.
    ///
    /// # Errors
    ///
    /// - [`TimeError::DegenerateTempoMap`] for fewer than two breakpoints,
    ///   which cannot span an interval;
    /// - [`TimeError::DiscontinuousTempoMap`] if two breakpoints share a
    ///   structural position, which is a jump - the map would still be
    ///   increasing, but it would not be continuous and not be onto an
    ///   interval (fixture F15);
    /// - [`TimeError::NonMonotoneTempoMap`] if either timeline fails to
    ///   increase strictly.
    pub fn new<I>(breakpoints: I) -> Result<Self, TimeError>
    where
        I: IntoIterator<Item = TempoBreakpoint>,
    {
        let breakpoints: Vec<TempoBreakpoint> = breakpoints.into_iter().collect();
        if breakpoints.len() < 2 {
            return Err(TimeError::DegenerateTempoMap);
        }
        for pair in breakpoints.windows(2) {
            if pair[0].beat == pair[1].beat {
                // Two clock times at one structural instant. Strictly
                // increasing in clock time, but not a homeomorphism.
                return Err(TimeError::DiscontinuousTempoMap {
                    beat: pair[0].beat.get().to_string(),
                });
            }
            if pair[1].beat < pair[0].beat || pair[1].clock <= pair[0].clock {
                return Err(TimeError::NonMonotoneTempoMap);
            }
        }
        Ok(Self { breakpoints })
    }

    /// A constant tempo over a structural span.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::DegenerateTempoMap`] for an instantaneous span,
    /// which has no interval to be a homeomorphism onto.
    pub fn constant(
        domain: &BeatSpan,
        start: ClockTime,
        rate: SecondsPerBeat,
    ) -> Result<Self, TimeError> {
        if domain.is_instant() {
            return Err(TimeError::DegenerateTempoMap);
        }
        let beats = ratio_to_f64(&domain.duration())?;
        let end = start.translate(Seconds::new(beats * rate.get())?);
        Self::new([
            TempoBreakpoint::new(domain.start().clone(), start),
            TempoBreakpoint::new(domain.end().clone(), end),
        ])
    }

    /// The breakpoints, in order.
    #[must_use]
    pub fn breakpoints(&self) -> &[TempoBreakpoint] {
        &self.breakpoints
    }

    /// The structural domain `I_b`.
    #[must_use]
    pub fn domain(&self) -> BeatSpan {
        BeatSpan::new(
            self.breakpoints[0].beat.clone(),
            self.breakpoints[self.breakpoints.len() - 1].beat.clone(),
        )
        .expect("breakpoints are validated to increase")
    }

    /// The clock range `I_c`.
    #[must_use]
    pub fn range(&self) -> TimeSpan {
        TimeSpan::new(
            self.breakpoints[0].clock,
            self.breakpoints[self.breakpoints.len() - 1].clock,
        )
        .expect("breakpoints are validated to increase")
    }

    /// `theta(t)`: the clock time of a structural position.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideBeatSpan`] for a position outside the
    /// declared domain. The map is a bijection onto its target interval and
    /// makes no claim beyond it.
    pub fn clock_time(&self, at: &BeatTime) -> Result<ClockTime, TimeError> {
        let index = self.segment_containing(at)?;
        let (left, right) = (&self.breakpoints[index], &self.breakpoints[index + 1]);
        let width = ratio_to_f64(&left.beat.interval_to(&right.beat))?;
        if width == 0.0 {
            return Ok(left.clock);
        }
        let position = ratio_to_f64(&left.beat.interval_to(at))? / width;
        let span = left.clock.interval_to(right.clock);
        Ok(left.clock.translate(Seconds::new(span.get() * position)?))
    }

    /// `theta^{-1}(c)`: the structural position of a clock time.
    ///
    /// The inverse exists and is continuous because the map is a
    /// homeomorphism, which is the whole point of enforcing that at
    /// construction.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideSpan`] for a clock time outside the range.
    pub fn beat_time(&self, at: ClockTime) -> Result<BeatTime, TimeError> {
        let range = self.range();
        if !range.contains(at) {
            return Err(TimeError::OutsideSpan {
                time: at.get(),
                start: range.start().get(),
                end: range.end().get(),
            });
        }
        let index = self
            .breakpoints
            .windows(2)
            .position(|pair| pair[0].clock <= at && at <= pair[1].clock)
            .unwrap_or(0);
        let (left, right) = (&self.breakpoints[index], &self.breakpoints[index + 1]);
        let width = left.clock.interval_to(right.clock).get();
        if width == 0.0 {
            return Ok(left.beat.clone());
        }
        let position = left.clock.interval_to(at).get() / width;
        let structural = left.beat.interval_to(&right.beat);
        Ok(left
            .beat
            .translate(&structural.scale(&f64_to_ratio(position)?)))
    }

    /// The realized clock span of a structural span.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideBeatSpan`] if the span leaves the domain.
    pub fn realize(&self, span: &BeatSpan) -> Result<TimeSpan, TimeError> {
        TimeSpan::new(self.clock_time(span.start())?, self.clock_time(span.end())?)
    }

    /// `theta'(t)`, in seconds per beat, where the derivative exists.
    ///
    /// Returns `None` at an interior breakpoint where the two adjacent slopes
    /// differ: the map has a corner there and no derivative. Section 9.9
    /// requires a profile exposing derivative-based tempo to state its
    /// regularity, and returning an honest `None` is that statement.
    ///
    /// The reciprocal beat rate is [`SecondsPerBeat::reciprocal`]; the two
    /// are different dimensions and this crate never conflates them.
    #[must_use]
    pub fn seconds_per_beat_at(&self, at: &BeatTime) -> Option<SecondsPerBeat> {
        let index = self.segment_containing(at).ok()?;
        // At an interior breakpoint there are two candidate slopes. They agree
        // only if the map happens to be smooth there; otherwise it is a corner
        // and the derivative does not exist.
        if let Some(position) = self.breakpoints.iter().position(|point| point.beat == *at)
            && position > 0
            && position + 1 < self.breakpoints.len()
        {
            let before = self.segment_rate(position - 1)?;
            let after = self.segment_rate(position)?;
            return if before == after { Some(before) } else { None };
        }
        self.segment_rate(index)
    }

    /// The constant rate of the segment with the given index.
    #[must_use]
    pub fn segment_rate(&self, index: usize) -> Option<SecondsPerBeat> {
        let left = self.breakpoints.get(index)?;
        let right = self.breakpoints.get(index + 1)?;
        let beats = ratio_to_f64(&left.beat.interval_to(&right.beat)).ok()?;
        let seconds = left.clock.interval_to(right.clock).get();
        SecondsPerBeat::new(seconds / beats).ok()
    }

    /// How many linear segments the map has.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.breakpoints.len() - 1
    }

    /// Whether the map has one constant rate throughout.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        let Some(first) = self.segment_rate(0) else {
            return false;
        };
        (1..self.segment_count()).all(|index| self.segment_rate(index) == Some(first))
    }

    /// Inserts a pause at a structural position, as an explicit structural
    /// span that is then stretched (UMT-3.2 section 5.8.4).
    ///
    /// Everything at or after `at` shifts later by `structural` beats and by
    /// `clock` seconds, so the result is still a homeomorphism: the pause
    /// occupies real structural time, which is precisely what section 5.8.4
    /// requires instead of a hidden discontinuity.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideBeatSpan`] if the position is outside the
    /// domain, and propagates the homeomorphism validation.
    pub fn with_structural_pause(
        &self,
        at: &BeatTime,
        structural: &BeatDuration,
        clock: Seconds,
    ) -> Result<Self, TimeError> {
        if !clock.is_positive() {
            return Err(TimeError::NonPositiveRate { rate: clock.get() });
        }
        let arrival = self.clock_time(at)?;
        let shift = structural.to_beats();

        let mut breakpoints = Vec::with_capacity(self.breakpoints.len() + 2);
        for point in &self.breakpoints {
            if point.beat < *at {
                breakpoints.push(point.clone());
            }
        }
        breakpoints.push(TempoBreakpoint::new(at.clone(), arrival));
        breakpoints.push(TempoBreakpoint::new(
            at.translate(&shift),
            arrival.translate(clock),
        ));
        for point in &self.breakpoints {
            if point.beat > *at {
                breakpoints.push(TempoBreakpoint::new(
                    point.beat.translate(&shift),
                    point.clock.translate(clock),
                ));
            }
        }
        Self::new(breakpoints)
    }

    fn segment_containing(&self, at: &BeatTime) -> Result<usize, TimeError> {
        let domain = self.domain();
        if !domain.contains(at) {
            return Err(TimeError::OutsideBeatSpan);
        }
        Ok(self
            .breakpoints
            .windows(2)
            .position(|pair| pair[0].beat <= *at && *at <= pair[1].beat)
            .unwrap_or(0))
    }
}

impl TryFrom<RawTempoMap> for TempoMap {
    type Error = TimeError;

    fn try_from(value: RawTempoMap) -> Result<Self, Self::Error> {
        Self::new(value.breakpoints)
    }
}

impl From<TempoMap> for RawTempoMap {
    fn from(value: TempoMap) -> Self {
        Self {
            breakpoints: value.breakpoints,
        }
    }
}

fn ratio_to_f64(beats: &Beats) -> Result<f64, TimeError> {
    if beats.is_zero() {
        return Ok(0.0);
    }
    let value = beats.get();
    crate::algebra::integer::ratio_to_f64(value.numer(), value.denom())
        .ok_or(TimeError::NonFiniteQuantity)
}

fn f64_to_ratio(value: f64) -> Result<Q, TimeError> {
    // The position within a segment is a measured real, so this is an L3-to-L1
    // crossing and it is approximate by nature. It is used only to place a
    // clock time back on the structural timeline, never to build structural
    // source data.
    Q::from_float(value).ok_or(TimeError::NonFiniteQuantity)
}

#[cfg(test)]
mod tests {
    use super::{PauseRepresentation, TempoBreakpoint, TempoMap};
    use crate::error::TimeError;
    use crate::time::beat::{BeatDuration, BeatSpan, BeatTime};
    use crate::time::rate::SecondsPerBeat;
    use crate::time::units::{ClockTime, Seconds};

    fn beat(value: i64) -> BeatTime {
        BeatTime::ratio(value, 1).unwrap()
    }

    fn clock(value: f64) -> ClockTime {
        ClockTime::new(value).unwrap()
    }

    fn four_beats() -> BeatSpan {
        BeatSpan::new(BeatTime::zero(), beat(4)).unwrap()
    }

    #[test]
    fn a_constant_tempo_is_affine_and_invertible() {
        let map = TempoMap::constant(
            &four_beats(),
            ClockTime::ZERO,
            SecondsPerBeat::from_bpm(120.0).unwrap(),
        )
        .unwrap();

        assert!(map.is_constant());
        assert_eq!(map.segment_count(), 1);
        assert!((map.clock_time(&beat(4)).unwrap().get() - 2.0).abs() < 1e-12);
        assert!(
            (map.clock_time(&BeatTime::ratio(1, 2).unwrap())
                .unwrap()
                .get()
                - 0.25)
                .abs()
                < 1e-12
        );

        // theta^{-1}(theta(t)) = t
        let back = map.beat_time(map.clock_time(&beat(3)).unwrap()).unwrap();
        assert_eq!(back, beat(3));

        assert_eq!(
            map.seconds_per_beat_at(&BeatTime::ratio(1, 2).unwrap()),
            Some(SecondsPerBeat::from_bpm(120.0).unwrap())
        );
    }

    #[test]
    fn f15_a_strictly_increasing_map_with_a_jump_is_rejected() {
        // Clock time leaps at beat 2 while structural time stands still. The
        // sequence of clock values is strictly increasing, so a monotonicity
        // check alone would pass it - and the map is still not a
        // homeomorphism onto an interval.
        let jump = TempoMap::new([
            TempoBreakpoint::new(BeatTime::zero(), clock(0.0)),
            TempoBreakpoint::new(beat(2), clock(1.0)),
            TempoBreakpoint::new(beat(2), clock(3.0)),
            TempoBreakpoint::new(beat(4), clock(4.0)),
        ]);
        assert!(
            matches!(jump, Err(TimeError::DiscontinuousTempoMap { .. })),
            "{jump:?}"
        );

        // The sanctioned representation of the same musical intent: give the
        // pause a structural span, then stretch it.
        let base = TempoMap::constant(
            &four_beats(),
            ClockTime::ZERO,
            SecondsPerBeat::new(0.5).unwrap(),
        )
        .unwrap();
        let paused = base
            .with_structural_pause(&beat(2), &BeatDuration::one(), Seconds::new(2.0).unwrap())
            .unwrap();

        // The pause is real structural time now, and the map is still valid.
        assert_eq!(*paused.domain().end(), beat(5));
        assert!((paused.clock_time(&beat(2)).unwrap().get() - 1.0).abs() < 1e-12);
        assert!((paused.clock_time(&beat(3)).unwrap().get() - 3.0).abs() < 1e-12);
        assert!((paused.clock_time(&beat(5)).unwrap().get() - 4.0).abs() < 1e-12);
        assert!(!paused.is_constant());

        // And the three sanctioned representations are nameable.
        assert_ne!(
            PauseRepresentation::StructuralSpan,
            PauseRepresentation::ConstraintVariable
        );
    }

    #[test]
    fn the_homeomorphism_conditions_are_construction_conditions() {
        // Fewer than two breakpoints: no interval.
        assert_eq!(
            TempoMap::new([TempoBreakpoint::new(BeatTime::zero(), clock(0.0))]),
            Err(TimeError::DegenerateTempoMap)
        );
        // Structural time going backwards.
        assert_eq!(
            TempoMap::new([
                TempoBreakpoint::new(beat(2), clock(0.0)),
                TempoBreakpoint::new(beat(1), clock(1.0)),
            ]),
            Err(TimeError::NonMonotoneTempoMap)
        );
        // Clock time standing still over positive structural time: not
        // strictly increasing, so not injective.
        assert_eq!(
            TempoMap::new([
                TempoBreakpoint::new(BeatTime::zero(), clock(1.0)),
                TempoBreakpoint::new(beat(2), clock(1.0)),
            ]),
            Err(TimeError::NonMonotoneTempoMap)
        );
        // A zero-length domain.
        assert_eq!(
            TempoMap::constant(
                &BeatSpan::new(beat(1), beat(1)).unwrap(),
                ClockTime::ZERO,
                SecondsPerBeat::new(0.5).unwrap()
            ),
            Err(TimeError::DegenerateTempoMap)
        );
    }

    #[test]
    fn a_corner_has_no_derivative_and_says_so() {
        let map = TempoMap::new([
            TempoBreakpoint::new(BeatTime::zero(), clock(0.0)),
            TempoBreakpoint::new(beat(2), clock(1.0)),
            TempoBreakpoint::new(beat(4), clock(3.0)),
        ])
        .unwrap();

        assert_eq!(map.segment_count(), 2);
        assert_eq!(
            map.seconds_per_beat_at(&BeatTime::ratio(1, 1).unwrap()),
            Some(SecondsPerBeat::new(0.5).unwrap())
        );
        assert_eq!(
            map.seconds_per_beat_at(&beat(3)),
            Some(SecondsPerBeat::new(1.0).unwrap())
        );
        assert_eq!(
            map.seconds_per_beat_at(&beat(2)),
            None,
            "the slopes disagree at the corner, so there is no derivative"
        );

        // The reciprocal is a different dimension and has to be asked for.
        let rate = map.segment_rate(0).unwrap();
        assert!((rate.reciprocal().unwrap().get() - 2.0).abs() < 1e-12);
        assert!((rate.get() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_ritardando_is_monotone_without_being_affine() {
        // Successive beats take longer and longer.
        let map = TempoMap::new([
            TempoBreakpoint::new(BeatTime::zero(), clock(0.0)),
            TempoBreakpoint::new(beat(1), clock(0.5)),
            TempoBreakpoint::new(beat(2), clock(1.1)),
            TempoBreakpoint::new(beat(3), clock(1.9)),
            TempoBreakpoint::new(beat(4), clock(3.0)),
        ])
        .unwrap();

        assert!(!map.is_constant());
        let rates: Vec<f64> = (0..map.segment_count())
            .map(|index| map.segment_rate(index).unwrap().get())
            .collect();
        for pair in rates.windows(2) {
            assert!(pair[1] > pair[0], "slowing down: {pair:?}");
        }

        // Still invertible everywhere, which is what the profile buys. The
        // inverse crosses L3, so it is compared against an exact tolerance
        // rather than for exact equality.
        let tolerance = crate::algebra::Q::new(
            crate::algebra::Z::from(1),
            crate::algebra::Z::from(1_000_000),
        );
        for tenth in 0..=40 {
            let at = BeatTime::ratio(tenth, 10).unwrap();
            let there = map.clock_time(&at).unwrap();
            let back = map.beat_time(there).unwrap();
            let error = back.interval_to(&at).abs();
            assert!(*error.get() < tolerance, "{at} -> {there} -> {back}");
        }
    }

    #[test]
    fn the_domain_is_a_boundary_not_a_suggestion() {
        let map = TempoMap::constant(
            &four_beats(),
            ClockTime::ZERO,
            SecondsPerBeat::new(0.5).unwrap(),
        )
        .unwrap();
        assert_eq!(map.clock_time(&beat(5)), Err(TimeError::OutsideBeatSpan));
        assert!(map.beat_time(clock(99.0)).is_err());
        assert!(map.seconds_per_beat_at(&beat(9)).is_none());

        let span = BeatSpan::new(beat(1), beat(3)).unwrap();
        let realized = map.realize(&span).unwrap();
        assert!((realized.duration().get() - 1.0).abs() < 1e-12);
    }
}
