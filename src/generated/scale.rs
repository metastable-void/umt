//! Modular generated sets, three-gap behaviour, and the MOS predicate
//! (UMT-3.2 sections 3.1 to 3.4).
//!
//! For a declared period `p` and generator `g`,
//!
//! ```text
//! G(g, p, n) = { j g mod p : j = 0, ..., n-1 }.
//! ```
//!
//! # The period and generator are designated data
//!
//! Section 3.1 is emphatic about why: "A rank-2 temperament does not
//! canonically determine which basis vector is `period` and which is
//! `generator`: changing a basis by an element of `GL(2,Z)` changes those
//! coordinates without changing the underlying free group. Therefore a
//! generated-scale object MUST store its designated period and generator
//! explicitly."
//!
//! So [`GeneratedSet`] stores both, and there is no constructor that derives
//! them from a temperament mapping. Which vector is the period is a
//! *decision*, and this type records the one that was made.
//!
//! # Three gaps, with the hypotheses recorded
//!
//! Section 3.2 uses the Three-Gap Theorem "as a property of generated
//! circular sets, not as a definition of every musical scale", and requires
//! four things to be recorded by anything claiming a three-gap result:
//! whether `g/p` is rational or irrational, how duplicates are handled when
//! the orbit closes, the cardinality, and the sorted gaps used. [`GapReport`]
//! carries all four.
//!
//! Whether `g/p` is rational cannot be decided from two `f64` values, so it is
//! *declared* through [`GeneratorRatio`] rather than guessed. An undeclared
//! ratio makes no closure claim at all.
//!
//! # MOS is an operational predicate, and "well-formed" is not a synonym
//!
//! Section 3.3 defines MOS operationally: a designated period-generator
//! construction at a cardinality where the projected generated set has two
//! positive step sizes. It then warns that "well-formed scale" has a broader
//! and historically specific literature, and that an implementation exposing
//! such a predicate "MUST declare the exact definition used rather than
//! treating `well-formed` and `two gap sizes` as interchangeable labels".
//!
//! This crate therefore exposes [`GeneratedSet::mos_verdict`] and **no**
//! well-formedness predicate. Adding one would mean choosing among several
//! incompatible definitions from that literature, and the honest way to offer
//! it is with the definition attached - which is work for whoever needs a
//! particular one.

use alloc::vec::Vec;
use num_traits::{Signed, ToPrimitive};

use crate::algebra::Q;
use crate::error::GeneratedError;
use crate::pitch::units::{Cents, Octaves};

/// The default tolerance for deciding that two circular gaps are the same
/// size, in octaves.
///
/// About a millionth of a cent. Generated points are computed in `f64`, so
/// gaps that are equal in exact arithmetic differ in their low bits; this is
/// wide enough to absorb that and far too narrow to merge sizes that are
/// genuinely different.
pub const DEFAULT_GAP_TOLERANCE: f64 = 1e-9;

/// Whether the generator-to-period ratio is rational, and if so which
/// rational (UMT-3.2 section 3.2).
///
/// Declared rather than inferred: `g/p` is computed from two real numbers, and
/// no finite computation on two `f64` values can decide rationality. A claim
/// about orbit closure rests on this declaration, so the declaration is where
/// it belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum GeneratorRatio {
    /// `g/p` is exactly this rational, so the orbit closes at its denominator.
    Rational(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))] Q),
    /// `g/p` is irrational, so the orbit never closes and every cardinality
    /// has as many distinct points as it has generators.
    Irrational,
    /// Not declared. No closure claim is made, and duplicates are found
    /// numerically within the declared tolerance rather than predicted.
    Undeclared,
}

impl GeneratorRatio {
    /// The cardinality at which the orbit closes, where that is known.
    ///
    /// `None` for an irrational or undeclared ratio - in the first case
    /// because it never closes, in the second because nothing was claimed.
    #[must_use]
    pub fn orbit_closure(&self) -> Option<usize> {
        match self {
            // A denominator too large for a `usize` is a cardinality no
            // generated scale will reach, so reporting no closure is the
            // honest answer rather than a truncation.
            Self::Rational(ratio) => ratio.denom().to_usize(),
            Self::Irrational | Self::Undeclared => None,
        }
    }
}

