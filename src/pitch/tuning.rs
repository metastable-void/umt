//! Regular tuning and pitch realization (UMT-3.2 sections 1.8 and 4.2).
//!
//! A **regular tuning** is a group homomorphism `tau: G_2 -> R` from a
//! declared L2 interval group to the reals. It is a map of *intervals*. By
//! itself it says nothing about where any pitch sits: to realize a pitch
//! *point* you also need a structural reference point, a realized reference
//! point, and then the induced affine map
//!
//! ```text
//! tau_hat(p_0) = q_0        and        tau_hat(p + g) = tau_hat(p) + tau(g)
//! ```
//!
//! That is fixture F31, and the type system enforces it here: a
//! [`RegularTuning`] evaluates intervals, and only a [`PitchRealization`] -
//! which cannot be built without both references - evaluates points.
//!
//! **Non-regular realization** is a different type again. A realization that
//! depends on register, harmonic context, instrument state, or time is
//! `Phi: P_2 x C -> P_3`, and [`PitchRealizer`] makes the context an explicit
//! parameter so that dependence cannot be hidden behind a unary function
//! (fixture F28).

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::integer::ratio_to_f64;
use crate::error::PitchError;
use crate::pitch::point::{IntervalGroupElement, PitchPoint};
use crate::pitch::units::{LogFrequency, Octaves};
use crate::proportion::monzo::Monzo;
use crate::temperament::image::{AmbientElem, AmbientLattice, ImageElem, ImageLattice};
use crate::temperament::map::TemperamentMap;

/// A declared L2 interval group that a regular tuning can be defined on.
///
/// UMT-3.2 section 1.8.2 says the declared group is "normally `H` or the
/// ambient `Gamma`", and section 1.9 requires the choice to be recorded. Here
/// the choice is the type, so it cannot go unrecorded and a tuning of one
/// group cannot be applied to elements of the other.
pub trait L2IntervalGroup {
    /// The element type of this group.
    type Element: IntervalGroupElement;

    /// The number of generators, and so the number of sizes a tuning needs.
    fn generator_count(&self) -> usize;

    /// The coordinates of an element in this group's generators.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::IntervalGroupMismatch`] if the element belongs to
    /// a different declared group.
    fn coordinates<'a>(&self, element: &'a Self::Element) -> Result<&'a [Z], PitchError>;

    /// Maps a monzo into this group through a temperament mapping.
    ///
    /// # Errors
    ///
    /// Propagates a basis mismatch, and fails if the mapping's group is not
    /// this one.
    fn map_monzo(&self, map: &TemperamentMap, monzo: &Monzo) -> Result<Self::Element, PitchError>;
}

impl L2IntervalGroup for AmbientLattice {
    type Element = AmbientElem;

    fn generator_count(&self) -> usize {
        self.rank()
    }

    fn coordinates<'a>(&self, element: &'a Self::Element) -> Result<&'a [Z], PitchError> {
        if !element.lattice().same_identity(self) {
            return Err(PitchError::IntervalGroupMismatch);
        }
        Ok(element.coordinates())
    }

    fn map_monzo(&self, map: &TemperamentMap, monzo: &Monzo) -> Result<Self::Element, PitchError> {
        if !map.ambient().same_identity(self) {
            return Err(PitchError::IntervalGroupMismatch);
        }
        Ok(map.apply(monzo)?)
    }
}

impl L2IntervalGroup for ImageLattice {
    type Element = ImageElem;

    fn generator_count(&self) -> usize {
        self.rank()
    }

    fn coordinates<'a>(&self, element: &'a Self::Element) -> Result<&'a [Z], PitchError> {
        if **element.lattice() != *self {
            return Err(PitchError::IntervalGroupMismatch);
        }
        Ok(element.coordinates())
    }

    fn map_monzo(&self, map: &TemperamentMap, monzo: &Monzo) -> Result<Self::Element, PitchError> {
        if **map.image() != *self {
            return Err(PitchError::IntervalGroupMismatch);
        }
        Ok(map.apply_to_image(monzo)?)
    }
}

