//! Physical time (UMT-3.2 section 5.1, prompt section 18).
//!
//! Only the *physical* half of the timeline exists here so far: [`Seconds`] as
//! an interval and [`ClockTime`] as a position. Structural beat time is exact
//! and arrives with the rhythm layer, deliberately as a different type - a
//! tempo map is a map between the two, and a single shared type would erase
//! it (UMT-3.2 section 5.8.3).
//!
//! This half is real-valued because physical time is measured, not written.
//! Nothing at L0 to L2 depends on it, so the exactness rule of section 0.6.1
//! is untouched.

use crate::error::TimeError;
use crate::quantity::{finite_newtype, interval_arithmetic};

finite_newtype!(
    Seconds,
    TimeError,
    TimeError::NonFiniteQuantity,
    "A physical time *interval*, in seconds.\n\nUMT layer: L3. Signed: it is the difference between two [`ClockTime`]\npositions, and a difference may run backwards. A *duration*, which may not,\nis a separate obligation checked where one is required."
);
finite_newtype!(
    ClockTime,
    TimeError,
    TimeError::NonFiniteQuantity,
    "A position on the physical timeline, in seconds from a declared zero.\n\nUMT layer: L3. This is a *point*: it supports `point + interval` and\n`point - point`, and deliberately not `point + point` (UMT-3.2 section\n1.10)."
);

interval_arithmetic!(Seconds, TimeError);

impl Seconds {
    /// Whether this interval runs forwards, that is, is strictly positive.
    #[must_use]
    pub fn is_positive(self) -> bool {
        self.get() > 0.0
    }
}

impl ClockTime {
    /// The origin of the physical timeline.
    pub const ZERO: Self = Self(0.0);

    /// Moves this position by an interval: the torsor action `p + g`.
    #[must_use]
    pub fn translate(self, interval: Seconds) -> Self {
        Self(self.0 + interval.get())
    }

    /// The interval from this position to another: `int(p, q)`.
    #[must_use]
    pub fn interval_to(self, other: Self) -> Seconds {
        Seconds(other.0 - self.0)
    }
}

impl core::fmt::Display for Seconds {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.6} s", self.0)
    }
}

impl core::fmt::Display for ClockTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "t={:.6} s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockTime, Seconds};
    use crate::error::TimeError;

    #[test]
    fn validation_rejects_what_it_must() {
        assert_eq!(Seconds::new(f64::NAN), Err(TimeError::NonFiniteQuantity));
        assert_eq!(
            ClockTime::new(f64::INFINITY),
            Err(TimeError::NonFiniteQuantity)
        );
        assert!(Seconds::new(-0.5).is_ok(), "a difference may run backwards");
        assert!(!Seconds::new(-0.5).unwrap().is_positive());
        assert!(Seconds::new(0.5).unwrap().is_positive());
        assert!(!Seconds::ZERO.is_positive(), "zero is not forwards");
    }

    #[test]
    fn the_physical_timeline_is_a_torsor() {
        let start = ClockTime::new(1.5).unwrap();
        let g = Seconds::new(0.25).unwrap();
        let h = Seconds::new(-1.0).unwrap();

        // (p + g) + h = p + (g + h)
        assert_eq!(start.translate(g).translate(h), start.translate(g + h));
        // p + 0 = p
        assert_eq!(start.translate(Seconds::ZERO), start);
        // p + int(p, q) = q
        let q = start.translate(g);
        assert_eq!(start.translate(start.interval_to(q)), q);
        // int(p, q) + int(q, r) = int(p, r)
        let r = q.translate(h);
        assert_eq!(
            start.interval_to(q) + q.interval_to(r),
            start.interval_to(r)
        );
    }

    #[test]
    fn ordering_and_display_are_available() {
        let mut times = [
            ClockTime::new(2.0).unwrap(),
            ClockTime::ZERO,
            ClockTime::new(-1.0).unwrap(),
        ];
        times.sort();
        assert_eq!(times[0], ClockTime::new(-1.0).unwrap());
        assert_eq!(
            alloc::format!("{}", Seconds::new(0.5).unwrap()),
            "0.500000 s"
        );
    }
}