/// What a three-gap claim has to record (UMT-3.2 section 3.2).
///
/// UMT layer: L3 values over a declared construction.
#[derive(Debug, Clone, PartialEq)]
pub struct GapReport {
    ratio: GeneratorRatio,
    cardinality: usize,
    generated: usize,
    distinct: usize,
    gaps: Vec<Octaves>,
    tolerance: f64,
}

impl GapReport {
    /// The declared rationality of `g/p`.
    #[must_use]
    pub fn ratio(&self) -> &GeneratorRatio {
        &self.ratio
    }

    /// The requested cardinality `n`.
    #[must_use]
    pub fn cardinality(&self) -> usize {
        self.cardinality
    }

    /// How many points were generated, which is `n`.
    #[must_use]
    pub fn generated(&self) -> usize {
        self.generated
    }

    /// How many distinct points remained after the orbit was closed.
    #[must_use]
    pub fn distinct(&self) -> usize {
        self.distinct
    }

    /// How many generated points were duplicates.
    #[must_use]
    pub fn duplicates(&self) -> usize {
        self.generated - self.distinct
    }

    /// The sorted circular gaps the test used.
    #[must_use]
    pub fn gaps(&self) -> &[Octaves] {
        &self.gaps
    }

    /// The tolerance used to decide that two gaps are the same size.
    #[must_use]
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// The distinct gap sizes, ascending.
    #[must_use]
    pub fn distinct_sizes(&self) -> Vec<Octaves> {
        let mut sizes: Vec<Octaves> = Vec::new();
        for gap in &self.gaps {
            if !sizes
                .iter()
                .any(|size| (size.get() - gap.get()).abs() <= self.tolerance)
            {
                sizes.push(*gap);
            }
        }
        sizes.sort();
        sizes
    }

    /// Whether the Three-Gap Theorem's bound is met.
    ///
    /// It always is, under the theorem's hypotheses. Checking it is how a
    /// conformance run demonstrates that the implementation computes gaps the
    /// way the theorem is about, rather than asserting the theorem.
    #[must_use]
    pub fn satisfies_three_gap_bound(&self) -> bool {
        self.distinct_sizes().len() <= 3
    }

    /// Whether the declared orbit closure agrees with what was observed.
    ///
    /// A disagreement is information, not a panic: it means the declaration
    /// and the arithmetic disagree, and a caller should be told rather than
    /// have one of them silently win.
    #[must_use]
    pub fn closure_matches_declaration(&self) -> bool {
        match self.ratio.orbit_closure() {
            Some(closure) => self.distinct == self.cardinality.min(closure),
            None => match self.ratio {
                GeneratorRatio::Irrational => self.distinct == self.cardinality,
                _ => true,
            },
        }
    }
}

/// Which MOS predicate is being applied (UMT-3.2 section 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum MosProfile {
    /// Exactly two distinct positive step sizes. The default reading of
    /// section 3.3.
    #[default]
    TwoStepSizes,
    /// Two, or one - the equal-step case admitted as degenerate, which
    /// section 3.3 allows a profile to do.
    TwoStepSizesAllowingEqual,
}

/// The verdict of a MOS test, with the profile that produced it
/// (UMT-3.2 section 3.3).
#[derive(Debug, Clone, PartialEq)]
pub struct MosVerdict {
    profile: MosProfile,
    sizes: Vec<Octaves>,
    report: GapReport,
}

impl MosVerdict {
    /// Whether the cardinality satisfies the selected predicate.
    #[must_use]
    pub fn is_mos(&self) -> bool {
        match self.profile {
            MosProfile::TwoStepSizes => self.sizes.len() == 2,
            MosProfile::TwoStepSizesAllowingEqual => self.sizes.len() == 2 || self.sizes.len() == 1,
        }
    }

    /// Which predicate was applied.
    #[must_use]
    pub fn profile(&self) -> MosProfile {
        self.profile
    }

    /// The distinct step sizes, ascending.
    #[must_use]
    pub fn step_sizes(&self) -> &[Octaves] {
        &self.sizes
    }

    /// The gap report the verdict rests on.
    #[must_use]
    pub fn report(&self) -> &GapReport {
        &self.report
    }
}

/// A modular generated set (UMT-3.2 section 3.1).
///
/// UMT layer: L3 realization space, over designated data.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GeneratedSet {
    period: Octaves,
    generator: Octaves,
    cardinality: usize,
    ratio: GeneratorRatio,
    tolerance: f64,
}

