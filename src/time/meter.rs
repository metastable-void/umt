//! Meter and grouping (UMT-3.2 section 5.4, prompt section 28).
//!
//! Meter is a family of nested periodic point sets on the timeline,
//!
//! ```text
//! ... subset L_2 subset L_1 subset L_0 subset T_b,
//! ```
//!
//! where the lower-indexed sets carry the finer pulses. Section 5.4.1 requires
//! the level-numbering convention to be *declared*, so [`LevelNumbering`] is a
//! stored field rather than a comment.
//!
//! Two things this deliberately does not do:
//!
//! - It does not require levels to be subgroups. An additive meter such as
//!   `2+2+3` has primary beats at `{0, 2, 4}` in a seven-unit bar, which is not
//!   closed under addition modulo the period, and section 5.4.1 says so
//!   explicitly.
//! - It does not merge meter with grouping. [`Grouping`] is a separate ordered
//!   segmentation, because phrase and motive boundaries need not coincide with
//!   metric ones, and prompt section 28 forbids reducing both to a single array
//!   of beat strengths.
//!
//! # Examples
//!
//! 6/8 and 3/4 share a total span and a pulse resolution while differing in
//! their primary beats (fixture F10):
//!
//! ```
//! use umt::time::{Meter, TimeSignature};
//!
//! let six_eight = Meter::compound(TimeSignature::new(6, 8)?)?;
//! let three_four = Meter::simple(TimeSignature::new(3, 4)?)?;
//!
//! assert_eq!(six_eight.period(), three_four.period());
//! assert_eq!(six_eight.pulse_count(), three_four.pulse_count());
//! assert_ne!(six_eight.primary_beat_pulses(), three_four.primary_beat_pulses());
//!
//! // In eighth-note units: {0, 3} against {0, 2, 4}.
//! assert_eq!(six_eight.primary_beat_pulses(), &[0, 3]);
//! assert_eq!(three_four.primary_beat_pulses(), &[0, 2, 4]);
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

use alloc::vec::Vec;

use crate::algebra::{Q, Z};
use crate::error::TimeError;
use crate::time::beat::{BeatDuration, BeatSpan, BeatTime, Beats};

/// The declared convention for numbering metrical levels
/// (UMT-3.2 section 5.4.1).
///
/// Both conventions appear in the literature and neither is standard, which is
/// exactly why the specification requires the choice to be recorded rather
/// than inferred from context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum LevelNumbering {
    /// Level 0 is the finest pulse; higher indices are coarser and stronger.
    /// This is the convention of the section 5.4.1 chain
    /// `L_2 subset L_1 subset L_0`.
    FinestIsZero,
    /// Level 0 is the coarsest level; higher indices are finer.
    CoarsestIsZero,
}

/// A notated time signature.
///
/// UMT layer: L0 metadata, exact. It is a *label*: what it means metrically -
/// simple, compound, additive - is a separate declared choice, which is why
/// the constructors on [`Meter`] are named for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeSignature {
    numerator: u32,
    denominator: u32,
}

