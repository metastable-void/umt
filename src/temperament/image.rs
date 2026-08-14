//! Ambient lattices and reachable images (UMT-3.2 section 1.4).
//!
//! A regular temperament mapping `V: Lambda_B -> Gamma` need not be
//! surjective, so the declared ambient group `Gamma` and the reachable image
//! `H = im(V) <= Gamma` are different objects and are represented separately.
//! An [`ImageElem`] carries *intrinsic* coordinates relative to the canonical
//! basis of `H`, never ambient coordinates, so the two cannot be confused by
//! accident.

use alloc::sync::Arc;
use alloc::vec::Vec;

use num_traits::Zero;

use crate::algebra::Z;
use crate::algebra::lattice::Sublattice;
use crate::algebra::matrix::IntMatrix;
use crate::error::TemperamentError;

/// Stable identity of a declared ambient lattice.
///
/// UMT layer: L2 metadata. Two ambient lattices of equal rank are not
/// interchangeable unless they are the same declared object, for the same
/// reason two bases of equal rank are not (prompt section 7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "alloc::string::String", from = "alloc::string::String")
)]
pub struct LatticeId(Arc<str>);

impl From<alloc::string::String> for LatticeId {
    fn from(value: alloc::string::String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<LatticeId> for alloc::string::String {
    fn from(value: LatticeId) -> Self {
        value.as_str().into()
    }
}

impl LatticeId {
    /// Wraps a stable lattice identity.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// The identity text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for LatticeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A declared ambient free abelian group `Gamma = Z^r` (UMT-3.2 section 1.4).
///
/// UMT layer: L2, exact. Equality is presentation equality over the identity
/// and the rank.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmbientLattice {
    id: LatticeId,
    rank: usize,
}

impl AmbientLattice {
    /// Declares an ambient lattice of the given rank.
    #[must_use]
    pub fn new(id: &str, rank: usize) -> Arc<Self> {
        Arc::new(Self {
            id: LatticeId::new(id),
            rank,
        })
    }

    /// The lattice identity.
    #[must_use]
    pub fn id(&self) -> &LatticeId {
        &self.id
    }

    /// The rank `r`.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// Whether two ambient lattices are the same declared object.
    #[must_use]
    pub fn same_identity(&self, other: &Self) -> bool {
        self == other
    }

    /// Builds an ambient element from coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::CoordinateRank`] if the coordinate count
    /// differs from the rank.
    pub fn element<I, T>(self: &Arc<Self>, coordinates: I) -> Result<AmbientElem, TemperamentError>
    where
        I: IntoIterator<Item = T>,
        T: Into<Z>,
    {
        let coordinates: Vec<Z> = coordinates.into_iter().map(Into::into).collect();
        if coordinates.len() != self.rank {
            return Err(TemperamentError::CoordinateRank {
                expected: self.rank,
                found: coordinates.len(),
            });
        }
        Ok(AmbientElem {
            lattice: Arc::clone(self),
            coordinates,
        })
    }

    /// The zero element.
    #[must_use]
    pub fn zero(self: &Arc<Self>) -> AmbientElem {
        AmbientElem {
            lattice: Arc::clone(self),
            coordinates: (0..self.rank).map(|_| Z::zero()).collect(),
        }
    }
}

/// An element of a declared ambient lattice, in ambient coordinates.
///
/// UMT layer: L2, exact. Equality is presentation equality: same declared
/// ambient lattice and same coordinates.
#[derive(Debug, Clone)]
pub struct AmbientElem {
    lattice: Arc<AmbientLattice>,
    coordinates: Vec<Z>,
}

impl AmbientElem {
    /// The lattice this element belongs to.
    #[must_use]
    pub fn lattice(&self) -> &Arc<AmbientLattice> {
        &self.lattice
    }

    /// The ambient coordinates.
    #[must_use]
    pub fn coordinates(&self) -> &[Z] {
        &self.coordinates
    }

    /// Whether this is the zero element.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.coordinates.iter().all(Zero::is_zero)
    }

    /// Group addition.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::AmbientMismatch`] if the operands belong to
    /// different declared ambient lattices.
    pub fn checked_add(&self, other: &Self) -> Result<Self, TemperamentError> {
        self.combine(other, false)
    }

    /// Group subtraction.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::AmbientMismatch`] if the operands belong to
    /// different declared ambient lattices.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, TemperamentError> {
        self.combine(other, true)
    }

    fn combine(&self, other: &Self, subtract: bool) -> Result<Self, TemperamentError> {
        if !compatible(&self.lattice, &other.lattice) {
            return Err(TemperamentError::AmbientMismatch {
                expected: self.lattice.id().clone(),
                found: other.lattice.id().clone(),
            });
        }
        Ok(Self {
            lattice: Arc::clone(&self.lattice),
            coordinates: self
                .coordinates
                .iter()
                .zip(&other.coordinates)
                .map(|(a, b)| if subtract { a - b } else { a + b })
                .collect(),
        })
    }
}

impl PartialEq for AmbientElem {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates == other.coordinates && compatible(&self.lattice, &other.lattice)
    }
}

impl Eq for AmbientElem {}