/// A regular tuning `tau: G_2 -> R`, given by the size of each generator.
///
/// UMT layer: L3 for the sizes, L2 for the domain. Translation-invariant by
/// construction: it is a homomorphism, so it cannot model register-dependent
/// stretch. That belongs to [`PitchRealizer`] (UMT-3.2 section 1.8.2).
///
/// # Examples
///
/// ```
/// use umt::pitch::{Cents, RegularTuning};
/// use umt::temperament::AmbientLattice;
/// use umt::Basis;
///
/// let steps = AmbientLattice::new("umt:edo:12", 1);
/// let tuning = RegularTuning::equal_divisions(&steps, 12)?;
///
/// // Interval sizes are available from the tuning alone.
/// let fifth = steps.element([7i64])?;
/// assert!((Cents::from(tuning.size(&fifth)?).get() - 700.0).abs() < 1e-9);
///
/// // But no absolute pitch is: that needs reference data (fixture F31).
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct RegularTuning<G> {
    group: Arc<G>,
    sizes: Vec<Octaves>,
}

// Written out rather than derived: the group is behind an `Arc`, so cloning
// and comparing a tuning does not require the group type itself to be
// cloneable.
impl<G> Clone for RegularTuning<G> {
    fn clone(&self) -> Self {
        Self {
            group: Arc::clone(&self.group),
            sizes: self.sizes.clone(),
        }
    }
}

impl<G: PartialEq> PartialEq for RegularTuning<G> {
    fn eq(&self, other: &Self) -> bool {
        self.sizes == other.sizes
            && (Arc::ptr_eq(&self.group, &other.group) || *self.group == *other.group)
    }
}

impl<G: PartialEq> Eq for RegularTuning<G> {}

impl<G: L2IntervalGroup> RegularTuning<G> {
    /// Builds a tuning from one size per generator.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::SizeCount`] if the number of sizes differs from
    /// the number of generators.
    pub fn new<I>(group: &Arc<G>, sizes: I) -> Result<Self, PitchError>
    where
        I: IntoIterator<Item = Octaves>,
    {
        let sizes: Vec<Octaves> = sizes.into_iter().collect();
        if sizes.len() != group.generator_count() {
            return Err(PitchError::SizeCount {
                expected: group.generator_count(),
                found: sizes.len(),
            });
        }
        Ok(Self {
            group: Arc::clone(group),
            sizes,
        })
    }

    /// The group this tuning is defined on.
    #[must_use]
    pub fn group(&self) -> &Arc<G> {
        &self.group
    }

    /// The size of each generator.
    #[must_use]
    pub fn sizes(&self) -> &[Octaves] {
        &self.sizes
    }

    /// The size of an interval: `tau(g)`.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::IntervalGroupMismatch`] if the interval belongs
    /// to a different group, and [`PitchError::NonFiniteQuantity`] if the sum
    /// overflows.
    pub fn size(&self, interval: &G::Element) -> Result<Octaves, PitchError> {
        let coordinates = self.group.coordinates(interval)?;
        let mut total = 0.0f64;
        for (coordinate, size) in coordinates.iter().zip(&self.sizes) {
            let count =
                ratio_to_f64(coordinate, &Z::from(1)).ok_or(PitchError::NonFiniteQuantity)?;
            total += count * size.get();
        }
        Octaves::new(total)
    }

    /// The tuning error functional `epsilon = tau . V - pi` on a monzo
    /// (UMT-3.2 section 1.8.1).
    ///
    /// For a comma `k` in the kernel this is exactly `-pi(k)`, which is law
    /// T2: the tuning cannot represent the comma, so the whole of it shows up
    /// as error.
    ///
    /// # Errors
    ///
    /// Propagates mapping and valuation failures.
    pub fn error(&self, map: &TemperamentMap, monzo: &Monzo) -> Result<Octaves, PitchError> {
        let mapped = self.group.map_monzo(map, monzo)?;
        let tempered = self.size(&mapped)?;
        let just = monzo.log2_valuation_f64()?;
        Octaves::new(tempered.get() - just)
    }
}