impl TimeSignature {
    /// Builds a time signature.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InvalidTimeSignature`] for a zero numerator or a
    /// denominator that is zero or not a power of two.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, TimeError> {
        if numerator == 0 || denominator == 0 || !denominator.is_power_of_two() {
            return Err(TimeError::InvalidTimeSignature {
                numerator,
                denominator,
            });
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// The upper number.
    #[must_use]
    pub fn numerator(self) -> u32 {
        self.numerator
    }

    /// The lower number.
    #[must_use]
    pub fn denominator(self) -> u32 {
        self.denominator
    }

    /// The duration of one notated pulse, in beats.
    ///
    /// A `4` denominator is one beat, an `8` is half a beat, and so on, since
    /// the declared beat unit is the quarter note.
    #[must_use]
    pub fn pulse_duration(self) -> BeatDuration {
        BeatDuration::new(Q::new(Z::from(4), Z::from(self.denominator)))
            .expect("a power-of-two denominator gives a positive duration")
    }

    /// The total duration of one bar, in beats.
    #[must_use]
    pub fn bar_duration(self) -> BeatDuration {
        BeatDuration::new(Q::new(
            Z::from(4 * u64::from(self.numerator)),
            Z::from(self.denominator),
        ))
        .expect("a positive numerator gives a positive duration")
    }
}

impl core::fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// A metrical hierarchy: nested periodic point sets over one bar
/// (UMT-3.2 section 5.4.1).
///
/// UMT layer: L1/L2, exact.
///
/// Levels are stored as pulse indices within one period, from the finest level
/// upward. Level 0 is always the full pulse lattice; each higher level is a
/// subset of the one below, which is validated at construction. The
/// [`LevelNumbering`] used to describe them is stored alongside.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "RawMeter", into = "RawMeter"))]
pub struct Meter {
    signature: Option<TimeSignature>,
    pulse: BeatDuration,
    pulses: u32,
    levels: Vec<Vec<u32>>,
    numbering: LevelNumbering,
}

/// A meter in wire form, revalidated on the way in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawMeter {
    /// The notated signature, where there is one.
    #[cfg_attr(feature = "serde", serde(default))]
    pub signature: Option<TimeSignature>,
    /// The duration of one pulse.
    pub pulse: BeatDuration,
    /// Pulses in one period.
    pub pulses: u32,
    /// Pulse indices at each level, finest first.
    pub levels: Vec<Vec<u32>>,
    /// The declared level-numbering convention.
    pub numbering: LevelNumbering,
}

impl Meter {
    /// Builds a metrical hierarchy from explicit level point sets.
    ///
    /// `levels[0]` is filled in as the full pulse lattice; the supplied levels
    /// are the coarser ones, each of which must be a subset of the previous.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyCycle`] for a zero-length period,
    /// [`TimeError::OnsetOutsideCycle`] for a pulse index outside the period,
    /// and [`TimeError::LevelNotNested`] if a level is not contained in the
    /// finer one below it.
    pub fn new(
        pulse: BeatDuration,
        pulses: u32,
        coarser_levels: &[&[u32]],
        numbering: LevelNumbering,
    ) -> Result<Self, TimeError> {
        if pulses == 0 {
            return Err(TimeError::EmptyCycle);
        }
        let mut levels: Vec<Vec<u32>> = Vec::with_capacity(coarser_levels.len() + 1);
        levels.push((0..pulses).collect());

        for (offset, level) in coarser_levels.iter().enumerate() {
            let mut points: Vec<u32> = level.to_vec();
            points.sort_unstable();
            points.dedup();
            for index in &points {
                if *index >= pulses {
                    return Err(TimeError::OnsetOutsideCycle {
                        index: *index,
                        pulses,
                    });
                }
            }
            let finer = &levels[offset];
            if !points
                .iter()
                .all(|index| finer.binary_search(index).is_ok())
            {
                return Err(TimeError::LevelNotNested { level: offset + 1 });
            }
            levels.push(points);
        }

        Ok(Self {
            signature: None,
            pulse,
            pulses,
            levels,
            numbering,
        })
    }

    /// A simple meter: every notated pulse is a primary beat.
    ///
    /// 3/4 becomes six eighth-note pulses with primary beats at `{0, 2, 4}`.
    /// The pulse resolution is deliberately one level finer than the notated
    /// denominator, so that a simple and a compound meter of the same total
    /// span can be compared on a common lattice, as fixture F10 requires.
    ///
    /// # Errors
    ///
    /// Propagates level validation.
    pub fn simple(signature: TimeSignature) -> Result<Self, TimeError> {
        let pulses = signature.numerator() * 2;
        let primaries: Vec<u32> = (0..signature.numerator()).map(|beat| beat * 2).collect();
        let mut meter = Self::new(
            signature.pulse_duration().scale(&half())?,
            pulses,
            &[&primaries, &[0]],
            LevelNumbering::FinestIsZero,
        )?;
        meter.signature = Some(signature);
        Ok(meter)
    }

