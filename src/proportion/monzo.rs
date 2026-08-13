//! Monzos: elements of the exact proportion lattice (UMT-3.2 section 1.1).

use alloc::sync::Arc;
use alloc::vec::Vec;

use num_traits::{Signed, ToPrimitive, Zero};

use crate::algebra::integer::{log2_ratio_f64, ratio_to_f64};
use crate::algebra::{Q, Z};
use crate::error::{MonzoError, ValuationError};
use crate::proportion::basis::Basis;

/// An element `m = (a_1, ..., a_k)` of `Lambda_B` (UMT-3.2 section 1.1).
///
/// UMT layer: L1, exact. Group addition corresponds to multiplication of the
/// represented proportions.
///
/// A monzo carries a handle to its basis, because an exponent vector alone is
/// not a semantic object: `[1, 0, 0]` over `(2, 3, 5)` and `[1, 0, 0]` over
/// `(2, 3, 7)` are different (prompt section 7). Arithmetic across unrelated
/// bases is therefore rejected rather than silently performed, and no
/// unguarded `impl Add` exists - see [`Monzo::checked_add`].
///
/// Equality is presentation equality: equal basis (by
/// [`Basis::same_identity`]) and equal exponents. Monzos over different bases
/// are unequal rather than incomparable.
#[derive(Debug, Clone)]
pub struct Monzo {
    basis: Arc<Basis>,
    exponents: Vec<Z>,
}

impl Monzo {
    /// Builds a monzo from an exponent vector.
    ///
    /// # Errors
    ///
    /// Returns [`MonzoError::RankMismatch`] if the vector length differs from
    /// the basis rank.
    pub fn new(basis: Arc<Basis>, exponents: Vec<Z>) -> Result<Self, MonzoError> {
        if exponents.len() != basis.rank() {
            return Err(MonzoError::RankMismatch {
                expected: basis.rank(),
                found: exponents.len(),
            });
        }
        Ok(Self { basis, exponents })
    }

    /// The identity element, representing the proportion 1.
    #[must_use]
    pub fn zero(basis: Arc<Basis>) -> Self {
        let exponents = (0..basis.rank()).map(|_| Z::zero()).collect();
        Self { basis, exponents }
    }

    /// The basis this monzo is expressed over.
    #[must_use]
    pub fn basis(&self) -> &Arc<Basis> {
        &self.basis
    }

    /// The exact exponent vector.
    #[must_use]
    pub fn exponents(&self) -> &[Z] {
        &self.exponents
    }

    /// The rank of the underlying lattice.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.exponents.len()
    }

    /// Whether this is the identity element.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.exponents.iter().all(Zero::is_zero)
    }

    /// Whether two monzos may be combined.
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        compatible(&self.basis, &other.basis)
    }

    /// Lattice addition, corresponding to multiplication of proportions.
    ///
    /// # Errors
    ///
    /// Returns [`MonzoError::BasisMismatch`] if the operands are over
    /// different bases.
    pub fn checked_add(&self, other: &Self) -> Result<Self, MonzoError> {
        self.combine(other, false)
    }

    /// Lattice subtraction, corresponding to division of proportions.
    ///
    /// # Errors
    ///
    /// Returns [`MonzoError::BasisMismatch`] if the operands are over
    /// different bases.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, MonzoError> {
        self.combine(other, true)
    }

    fn combine(&self, other: &Self, subtract: bool) -> Result<Self, MonzoError> {
        if !self.is_compatible_with(other) {
            return Err(MonzoError::BasisMismatch {
                left: self.basis.id().clone(),
                right: other.basis.id().clone(),
            });
        }
        let exponents = self
            .exponents
            .iter()
            .zip(&other.exponents)
            .map(|(a, b)| if subtract { a - b } else { a + b })
            .collect();
        Ok(Self {
            basis: Arc::clone(&self.basis),
            exponents,
        })
    }

    /// Integer scaling, corresponding to raising the proportion to a power.
    ///
    /// Always well defined: no basis mismatch is possible.
    #[must_use]
    pub fn scale(&self, factor: &Z) -> Self {
        Self {
            basis: Arc::clone(&self.basis),
            exponents: self.exponents.iter().map(|a| a * factor).collect(),
        }
    }

    /// The exact ratio `r(m) = prod nu(beta_i)^{a_i}` of section 1.1.1.
    ///
    /// UMT layer: L1, exact.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::NotRationalProfile`] if any generator has
    /// only a symbolic-real valuation, and
    /// [`ValuationError::ExponentOutOfRange`] if an exponent is too large to
    /// materialize as a power. The lattice arithmetic itself has no such
    /// bound; only evaluating the ratio does.
    pub fn exact_ratio(&self) -> Result<Q, ValuationError> {
        let mut numer = Z::from(1);
        let mut denom = Z::from(1);

        for (index, (generator, exponent)) in self
            .basis
            .generators()
            .iter()
            .zip(&self.exponents)
            .enumerate()
        {
            let value = generator
                .valuation()
                .as_rational()
                .ok_or(ValuationError::NotRationalProfile { index })?;
            if exponent.is_zero() {
                continue;
            }
            let magnitude = exponent
                .abs()
                .to_u32()
                .ok_or(ValuationError::ExponentOutOfRange { index })?;
            let (up, down) = (value.value().numer(), value.value().denom());
            if exponent.is_positive() {
                numer *= up.pow(magnitude);
                denom *= down.pow(magnitude);
            } else {
                numer *= down.pow(magnitude);
                denom *= up.pow(magnitude);
            }
        }

        Ok(Q::new(numer, denom))
    }

    /// L3 real approximation of `log2` of the represented proportion.
    ///
    /// UMT layer: L3. In the rational profile the exact ratio is formed first
    /// and its logarithm taken once, so small commas keep full relative
    /// precision instead of cancelling. In a symbolic-real or mixed profile the
    /// weighted sum of generator logarithms is used.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::ExponentOutOfRange`] if the rational path
    /// cannot materialize the ratio.
    pub fn log2_valuation_f64(&self) -> Result<f64, ValuationError> {
        if self.basis.is_rational_profile() {
            let ratio = self.exact_ratio()?;
            return Ok(log2_ratio_f64(ratio.numer(), ratio.denom())
                .expect("invariant: an exact ratio of positive valuations is positive"));
        }
        let mut total = 0.0f64;
        for (generator, exponent) in self.basis.generators().iter().zip(&self.exponents) {
            let coefficient = ratio_to_f64(exponent, &Z::from(1))
                .expect("invariant: denominator one is non-zero");
            total += coefficient * generator.valuation().log2_f64();
        }
        Ok(total)
    }
}