impl core::ops::Neg for &AmbientElem {
    type Output = AmbientElem;

    fn neg(self) -> AmbientElem {
        AmbientElem {
            lattice: Arc::clone(&self.lattice),
            coordinates: self.coordinates.iter().map(core::ops::Neg::neg).collect(),
        }
    }
}

/// The reachable image `H = im(V) <= Gamma` (UMT-3.2 section 1.4).
///
/// UMT layer: L2, exact. The basis is canonical, so two image lattices over
/// the same ambient lattice are equal exactly when they are the same subgroup.
///
/// The 6-EDO fixture is the motivating case: with `Gamma = Z` and
/// `H = 2Z`, an odd ambient step is a perfectly good element of `Gamma` that
/// simply is not in `H`, and [`ImageLattice::from_ambient`] says so instead of
/// inventing a coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageLattice {
    ambient: Arc<AmbientLattice>,
    sublattice: Sublattice,
}

impl ImageLattice {
    /// Builds an image lattice from a sublattice of the ambient lattice.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::CoordinateRank`] if the sublattice does not
    /// live in an ambient space of the declared rank.
    pub fn new(
        ambient: &Arc<AmbientLattice>,
        sublattice: Sublattice,
    ) -> Result<Arc<Self>, TemperamentError> {
        if sublattice.ambient_rank() != ambient.rank() {
            return Err(TemperamentError::CoordinateRank {
                expected: ambient.rank(),
                found: sublattice.ambient_rank(),
            });
        }
        Ok(Arc::new(Self {
            ambient: Arc::clone(ambient),
            sublattice,
        }))
    }

    /// The ambient lattice this is a subgroup of.
    #[must_use]
    pub fn ambient(&self) -> &Arc<AmbientLattice> {
        &self.ambient
    }

    /// The underlying sublattice.
    #[must_use]
    pub fn sublattice(&self) -> &Sublattice {
        &self.sublattice
    }

    /// The rank of the image.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.sublattice.rank()
    }

    /// The canonical basis of the image, as columns in ambient coordinates.
    #[must_use]
    pub fn basis(&self) -> &IntMatrix {
        self.sublattice.basis()
    }

    /// Whether the image is the whole ambient lattice, that is, whether the
    /// mapping is surjective.
    ///
    /// This is a statement about the image alone. It is unrelated to
    /// saturation of the kernel, which is automatic for a map into a free
    /// abelian group (UMT-3.2 sections 1.4.1 and 1.4.2).
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.sublattice.is_full()
    }

    /// Builds an image element from intrinsic coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::CoordinateRank`] if the coordinate count
    /// differs from the image rank.
    pub fn element<I, T>(self: &Arc<Self>, coordinates: I) -> Result<ImageElem, TemperamentError>
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
        Ok(ImageElem {
            lattice: Arc::clone(self),
            coordinates,
        })
    }

    /// Embeds an image element into the ambient lattice.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::ImageMismatch`] if the element belongs to a
    /// different image lattice.
    pub fn embed(self: &Arc<Self>, element: &ImageElem) -> Result<AmbientElem, TemperamentError> {
        if !Arc::ptr_eq(self, &element.lattice) && **self != *element.lattice {
            return Err(TemperamentError::ImageMismatch);
        }
        let coordinates = self.sublattice.embed(&element.coordinates)?;
        Ok(AmbientElem {
            lattice: Arc::clone(&self.ambient),
            coordinates,
        })
    }

    /// Converts an ambient element into intrinsic image coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::AmbientMismatch`] if the element belongs to
    /// a different ambient lattice, and [`TemperamentError::NotInImage`] if it
    /// is not reachable. The second case is not a defect: it is what makes an
    /// odd 6-EDO step undetemperable under `[6, 10, 14]`.
    pub fn from_ambient(
        self: &Arc<Self>,
        element: &AmbientElem,
    ) -> Result<ImageElem, TemperamentError> {
        if !compatible(&self.ambient, &element.lattice) {
            return Err(TemperamentError::AmbientMismatch {
                expected: self.ambient.id().clone(),
                found: element.lattice.id().clone(),
            });
        }
        match self.sublattice.coordinates(&element.coordinates)? {
            Some(coordinates) => Ok(ImageElem {
                lattice: Arc::clone(self),
                coordinates,
            }),
            None => Err(TemperamentError::NotInImage {
                coordinates: element.coordinates.clone(),
            }),
        }
    }

    /// Whether an ambient element is reachable.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::AmbientMismatch`] if the element belongs to
    /// a different ambient lattice.
    pub fn contains(self: &Arc<Self>, element: &AmbientElem) -> Result<bool, TemperamentError> {
        match self.from_ambient(element) {
            Ok(_) => Ok(true),
            Err(TemperamentError::NotInImage { .. }) => Ok(false),
            Err(other) => Err(other),
        }
    }
}

/// An element of the reachable image, in intrinsic image coordinates.
///
/// UMT layer: L2, exact. These coordinates are *not* ambient coordinates: for
/// `H = 2Z <= Z`, the image element with coordinate 3 is the ambient step 6.
/// Use [`ImageLattice::embed`] to move to ambient coordinates.
#[derive(Debug, Clone)]
pub struct ImageElem {
    lattice: Arc<ImageLattice>,
    coordinates: Vec<Z>,
}