    /// A compound meter: notated pulses group in threes.
    ///
    /// 6/8 becomes six eighth-note pulses with primary beats at `{0, 3}`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InvalidTimeSignature`] if the numerator is not a
    /// multiple of three, which is what "compound" means.
    pub fn compound(signature: TimeSignature) -> Result<Self, TimeError> {
        if signature.numerator() % 3 != 0 {
            return Err(TimeError::InvalidTimeSignature {
                numerator: signature.numerator(),
                denominator: signature.denominator(),
            });
        }
        let pulses = signature.numerator();
        let primaries: Vec<u32> = (0..pulses / 3).map(|group| group * 3).collect();
        let mut meter = Self::new(
            signature.pulse_duration(),
            pulses,
            &[&primaries, &[0]],
            LevelNumbering::FinestIsZero,
        )?;
        meter.signature = Some(signature);
        Ok(meter)
    }

    /// An additive meter such as `2+2+3`.
    ///
    /// The primary beats sit at the cumulative group boundaries, which in
    /// general are *not* closed under addition modulo the period. Section
    /// 5.4.1 permits exactly that, and this constructor is the reason the
    /// nesting check does not also demand a subgroup.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyDivision`] for an empty grouping and
    /// [`TimeError::NonPositiveWeight`] for a zero group.
    pub fn additive(pulse: BeatDuration, groups: &[u32]) -> Result<Self, TimeError> {
        if groups.is_empty() {
            return Err(TimeError::EmptyDivision);
        }
        if groups.contains(&0) {
            return Err(TimeError::NonPositiveWeight);
        }
        let mut primaries = Vec::with_capacity(groups.len());
        let mut cumulative = 0u32;
        for group in groups {
            primaries.push(cumulative);
            cumulative += group;
        }
        Self::new(
            pulse,
            cumulative,
            &[&primaries, &[0]],
            LevelNumbering::FinestIsZero,
        )
    }

    /// The notated signature, where the meter came from one.
    #[must_use]
    pub fn signature(&self) -> Option<TimeSignature> {
        self.signature
    }

    /// The declared level-numbering convention.
    #[must_use]
    pub fn numbering(&self) -> LevelNumbering {
        self.numbering
    }

    /// The duration of one pulse.
    #[must_use]
    pub fn pulse(&self) -> &BeatDuration {
        &self.pulse
    }

    /// Pulses in one period.
    #[must_use]
    pub fn pulse_count(&self) -> u32 {
        self.pulses
    }

    /// The total duration of one period.
    #[must_use]
    pub fn period(&self) -> Beats {
        self.pulse.to_beats().scale(&Q::from(Z::from(self.pulses)))
    }

    /// How many levels the hierarchy has, including the full pulse lattice.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// The pulse indices at a level, in the stored finest-first order.
    ///
    /// Index 0 is always the full pulse lattice, whatever
    /// [`Meter::numbering`] reports; the numbering describes how the levels
    /// are *named* to a user, not how they are stored.
    #[must_use]
    pub fn level(&self, index: usize) -> Option<&[u32]> {
        self.levels.get(index).map(Vec::as_slice)
    }

    /// The primary beats: the level immediately above the full pulse lattice.
    #[must_use]
    pub fn primary_beat_pulses(&self) -> &[u32] {
        self.levels.get(1).map_or(&[], Vec::as_slice)
    }

    /// The metrical weight of a pulse: how many levels contain it.
    ///
    /// A pulse on no level but the finest has weight 1; a downbeat present at
    /// every level has weight [`Meter::level_count`]. This is a *derived*
    /// convenience, not a definition of metrical accent - section 5.4.2 is
    /// explicit that UMT-3.2 imposes no universal scalar of that kind.
    #[must_use]
    pub fn weight(&self, pulse: u32) -> usize {
        self.levels
            .iter()
            .filter(|level| level.binary_search(&pulse).is_ok())
            .count()
    }