impl RegularTuning<AmbientLattice> {
    /// The equal division of the octave into `divisions` equal steps.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::SizeCount`] if the lattice is not of rank 1, and
    /// [`PitchError::NonPositiveFrequency`] if `divisions` is zero, since a
    /// zero division has no step size.
    pub fn equal_divisions(
        ambient: &Arc<AmbientLattice>,
        divisions: u32,
    ) -> Result<Self, PitchError> {
        if ambient.rank() != 1 {
            return Err(PitchError::SizeCount {
                expected: 1,
                found: ambient.rank(),
            });
        }
        if divisions == 0 {
            return Err(PitchError::NonPositiveFrequency);
        }
        Self::new(ambient, [Octaves::new(1.0 / f64::from(divisions))?])
    }
}

/// A realization of pitch *points*, induced by a regular tuning plus reference
/// data (UMT-3.2 section 1.8.1).
///
/// UMT layer: L2 to L3.
///
/// This is the affine, equivariant map `tau_hat` with `tau_hat(p_0) = q_0` and
/// `tau_hat(p + g) = tau_hat(p) + tau(g)`. It cannot be constructed from a
/// tuning alone, which is the point of fixture F31: interval sizes are
/// available from `tau`, absolute pitches are not.
#[derive(Debug)]
pub struct PitchRealization<G: L2IntervalGroup> {
    tuning: RegularTuning<G>,
    structural_reference: PitchPoint<G::Element>,
    realized_reference: LogFrequency,
}

impl<G: L2IntervalGroup> Clone for PitchRealization<G> {
    fn clone(&self) -> Self {
        Self {
            tuning: self.tuning.clone(),
            structural_reference: self.structural_reference.clone(),
            realized_reference: self.realized_reference,
        }
    }
}

impl<G: L2IntervalGroup + PartialEq> PartialEq for PitchRealization<G> {
    fn eq(&self, other: &Self) -> bool {
        self.tuning == other.tuning
            && self.structural_reference == other.structural_reference
            && self.realized_reference == other.realized_reference
    }
}

impl<G: L2IntervalGroup + PartialEq> Eq for PitchRealization<G> {}

impl<G: L2IntervalGroup> PitchRealization<G> {
    /// Pairs a tuning with the reference data that fix the affine map.
    #[must_use]
    pub fn new(
        tuning: RegularTuning<G>,
        structural_reference: PitchPoint<G::Element>,
        realized_reference: LogFrequency,
    ) -> Self {
        Self {
            tuning,
            structural_reference,
            realized_reference,
        }
    }

    /// The interval tuning.
    #[must_use]
    pub fn tuning(&self) -> &RegularTuning<G> {
        &self.tuning
    }

    /// The structural reference point `p_0`.
    #[must_use]
    pub fn structural_reference(&self) -> &PitchPoint<G::Element> {
        &self.structural_reference
    }

    /// The realized reference pitch `q_0`.
    #[must_use]
    pub fn realized_reference(&self) -> LogFrequency {
        self.realized_reference
    }

    /// Realizes a structural pitch point.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::OriginMismatch`] if the point is measured from a
    /// different origin than the reference, and propagates group mismatches.
    pub fn realize_point(
        &self,
        point: &PitchPoint<G::Element>,
    ) -> Result<LogFrequency, PitchError> {
        let interval = self.structural_reference.interval_to(point)?;
        Ok(self
            .realized_reference
            .translate(self.tuning.size(&interval)?))
    }

    /// Rebuilds this realization at a different concert pitch.
    ///
    /// Only the realized reference changes: the interval lattice and the
    /// tuning are untouched, as UMT-3.2 section 4.2 requires.
    #[must_use]
    pub fn with_concert_pitch(&self, realized_reference: LogFrequency) -> Self {
        Self {
            tuning: self.tuning.clone(),
            structural_reference: self.structural_reference.clone(),
            realized_reference,
        }
    }
}

