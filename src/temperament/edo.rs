//! Equal divisions and patent vals (UMT-3.2 section 1.6).
//!
//! For an `N`-division model the ambient step lattice is `Gamma_N = Z`, whose
//! element `1` means one step. A patent-val-style mapping sends a monzo to
//! `V_N(a_1, ..., a_k) = sum_i a_i v_i` with
//! `v_i = round(N * log2(nu_3(beta_i)))` under a declared rounding convention.
//!
//! The image is `H_N = gcd(v_1, ..., v_k) Z`, which may be a proper subgroup of
//! `Gamma_N`. Keeping the two apart is the point of fixture F4: the 5-limit
//! patent val for 6-EDO is `[6, 10, 14]`, whose image is `2Z`, so an odd
//! ambient step has no automatic detempering under that mapping.
//!
//! A [`PatentVal`] is a *constructor* for a [`TemperamentMap`] plus the
//! provenance of how its entries were derived. The structural behaviour is the
//! general one; the convenience accessors here are scalar views of it, valid
//! because the ambient rank is 1.

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use num_traits::Zero;

use crate::algebra::integer::round_n_log2;
use crate::algebra::matrix::IntMatrix;
use crate::algebra::{RoundingConvention, Z};
use crate::error::{PatentValError, TemperamentError};
use crate::proportion::GeneratorValuation;
use crate::proportion::basis::Basis;
use crate::proportion::monzo::Monzo;
use crate::temperament::image::AmbientLattice;
use crate::temperament::map::{RawTemperamentMap, TemperamentMap};

/// Whether a derived structural object was computed exactly.
///
/// UMT-3.2 section 0.6.1 forbids floating point from deciding identity,
/// equality, or quotient membership at L0 to L2. A mapping built from an
/// exact rational basis satisfies that unconditionally; one built from a
/// symbolic-real basis cannot, and must say so rather than pass itself off as
/// an exact structural object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Exactness {
    /// Every entry was decided by exact integer arithmetic.
    Exact,
    /// At least one entry was decided from an L3 real valuation.
    RealValued,
}

/// A patent-val-style equal-division mapping `V_N: Lambda_B -> Gamma_N = Z`
/// (UMT-3.2 section 1.6).
///
/// UMT layer: L2 structural map, exact whenever [`PatentVal::exactness`] is
/// [`Exactness::Exact`].
///
/// This is the rank-1 case of [`TemperamentMap`], which it wraps. It is
/// deliberately a distinct type from a tuning: it maps lattice elements to
/// integer step counts, not to real interval sizes.
///
/// Equality is presentation equality over the underlying mapping, the division
/// count, and the declared rounding convention.
///
/// # Examples
///
/// ```
/// use umt::{Basis, PatentVal, RoundingConvention, Z};
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let val = PatentVal::new(&basis, 6, RoundingConvention::NearestHalfAwayFromZero)?;
///
/// assert_eq!(val.entries(), &[Z::from(6), Z::from(10), Z::from(14)]);
///
/// // The ambient step lattice is Z, but the image is 2Z.
/// assert_eq!(val.image_generator(), Z::from(2));
/// assert!(!val.is_surjective());
/// assert!(!val.contains_ambient(&Z::from(1)));
/// assert!(val.contains_ambient(&Z::from(4)));
///
/// // The full structural mapping is available for anything else.
/// assert_eq!(val.map().kernel().rank(), 2);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatentVal {
    map: TemperamentMap,
    divisions: u32,
    convention: RoundingConvention,
    exactness: Exactness,
}