    /// The exact structural time of a pulse within a bar starting at `origin`.
    #[must_use]
    pub fn pulse_time(&self, origin: &BeatTime, pulse: u32) -> BeatTime {
        origin.translate(&self.pulse.to_beats().scale(&Q::from(Z::from(pulse))))
    }

    /// The span of one bar starting at `origin`.
    #[must_use]
    pub fn bar_span(&self, origin: &BeatTime) -> BeatSpan {
        BeatSpan::new(origin.clone(), self.pulse_time(origin, self.pulses))
            .expect("a non-negative number of pulses gives a forward span")
    }
}

impl TryFrom<RawMeter> for Meter {
    type Error = TimeError;

    fn try_from(value: RawMeter) -> Result<Self, Self::Error> {
        let coarser: Vec<&[u32]> = value.levels.iter().skip(1).map(Vec::as_slice).collect();
        let mut meter = Self::new(value.pulse, value.pulses, &coarser, value.numbering)?;
        meter.signature = value.signature;
        Ok(meter)
    }
}

impl From<Meter> for RawMeter {
    fn from(value: Meter) -> Self {
        Self {
            signature: value.signature,
            pulse: value.pulse,
            pulses: value.pulses,
            levels: value.levels,
            numbering: value.numbering,
        }
    }
}

fn half() -> Q {
    Q::new(Z::from(1), Z::from(2))
}

/// A grouping structure: an ordered segmentation over spans
/// (UMT-3.2 section 5.4.2).
///
/// UMT layer: L1, exact.
///
/// Separate from [`Meter`] on purpose. Phrase grouping, motive grouping, and
/// additive grouping need not coincide with metrical structure, and an
/// anacrusis is precisely a group that begins before the metrical reference it
/// belongs to (section 5.4.3) - which a meter alone cannot express.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Grouping {
    span: BeatSpan,
    label: Option<alloc::string::String>,
    children: Vec<Grouping>,
}

impl Grouping {
    /// A group covering a span, with no subdivision.
    #[must_use]
    pub fn leaf(span: BeatSpan) -> Self {
        Self {
            span,
            label: None,
            children: Vec::new(),
        }
    }

    /// Names this group.
    #[must_use]
    pub fn labelled(mut self, label: &str) -> Self {
        self.label = Some(label.into());
        self
    }

    /// A group with ordered subgroups.
    ///
    /// Subgroups must lie inside the parent span and must not overlap, but
    /// they need not tile it: a group can contain gaps, which is what makes
    /// motivic grouping expressible.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::GroupOutsideParent`] if a child leaves the parent
    /// span, and [`TimeError::OverlappingGroups`] if two children overlap.
    pub fn with_children<I>(span: BeatSpan, children: I) -> Result<Self, TimeError>
    where
        I: IntoIterator<Item = Grouping>,
    {
        let children: Vec<Grouping> = children.into_iter().collect();
        for child in &children {
            if !span.contains(child.span.start()) || !span.contains(child.span.end()) {
                return Err(TimeError::GroupOutsideParent);
            }
        }
        for pair in children.windows(2) {
            if pair[1].span.start() < pair[0].span.end() {
                return Err(TimeError::OverlappingGroups);
            }
        }
        Ok(Self {
            span,
            label: None,
            children,
        })
    }

    /// The span this group covers.
    #[must_use]
    pub fn span(&self) -> &BeatSpan {
        &self.span
    }

    /// The group's label, if it has one.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The ordered subgroups.
    #[must_use]
    pub fn children(&self) -> &[Grouping] {
        &self.children
    }

