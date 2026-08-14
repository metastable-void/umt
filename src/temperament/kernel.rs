//! Kernels and comma subgroups (UMT-3.2 sections 1.4.1 and 1.5).
//!
//! Two constructions arrive here and they are deliberately not the same
//! function:
//!
//! - [`KernelLattice::of_map`] takes the kernel of a mapping into a free
//!   abelian group. Such a kernel is *automatically* saturated (section
//!   1.4.1), so imposing a saturation check on it would be a redundant
//!   rejection of valid data.
//! - [`KernelLattice::from_direct_commas`] takes a subgroup supplied directly
//!   by a user. That one MUST be validated, because an unsaturated comma
//!   subgroup produces a quotient with torsion, which no homomorphism into a
//!   torsion-free real group can realize (section 1.5).

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::lattice::Sublattice;
use crate::algebra::matrix::IntMatrix;
use crate::algebra::normal_form::SmithNormalForm;
use crate::error::TemperamentError;
use crate::proportion::basis::Basis;
use crate::proportion::monzo::Monzo;

/// What to do with a directly supplied comma subgroup that is not saturated.
///
/// UMT-3.2 section 1.5 requires an implementation to do one of these, and to
/// report the change if it saturates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaturationPolicy {
    /// Reject the subgroup.
    Reject,
    /// Replace it by its saturation and report the change.
    Saturate,
}

/// What happened to a directly supplied comma subgroup.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SaturationReport {
    /// Whether the supplied subgroup was already saturated.
    pub was_saturated: bool,
    /// Rank of the supplied subgroup. Saturation never changes the rank.
    pub rank: usize,
    /// The invariant factors of the supplied subgroup that exceed 1. These are
    /// exactly the torsion orders of the quotient, so an empty list means the
    /// quotient is torsion-free.
    pub torsion_invariants: Vec<Z>,
}

/// A subgroup `K <= Lambda_B` of the exact proportion lattice.
///
/// UMT layer: L1/L2, exact. Used both for the kernel of a temperament mapping
/// and for a directly specified comma subgroup; which one it is, and whether
/// it was validated for saturation, is recorded by how it was constructed.
///
/// The basis is canonical, so two kernel lattices over the same basis are
/// equal exactly when they are the same subgroup - which is the honest test
/// for "do these two mappings temper out the same commas".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelLattice {
    domain: Arc<Basis>,
    sublattice: Sublattice,
}

impl KernelLattice {
    /// Builds the kernel of a mapping from a basis of that kernel.
    ///
    /// No saturation validation is performed, and none is needed: the kernel
    /// of a homomorphism into a torsion-free group is saturated as a theorem
    /// (UMT-3.2 section 1.4.1). [`KernelLattice::is_saturated`] can still be
    /// asked, and always answers `true` for such a kernel.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::CoordinateRank`] if the generator matrix
    /// does not have one row per basis generator.
    pub fn of_map(
        domain: &Arc<Basis>,
        generators: &IntMatrix,
    ) -> Result<Arc<Self>, TemperamentError> {
        let sublattice = Sublattice::from_generators(domain.rank(), generators)?;
        Ok(Arc::new(Self {
            domain: Arc::clone(domain),
            sublattice,
        }))
    }