impl PatentVal {
    /// Builds the patent val for `divisions` equal steps of the unit over
    /// `basis`, under a declared rounding convention.
    ///
    /// Each entry is `round(divisions * log2(nu_3(beta_i)))`. For a generator
    /// with an exact rational valuation the entry is computed by exact integer
    /// arithmetic, so it does not depend on floating point; for a
    /// symbolic-real generator the L3 valuation is used and the result is
    /// marked [`Exactness::RealValued`].
    ///
    /// A generator whose exact valuation is 2 therefore always receives the
    /// entry `divisions`, as UMT-3.2 section 1.6 requires, with no special
    /// case in the code.
    ///
    /// `divisions == 0` is permitted and yields the zero mapping, whose image
    /// is the trivial group.
    ///
    /// The ambient lattice is declared as `umt:edo:<divisions>`: two patent
    /// vals for the same division count share one ambient step lattice, even
    /// over different bases, because `Gamma_N = Z` is the same declared
    /// object.
    ///
    /// # Errors
    ///
    /// Returns [`PatentValError::UnusableValuation`] if a generator's
    /// valuation cannot produce an entry, which for a validated basis means an
    /// L3 valuation whose scaled logarithm is not representable.
    pub fn new(
        basis: &Arc<Basis>,
        divisions: u32,
        convention: RoundingConvention,
    ) -> Result<Self, PatentValError> {
        let mut entries = Vec::with_capacity(basis.rank());

        for (index, generator) in basis.generators().iter().enumerate() {
            let entry = match generator.valuation() {
                GeneratorValuation::Rational(value) => round_n_log2(
                    divisions,
                    value.value().numer(),
                    value.value().denom(),
                    convention,
                )
                .ok_or_else(|| PatentValError::UnusableValuation {
                    index,
                    reason: "exact valuation is not strictly positive".into(),
                })?,
                GeneratorValuation::SymbolicReal(value) => {
                    let scaled = f64::from(divisions) * value.log2_f64();
                    let rounded = convention.apply_f64(scaled);
                    if !rounded.is_finite() || libm::fabs(rounded) > 9_007_199_254_740_992.0 {
                        return Err(PatentValError::UnusableValuation {
                            index,
                            reason: "scaled logarithm of the L3 valuation is not representable"
                                .into(),
                        });
                    }
                    Z::from(rounded as i64)
                }
            };
            entries.push(entry);
        }

        let exactness = if basis.is_rational_profile() {
            Exactness::Exact
        } else {
            Exactness::RealValued
        };

        let mut ambient_id = alloc::string::String::from("umt:edo:");
        ambient_id.push_str(&divisions.to_string());
        let ambient = AmbientLattice::new(&ambient_id, 1);
        let matrix = IntMatrix::new(1, entries.len(), entries)
            .map_err(|error| PatentValError::Temperament(error.into()))?;

        let map = TemperamentMap::new(RawTemperamentMap {
            domain: Arc::clone(basis),
            ambient,
            matrix,
        })?;

        Ok(Self {
            map,
            divisions,
            convention,
            exactness,
        })
    }

    /// The underlying structural mapping.
    ///
    /// Everything the general API offers - kernel, image lattice, invariant
    /// factors, intrinsic image coordinates - is reached through here.
    #[must_use]
    pub fn map(&self) -> &TemperamentMap {
        &self.map
    }

    /// Consumes the patent val, returning the structural mapping.
    #[must_use]
    pub fn into_map(self) -> TemperamentMap {
        self.map
    }

    /// The basis this mapping is defined over.
    #[must_use]
    pub fn basis(&self) -> &Arc<Basis> {
        self.map.domain()
    }

    /// The number of equal divisions `N`.
    #[must_use]
    pub fn divisions(&self) -> u32 {
        self.divisions
    }

    /// The mapping row `[v_1, ..., v_k]`.
    #[must_use]
    pub fn entries(&self) -> &[Z] {
        self.map
            .matrix()
            .row(0)
            .expect("invariant: an equal-division mapping has exactly one row")
    }

    /// The rounding convention used to derive the entries.
    ///
    /// The convention is part of the result, not an implementation detail
    /// (UMT-3.2 section 1.6).
    #[must_use]
    pub fn convention(&self) -> RoundingConvention {
        self.convention
    }

    /// Whether the entries were decided exactly.
    #[must_use]
    pub fn exactness(&self) -> Exactness {
        self.exactness
    }