    /// Whether this group starts before a designated metrical reference, that
    /// is, whether it is an anacrusis to it (UMT-3.2 section 5.4.3).
    #[must_use]
    pub fn is_anacrusis_to(&self, reference: &BeatTime) -> bool {
        self.span.start() < reference && self.span.end() > reference
    }
}

/// Two simultaneous metric layers and how their periods relate
/// (UMT-3.2 section 5.5).
///
/// UMT layer: L1, exact.
///
/// The operational convention of section 5.5: layers sharing a reference span
/// while differing in internal subdivision are *polyrhythm*; layers with
/// distinct recurring periods are *polymeter*. This type computes which of the
/// two a pair of meters is under that convention, and how long it takes them
/// to realign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricLayering {
    lower: Meter,
    upper: Meter,
}

/// Whether two layers differ in period or only in internal organization
/// (UMT-3.2 section 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LayerRelation {
    /// Equal periods, different internal pulse organization.
    Polyrhythm,
    /// Different periods, so major reference points realign only
    /// periodically.
    Polymeter,
}

impl MetricLayering {
    /// Pairs two metric layers on one common timeline.
    #[must_use]
    pub fn new(lower: Meter, upper: Meter) -> Self {
        Self { lower, upper }
    }

    /// The lower layer.
    #[must_use]
    pub fn lower(&self) -> &Meter {
        &self.lower
    }

    /// The upper layer.
    #[must_use]
    pub fn upper(&self) -> &Meter {
        &self.upper
    }

    /// Which relation the two layers stand in, under the section 5.5
    /// convention.
    #[must_use]
    pub fn relation(&self) -> LayerRelation {
        if self.lower.period() == self.upper.period() {
            LayerRelation::Polyrhythm
        } else {
            LayerRelation::Polymeter
        }
    }

    /// The structural duration after which both layers begin a period
    /// together, that is, the least common period.
    ///
    /// Exact: for rational periods `a` and `b` this is the least rational `c`
    /// with `c/a` and `c/b` both integers.
    #[must_use]
    pub fn realignment_period(&self) -> Beats {
        let a = self.lower.period();
        let b = self.upper.period();
        if a.is_zero() || b.is_zero() {
            return Beats::zero();
        }
        // lcm(p/q, r/s) = lcm(p, r) / gcd(q, s).
        let (an, ad) = (a.get().numer().clone(), a.get().denom().clone());
        let (bn, bd) = (b.get().numer().clone(), b.get().denom().clone());
        let numerator = lcm(&an, &bn);
        let denominator = gcd(&ad, &bd);
        Beats::new(Q::new(numerator, denominator))
    }
}

fn gcd(a: &Z, b: &Z) -> Z {
    num_integer::Integer::gcd(a, b)
}

fn lcm(a: &Z, b: &Z) -> Z {
    num_integer::Integer::lcm(a, b)
}

#[cfg(test)]
mod tests {
    use super::{Grouping, LayerRelation, LevelNumbering, Meter, MetricLayering, TimeSignature};
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;
    use crate::time::beat::{BeatDuration, BeatSpan, BeatTime, Beats};

    #[test]
    fn f10_six_eight_and_three_four_share_a_span_and_differ_in_structure() {
        let six_eight = Meter::compound(TimeSignature::new(6, 8).unwrap()).unwrap();
        let three_four = Meter::simple(TimeSignature::new(3, 4).unwrap()).unwrap();

        // Same total span, same pulse resolution.
        assert_eq!(six_eight.period(), Beats::ratio(3, 1).unwrap());
        assert_eq!(three_four.period(), Beats::ratio(3, 1).unwrap());
        assert_eq!(six_eight.pulse_count(), 6);
        assert_eq!(three_four.pulse_count(), 6);
        assert_eq!(six_eight.pulse(), three_four.pulse());
        assert_eq!(*six_eight.pulse(), BeatDuration::ratio(1, 2).unwrap());

        // Different primary-beat point sets, in eighth-note units.
        assert_eq!(six_eight.primary_beat_pulses(), &[0, 3]);
        assert_eq!(three_four.primary_beat_pulses(), &[0, 2, 4]);
        assert_ne!(six_eight, three_four);

        // And therefore different metrical weights away from the downbeat.
        assert_eq!(six_eight.weight(0), 3, "downbeat, present at every level");
        assert_eq!(six_eight.weight(3), 2);
        assert_eq!(six_eight.weight(2), 1);
        assert_eq!(three_four.weight(2), 2);
        assert_eq!(three_four.weight(3), 1);

        // The exact structural time of the second primary beat differs.
        let origin = BeatTime::zero();
        assert_eq!(
            *six_eight.pulse_time(&origin, 3).get(),
            Q::new(Z::from(3), Z::from(2))
        );
        assert_eq!(
            *three_four.pulse_time(&origin, 2).get(),
            Q::from(Z::from(1))
        );
    }

