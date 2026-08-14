//! Quotients of a free abelian group by a sublattice.
//!
//! By the structure theorem, `Z^n / L` is `Z^{n - rank(L)}` plus a finite part
//! determined by the invariant factors of `L`. Both parts matter here: a
//! quotient with free rank left over is infinite, and a quotient whose torsion
//! is nontrivial is what an unsaturated comma subgroup produces (UMT-3.2
//! section 1.5).

use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::lattice::Sublattice;
use crate::algebra::normal_form::SmithNormalForm;
use crate::error::MatrixError;

/// The isomorphism type of `Z^n / L`.
///
/// UMT layer: L2, exact. Equality is equality of the isomorphism type: same
/// free rank and same torsion invariants. That is a genuine mathematical
/// equality, not merely presentation equality, because the invariant factors
/// are a complete invariant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuotientGroup {
    free_rank: usize,
    torsion: Vec<Z>,
}

impl QuotientGroup {
    /// The quotient of `Z^{ambient_rank}` by a sublattice.
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::DimensionMismatch`] if the sublattice does not
    /// live in `Z^{ambient_rank}`.
    pub fn of(ambient_rank: usize, sublattice: &Sublattice) -> Result<Self, MatrixError> {
        if sublattice.ambient_rank() != ambient_rank {
            return Err(MatrixError::DimensionMismatch {
                left: ambient_rank,
                right: sublattice.ambient_rank(),
            });
        }
        let smith = SmithNormalForm::of(sublattice.basis());
        let torsion: Vec<Z> = smith
            .invariant_factors()
            .iter()
            .filter(|factor| **factor != Z::from(1))
            .cloned()
            .collect();
        Ok(Self {
            free_rank: ambient_rank - smith.rank(),
            torsion,
        })
    }

    /// The trivial group.
    #[must_use]
    pub fn trivial() -> Self {
        Self {
            free_rank: 0,
            torsion: Vec::new(),
        }
    }

    /// The rank of the free part.
    #[must_use]
    pub fn free_rank(&self) -> usize {
        self.free_rank
    }

    /// The torsion invariants, each greater than 1 and each dividing the next.
    #[must_use]
    pub fn torsion(&self) -> &[Z] {
        &self.torsion
    }

    /// Whether the quotient is finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.free_rank == 0
    }

    /// Whether the quotient is trivial.
    #[must_use]
    pub fn is_trivial(&self) -> bool {
        self.free_rank == 0 && self.torsion.is_empty()
    }

    /// Whether the quotient is torsion-free, that is, whether the sublattice
    /// was saturated.
    #[must_use]
    pub fn is_torsion_free(&self) -> bool {
        self.torsion.is_empty()
    }

    /// The number of elements, when the quotient is finite.
    ///
    /// Returns `None` for an infinite quotient rather than a misleading zero.
    #[must_use]
    pub fn order(&self) -> Option<Z> {
        if !self.is_finite() {
            return None;
        }
        Some(
            self.torsion
                .iter()
                .fold(Z::from(1), |product, factor| product * factor),
        )
    }
}

impl core::fmt::Display for QuotientGroup {
    /// Writes the isomorphism type, for example `Z/12Z` or `Z^2 (+) Z/3Z`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_trivial() {
            return f.write_str("0");
        }
        let mut first = true;
        if self.free_rank == 1 {
            f.write_str("Z")?;
            first = false;
        } else if self.free_rank > 1 {
            write!(f, "Z^{}", self.free_rank)?;
            first = false;
        }
        for factor in &self.torsion {
            if !first {
                f.write_str(" (+) ")?;
            }
            write!(f, "Z/{factor}Z")?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::QuotientGroup;
    use crate::algebra::Z;
    use crate::algebra::lattice::Sublattice;
    use crate::algebra::matrix::IntMatrix;
    use alloc::string::ToString;

    #[test]
    fn cyclic_quotients() {
        let twelve =
            Sublattice::from_generators(1, &IntMatrix::from_rows([[12i64]]).unwrap()).unwrap();
        let quotient = QuotientGroup::of(1, &twelve).unwrap();
        assert_eq!(quotient.to_string(), "Z/12Z");
        assert!(quotient.is_finite());
        assert_eq!(quotient.order(), Some(Z::from(12)));
        assert!(!quotient.is_torsion_free());
    }

    #[test]
    fn the_trivial_and_full_extremes() {
        let quotient = QuotientGroup::of(3, &Sublattice::trivial(3)).unwrap();
        assert_eq!(quotient.to_string(), "Z^3");
        assert_eq!(quotient.free_rank(), 3);
        assert!(!quotient.is_finite());
        assert_eq!(quotient.order(), None);

        let quotient = QuotientGroup::of(3, &Sublattice::full(3)).unwrap();
        assert!(quotient.is_trivial());
        assert_eq!(quotient.to_string(), "0");
        assert_eq!(quotient.order(), Some(Z::from(1)));
    }

    #[test]
    fn mixed_free_and_torsion_parts() {
        // The subgroup generated by 2e1 inside Z^2.
        let lattice =
            Sublattice::from_generators(2, &IntMatrix::from_rows([[2i64], [0]]).unwrap()).unwrap();
        let quotient = QuotientGroup::of(2, &lattice).unwrap();
        assert_eq!(quotient.free_rank(), 1);
        assert_eq!(quotient.torsion(), &[Z::from(2)]);
        assert_eq!(quotient.to_string(), "Z (+) Z/2Z");
        assert!(!quotient.is_finite());
        assert_eq!(quotient.order(), None);
    }

    #[test]
    fn a_saturated_sublattice_leaves_no_torsion() {
        let lattice =
            Sublattice::from_generators(3, &IntMatrix::from_rows([[-4i64], [4], [-1]]).unwrap())
                .unwrap();
        let quotient = QuotientGroup::of(3, &lattice).unwrap();
        assert!(quotient.is_torsion_free());
        assert_eq!(quotient.free_rank(), 2);

        // Twice the same comma leaves 2-torsion behind.
        let doubled =
            Sublattice::from_generators(3, &IntMatrix::from_rows([[-8i64], [8], [-2]]).unwrap())
                .unwrap();
        let quotient = QuotientGroup::of(3, &doubled).unwrap();
        assert!(!quotient.is_torsion_free());
        assert_eq!(quotient.torsion(), &[Z::from(2)]);
        assert_eq!(quotient.to_string(), "Z^2 (+) Z/2Z");
    }

    #[test]
    fn rank_is_validated() {
        assert!(QuotientGroup::of(2, &Sublattice::trivial(3)).is_err());
    }
}