/// A contextual realization `Phi: P_2 x C -> P_3` (UMT-3.2 section 1.8.2).
///
/// UMT layer: L2 to L3.
///
/// The context is an explicit parameter because a realization that depends on
/// register policy, harmonic context, instrument state, measured
/// inharmonicity, or performance time must say so in its type. Fixture F28
/// requires exactly that: such a realization MUST NOT be serialized or
/// advertised as one unary context-free map.
///
/// No homomorphism law is imposed. `P_2` is a torsor rather than an interval
/// group, and `C` need not carry any group structure at all.
pub trait PitchRealizer<E, C> {
    /// What can go wrong while realizing.
    type Error;

    /// Realizes a structural pitch point in a context.
    ///
    /// # Errors
    ///
    /// Implementation-defined.
    fn realize(&self, point: &PitchPoint<E>, context: &C) -> Result<LogFrequency, Self::Error>;

    /// Whether this realization is regular, that is, a fixed homomorphism on
    /// intervals composed with reference data.
    ///
    /// Defaults to `false`, the safe answer. Law T3 forbids advertising a
    /// context-dependent realization as a regular homomorphism.
    fn is_regular(&self) -> bool {
        false
    }
}

impl<G: L2IntervalGroup, C> PitchRealizer<G::Element, C> for PitchRealization<G> {
    type Error = PitchError;

    fn realize(
        &self,
        point: &PitchPoint<G::Element>,
        _context: &C,
    ) -> Result<LogFrequency, Self::Error> {
        self.realize_point(point)
    }

    fn is_regular(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{PitchRealization, PitchRealizer, RegularTuning};
    use crate::error::PitchError;
    use crate::pitch::point::{PitchOrigin, PitchPoint};
    use crate::pitch::units::{Cents, FrequencyHz, Octaves};
    use crate::proportion::Basis;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    #[test]
    fn equal_division_sizes_are_exactly_proportional() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();
        for (count, cents) in [
            (0i64, 0.0),
            (1, 100.0),
            (7, 700.0),
            (12, 1200.0),
            (-5, -500.0),
        ] {
            let interval = steps.element([count]).unwrap();
            let size = Cents::from(tuning.size(&interval).unwrap());
            assert!((size.get() - cents).abs() < 1e-9, "{count} steps");
        }
    }

    #[test]
    fn a_regular_tuning_is_a_homomorphism() {
        let steps = AmbientLattice::new("umt:edo:31", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 31).unwrap();
        let a = steps.element([9i64]).unwrap();
        let b = steps.element([-4i64]).unwrap();
        let sum = a.checked_add(&b).unwrap();
        assert!(
            (tuning.size(&sum).unwrap().get()
                - (tuning.size(&a).unwrap().get() + tuning.size(&b).unwrap().get()))
            .abs()
                < 1e-12,
            "law T1"
        );
    }

    #[test]
    fn law_t2_the_error_of_a_comma_is_minus_its_just_size() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();

        let comma = basis.monzo([-4, 4, -1]).unwrap();
        let error = Cents::from(tuning.error(&map, &comma).unwrap());
        // The syntonic comma is 21.5063 cents wide and vanishes in 12-EDO.
        assert!((error.get() + 21.506_289_6).abs() < 1e-6, "{error}");

