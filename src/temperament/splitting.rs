//! Homomorphic splittings (UMT-3.2 section 1.7.1).
//!
//! Because `H` is free abelian, the exact sequence
//! `0 -> K -> Lambda_B -> H -> 0` splits, so there exists a group homomorphism
//! `s: H -> Lambda_B` with `V . s = id_H`, and then
//! `Lambda_B ~ H (+) K` once `s` is chosen.
//!
//! A splitting is *not* the same thing as a representative policy. Every
//! splitting is a right inverse, but most musically interesting right inverses
//! are not homomorphisms: minimum-complexity spelling, context-sensitive
//! detempering, and adaptive lift selection all fail additivity. The reverse
//! conversion is therefore not generally valid, and the two live in separate
//! modules with separate traits (prompt section 14).

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::matrix::IntMatrix;
use crate::error::TemperamentError;
use crate::proportion::monzo::Monzo;
use crate::temperament::image::ImageElem;
use crate::temperament::map::TemperamentMap;

/// A group homomorphism `s: H -> Lambda_B` with `V . s = id_H`.
///
/// UMT layer: L2 structural map, exact.
///
/// Implementors must satisfy both the right-inverse law and additivity;
/// UMT-3.2 law P11 requires additivity to be tested whenever it is claimed,
/// and implementing this trait is such a claim.
pub trait HomomorphicSplit {
    /// The mapping this splits.
    fn map(&self) -> &TemperamentMap;

    /// Chooses the lift of a tempered class.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::ImageMismatch`] if the class belongs to a
    /// different image lattice.
    fn split(&self, class: &ImageElem) -> Result<Monzo, TemperamentError>;
}

/// The splitting derived from the Smith normal form of a mapping.
///
/// UMT layer: L2 structural map, exact.
///
/// Built by solving `V(p_j) = b_j` for each canonical basis vector `b_j` of
/// the image and extending linearly, which is a homomorphism by construction.
///
/// The particular splitting depends on the Smith transformation matrices,
/// which are not canonical, so this is a deterministic choice rather than a
/// canonical one: it is stable for a given mapping and version of this crate,
/// and it is not a musical spelling policy. Anything that must survive a
/// round trip should record the resolved lifts or a named policy rather than
/// assume this construction (UMT-3.2 section 8.8).
///
/// # Examples
///
/// ```
/// use umt::temperament::{AmbientLattice, HomomorphicSplit, LinearSplit, TemperamentMap};
/// use umt::Basis;
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let steps = AmbientLattice::new("umt:edo:12", 1);
/// let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;
/// let split = LinearSplit::of(&map)?;
///
/// // Splitting a class and mapping back is the identity on H.
/// let class = map.apply_to_image(&basis.monzo([-1, 1, 0])?)?;
/// assert_eq!(map.apply_to_image(&split.split(&class)?)?, class);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearSplit {
    map: TemperamentMap,
    section: IntMatrix,
}

impl LinearSplit {
    /// Derives a splitting of `map`.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::NotARightInverse`] if the derived section
    /// fails its own verification, which would indicate a defect in this
    /// crate rather than bad input.
    pub fn of(map: &TemperamentMap) -> Result<Self, TemperamentError> {
        let rank = map.image().rank();
        let domain_rank = map.domain().rank();

        let mut columns: Vec<Vec<Z>> = Vec::with_capacity(rank);
        for index in 0..rank {
            let basis_vector = map
                .image()
                .basis()
                .column(index)
                .expect("invariant: image basis column index is in range");
            let target = map.ambient().element(basis_vector)?;
            columns.push(map.preimage(&target)?.exponents().to_vec());
        }

        let mut data = Vec::with_capacity(domain_rank * rank);
        for row in 0..domain_rank {
            for column in &columns {
                data.push(column[row].clone());
            }
        }
        let section = IntMatrix::new(domain_rank, rank, data)?;

        // `V S` must be the image basis; otherwise the section is not a right
        // inverse and nothing built on it would be trustworthy.
        if map.matrix().multiply(&section)? != *map.image().basis() {
            return Err(TemperamentError::NotARightInverse);
        }

        Ok(Self {
            map: map.clone(),
            section,
        })
    }

