//! Unit equivalence (UMT-3.2 section 1.9).
//!
//! A domain may introduce an equivalence by a designated unit element, giving
//! the quotient `G_2 / <V(u)>`. The declared interval group `G_2` may be the
//! reachable image `H` or the ambient group `Gamma`, and *which one was
//! chosen MUST be recorded*, because the answers differ.
//!
//! The 6-EDO fixture is the case that makes this concrete. With `H = 2Z`,
//! `Gamma = Z`, and the octave mapping to 6 steps:
//!
//! ```text
//! H / 6Z      = 2Z / 6Z  ~ Z/3Z
//! Gamma / 6Z  =  Z / 6Z  ~ Z/6Z
//! ```
//!
//! Three reachable pitch classes, six ambient ones. Silently identifying them
//! is the error this module exists to prevent.

use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::lattice::Sublattice;
use crate::algebra::matrix::IntMatrix;
use crate::algebra::quotient::QuotientGroup;
use crate::error::TemperamentError;
use crate::proportion::monzo::Monzo;
use crate::temperament::map::TemperamentMap;

/// Which interval group the equivalence was formed on (UMT-3.2 section 1.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum EquivalenceDomain {
    /// Formed on the ambient group `Gamma`.
    Ambient,
    /// Formed on the reachable image `H = im(V)`.
    Image,
}

/// The quotient of a declared interval group by a designated unit.
///
/// UMT layer: L2, exact.
///
/// Unit equivalence is not temperament. Temperament is the quotient by the
/// kernel of a mapping; this is a further, optional quotient by one designated
/// element, and section 1.9 requires the two never be conflated.
///
/// # Examples
///
/// ```
/// use umt::temperament::{AmbientLattice, EquivalenceDomain, TemperamentMap, UnitEquivalence};
/// use umt::Basis;
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let steps = AmbientLattice::new("umt:edo:6", 1);
/// let map = TemperamentMap::from_rows(&basis, &steps, [[6i64, 10, 14]])?;
/// let octave = basis.monzo([1, 0, 0])?;
///
/// let ambient = UnitEquivalence::on_ambient(&map, &octave)?;
/// let reachable = UnitEquivalence::on_image(&map, &octave)?;
///
/// assert_eq!(ambient.quotient().to_string(), "Z/6Z");
/// assert_eq!(reachable.quotient().to_string(), "Z/3Z");
/// assert_eq!(ambient.domain(), EquivalenceDomain::Ambient);
/// assert_ne!(ambient.quotient(), reachable.quotient());
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitEquivalence {
    domain: EquivalenceDomain,
    unit_coordinates: Vec<Z>,
    quotient: QuotientGroup,
}

impl UnitEquivalence {
    /// Forms the equivalence on the ambient group `Gamma`.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the unit is over an
    /// unrelated basis.
    pub fn on_ambient(map: &TemperamentMap, unit: &Monzo) -> Result<Self, TemperamentError> {
        let coordinates = map.apply(unit)?.coordinates().to_vec();
        Self::build(
            EquivalenceDomain::Ambient,
            map.ambient().rank(),
            coordinates,
        )
    }

    /// Forms the equivalence on the reachable image `H`.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the unit is over an
    /// unrelated basis.
    pub fn on_image(map: &TemperamentMap, unit: &Monzo) -> Result<Self, TemperamentError> {
        let coordinates = map.apply_to_image(unit)?.coordinates().to_vec();
        Self::build(EquivalenceDomain::Image, map.image().rank(), coordinates)
    }

    fn build(
        domain: EquivalenceDomain,
        rank: usize,
        unit_coordinates: Vec<Z>,
    ) -> Result<Self, TemperamentError> {
        let generator = IntMatrix::new(rank, 1, unit_coordinates.clone())?;
        let sublattice = Sublattice::from_generators(rank, &generator)?;
        let quotient = QuotientGroup::of(rank, &sublattice)?;
        Ok(Self {
            domain,
            unit_coordinates,
            quotient,
        })
    }