    /// Applies the mapping: `V_N(m) = sum_i a_i v_i`, in ambient steps.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    pub fn apply(&self, monzo: &Monzo) -> Result<Z, TemperamentError> {
        Ok(self.map.apply(monzo)?.coordinates()[0].clone())
    }

    /// The positive generator of the image `H = im(V_N) = g Z`, or zero for
    /// the zero mapping.
    ///
    /// UMT-3.2 section 1.6 fixes the convention `gcd(0, ..., 0) = 0`.
    #[must_use]
    pub fn image_generator(&self) -> Z {
        if self.map.image().rank() == 0 {
            Z::zero()
        } else {
            self.map.image().basis().at(0, 0).clone()
        }
    }

    /// The rank of the image: 1 for any nonzero mapping, 0 for the zero
    /// mapping.
    #[must_use]
    pub fn image_rank(&self) -> usize {
        self.map.image().rank()
    }

    /// Whether the mapping reaches all of the ambient step lattice.
    ///
    /// Surjectivity is a statement about the image. It is unrelated to
    /// saturation of the kernel, which is automatic here because the ambient
    /// group is torsion-free (UMT-3.2 sections 1.4.1 and 1.4.2).
    #[must_use]
    pub fn is_surjective(&self) -> bool {
        self.map.is_surjective()
    }

    /// Whether an ambient step lies in the image.
    ///
    /// For the 6-EDO patent val `[6, 10, 14]` this is false for every odd
    /// step: those steps exist in `Gamma`, but the mapping does not reach
    /// them, so they have no automatic L1 detempering.
    #[must_use]
    pub fn contains_ambient(&self, step: &Z) -> bool {
        let element = self
            .map
            .ambient()
            .element([step.clone()])
            .expect("invariant: the ambient step lattice has rank one");
        self.map
            .image()
            .contains(&element)
            .expect("invariant: the element was built from this ambient lattice")
    }

    /// Converts an ambient step into the intrinsic coordinate of the image
    /// lattice `H = g Z`.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::NotInImage`] if the step is not reachable,
    /// and [`TemperamentError::TrivialImage`] if the mapping is the zero map,
    /// whose image has rank zero and therefore no scalar coordinate.
    pub fn image_coordinate(&self, step: &Z) -> Result<Z, TemperamentError> {
        if self.map.image().rank() == 0 {
            return Err(TemperamentError::TrivialImage);
        }
        let element = self.map.ambient().element([step.clone()])?;
        Ok(self.map.image().from_ambient(&element)?.coordinates()[0].clone())
    }

    /// Embeds an intrinsic image coordinate back into the ambient lattice.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::TrivialImage`] if the mapping is the zero
    /// map.
    pub fn embed_image(&self, coordinate: &Z) -> Result<Z, TemperamentError> {
        if self.map.image().rank() == 0 {
            return Err(TemperamentError::TrivialImage);
        }
        let element = self.map.image().element([coordinate.clone()])?;
        Ok(self.map.image().embed(&element)?.coordinates()[0].clone())
    }
}

impl core::fmt::Display for PatentVal {
    /// Writes standard val notation, for example `<12 19 28]`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<")?;
        for (index, entry) in self.entries().iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            core::fmt::Display::fmt(entry, f)?;
        }
        f.write_str("]")
    }
}

#[cfg(test)]
mod tests {
    use super::{Exactness, PatentVal};
    use crate::algebra::{RoundingConvention, Z};
    use crate::error::TemperamentError;
    use crate::proportion::{Basis, PositiveFinite, RealValuation};
    use alloc::string::ToString;
    use alloc::sync::Arc;
    use alloc::vec;

    const NEAREST: RoundingConvention = RoundingConvention::NearestHalfAwayFromZero;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    #[test]
    fn twelve_edo() {
        let basis = five_limit();
        let val = PatentVal::new(&basis, 12, NEAREST).unwrap();
        assert_eq!(val.to_string(), "<12 19 28]");
        assert_eq!(val.exactness(), Exactness::Exact);
        assert!(val.is_surjective());
        assert_eq!(val.image_generator(), Z::from(1));

        // The syntonic comma is in the kernel of 12-EDO.
        let comma = basis.monzo([-4, 4, -1]).unwrap();
        assert_eq!(val.apply(&comma).unwrap(), Z::from(0));
        assert!(val.map().kernel().contains(&comma).unwrap());
        // The pythagorean comma is too.
        let comma = basis.monzo([-19, 12, 0]).unwrap();
        assert_eq!(val.apply(&comma).unwrap(), Z::from(0));
        assert!(val.map().kernel().contains(&comma).unwrap());
        assert_eq!(val.map().kernel().rank(), 2);
    }

