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

use alloc::sync::Arc;
use alloc::vec::Vec;

use num_integer::Integer;
use num_traits::Zero;

use crate::algebra::integer::round_n_log2;
use crate::algebra::{RoundingConvention, Z};
use crate::error::PatentValError;
use crate::proportion::GeneratorValuation;
use crate::proportion::basis::Basis;
use crate::proportion::monzo::{Monzo, compatible};

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
/// This is the rank-1 special case of a regular temperament mapping. It is
/// deliberately a distinct type from a tuning: it maps lattice elements to
/// integer step counts, not to real interval sizes.
///
/// Equality is presentation equality over the basis, the division count, the
/// entries, and the declared rounding convention.
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
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatentVal {
    basis: Arc<Basis>,
    divisions: u32,
    entries: Vec<Z>,
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

        Ok(Self {
            basis: Arc::clone(basis),
            divisions,
            entries,
            convention,
            exactness,
        })
    }

    /// The basis this mapping is defined over.
    #[must_use]
    pub fn basis(&self) -> &Arc<Basis> {
        &self.basis
    }

    /// The number of equal divisions `N`.
    #[must_use]
    pub fn divisions(&self) -> u32 {
        self.divisions
    }

    /// The mapping row `[v_1, ..., v_k]`.
    #[must_use]
    pub fn entries(&self) -> &[Z] {
        &self.entries
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

    /// Applies the mapping: `V_N(m) = sum_i a_i v_i`.
    ///
    /// The result is an ambient coordinate in `Gamma_N = Z`, counting steps.
    ///
    /// # Errors
    ///
    /// Returns [`PatentValError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    pub fn apply(&self, monzo: &Monzo) -> Result<Z, PatentValError> {
        if !compatible(monzo.basis(), &self.basis) {
            return Err(PatentValError::BasisMismatch {
                expected: self.basis.id().clone(),
                found: monzo.basis().id().clone(),
            });
        }
        Ok(monzo
            .exponents()
            .iter()
            .zip(&self.entries)
            .map(|(a, v)| a * v)
            .sum())
    }

    /// The positive generator of the image `H = im(V_N) = g Z`, or zero for
    /// the zero mapping.
    ///
    /// UMT-3.2 section 1.6 fixes the convention `gcd(0, ..., 0) = 0`.
    #[must_use]
    pub fn image_generator(&self) -> Z {
        self.entries
            .iter()
            .fold(Z::zero(), |accumulator, entry| accumulator.gcd(entry))
    }

    /// The rank of the image: 1 for any nonzero mapping, 0 for the zero
    /// mapping.
    #[must_use]
    pub fn image_rank(&self) -> usize {
        usize::from(!self.image_generator().is_zero())
    }

    /// Whether the mapping reaches all of the ambient step lattice.
    ///
    /// Surjectivity is a statement about the image. It is unrelated to
    /// saturation of the kernel, which is automatic here because the ambient
    /// group is torsion-free (UMT-3.2 sections 1.4.1 and 1.4.2).
    #[must_use]
    pub fn is_surjective(&self) -> bool {
        self.image_generator() == Z::from(1)
    }

    /// Whether an ambient step lies in the image.
    ///
    /// For the 6-EDO patent val `[6, 10, 14]` this is false for every odd
    /// step: those steps exist in `Gamma`, but the mapping does not reach
    /// them, so they have no automatic L1 detempering.
    #[must_use]
    pub fn contains_ambient(&self, step: &Z) -> bool {
        let generator = self.image_generator();
        if generator.is_zero() {
            step.is_zero()
        } else {
            (step % generator).is_zero()
        }
    }

    /// Converts an ambient coordinate into the intrinsic coordinate of the
    /// image lattice `H = g Z`.
    ///
    /// This is the rank-1 form of the ambient-to-image conversion required by
    /// prompt section 11: image elements are expressed in their own
    /// coordinates rather than being confused with ambient ones.
    ///
    /// # Errors
    ///
    /// Returns [`PatentValError::NotInImage`] if the step is not reachable,
    /// and [`PatentValError::TrivialImage`] if the mapping is the zero map, so
    /// its image has rank zero and no integer coordinate.
    pub fn image_coordinate(&self, step: &Z) -> Result<Z, PatentValError> {
        let generator = self.image_generator();
        if generator.is_zero() {
            return Err(PatentValError::TrivialImage);
        }
        if !(step % &generator).is_zero() {
            return Err(PatentValError::NotInImage { step: step.clone() });
        }
        Ok(step / generator)
    }

    /// Embeds an intrinsic image coordinate back into the ambient lattice.
    ///
    /// # Errors
    ///
    /// Returns [`PatentValError::TrivialImage`] if the mapping is the zero
    /// map.
    pub fn embed_image(&self, coordinate: &Z) -> Result<Z, PatentValError> {
        let generator = self.image_generator();
        if generator.is_zero() {
            return Err(PatentValError::TrivialImage);
        }
        Ok(coordinate * generator)
    }
}

impl core::fmt::Display for PatentVal {
    /// Writes standard val notation, for example `<12 19 28]`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<")?;
        for (index, entry) in self.entries.iter().enumerate() {
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
    use crate::error::PatentValError;
    use crate::proportion::{Basis, PositiveFinite, RealValuation};
    use alloc::string::ToString;
    use alloc::sync::Arc;

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
        // The pythagorean comma is too.
        let comma = basis.monzo([-19, 12, 0]).unwrap();
        assert_eq!(val.apply(&comma).unwrap(), Z::from(0));
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
            Err(PatentValError::NotInImage { step: Z::from(1) })
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
            Err(PatentValError::TrivialImage)
        );
        assert_eq!(
            val.embed_image(&Z::from(0)),
            Err(PatentValError::TrivialImage)
        );
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
            Err(PatentValError::BasisMismatch { .. })
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
    }
}
