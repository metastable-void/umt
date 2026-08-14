//! Closed intervals of physical time.
//!
//! A pitch trajectory is a map `gamma: [t0, t1] -> P_3` (UMT-3.2 section 4.7),
//! so the domain needs to be a first-class object: an ordered pair that cannot
//! be built backwards, knows its own duration, and can say whether a time lies
//! inside it.

use crate::error::TimeError;
use crate::time::units::{ClockTime, Seconds};

/// A closed interval `[start, end]` of physical time, with `start <= end`.
///
/// UMT layer: L3. Degenerate spans (`start == end`) are legal and represent an
/// instant; reversed spans are not, and are rejected at construction rather
/// than silently normalized, because a reversed span in a document is a defect
/// and not a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawTimeSpan", into = "RawTimeSpan")
)]
pub struct TimeSpan {
    start: ClockTime,
    end: ClockTime,
}

/// A time span in wire form, validated on the way in.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawTimeSpan {
    /// The earlier endpoint.
    pub start: ClockTime,
    /// The later endpoint.
    pub end: ClockTime,
}

impl TimeSpan {
    /// Builds `[start, end]`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedSpan`] if `end` precedes `start`.
    pub fn new(start: ClockTime, end: ClockTime) -> Result<Self, TimeError> {
        if end < start {
            return Err(TimeError::ReversedSpan {
                start: start.get(),
                end: end.get(),
            });
        }
        Ok(Self { start, end })
    }

    /// Builds `[start, start + duration]`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedSpan`] for a negative duration.
    pub fn from_duration(start: ClockTime, duration: Seconds) -> Result<Self, TimeError> {
        Self::new(start, start.translate(duration))
    }

    /// The earlier endpoint.
    #[must_use]
    pub fn start(self) -> ClockTime {
        self.start
    }

    /// The later endpoint.
    #[must_use]
    pub fn end(self) -> ClockTime {
        self.end
    }

    /// The length of the span, which is never negative.
    #[must_use]
    pub fn duration(self) -> Seconds {
        self.start.interval_to(self.end)
    }

    /// Whether the span is a single instant.
    #[must_use]
    pub fn is_instant(self) -> bool {
        self.start == self.end
    }

    /// Whether a time lies within the closed span.
    #[must_use]
    pub fn contains(self, at: ClockTime) -> bool {
        self.start <= at && at <= self.end
    }

    /// The normalized position of a time within the span, in `[0, 1]`.
    ///
    /// An instant reports `0.0` for its single time: there is no other
    /// defensible answer, and returning a NaN from a division by zero would
    /// poison everything downstream.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideSpan`] for a time outside the span. A
    /// trajectory is defined on its domain and nowhere else, so extrapolation
    /// is refused rather than guessed.
    pub fn position(self, at: ClockTime) -> Result<f64, TimeError> {
        if !self.contains(at) {
            return Err(TimeError::OutsideSpan {
                time: at.get(),
                start: self.start.get(),
                end: self.end.get(),
            });
        }
        let length = self.duration().get();
        if length == 0.0 {
            return Ok(0.0);
        }
        Ok(self.start.interval_to(at).get() / length)
    }

    /// The elapsed time since the start of the span.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OutsideSpan`] for a time outside the span.
    pub fn elapsed(self, at: ClockTime) -> Result<Seconds, TimeError> {
        if !self.contains(at) {
            return Err(TimeError::OutsideSpan {
                time: at.get(),
                start: self.start.get(),
                end: self.end.get(),
            });
        }
        Ok(self.start.interval_to(at))
    }
}

impl TryFrom<RawTimeSpan> for TimeSpan {
    type Error = TimeError;

    fn try_from(value: RawTimeSpan) -> Result<Self, Self::Error> {
        Self::new(value.start, value.end)
    }
}

impl From<TimeSpan> for RawTimeSpan {
    fn from(value: TimeSpan) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl core::fmt::Display for TimeSpan {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{:.6}, {:.6}] s", self.start.get(), self.end.get())
    }
}

#[cfg(test)]
mod tests {
    use super::TimeSpan;
    use crate::error::TimeError;
    use crate::time::units::{ClockTime, Seconds};

    fn at(seconds: f64) -> ClockTime {
        ClockTime::new(seconds).unwrap()
    }

    #[test]
    fn a_reversed_span_is_rejected() {
        assert!(matches!(
            TimeSpan::new(at(1.0), at(0.5)),
            Err(TimeError::ReversedSpan { .. })
        ));
        assert!(
            TimeSpan::new(at(1.0), at(1.0)).is_ok(),
            "an instant is legal"
        );
    }

    #[test]
    fn position_is_normalized_and_bounded() {
        let span = TimeSpan::from_duration(at(2.0), Seconds::new(4.0).unwrap()).unwrap();
        assert_eq!(span.duration(), Seconds::new(4.0).unwrap());
        assert_eq!(span.position(at(2.0)).unwrap(), 0.0);
        assert_eq!(span.position(at(4.0)).unwrap(), 0.5);
        assert_eq!(span.position(at(6.0)).unwrap(), 1.0);
        assert!(matches!(
            span.position(at(6.5)),
            Err(TimeError::OutsideSpan { .. })
        ));
        assert!(matches!(
            span.elapsed(at(1.9)),
            Err(TimeError::OutsideSpan { .. })
        ));
    }

    #[test]
    fn an_instant_has_a_defined_position_rather_than_a_nan() {
        let span = TimeSpan::new(at(3.0), at(3.0)).unwrap();
        assert!(span.is_instant());
        assert_eq!(span.duration(), Seconds::ZERO);
        assert_eq!(span.position(at(3.0)).unwrap(), 0.0);
    }
}