    /// Validates a directly supplied comma subgroup (UMT-3.2 section 1.5).
    ///
    /// Under [`SaturationPolicy::Reject`] an unsaturated subgroup is refused.
    /// Under [`SaturationPolicy::Saturate`] it is replaced by its saturation
    /// and the returned report records the change, including the torsion
    /// orders that were removed.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::UnsaturatedCommaSubgroup`] under the
    /// rejecting policy, and [`TemperamentError::CoordinateRank`] on a shape
    /// mismatch.
    ///
    /// # Examples
    ///
    /// Twice the syntonic comma generates a subgroup with 2-torsion:
    ///
    /// ```
    /// use umt::temperament::kernel::{KernelLattice, SaturationPolicy};
    /// use umt::{Basis, IntMatrix};
    ///
    /// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    /// let doubled = IntMatrix::from_rows([[-8i64], [8], [-2]])?;
    ///
    /// assert!(
    ///     KernelLattice::from_direct_commas(&basis, &doubled, SaturationPolicy::Reject)
    ///         .is_err()
    /// );
    ///
    /// let (kernel, report) =
    ///     KernelLattice::from_direct_commas(&basis, &doubled, SaturationPolicy::Saturate)?;
    /// assert!(!report.was_saturated);
    /// assert!(kernel.contains(&basis.monzo([-4, 4, -1])?)?);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn from_direct_commas(
        domain: &Arc<Basis>,
        generators: &IntMatrix,
        policy: SaturationPolicy,
    ) -> Result<(Arc<Self>, SaturationReport), TemperamentError> {
        let sublattice = Sublattice::from_generators(domain.rank(), generators)?;
        let smith = SmithNormalForm::of(sublattice.basis());
        let torsion_invariants: Vec<Z> = smith
            .invariant_factors()
            .iter()
            .filter(|factor| **factor != Z::from(1))
            .cloned()
            .collect();
        let was_saturated = torsion_invariants.is_empty();

        let report = SaturationReport {
            was_saturated,
            rank: sublattice.rank(),
            torsion_invariants,
        };

        if !was_saturated {
            match policy {
                SaturationPolicy::Reject => {
                    return Err(TemperamentError::UnsaturatedCommaSubgroup {
                        torsion_invariants: report.torsion_invariants,
                    });
                }
                SaturationPolicy::Saturate => {
                    return Ok((
                        Arc::new(Self {
                            domain: Arc::clone(domain),
                            sublattice: sublattice.saturation(),
                        }),
                        report,
                    ));
                }
            }
        }

        Ok((
            Arc::new(Self {
                domain: Arc::clone(domain),
                sublattice,
            }),
            report,
        ))
    }

    /// The basis whose lattice this is a subgroup of.
    #[must_use]
    pub fn domain(&self) -> &Arc<Basis> {
        &self.domain
    }

    /// The underlying sublattice.
    #[must_use]
    pub fn sublattice(&self) -> &Sublattice {
        &self.sublattice
    }

    /// The rank of the kernel.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.sublattice.rank()
    }

    /// The canonical basis, as columns of monzo coordinates.
    #[must_use]
    pub fn basis(&self) -> &IntMatrix {
        self.sublattice.basis()
    }

    /// The canonical basis, as monzos.
    ///
    /// These are the canonical Hermite basis vectors, whose leading nonzero
    /// coordinate is positive. That is a presentation choice of the canonical
    /// form, not a musical one: the subgroup generated by the syntonic comma
    /// `81/80` is presented by the monzo of `80/81`, since both generate the
    /// same subgroup and only one of them is canonical. Callers that want a
    /// conventional comma orientation should negate as they see fit.
    #[must_use]
    pub fn basis_monzos(&self) -> Vec<Monzo> {
        (0..self.rank())
            .map(|column| {
                let coordinates = self
                    .basis()
                    .column(column)
                    .expect("invariant: column index is in range");
                Monzo::new(Arc::clone(&self.domain), coordinates)
                    .expect("invariant: kernel basis columns have domain rank")
            })
            .collect()
    }

    /// Whether this subgroup is saturated in the domain lattice.
    #[must_use]
    pub fn is_saturated(&self) -> bool {
        self.sublattice.is_saturated()
    }

    /// Builds a kernel element from intrinsic coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::CoordinateRank`] if the coordinate count
    /// differs from the kernel rank.
    pub fn element<I, T>(self: &Arc<Self>, coordinates: I) -> Result<KernelElem, TemperamentError>
    where
        I: IntoIterator<Item = T>,
        T: Into<Z>,
    {
        let coordinates: Vec<Z> = coordinates.into_iter().map(Into::into).collect();
        if coordinates.len() != self.rank() {
            return Err(TemperamentError::CoordinateRank {
                expected: self.rank(),
                found: coordinates.len(),
            });
        }
        Ok(KernelElem {
            lattice: Arc::clone(self),
            coordinates,
        })
    }

    /// Embeds a kernel element into the proportion lattice.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::KernelMismatch`] if the element belongs to
    /// a different kernel lattice.
    pub fn embed(self: &Arc<Self>, element: &KernelElem) -> Result<Monzo, TemperamentError> {
        if !Arc::ptr_eq(self, &element.lattice) && **self != *element.lattice {
            return Err(TemperamentError::KernelMismatch);
        }
        let coordinates = self.sublattice.embed(&element.coordinates)?;
        Ok(Monzo::new(Arc::clone(&self.domain), coordinates)
            .expect("invariant: embedded coordinates have domain rank"))
    }

    /// The intrinsic kernel coordinates of a monzo, or `None` if it is not in
    /// the kernel.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    pub fn coordinates(
        self: &Arc<Self>,
        monzo: &Monzo,
    ) -> Result<Option<KernelElem>, TemperamentError> {
        if !crate::proportion::monzo::compatible(monzo.basis(), &self.domain) {
            return Err(TemperamentError::BasisMismatch {
                expected: self.domain.id().clone(),
                found: monzo.basis().id().clone(),
            });
        }
        Ok(self
            .sublattice
            .coordinates(monzo.exponents())?
            .map(|coordinates| KernelElem {
                lattice: Arc::clone(self),
                coordinates,
            }))
    }

    /// Whether a monzo lies in this subgroup.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    pub fn contains(self: &Arc<Self>, monzo: &Monzo) -> Result<bool, TemperamentError> {
        Ok(self.coordinates(monzo)?.is_some())
    }
}