/// Whether two basis handles denote the same declared basis.
///
/// The pointer check is a fast path for the common case of a shared handle;
/// the structural check is what makes the guarantee sound across handles
/// rebuilt from serialized data.
pub(crate) fn compatible(left: &Arc<Basis>, right: &Arc<Basis>) -> bool {
    Arc::ptr_eq(left, right) || left.same_identity(right)
}

impl PartialEq for Monzo {
    fn eq(&self, other: &Self) -> bool {
        self.exponents == other.exponents && self.is_compatible_with(other)
    }
}

impl Eq for Monzo {}

impl core::hash::Hash for Monzo {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Consistent with `PartialEq`: equal monzos agree on both components,
        // and hashing only the basis identity keeps the contract when two
        // bases share an identifier but differ in content.
        self.basis.id().hash(state);
        self.exponents.hash(state);
    }
}

impl core::ops::Neg for &Monzo {
    type Output = Monzo;

    fn neg(self) -> Monzo {
        Monzo {
            basis: Arc::clone(&self.basis),
            exponents: self.exponents.iter().map(core::ops::Neg::neg).collect(),
        }
    }
}

impl core::ops::Neg for Monzo {
    type Output = Monzo;

    fn neg(self) -> Monzo {
        -&self
    }
}

impl core::fmt::Display for Monzo {
    /// Writes standard monzo notation, for example `[-4 4 -1>`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("[")?;
        for (index, exponent) in self.exponents.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            core::fmt::Display::fmt(exponent, f)?;
        }
        f.write_str(">")
    }
}

#[cfg(test)]
mod tests {
    use super::Monzo;
    use crate::algebra::Z;
    use crate::error::MonzoError;
    use crate::proportion::Basis;
    use alloc::string::ToString;

    #[test]
    fn addition_is_multiplication_of_ratios() {
        let basis = Basis::primes("p", &[2, 3, 5]).unwrap();
        let fifth = basis.monzo([-1, 1, 0]).unwrap();
        let major_third = basis.monzo([-2, 0, 1]).unwrap();
        let sum = fifth.checked_add(&major_third).unwrap();
        assert_eq!(sum.exact_ratio().unwrap().to_string(), "15/8");
    }

    #[test]
    fn cross_basis_arithmetic_is_rejected() {
        let five_limit = Basis::primes("p5", &[2, 3, 5]).unwrap();
        let seven_limit = Basis::primes("p7", &[2, 3, 7]).unwrap();
        let a = five_limit.monzo([1, 0, 0]).unwrap();
        let b = seven_limit.monzo([1, 0, 0]).unwrap();
        assert!(!a.is_compatible_with(&b));
        assert!(matches!(
            a.checked_add(&b),
            Err(MonzoError::BasisMismatch { .. })
        ));
        assert_ne!(a, b);
    }

    #[test]
    fn rank_mismatch_is_rejected() {
        let basis = Basis::primes("p", &[2, 3, 5]).unwrap();
        assert!(matches!(
            basis.monzo([1, 0]),
            Err(MonzoError::RankMismatch {
                expected: 3,
                found: 2
            })
        ));
    }

    #[test]
    fn negation_and_scaling_are_always_defined() {
        let basis = Basis::primes("p", &[2, 3, 5]).unwrap();
        let comma = basis.monzo([-4, 4, -1]).unwrap();
        assert_eq!(comma.checked_add(&-&comma).unwrap(), basis.zero());
        assert_eq!(
            comma.scale(&Z::from(2)).exact_ratio().unwrap().to_string(),
            "6561/6400"
        );
    }

    #[test]
    fn display_uses_monzo_notation() {
        let basis = Basis::primes("p", &[2, 3, 5]).unwrap();
        assert_eq!(basis.monzo([-4, 4, -1]).unwrap().to_string(), "[-4 4 -1>");
        assert_eq!(Monzo::zero(basis).to_string(), "[0 0 0>");
    }

    #[test]
    fn log2_of_a_comma_keeps_precision() {
        let basis = Basis::primes("p", &[2, 3, 5]).unwrap();
        let comma = basis.monzo([-4, 4, -1]).unwrap();
        let cents = comma.log2_valuation_f64().unwrap() * 1200.0;
        assert!((cents - 21.506_289_596_7).abs() < 1e-9, "{cents}");
    }
}