    /// Which interval group this equivalence was formed on.
    #[must_use]
    pub fn domain(&self) -> EquivalenceDomain {
        self.domain
    }

    /// The coordinates of the unit's image, in the coordinates of the chosen
    /// group.
    ///
    /// For the 6-EDO octave these are `[6]` on the ambient group and `[3]` on
    /// the image, which is exactly why the two quotients differ.
    #[must_use]
    pub fn unit_coordinates(&self) -> &[Z] {
        &self.unit_coordinates
    }

    /// The resulting quotient group.
    #[must_use]
    pub fn quotient(&self) -> &QuotientGroup {
        &self.quotient
    }

    /// The number of equivalence classes, when there are finitely many.
    #[must_use]
    pub fn class_count(&self) -> Option<Z> {
        self.quotient.order()
    }
}

#[cfg(test)]
mod tests {
    use super::{EquivalenceDomain, UnitEquivalence};
    use crate::algebra::Z;
    use crate::proportion::Basis;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use alloc::string::ToString;
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    #[test]
    fn twelve_edo_pitch_classes() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        let octave = basis.monzo([1, 0, 0]).unwrap();

        let ambient = UnitEquivalence::on_ambient(&map, &octave).unwrap();
        assert_eq!(ambient.quotient().to_string(), "Z/12Z");
        assert_eq!(ambient.class_count(), Some(Z::from(12)));
        assert_eq!(ambient.unit_coordinates(), &[Z::from(12)]);

        // The mapping is surjective, so both groups agree here.
        let reachable = UnitEquivalence::on_image(&map, &octave).unwrap();
        assert_eq!(reachable.quotient(), ambient.quotient());
        assert_eq!(reachable.domain(), EquivalenceDomain::Image);
    }

    #[test]
    fn six_edo_reachable_and_ambient_classes_differ() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:6", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[6i64, 10, 14]]).unwrap();
        let octave = basis.monzo([1, 0, 0]).unwrap();

        let ambient = UnitEquivalence::on_ambient(&map, &octave).unwrap();
        let reachable = UnitEquivalence::on_image(&map, &octave).unwrap();

        assert_eq!(ambient.class_count(), Some(Z::from(6)));
        assert_eq!(reachable.class_count(), Some(Z::from(3)));
        assert_ne!(ambient.quotient(), reachable.quotient());
        assert_eq!(ambient.unit_coordinates(), &[Z::from(6)]);
        assert_eq!(reachable.unit_coordinates(), &[Z::from(3)]);
    }

    #[test]
    fn a_unit_that_is_tempered_out_gives_no_finite_quotient() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();

        // The syntonic comma vanishes, so quotienting by it changes nothing.
        let comma = basis.monzo([-4, 4, -1]).unwrap();
        let equivalence = UnitEquivalence::on_ambient(&map, &comma).unwrap();
        assert_eq!(equivalence.quotient().to_string(), "Z");
        assert_eq!(equivalence.class_count(), None);
    }

    #[test]
    fn rank_two_unit_equivalence() {
        let basis = five_limit();
        let ambient = AmbientLattice::new("umt:meantone-coords", 2);
        let map = TemperamentMap::from_rows(&basis, &ambient, [[1i64, 0, -4], [0, 1, 4]]).unwrap();
        let octave = basis.monzo([1, 0, 0]).unwrap();

        // The octave is the first generator, so the quotient keeps one free
        // rank: octave equivalence does not make a rank-2 temperament finite.
        let equivalence = UnitEquivalence::on_ambient(&map, &octave).unwrap();
        assert_eq!(equivalence.quotient().free_rank(), 1);
        assert_eq!(equivalence.class_count(), None);
        assert_eq!(equivalence.quotient().to_string(), "Z");
    }

    #[test]
    fn foreign_units_are_rejected() {
        let basis = five_limit();
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        assert!(UnitEquivalence::on_ambient(&map, &other.monzo([1, 0, 0]).unwrap()).is_err());
    }
}