/// An element of a kernel or comma subgroup, in intrinsic coordinates.
///
/// UMT layer: L1/L2, exact. This is the type of a temperament residue: the
/// exact structural information about which lift of a tempered class was
/// present, not a floating-point error and not a tuning deviation (UMT-3.2
/// sections 1.7.2 and 7.9).
#[derive(Debug, Clone)]
pub struct KernelElem {
    lattice: Arc<KernelLattice>,
    coordinates: Vec<Z>,
}

impl KernelElem {
    /// The kernel lattice this element belongs to.
    #[must_use]
    pub fn lattice(&self) -> &Arc<KernelLattice> {
        &self.lattice
    }

    /// The intrinsic coordinates.
    #[must_use]
    pub fn coordinates(&self) -> &[Z] {
        &self.coordinates
    }

    /// Whether this is the zero element, that is, no comma residue.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.coordinates.iter().all(num_traits::Zero::is_zero)
    }
}

impl PartialEq for KernelElem {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates == other.coordinates
            && (Arc::ptr_eq(&self.lattice, &other.lattice) || *self.lattice == *other.lattice)
    }
}

impl Eq for KernelElem {}

#[cfg(test)]
mod tests {
    use super::{KernelLattice, SaturationPolicy};
    use crate::algebra::Z;
    use crate::algebra::matrix::IntMatrix;
    use crate::error::TemperamentError;
    use crate::proportion::Basis;

    #[test]
    fn direct_comma_subgroups_are_validated() {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let doubled = IntMatrix::from_rows([[-8i64], [8], [-2]]).unwrap();

        assert_eq!(
            KernelLattice::from_direct_commas(&basis, &doubled, SaturationPolicy::Reject)
                .unwrap_err(),
            TemperamentError::UnsaturatedCommaSubgroup {
                torsion_invariants: alloc::vec![Z::from(2)]
            }
        );

        let (kernel, report) =
            KernelLattice::from_direct_commas(&basis, &doubled, SaturationPolicy::Saturate)
                .unwrap();
        assert!(!report.was_saturated);
        assert_eq!(report.rank, 1);
        assert_eq!(report.torsion_invariants, alloc::vec![Z::from(2)]);
        assert!(kernel.is_saturated());

        let comma = basis.monzo([-4, 4, -1]).unwrap();
        assert!(kernel.contains(&comma).unwrap());
        // The canonical basis presents the reciprocal, whose leading
        // coordinate is positive; it generates the same subgroup.
        assert_eq!(
            kernel.basis_monzos(),
            alloc::vec![basis.monzo([4, -4, 1]).unwrap()]
        );
        assert!(kernel.contains(&basis.monzo([4, -4, 1]).unwrap()).unwrap());
    }

    #[test]
    fn a_saturated_subgroup_passes_both_policies() {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let comma = IntMatrix::from_rows([[-4i64], [4], [-1]]).unwrap();

        for policy in [SaturationPolicy::Reject, SaturationPolicy::Saturate] {
            let (kernel, report) =
                KernelLattice::from_direct_commas(&basis, &comma, policy).unwrap();
            assert!(report.was_saturated);
            assert!(report.torsion_invariants.is_empty());
            assert_eq!(kernel.rank(), 1);
        }
    }

    #[test]
    fn kernel_coordinates_round_trip() {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let commas = IntMatrix::from_rows([[-4i64, -19], [4, 12], [-1, 0]]).unwrap();
        let kernel = KernelLattice::of_map(&basis, &commas).unwrap();
        assert_eq!(kernel.rank(), 2);

        let element = kernel.element([3i64, -2]).unwrap();
        let monzo = kernel.embed(&element).unwrap();
        assert_eq!(kernel.coordinates(&monzo).unwrap(), Some(element));

        // A monzo outside the kernel has no coordinates.
        let fifth = basis.monzo([-1, 1, 0]).unwrap();
        assert_eq!(kernel.coordinates(&fifth).unwrap(), None);
        assert!(!kernel.contains(&fifth).unwrap());
    }

    #[test]
    fn foreign_monzos_are_rejected() {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        let kernel =
            KernelLattice::of_map(&basis, &IntMatrix::from_rows([[-4i64], [4], [-1]]).unwrap())
                .unwrap();
        assert!(matches!(
            kernel.contains(&other.monzo([1, 0, 0]).unwrap()),
            Err(TemperamentError::BasisMismatch { .. })
        ));
    }
}