    /// The section matrix, with one column per image basis vector.
    #[must_use]
    pub fn section(&self) -> &IntMatrix {
        &self.section
    }
}

impl HomomorphicSplit for LinearSplit {
    fn map(&self) -> &TemperamentMap {
        &self.map
    }

    fn split(&self, class: &ImageElem) -> Result<Monzo, TemperamentError> {
        if !Arc::ptr_eq(class.lattice(), self.map.image())
            && **class.lattice() != **self.map.image()
        {
            return Err(TemperamentError::ImageMismatch);
        }
        let exponents = self.section.apply(class.coordinates())?;
        Ok(Monzo::new(Arc::clone(self.map.domain()), exponents)
            .expect("invariant: the section has one row per basis generator"))
    }
}

#[cfg(test)]
mod tests {
    use super::{HomomorphicSplit, LinearSplit};
    use crate::proportion::Basis;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    #[test]
    fn splitting_is_a_right_inverse() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        for coordinate in [-9i64, -1, 0, 1, 7, 19] {
            let class = map.image().element([coordinate]).unwrap();
            let lift = split.split(&class).unwrap();
            assert_eq!(map.apply_to_image(&lift).unwrap(), class);
        }
    }

    #[test]
    fn splitting_is_additive() {
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:31", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[31i64, 49, 72]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        let a = map.image().element([5i64]).unwrap();
        let b = map.image().element([-13i64]).unwrap();
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(
            split.split(&sum).unwrap(),
            split
                .split(&a)
                .unwrap()
                .checked_add(&split.split(&b).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn splitting_works_when_the_image_is_proper() {
        // 6-EDO: H = 2Z, so the section is defined on image coordinates, not
        // on ambient steps.
        let basis = five_limit();
        let steps = AmbientLattice::new("umt:edo:6", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[6i64, 10, 14]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        let class = map.image().element([3i64]).unwrap();
        let lift = split.split(&class).unwrap();
        // Intrinsic coordinate 3 is ambient step 6.
        assert_eq!(map.apply(&lift).unwrap(), steps.element([6i64]).unwrap());
    }

    #[test]
    fn a_rank_two_mapping_splits() {
        let basis = five_limit();
        let ambient = AmbientLattice::new("umt:meantone-coords", 2);
        let map = TemperamentMap::from_rows(&basis, &ambient, [[1i64, 0, -4], [0, 1, 4]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        for coordinates in [[0i64, 0], [1, 0], [0, 1], [-3, 5]] {
            let class = map.image().element(coordinates).unwrap();
            assert_eq!(
                map.apply_to_image(&split.split(&class).unwrap()).unwrap(),
                class
            );
        }
    }

    #[test]
    fn the_zero_mapping_splits_trivially() {
        let basis = five_limit();
        let ambient = AmbientLattice::new("umt:zero", 1);
        let map = TemperamentMap::from_rows(&basis, &ambient, [[0i64, 0, 0]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        // The image is trivial, so the only class is the empty coordinate
        // vector and its lift is the identity monzo.
        let class = map.image().element(Vec::<i64>::new()).unwrap();
        assert_eq!(split.split(&class).unwrap(), basis.zero());
    }

    #[test]
    fn foreign_classes_are_rejected() {
        let basis = five_limit();
        let twelve = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &twelve, [[12i64, 19, 28]]).unwrap();
        let split = LinearSplit::of(&map).unwrap();

        let other = TemperamentMap::from_rows(
            &basis,
            &AmbientLattice::new("umt:edo:6", 1),
            [[6i64, 10, 14]],
        )
        .unwrap();
        let foreign = other.image().element([1i64]).unwrap();
        assert!(split.split(&foreign).is_err());
    }
}