impl GeneratedSet {
    /// Declares a generated set.
    ///
    /// # Errors
    ///
    /// Returns [`GeneratedError::NonPositivePeriod`] for a period that is not
    /// strictly positive, and [`GeneratedError::EmptyCardinality`] for a
    /// cardinality of zero. A generator of zero is legal and degenerate: it
    /// generates the single point 0.
    pub fn new(
        period: Octaves,
        generator: Octaves,
        cardinality: usize,
        ratio: GeneratorRatio,
    ) -> Result<Self, GeneratedError> {
        if period <= Octaves::ZERO {
            return Err(GeneratedError::NonPositivePeriod);
        }
        if cardinality == 0 {
            return Err(GeneratedError::EmptyCardinality);
        }
        Ok(Self {
            period,
            generator,
            cardinality,
            ratio,
            tolerance: DEFAULT_GAP_TOLERANCE,
        })
    }

    /// Declares a set from values in cents.
    ///
    /// # Errors
    ///
    /// As [`GeneratedSet::new`].
    pub fn from_cents(
        period: Cents,
        generator: Cents,
        cardinality: usize,
        ratio: GeneratorRatio,
    ) -> Result<Self, GeneratedError> {
        Self::new(
            Octaves::from(period),
            Octaves::from(generator),
            cardinality,
            ratio,
        )
    }