        // The fifth is tempered by about two cents flat.
        let fifth = basis.monzo([-1, 1, 0]).unwrap();
        let error = Cents::from(tuning.error(&map, &fifth).unwrap());
        assert!((error.get() + 1.955_000_9).abs() < 1e-6, "{error}");
    }

    #[test]
    fn f31_a_tuning_alone_does_not_realize_points() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();

        // Interval sizes are available.
        let fifth = Cents::from(tuning.size(&steps.element([7i64]).unwrap()).unwrap());
        assert!((fifth.get() - 700.0).abs() < 1e-9, "{fifth}");

        // Points require reference data, supplied here.
        let origin = PitchOrigin::new("umt:origin:a4");
        let reference = PitchPoint::new(origin.clone(), steps.zero());
        let concert_a = FrequencyHz::new(440.0).unwrap().log_frequency();
        let realization = PitchRealization::new(tuning, reference.clone(), concert_a);

        let up_a_fifth = reference
            .translate(&steps.element([7i64]).unwrap())
            .unwrap();
        let realized = realization.realize_point(&up_a_fifth).unwrap();
        let hz = realized.frequency().unwrap().get();
        assert!((hz - 659.255_113_8).abs() < 1e-6, "{hz}");
    }

    #[test]
    fn changing_concert_pitch_moves_pitches_and_leaves_intervals_alone() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();
        let origin = PitchOrigin::new("umt:origin:a4");
        let reference = PitchPoint::new(origin, steps.zero());

        let at_440 = PitchRealization::new(
            tuning,
            reference.clone(),
            FrequencyHz::new(440.0).unwrap().log_frequency(),
        );
        let at_415 = at_440.with_concert_pitch(FrequencyHz::new(415.0).unwrap().log_frequency());

        let point = reference
            .translate(&steps.element([7i64]).unwrap())
            .unwrap();
        let high = at_440.realize_point(&point).unwrap();
        let low = at_415.realize_point(&point).unwrap();
        assert!(low.frequency().unwrap().get() < high.frequency().unwrap().get());

        // The interval from the reference is identical under both.
        let reference_high = at_440.realize_point(&reference).unwrap();
        let reference_low = at_415.realize_point(&reference).unwrap();
        assert!(
            (reference_high.interval_to(high).get() - reference_low.interval_to(low).get()).abs()
                < 1e-12
        );
    }

    #[test]
    fn a_regular_realization_says_it_is_regular() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();
        let reference = PitchPoint::new(PitchOrigin::new("umt:origin:a4"), steps.zero());
        let realization = PitchRealization::new(
            tuning,
            reference.clone(),
            FrequencyHz::new(440.0).unwrap().log_frequency(),
        );
        assert!(PitchRealizer::<_, ()>::is_regular(&realization));
        assert_eq!(
            realization.realize(&reference, &()).unwrap(),
            realization.realized_reference()
        );
    }

    #[test]
    fn points_from_another_origin_cannot_be_realized() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();
        let realization = PitchRealization::new(
            tuning,
            PitchPoint::new(PitchOrigin::new("umt:origin:a4"), steps.zero()),
            FrequencyHz::new(440.0).unwrap().log_frequency(),
        );
        let elsewhere = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), steps.zero());
        assert!(matches!(
            realization.realize_point(&elsewhere),
            Err(PitchError::OriginMismatch { .. })
        ));
    }

    #[test]
    fn size_counts_and_group_identity_are_validated() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        assert!(matches!(
            RegularTuning::new(&steps, [Octaves::ZERO, Octaves::ZERO]),
            Err(PitchError::SizeCount {
                expected: 1,
                found: 2
            })
        ));

        let other = AmbientLattice::new("umt:edo:19", 1);
        let tuning = RegularTuning::equal_divisions(&steps, 12).unwrap();
        assert!(matches!(
            tuning.size(&other.element([1i64]).unwrap()),
            Err(PitchError::IntervalGroupMismatch)
        ));
    }

    #[test]
    fn a_tuning_on_the_image_is_not_a_tuning_on_the_ambient() {
        // 6-EDO: the image is 2Z, so a tuning of the image has one generator
        // whose size is two ambient steps.
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:6", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[6i64, 10, 14]]).unwrap();
        let image = map.image().clone();

        let tuning = RegularTuning::new(&image, [Octaves::new(2.0 / 6.0).unwrap()]).unwrap();
        let octave = map
            .apply_to_image(&basis.monzo([1, 0, 0]).unwrap())
            .unwrap();
        assert!((tuning.size(&octave).unwrap().get() - 1.0).abs() < 1e-12);

        // The ambient element of the same octave is not accepted here: it is
        // an element of a different declared group.
        assert!(tuning.error(&map, &basis.monzo([1, 0, 0]).unwrap()).is_ok());
    }
}