    #[test]
    fn an_additive_meter_is_not_a_subgroup() {
        // 2+2+3 in eighths: primaries at {0, 2, 4} of a seven-pulse bar.
        let bar = Meter::additive(BeatDuration::ratio(1, 2).unwrap(), &[2, 2, 3]).unwrap();
        assert_eq!(bar.pulse_count(), 7);
        assert_eq!(bar.primary_beat_pulses(), &[0, 2, 4]);
        assert_eq!(bar.period(), Beats::ratio(7, 2).unwrap());

        // 2 and 4 are primaries; 2 + 4 = 6 mod 7 is not. A subgroup would have
        // to contain it, and section 5.4.1 says levels need not be subgroups.
        assert!(bar.primary_beat_pulses().contains(&2));
        assert!(bar.primary_beat_pulses().contains(&4));
        assert!(!bar.primary_beat_pulses().contains(&6));
    }

    #[test]
    fn levels_must_nest_and_the_numbering_is_recorded() {
        let pulse = BeatDuration::ratio(1, 2).unwrap();
        // A level that is not a subset of the finer one below it.
        assert_eq!(
            Meter::new(
                pulse.clone(),
                4,
                &[&[0, 2], &[1]],
                LevelNumbering::FinestIsZero
            ),
            Err(TimeError::LevelNotNested { level: 2 })
        );
        // A pulse index outside the period.
        assert!(matches!(
            Meter::new(pulse.clone(), 4, &[&[4]], LevelNumbering::FinestIsZero),
            Err(TimeError::OnsetOutsideCycle { .. })
        ));

        let meter = Meter::new(pulse, 4, &[&[0, 2], &[0]], LevelNumbering::FinestIsZero).unwrap();
        assert_eq!(meter.numbering(), LevelNumbering::FinestIsZero);
        assert_eq!(meter.level_count(), 3);
        assert_eq!(meter.level(0).unwrap(), &[0, 1, 2, 3]);
        assert_eq!(meter.level(2).unwrap(), &[0]);
        assert!(meter.level(3).is_none());
    }

    #[test]
    fn time_signatures_validate_and_measure() {
        assert!(TimeSignature::new(0, 4).is_err());
        assert!(TimeSignature::new(4, 0).is_err());
        assert!(TimeSignature::new(4, 3).is_err(), "not a power of two");

        let four_four = TimeSignature::new(4, 4).unwrap();
        assert_eq!(four_four.pulse_duration(), BeatDuration::one());
        assert_eq!(four_four.bar_duration(), BeatDuration::ratio(4, 1).unwrap());
        assert_eq!(four_four.to_string(), "4/4");

        let six_eight = TimeSignature::new(6, 8).unwrap();
        assert_eq!(six_eight.bar_duration(), BeatDuration::ratio(3, 1).unwrap());

        // A compound constructor rejects a numerator that is not a multiple
        // of three, because that is what compound means.
        assert!(Meter::compound(TimeSignature::new(4, 8).unwrap()).is_err());
    }