    /// Replaces the gap-comparison tolerance.
    ///
    /// # Errors
    ///
    /// Returns [`GeneratedError::InvalidTolerance`] for a negative or
    /// non-finite tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Result<Self, GeneratedError> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(GeneratedError::InvalidTolerance);
        }
        self.tolerance = tolerance;
        Ok(self)
    }

    /// The same construction at a different cardinality.
    ///
    /// # Errors
    ///
    /// Returns [`GeneratedError::EmptyCardinality`] for zero.
    pub fn at_cardinality(&self, cardinality: usize) -> Result<Self, GeneratedError> {
        if cardinality == 0 {
            return Err(GeneratedError::EmptyCardinality);
        }
        Ok(Self {
            cardinality,
            ..self.clone()
        })
    }

    /// The designated period `p`.
    #[must_use]
    pub fn period(&self) -> Octaves {
        self.period
    }

    /// The designated generator `g`.
    #[must_use]
    pub fn generator(&self) -> Octaves {
        self.generator
    }

    /// The cardinality `n`.
    #[must_use]
    pub fn cardinality(&self) -> usize {
        self.cardinality
    }

    /// The declared rationality of `g/p`.
    #[must_use]
    pub fn ratio(&self) -> &GeneratorRatio {
        &self.ratio
    }

    /// The gap-comparison tolerance.
    #[must_use]
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// The generated points `j g mod p`, in generation order.
    ///
    /// Duplicates are *kept* here: section 9.11 requires orbit closure and
    /// duplicates to be handled explicitly, and discarding them at this stage
    /// would hide which generator step produced which point.
    #[must_use]
    pub fn points(&self) -> Vec<Octaves> {
        (0..self.cardinality)
            .map(|step| {
                let raw = self.generator.get() * step as f64;
                Octaves::new(modulo(raw, self.period.get()))
                    .expect("a finite value modulo a finite positive period is finite")
            })
            .collect()
    }

    /// The generated points, sorted and with duplicates removed.
    ///
    /// Section 9.11 requires circular gaps to be computed "from sorted
    /// distinct points", so this is what the gap machinery uses.
    #[must_use]
    pub fn sorted_distinct_points(&self) -> Vec<Octaves> {
        let mut points = self.points();
        points.sort();
        points.dedup_by(|left, right| (left.get() - right.get()).abs() <= self.tolerance);
        points
    }

    /// The circular gaps between consecutive sorted distinct points.
    ///
    /// The last gap wraps from the highest point back to the first, one period
    /// up, so the gaps always sum to the period.
    #[must_use]
    pub fn circular_gaps(&self) -> Vec<Octaves> {
        let points = self.sorted_distinct_points();
        if points.is_empty() {
            return Vec::new();
        }
        let mut gaps = Vec::with_capacity(points.len());
        for pair in points.windows(2) {
            gaps.push(
                Octaves::new(pair[1].get() - pair[0].get()).expect("a difference of finites"),
            );
        }
        gaps.push(
            Octaves::new(self.period.get() + points[0].get() - points[points.len() - 1].get())
                .expect("a difference of finites"),
        );
        gaps
    }

    /// The four things a three-gap claim must record
    /// (UMT-3.2 section 3.2).
    #[must_use]
    pub fn gap_report(&self) -> GapReport {
        let distinct = self.sorted_distinct_points().len();
        GapReport {
            ratio: self.ratio.clone(),
            cardinality: self.cardinality,
            generated: self.cardinality,
            distinct,
            gaps: self.circular_gaps(),
            tolerance: self.tolerance,
        }
    }

    /// Whether this cardinality satisfies the selected MOS predicate
    /// (UMT-3.2 section 3.3).
    #[must_use]
    pub fn mos_verdict(&self, profile: MosProfile) -> MosVerdict {
        let report = self.gap_report();
        MosVerdict {
            profile,
            sizes: report.distinct_sizes(),
            report,
        }
    }

    /// Every cardinality from 1 to `max` that satisfies the selected
    /// predicate.
    ///
    /// Section 3.3 warns that "lists of MOS cardinalities do not imply the
    /// nonexistence of generated scales at intervening cardinalities", which
    /// is why [`GeneratedSet::at_cardinality`] works for every `n` and this
    /// method is named for the predicate rather than for the family.
    ///
    /// # Errors
    ///
    /// Propagates cardinality validation.
    pub fn mos_cardinalities(
        &self,
        max: usize,
        profile: MosProfile,
    ) -> Result<Vec<usize>, GeneratedError> {
        let mut out = Vec::new();
        for cardinality in 1..=max {
            if self
                .at_cardinality(cardinality)?
                .mos_verdict(profile)
                .is_mos()
            {
                out.push(cardinality);
            }
        }
        Ok(out)
    }

    /// The ordered step pattern, as indices into the distinct step sizes.
    ///
    /// The usual `LLsLLLs`-style word, with `0` the smallest size. Useful for
    /// comparing modes, which are rotations of this pattern
    /// (UMT-3.2 section 3.4).
    #[must_use]
    pub fn step_pattern(&self) -> Vec<usize> {
        let report = self.gap_report();
        let sizes = report.distinct_sizes();
        report
            .gaps()
            .iter()
            .map(|gap| {
                sizes
                    .iter()
                    .position(|size| (size.get() - gap.get()).abs() <= self.tolerance)
                    .unwrap_or(0)
            })
            .collect()
    }

    /// The step pattern rotated to begin at a given degree
    /// (UMT-3.2 section 3.4).
    ///
    /// A *cyclic mode*. Whether two rotations count as the same scale is a
    /// declared equivalence of the application, not a fact this crate
    /// asserts: section 3.4 declines to force mode identity at the core
    /// because modal function may depend on a designated reference degree.
    ///
    /// # Errors
    ///
    /// Returns [`GeneratedError::DegreeOutOfRange`] for a degree at or beyond
    /// the number of steps.
    pub fn mode(&self, degree: usize) -> Result<Vec<usize>, GeneratedError> {
        let pattern = self.step_pattern();
        if pattern.is_empty() || degree >= pattern.len() {
            return Err(GeneratedError::DegreeOutOfRange {
                degree,
                steps: pattern.len(),
            });
        }
        let mut rotated = pattern[degree..].to_vec();
        rotated.extend_from_slice(&pattern[..degree]);
        Ok(rotated)
    }
}

/// `x mod p` for a strictly positive `p`, landing in `[0, p)`.
fn modulo(value: f64, period: f64) -> f64 {
    let remainder = libm::fmod(value, period);
    if remainder.is_sign_negative() && remainder != 0.0 {
        remainder + period
    } else {
        remainder
    }
}

/// The quarter-comma-meantone generator, `300 log2(5)` cents
/// (UMT-3.2 fixture F35).
///
/// Its ratio to the octave is `log2(5) / 4`, which is irrational.
#[must_use]
pub fn quarter_comma_meantone_generator() -> Cents {
    Cents::new(300.0 * libm::log2(5.0)).expect("a finite logarithm")
}

/// Whether an exact rational is strictly positive, for validation.
#[must_use]
pub fn ratio_is_positive(ratio: &Q) -> bool {
    ratio.is_positive()
}