    #[test]
    fn six_edo_image_is_two_z() {
        let basis = five_limit();
        let val = PatentVal::new(&basis, 6, NEAREST).unwrap();
        assert_eq!(val.entries(), &[Z::from(6), Z::from(10), Z::from(14)]);
        assert_eq!(val.image_generator(), Z::from(2));
        assert!(!val.is_surjective());
        assert_eq!(val.image_rank(), 1);

        assert!(!val.contains_ambient(&Z::from(1)));
        assert!(!val.contains_ambient(&Z::from(-3)));
        assert!(val.contains_ambient(&Z::from(0)));
        assert!(val.contains_ambient(&Z::from(6)));

        assert_eq!(
            val.image_coordinate(&Z::from(1)),
            Err(TemperamentError::NotInImage {
                coordinates: vec![Z::from(1)]
            })
        );
        assert_eq!(val.image_coordinate(&Z::from(6)), Ok(Z::from(3)));
        assert_eq!(val.embed_image(&Z::from(3)), Ok(Z::from(6)));
    }

    #[test]
    fn zero_division_mapping_has_trivial_image() {
        let basis = five_limit();
        let val = PatentVal::new(&basis, 0, NEAREST).unwrap();
        assert_eq!(val.entries(), &[Z::from(0), Z::from(0), Z::from(0)]);
        assert_eq!(val.image_generator(), Z::from(0));
        assert_eq!(val.image_rank(), 0);
        assert!(!val.is_surjective());
        assert!(val.contains_ambient(&Z::from(0)));
        assert!(!val.contains_ambient(&Z::from(1)));
        assert_eq!(
            val.image_coordinate(&Z::from(0)),
            Err(TemperamentError::TrivialImage)
        );
        assert_eq!(
            val.embed_image(&Z::from(0)),
            Err(TemperamentError::TrivialImage)
        );
        // Everything is tempered out by the zero mapping.
        assert_eq!(val.map().kernel().rank(), 3);
    }

    #[test]
    fn mapping_is_a_homomorphism() {
        let basis = five_limit();
        let val = PatentVal::new(&basis, 31, NEAREST).unwrap();
        let a = basis.monzo([-1, 1, 0]).unwrap();
        let b = basis.monzo([2, 0, -1]).unwrap();
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(
            val.apply(&sum).unwrap(),
            val.apply(&a).unwrap() + val.apply(&b).unwrap()
        );
    }

    #[test]
    fn foreign_monzos_are_rejected() {
        let val = PatentVal::new(&five_limit(), 12, NEAREST).unwrap();
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        let monzo = other.monzo([1, 0, 0]).unwrap();
        assert!(matches!(
            val.apply(&monzo),
            Err(TemperamentError::BasisMismatch { .. })
        ));
    }

    #[test]
    fn symbolic_real_basis_is_marked_inexact() {
        let basis = Basis::builder("empirical")
            .symbolic_real_generator(
                "octave",
                RealValuation::new(PositiveFinite::new(2.0).unwrap()),
            )
            .symbolic_real_generator(
                "twelfth",
                RealValuation::new(PositiveFinite::new(3.0).unwrap()),
            )
            .build()
            .unwrap();
        let val = PatentVal::new(&basis, 12, NEAREST).unwrap();
        assert_eq!(val.exactness(), Exactness::RealValued);
        // Same entries as the exact path, but the object records that a
        // floating-point valuation decided them.
        assert_eq!(val.entries(), &[Z::from(12), Z::from(19)]);
    }

    #[test]
    fn rounding_convention_is_part_of_the_result() {
        let basis = five_limit();
        let nearest = PatentVal::new(&basis, 5, NEAREST).unwrap();
        let floor = PatentVal::new(&basis, 5, RoundingConvention::Floor).unwrap();
        assert_eq!(nearest.entries(), &[Z::from(5), Z::from(8), Z::from(12)]);
        assert_eq!(floor.entries(), &[Z::from(5), Z::from(7), Z::from(11)]);
        assert_ne!(nearest, floor);
        // Different rounding gives a different structural mapping, hence a
        // different kernel.
        assert_ne!(nearest.map().kernel(), floor.map().kernel());
    }

    #[test]
    fn the_ambient_step_lattice_is_shared_across_bases() {
        let five = PatentVal::new(&five_limit(), 12, NEAREST).unwrap();
        let seven = PatentVal::new(
            &Basis::primes("umt:prime:2.3.5.7", &[2, 3, 5, 7]).unwrap(),
            12,
            NEAREST,
        )
        .unwrap();
        assert_eq!(five.map().ambient(), seven.map().ambient());
    }
}