    #[test]
    fn grouping_is_separate_from_meter_and_can_precede_the_downbeat() {
        let bar_start = BeatTime::ratio(4, 1).unwrap();
        // A phrase beginning a beat before the barline: an anacrusis.
        let phrase = Grouping::leaf(
            BeatSpan::new(
                BeatTime::ratio(3, 1).unwrap(),
                BeatTime::ratio(8, 1).unwrap(),
            )
            .unwrap(),
        )
        .labelled("phrase 1");
        assert!(phrase.is_anacrusis_to(&bar_start));
        assert_eq!(phrase.label(), Some("phrase 1"));

        // A group entirely after the reference is not.
        let later = Grouping::leaf(
            BeatSpan::new(
                BeatTime::ratio(4, 1).unwrap(),
                BeatTime::ratio(8, 1).unwrap(),
            )
            .unwrap(),
        );
        assert!(!later.is_anacrusis_to(&bar_start));
    }

    #[test]
    fn nested_groups_are_validated() {
        let outer = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(8, 1).unwrap()).unwrap();
        let inside = Grouping::leaf(
            BeatSpan::new(BeatTime::zero(), BeatTime::ratio(4, 1).unwrap()).unwrap(),
        );
        let after = Grouping::leaf(
            BeatSpan::new(
                BeatTime::ratio(4, 1).unwrap(),
                BeatTime::ratio(8, 1).unwrap(),
            )
            .unwrap(),
        );
        assert!(Grouping::with_children(outer.clone(), [inside.clone(), after.clone()]).is_ok());

        let overlapping = Grouping::leaf(
            BeatSpan::new(
                BeatTime::ratio(3, 1).unwrap(),
                BeatTime::ratio(8, 1).unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            Grouping::with_children(outer.clone(), [inside, overlapping]),
            Err(TimeError::OverlappingGroups)
        );

        let escaping = Grouping::leaf(
            BeatSpan::new(BeatTime::zero(), BeatTime::ratio(9, 1).unwrap()).unwrap(),
        );
        assert_eq!(
            Grouping::with_children(outer, [escaping]),
            Err(TimeError::GroupOutsideParent)
        );
    }

    #[test]
    fn polyrhythm_and_polymeter_are_distinguished_by_period() {
        let three_four = Meter::simple(TimeSignature::new(3, 4).unwrap()).unwrap();
        let six_eight = Meter::compound(TimeSignature::new(6, 8).unwrap()).unwrap();
        let four_four = Meter::simple(TimeSignature::new(4, 4).unwrap()).unwrap();

        // Same period, different internal organization.
        let hemiola = MetricLayering::new(three_four.clone(), six_eight);
        assert_eq!(hemiola.relation(), LayerRelation::Polyrhythm);
        assert_eq!(hemiola.realignment_period(), Beats::ratio(3, 1).unwrap());

        // Different periods: 3 beats against 4 realign after 12.
        let against = MetricLayering::new(three_four, four_four);
        assert_eq!(against.relation(), LayerRelation::Polymeter);
        assert_eq!(against.realignment_period(), Beats::ratio(12, 1).unwrap());
    }

    #[test]
    fn realignment_is_exact_for_fractional_periods() {
        // A seven-eighths bar against a three-quarters bar: 7/2 and 3.
        // lcm(7/2, 3/1) = lcm(7, 3) / gcd(2, 1) = 21.
        let seven_eight = Meter::additive(BeatDuration::ratio(1, 2).unwrap(), &[2, 2, 3]).unwrap();
        let three_four = Meter::simple(TimeSignature::new(3, 4).unwrap()).unwrap();
        let layering = MetricLayering::new(seven_eight, three_four);
        assert_eq!(layering.relation(), LayerRelation::Polymeter);
        assert_eq!(
            layering.realignment_period(),
            Beats::ratio(21, 1).unwrap(),
            "six seven-eight bars against seven three-four bars"
        );
    }
}