#[cfg(test)]
mod tests {
    use super::{GeneratedSet, GeneratorRatio, MosProfile, quarter_comma_meantone_generator};
    use crate::algebra::{Q, Z};
    use crate::error::GeneratedError;
    use crate::pitch::units::{Cents, Octaves};
    use alloc::vec::Vec;

    fn meantone(cardinality: usize) -> GeneratedSet {
        GeneratedSet::from_cents(
            Cents::new(1200.0).unwrap(),
            quarter_comma_meantone_generator(),
            cardinality,
            GeneratorRatio::Irrational,
        )
        .unwrap()
    }

    fn cents_of(values: &[Octaves]) -> Vec<f64> {
        values
            .iter()
            .map(|value| Cents::from(*value).get())
            .collect()
    }

    #[test]
    fn the_period_and_generator_are_stored_rather_than_derived() {
        let scale = meantone(7);
        assert_eq!(scale.period(), Octaves::new(1.0).unwrap());
        assert!((Cents::from(scale.generator()).get() - 696.578_428_5).abs() < 1e-6);
        assert_eq!(scale.cardinality(), 7);
        assert_eq!(scale.ratio(), &GeneratorRatio::Irrational);

        // The same free group with period and generator swapped is a
        // different designated object, as section 3.1 insists.
        let swapped = GeneratedSet::from_cents(
            quarter_comma_meantone_generator(),
            Cents::new(1200.0).unwrap(),
            7,
            GeneratorRatio::Irrational,
        )
        .unwrap();
        assert_ne!(swapped, scale);
    }