impl ImageElem {
    /// The image lattice this element belongs to.
    #[must_use]
    pub fn lattice(&self) -> &Arc<ImageLattice> {
        &self.lattice
    }

    /// The intrinsic coordinates.
    #[must_use]
    pub fn coordinates(&self) -> &[Z] {
        &self.coordinates
    }

    /// Whether this is the zero element.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.coordinates.iter().all(Zero::is_zero)
    }

    /// Group addition.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::ImageMismatch`] if the operands belong to
    /// different image lattices.
    pub fn checked_add(&self, other: &Self) -> Result<Self, TemperamentError> {
        self.combine(other, false)
    }

    /// Group subtraction.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::ImageMismatch`] if the operands belong to
    /// different image lattices.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, TemperamentError> {
        self.combine(other, true)
    }

    fn combine(&self, other: &Self, subtract: bool) -> Result<Self, TemperamentError> {
        if !Arc::ptr_eq(&self.lattice, &other.lattice) && *self.lattice != *other.lattice {
            return Err(TemperamentError::ImageMismatch);
        }
        Ok(Self {
            lattice: Arc::clone(&self.lattice),
            coordinates: self
                .coordinates
                .iter()
                .zip(&other.coordinates)
                .map(|(a, b)| if subtract { a - b } else { a + b })
                .collect(),
        })
    }
}

impl core::ops::Neg for &ImageElem {
    type Output = ImageElem;

    fn neg(self) -> ImageElem {
        ImageElem {
            lattice: Arc::clone(&self.lattice),
            coordinates: self.coordinates.iter().map(core::ops::Neg::neg).collect(),
        }
    }
}

impl PartialEq for ImageElem {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates == other.coordinates
            && (Arc::ptr_eq(&self.lattice, &other.lattice) || *self.lattice == *other.lattice)
    }
}

impl Eq for ImageElem {}

fn compatible(left: &Arc<AmbientLattice>, right: &Arc<AmbientLattice>) -> bool {
    Arc::ptr_eq(left, right) || left.same_identity(right)
}

#[cfg(test)]
mod tests {
    use super::{AmbientLattice, ImageLattice};
    use crate::algebra::Z;
    use crate::algebra::lattice::Sublattice;
    use crate::algebra::matrix::IntMatrix;
    use crate::error::TemperamentError;

    #[test]
    fn ambient_elements_form_a_group() {
        let gamma = AmbientLattice::new("umt:edo:12", 1);
        let a = gamma.element([5i64]).unwrap();
        let b = gamma.element([-2i64]).unwrap();
        assert_eq!(a.checked_add(&b).unwrap(), gamma.element([3i64]).unwrap());
        assert_eq!(a.checked_sub(&a).unwrap(), gamma.zero());
        assert_eq!(a.checked_add(&-&a).unwrap(), gamma.zero());
    }

    #[test]
    fn ambient_lattices_of_equal_rank_are_not_interchangeable() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let other = AmbientLattice::new("umt:tempo-grid", 1);
        let a = steps.element([1i64]).unwrap();
        let b = other.element([1i64]).unwrap();
        assert_ne!(a, b);
        assert!(matches!(
            a.checked_add(&b),
            Err(TemperamentError::AmbientMismatch { .. })
        ));
    }

    #[test]
    fn image_coordinates_are_intrinsic() {
        let gamma = AmbientLattice::new("umt:edo:6", 1);
        let doubled =
            Sublattice::from_generators(1, &IntMatrix::from_rows([[2i64]]).unwrap()).unwrap();
        let image = ImageLattice::new(&gamma, doubled).unwrap();

        assert!(!image.is_full());
        assert_eq!(image.rank(), 1);

        // Intrinsic coordinate 3 is ambient step 6.
        let element = image.element([3i64]).unwrap();
        assert_eq!(
            image.embed(&element).unwrap(),
            gamma.element([6i64]).unwrap()
        );
        assert_eq!(
            image.from_ambient(&gamma.element([6i64]).unwrap()).unwrap(),
            element
        );

        // An odd ambient step is not reachable.
        let odd = gamma.element([1i64]).unwrap();
        assert!(!image.contains(&odd).unwrap());
        assert_eq!(
            image.from_ambient(&odd),
            Err(TemperamentError::NotInImage {
                coordinates: alloc::vec![Z::from(1)]
            })
        );
    }

    #[test]
    fn a_full_image_is_recognized() {
        let gamma = AmbientLattice::new("umt:edo:12", 1);
        let image = ImageLattice::new(&gamma, Sublattice::full(1)).unwrap();
        assert!(image.is_full());
        assert!(image.contains(&gamma.element([7i64]).unwrap()).unwrap());
    }

    #[test]
    fn rank_mismatches_are_rejected() {
        let gamma = AmbientLattice::new("umt:rank2", 2);
        assert!(gamma.element([1i64]).is_err());
        assert!(ImageLattice::new(&gamma, Sublattice::full(3)).is_err());
    }
}