    #[test]
    fn generated_points_are_taken_modulo_the_period() {
        let scale = meantone(3);
        let points = cents_of(&scale.points());
        assert!((points[0] - 0.0).abs() < 1e-9);
        assert!((points[1] - 696.578_428_5).abs() < 1e-6);
        assert!(
            (points[2] - 193.156_857_0).abs() < 1e-6,
            "two generators wrap around the period"
        );

        // Sorted distinct points are what the gap machinery uses.
        let sorted = cents_of(&scale.sorted_distinct_points());
        assert!(sorted.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn circular_gaps_sum_to_the_period() {
        for cardinality in [1usize, 2, 3, 5, 7, 12, 19, 31] {
            let scale = meantone(cardinality);
            let total: f64 = scale.circular_gaps().iter().map(|gap| gap.get()).sum();
            assert!(
                (total - 1.0).abs() < 1e-9,
                "gaps at n = {cardinality} sum to {total}"
            );
            assert_eq!(scale.circular_gaps().len(), cardinality);
        }
    }

    #[test]
    fn the_three_gap_bound_holds_and_the_report_records_its_hypotheses() {
        for cardinality in 1..=40 {
            let scale = meantone(cardinality);
            let report = scale.gap_report();
            assert!(
                report.satisfies_three_gap_bound(),
                "n = {cardinality} produced {} sizes",
                report.distinct_sizes().len()
            );
            // The four things section 3.2 requires be recorded.
            assert_eq!(report.ratio(), &GeneratorRatio::Irrational);
            assert_eq!(report.cardinality(), cardinality);
            assert_eq!(
                report.distinct(),
                cardinality,
                "an irrational ratio never closes"
            );
            assert_eq!(report.duplicates(), 0);
            assert_eq!(report.gaps().len(), cardinality);
            assert!(report.closure_matches_declaration());
        }
    }

    #[test]
    fn a_rational_ratio_closes_its_orbit_and_the_duplicates_are_reported() {
        // Seven steps of 12-EDO: g/p = 7/12, so the orbit closes at 12.
        let edo = GeneratedSet::from_cents(
            Cents::new(1200.0).unwrap(),
            Cents::new(700.0).unwrap(),
            20,
            GeneratorRatio::Rational(Q::new(Z::from(7), Z::from(12))),
        )
        .unwrap();

        assert_eq!(edo.ratio().orbit_closure(), Some(12));
        let report = edo.gap_report();
        assert_eq!(report.generated(), 20);
        assert_eq!(report.distinct(), 12, "the orbit closed");
        assert_eq!(report.duplicates(), 8);
        assert!(report.closure_matches_declaration());

        // Twelve equal steps: one gap size, so not MOS under the strict
        // predicate and MOS under the degenerate-admitting one.
        assert_eq!(report.distinct_sizes().len(), 1);
        assert!(!edo.mos_verdict(MosProfile::TwoStepSizes).is_mos());
        assert!(
            edo.mos_verdict(MosProfile::TwoStepSizesAllowingEqual)
                .is_mos()
        );
    }

    #[test]
    fn an_irrational_declaration_that_the_arithmetic_contradicts_is_reported() {
        // Declared irrational, but the generator really is 7/12 of the period,
        // so duplicates appear. The report says the declaration and the
        // arithmetic disagree rather than picking one.
        let mislabelled = GeneratedSet::from_cents(
            Cents::new(1200.0).unwrap(),
            Cents::new(700.0).unwrap(),
            20,
            GeneratorRatio::Irrational,
        )
        .unwrap();
        let report = mislabelled.gap_report();
        assert_eq!(report.distinct(), 12);
        assert!(
            !report.closure_matches_declaration(),
            "an irrational ratio would have given 20 distinct points"
        );
    }

    #[test]
    fn meantone_mos_cardinalities_match_the_specification_list() {
        // UMT-3.2 section 3.3 lists 2, 3, 5, 7, 12, 19, 31 for this generator.
        let cardinalities = meantone(1)
            .mos_cardinalities(31, MosProfile::TwoStepSizes)
            .unwrap();
        assert_eq!(cardinalities, [2, 3, 5, 7, 12, 19, 31]);

        // Section 3.3: the list does not imply that generated scales at the
        // intervening cardinalities do not exist. They do; they are simply not
        // MOS.
        for cardinality in [4usize, 6, 8, 11, 20] {
            let scale = meantone(cardinality);
            assert_eq!(scale.sorted_distinct_points().len(), cardinality);
            assert!(!scale.mos_verdict(MosProfile::TwoStepSizes).is_mos());
            assert_eq!(scale.gap_report().distinct_sizes().len(), 3);
        }
    }

    #[test]
    fn the_step_pattern_and_its_modes_are_rotations() {
        // The diatonic scale as a step word: five large steps and two small.
        let scale = meantone(7);
        let pattern = scale.step_pattern();
        assert_eq!(pattern.len(), 7);
        assert_eq!(pattern.iter().filter(|step| **step == 0).count(), 2);
        assert_eq!(pattern.iter().filter(|step| **step == 1).count(), 5);

        // A mode is a rotation, and its multiset of steps is unchanged.
        let mode = scale.mode(2).unwrap();
        assert_eq!(mode.len(), pattern.len());
        let mut sorted_pattern = pattern.clone();
        let mut sorted_mode = mode.clone();
        sorted_pattern.sort_unstable();
        sorted_mode.sort_unstable();
        assert_eq!(sorted_pattern, sorted_mode);
        assert_ne!(mode, pattern, "and it is a different ordered word");

        assert_eq!(scale.mode(0).unwrap(), pattern);
        assert!(matches!(
            scale.mode(7),
            Err(GeneratedError::DegreeOutOfRange { .. })
        ));
    }

    #[test]
    fn construction_is_validated() {
        assert!(matches!(
            GeneratedSet::new(
                Octaves::ZERO,
                Octaves::new(0.5).unwrap(),
                3,
                GeneratorRatio::Undeclared
            ),
            Err(GeneratedError::NonPositivePeriod)
        ));
        assert!(matches!(
            meantone(1).at_cardinality(0),
            Err(GeneratedError::EmptyCardinality)
        ));
        assert!(meantone(1).with_tolerance(-1.0).is_err());
        assert!(meantone(1).with_tolerance(1e-6).is_ok());
    }

    #[test]
    fn an_undeclared_ratio_makes_no_closure_claim() {
        let unknown = GeneratedSet::from_cents(
            Cents::new(1200.0).unwrap(),
            Cents::new(700.0).unwrap(),
            20,
            GeneratorRatio::Undeclared,
        )
        .unwrap();
        assert_eq!(unknown.ratio().orbit_closure(), None);
        let report = unknown.gap_report();
        assert_eq!(
            report.distinct(),
            12,
            "duplicates are still found numerically"
        );
        assert!(
            report.closure_matches_declaration(),
            "no claim was made, so none is contradicted"
        );
    }
}
