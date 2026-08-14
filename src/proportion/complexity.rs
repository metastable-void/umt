//! Complexity functions (UMT-3.2 sections 1.3 and 9.2).
//!
//! A complexity function is a declared map `h: Lambda_B -> R_{>=0}`. UMT-3.2
//! does not require every one to be a norm, and it deliberately does not use
//! the bare word *norm*, because the term means different things for groups
//! and for vector spaces. Instead every complexity declares which laws it
//! satisfies, and the declaration is part of its type:
//!
//! - `group_length` - `h(0) = 0`, `h(-m) = h(m)`, subadditivity. May have
//!   nonzero elements of zero length unless it also claims separation.
//! - `lattice_seminorm` - all of the above plus integer homogeneity
//!   `h(nm) = |n| h(m)`, so its null set is a subgroup.
//! - `lattice_norm` - all of the above plus `h(m) = 0` only for `m = 0`.
//! - `cost` - nonnegative, nothing else implied.
//!
//! Arithmetic complexity is a structural or heuristic quantity. Section 1.3.4
//! is explicit that it MUST NOT be identified with sensory dissonance, and
//! nothing here does.

use alloc::sync::Arc;
use alloc::vec::Vec;

use num_traits::{Signed, ToPrimitive, Zero};

use crate::algebra::integer::ratio_to_f64;
use crate::algebra::rational::log2_q_f64;
use crate::algebra::{Q, Z};
use crate::error::ComplexityError;
use crate::proportion::basis::{Basis, IndependenceContract};
use crate::proportion::monzo::{Monzo, compatible};
use crate::proportion::valuation::PositiveFinite;

/// Which laws a complexity function claims (UMT-3.2 section 9.2).
///
/// The claim is checkable, and the conformance suite checks it: advertising a
/// profile whose laws do not hold is the failure mode fixture F34 is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ComplexityProfile {
    /// `h(0) = 0`, symmetry, and subadditivity. No homogeneity claimed.
    GroupLength {
        /// Whether `h(m) = 0` implies `m = 0`.
        separating: bool,
    },
    /// Group-length laws plus integer homogeneity.
    LatticeSeminorm,
    /// Seminorm laws plus identity of indiscernibles.
    LatticeNorm,
    /// Nonnegativity only.
    Cost,
}

impl ComplexityProfile {
    /// Whether this profile claims integer homogeneity `h(nm) = |n| h(m)`.
    #[must_use]
    pub fn claims_homogeneity(self) -> bool {
        matches!(self, Self::LatticeSeminorm | Self::LatticeNorm)
    }

    /// Whether this profile claims that only zero has zero complexity.
    #[must_use]
    pub fn claims_separation(self) -> bool {
        match self {
            Self::LatticeNorm => true,
            Self::GroupLength { separating } => separating,
            Self::LatticeSeminorm | Self::Cost => false,
        }
    }

    /// Whether this profile claims subadditivity `h(m + n) <= h(m) + h(n)`.
    #[must_use]
    pub fn claims_subadditivity(self) -> bool {
        !matches!(self, Self::Cost)
    }
}

/// The value of a complexity function, exact where the function is exact.
///
/// UMT layer: L1 when [`ComplexityValue::Exact`], L3 when
/// [`ComplexityValue::Real`]. The distinction is preserved rather than
/// flattened to `f64`, so an exact comparison stays exact and a real one is
/// visibly real.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ComplexityValue {
    /// An exact rational value.
    Exact(Q),
    /// An L3 real observation.
    Real(f64),
}

impl ComplexityValue {
    /// The value as a real number.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Exact(value) => ratio_to_f64(value.numer(), value.denom())
                .expect("invariant: an exact value has a nonzero denominator"),
            Self::Real(value) => *value,
        }
    }

    /// The exact value, if this is one.
    #[must_use]
    pub fn exact(&self) -> Option<&Q> {
        match self {
            Self::Exact(value) => Some(value),
            Self::Real(_) => None,
        }
    }
}

impl PartialEq for ComplexityValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => left == right,
            _ => self.as_f64() == other.as_f64(),
        }
    }
}

impl PartialOrd for ComplexityValue {
    /// Exact values compare exactly; anything involving a real value compares
    /// as reals, which is the honest weaker answer.
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => Some(left.cmp(right)),
            _ => self.as_f64().partial_cmp(&other.as_f64()),
        }
    }
}

/// A declared complexity function on a proportion lattice.
///
/// UMT layer: L3 for the value, L1 for the structure it reads. The value is a
/// real observation; the exponents it is computed from are exact.
pub trait Complexity {
    /// The basis this complexity is defined over.
    fn basis(&self) -> &Arc<Basis>;

    /// The laws this complexity claims to satisfy.
    fn profile(&self) -> ComplexityProfile;

    /// The real value.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    fn value_f64(&self, monzo: &Monzo) -> Result<f64, ComplexityError>;

    /// The exact value, where this complexity has one.
    ///
    /// A logarithmically weighted complexity such as Tenney height does not:
    /// its value is an L3 real observation, and saying so is the point.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    fn exact_value(&self, monzo: &Monzo) -> Result<Option<Q>, ComplexityError> {
        let _ = self.value_f64(monzo)?;
        Ok(None)
    }

    /// The value, exact where this complexity has an exact one.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    fn value(&self, monzo: &Monzo) -> Result<ComplexityValue, ComplexityError> {
        match self.exact_value(monzo)? {
            Some(exact) => Ok(ComplexityValue::Exact(exact)),
            None => Ok(ComplexityValue::Real(self.value_f64(monzo)?)),
        }
    }
}

/// A complexity that weights each basis coordinate independently.
///
/// Knowing the per-coordinate weights is what makes an exhaustive
/// minimum-complexity search provable: from `h(m) >= w_i |a_i|` a bounded
/// search region can be derived, rather than guessed at.
pub trait CoordinateWeighted: Complexity {
    /// A positive lower bound on the weight of coordinate `index`.
    ///
    /// Returns `None` when the index is out of range or the weight is zero,
    /// in which case that coordinate is unbounded by this complexity and no
    /// finite search region exists along it.
    fn coordinate_weight_f64(&self, index: usize) -> Option<f64>;
}

fn check_basis(basis: &Arc<Basis>, monzo: &Monzo) -> Result<(), ComplexityError> {
    if compatible(monzo.basis(), basis) {
        Ok(())
    } else {
        Err(ComplexityError::BasisMismatch {
            expected: basis.id().clone(),
            found: monzo.basis().id().clone(),
        })
    }
}

/// A weighted `l1` complexity with exact rational weights (UMT-3.2 section
/// 1.3.1).
///
/// `h(m) = sum_i w_i |a_i|`.
///
/// With every weight strictly positive this is a `lattice_norm`. With some
/// weight zero it is a `lattice_seminorm` whose null subgroup is spanned by
/// the corresponding generators - which is how section 1.3.3 models
/// octave-equivalent complexity, by giving the octave generator weight zero.
///
/// Weights are exact, so the value is exact.
///
/// # Examples
///
/// ```
/// use umt::proportion::{Complexity, ComplexityProfile, WeightedL1};
/// use umt::{Basis, Q, Z};
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let one = || Q::new(Z::from(1), Z::from(1));
/// let norm = WeightedL1::new(&basis, [one(), one(), one()])?;
///
/// assert_eq!(norm.profile(), ComplexityProfile::LatticeNorm);
/// assert_eq!(
///     norm.exact_value(&basis.monzo([-4, 4, -1])?)?.unwrap(),
///     Q::new(Z::from(9), Z::from(1))
/// );
///
/// // Weight zero on the octave makes it octave-equivalent, and a seminorm.
/// let zero = Q::new(Z::from(0), Z::from(1));
/// let seminorm = WeightedL1::new(&basis, [zero, one(), one()])?;
/// assert_eq!(seminorm.profile(), ComplexityProfile::LatticeSeminorm);
/// assert_eq!(
///     seminorm.exact_value(&basis.monzo([3, 0, 0])?)?.unwrap(),
///     Q::new(Z::from(0), Z::from(1))
/// );
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedL1 {
    basis: Arc<Basis>,
    weights: Vec<Q>,
    profile: ComplexityProfile,
}

impl WeightedL1 {
    /// Builds a weighted complexity from exact nonnegative weights.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::WeightCount`] if the number of weights
    /// differs from the basis rank, and [`ComplexityError::NegativeWeight`]
    /// if any weight is negative. Negative weights are rejected because they
    /// would break both nonnegativity and the triangle inequality.
    pub fn new<I>(basis: &Arc<Basis>, weights: I) -> Result<Self, ComplexityError>
    where
        I: IntoIterator<Item = Q>,
    {
        let weights: Vec<Q> = weights.into_iter().collect();
        if weights.len() != basis.rank() {
            return Err(ComplexityError::WeightCount {
                expected: basis.rank(),
                found: weights.len(),
            });
        }
        for (index, weight) in weights.iter().enumerate() {
            if weight.is_negative() {
                return Err(ComplexityError::NegativeWeight { index });
            }
        }
        let profile = if weights.iter().all(|weight| weight.is_positive()) {
            ComplexityProfile::LatticeNorm
        } else {
            ComplexityProfile::LatticeSeminorm
        };
        Ok(Self {
            basis: Arc::clone(basis),
            weights,
            profile,
        })
    }

    /// All weights equal to one: the plain `l1` complexity.
    #[must_use]
    pub fn uniform(basis: &Arc<Basis>) -> Self {
        let weights = (0..basis.rank()).map(|_| Q::new(Z::from(1), Z::from(1)));
        Self::new(basis, weights).expect("invariant: one weight per generator, all positive")
    }

    /// The exact weights.
    #[must_use]
    pub fn weights(&self) -> &[Q] {
        &self.weights
    }
}

impl Complexity for WeightedL1 {
    fn basis(&self) -> &Arc<Basis> {
        &self.basis
    }

    fn profile(&self) -> ComplexityProfile {
        self.profile
    }

    fn value_f64(&self, monzo: &Monzo) -> Result<f64, ComplexityError> {
        let exact = self
            .exact_value(monzo)?
            .expect("invariant: a weighted l1 complexity is always exact");
        Ok(ratio_to_f64(exact.numer(), exact.denom())
            .expect("invariant: an exact value has a nonzero denominator"))
    }

    fn exact_value(&self, monzo: &Monzo) -> Result<Option<Q>, ComplexityError> {
        check_basis(&self.basis, monzo)?;
        let mut total = Q::new(Z::from(0), Z::from(1));
        for (weight, exponent) in self.weights.iter().zip(monzo.exponents()) {
            if exponent.is_zero() || weight.is_zero() {
                continue;
            }
            total += weight * Q::new(exponent.abs(), Z::from(1));
        }
        Ok(Some(total))
    }
}

impl CoordinateWeighted for WeightedL1 {
    fn coordinate_weight_f64(&self, index: usize) -> Option<f64> {
        let weight = self.weights.get(index)?;
        if weight.is_positive() {
            ratio_to_f64(weight.numer(), weight.denom())
        } else {
            None
        }
    }
}

/// A weighted `l1` complexity with real weights, including Tenney height
/// (UMT-3.2 section 1.3.2).
///
/// `h(m) = sum_i w_i |a_i|` with `w_i > 0` real.
///
/// The value is an L3 real observation, so there is no exact value. Weights
/// must be supplied or derived explicitly, and every derivation validates
/// positivity: a generator whose valuation is below 1 has a *negative*
/// logarithm, and using that as a norm weight would silently produce a
/// function that is not a norm at all. That trap is fixture F5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogWeightedL1 {
    basis: Arc<Basis>,
    weights: Vec<PositiveFinite>,
}

impl LogWeightedL1 {
    /// Builds a real-weighted complexity from explicit positive weights.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::WeightCount`] if the number of weights
    /// differs from the basis rank.
    pub fn new<I>(basis: &Arc<Basis>, weights: I) -> Result<Self, ComplexityError>
    where
        I: IntoIterator<Item = PositiveFinite>,
    {
        let weights: Vec<PositiveFinite> = weights.into_iter().collect();
        if weights.len() != basis.rank() {
            return Err(ComplexityError::WeightCount {
                expected: basis.rank(),
                found: weights.len(),
            });
        }
        Ok(Self {
            basis: Arc::clone(basis),
            weights,
        })
    }

    /// Derives weights as `log2` of each generator's valuation.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::NonPositiveWeight`] if any generator's
    /// valuation is at most 1, because its logarithm is then zero or negative
    /// and cannot be a norm weight. UMT-3.2 fixture F5 requires exactly this
    /// rejection rather than a silently degenerate function.
    pub fn from_log2_valuations(basis: &Arc<Basis>) -> Result<Self, ComplexityError> {
        let mut weights = Vec::with_capacity(basis.rank());
        for (index, generator) in basis.generators().iter().enumerate() {
            let weight = generator.valuation().log2_f64();
            weights.push(
                PositiveFinite::new(weight)
                    .map_err(|_| ComplexityError::NonPositiveWeight { index, weight })?,
            );
        }
        Self::new(basis, weights)
    }

    /// Tenney height on prime coordinates (UMT-3.2 section 1.3.2).
    ///
    /// `h_T(m) = sum_i |a_i| log2 p_i`, which for a reduced `r(m) = n/d`
    /// equals `log2(n d)`. That identity is specific to prime-factor
    /// coordinates, so this constructor insists on a basis that actually
    /// declares prime factorization as its independence contract.
    ///
    /// # Errors
    ///
    /// Returns [`ComplexityError::NotPrimeBasis`] if the basis does not
    /// declare [`IndependenceContract::PrimeFactorization`], and
    /// [`ComplexityError::NonPositiveWeight`] if a generator valuation is at
    /// most 1.
    pub fn tenney(basis: &Arc<Basis>) -> Result<Self, ComplexityError> {
        if *basis.independence() != IndependenceContract::PrimeFactorization {
            return Err(ComplexityError::NotPrimeBasis);
        }
        Self::from_log2_valuations(basis)
    }

    /// The weights.
    #[must_use]
    pub fn weights(&self) -> &[PositiveFinite] {
        &self.weights
    }
}

impl Complexity for LogWeightedL1 {
    fn basis(&self) -> &Arc<Basis> {
        &self.basis
    }

    fn profile(&self) -> ComplexityProfile {
        // Every weight is positive by construction.
        ComplexityProfile::LatticeNorm
    }

    fn value_f64(&self, monzo: &Monzo) -> Result<f64, ComplexityError> {
        check_basis(&self.basis, monzo)?;
        let mut total = 0.0f64;
        for (weight, exponent) in self.weights.iter().zip(monzo.exponents()) {
            if exponent.is_zero() {
                continue;
            }
            let magnitude = exponent
                .abs()
                .to_f64()
                .ok_or(ComplexityError::ExponentOutOfRange)?;
            total += weight.get() * magnitude;
        }
        Ok(total)
    }
}

impl CoordinateWeighted for LogWeightedL1 {
    fn coordinate_weight_f64(&self, index: usize) -> Option<f64> {
        self.weights.get(index).map(|weight| weight.get())
    }
}

/// The Tenney-height identity: `h_T(m) = log2(n d)` for reduced `r(m) = n/d`.
///
/// Provided as a checkable function because UMT-3.2 section 9.2 requires the
/// identity to hold within the tolerance of the logarithm evaluation, and a
/// conformance suite has to be able to evaluate the right-hand side
/// independently of the complexity implementation.
///
/// # Errors
///
/// Returns [`ComplexityError::NotRationalProfile`] if the monzo's basis has a
/// generator without an exact rational valuation.
pub fn log2_numerator_denominator_product(monzo: &Monzo) -> Result<f64, ComplexityError> {
    let ratio = monzo
        .exact_ratio()
        .map_err(|_| ComplexityError::NotRationalProfile)?;
    let product = ratio.numer().abs() * ratio.denom();
    log2_q_f64(&Q::new(product, Z::from(1))).ok_or(ComplexityError::NotRationalProfile)
}

#[cfg(test)]
mod tests {
    use super::{
        Complexity, ComplexityProfile, LogWeightedL1, WeightedL1,
        log2_numerator_denominator_product,
    };
    use crate::algebra::{Q, Z};
    use crate::error::ComplexityError;
    use crate::proportion::{Basis, PositiveFinite, PositiveQ, RealValuation};
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    fn q(value: i64) -> Q {
        Q::new(Z::from(value), Z::from(1))
    }

    #[test]
    fn weighted_l1_is_exact_and_declares_the_right_profile() {
        let basis = five_limit();
        let norm = WeightedL1::uniform(&basis);
        assert_eq!(norm.profile(), ComplexityProfile::LatticeNorm);
        assert!(norm.profile().claims_homogeneity());
        assert!(norm.profile().claims_separation());

        let comma = basis.monzo([-4, 4, -1]).unwrap();
        assert_eq!(norm.exact_value(&comma).unwrap(), Some(q(9)));
        assert!((norm.value_f64(&comma).unwrap() - 9.0).abs() < 1e-12);
        assert_eq!(norm.exact_value(&basis.zero()).unwrap(), Some(q(0)));
    }

    #[test]
    fn a_zero_weight_downgrades_the_claim_to_a_seminorm() {
        let basis = five_limit();
        let seminorm = WeightedL1::new(&basis, [q(0), q(1), q(1)]).unwrap();
        assert_eq!(seminorm.profile(), ComplexityProfile::LatticeSeminorm);
        assert!(seminorm.profile().claims_homogeneity());
        assert!(
            !seminorm.profile().claims_separation(),
            "octaves have zero cost, so separation must not be advertised"
        );

        // A nonzero element of the null subgroup.
        let octave = basis.monzo([3, 0, 0]).unwrap();
        assert_eq!(seminorm.exact_value(&octave).unwrap(), Some(q(0)));
        assert!(!octave.is_zero());
    }

    #[test]
    fn weights_are_validated() {
        let basis = five_limit();
        assert_eq!(
            WeightedL1::new(&basis, [q(1), q(1)]).unwrap_err(),
            ComplexityError::WeightCount {
                expected: 3,
                found: 2
            }
        );
        assert_eq!(
            WeightedL1::new(&basis, [q(1), q(-1), q(1)]).unwrap_err(),
            ComplexityError::NegativeWeight { index: 1 }
        );
    }

    #[test]
    fn tenney_height_matches_its_reduced_rational_identity() {
        let basis = five_limit();
        let tenney = LogWeightedL1::tenney(&basis).unwrap();

        for exponents in [
            [-4i64, 4, -1],
            [-1, 1, 0],
            [1, 0, 0],
            [0, 0, 0],
            [-19, 12, 0],
            [7, -3, -1],
        ] {
            let monzo = basis.monzo(exponents).unwrap();
            let height = tenney.value_f64(&monzo).unwrap();
            let identity = log2_numerator_denominator_product(&monzo).unwrap();
            assert!(
                (height - identity).abs() < 1e-9,
                "{exponents:?}: {height} versus {identity}"
            );
        }
    }

    #[test]
    fn tenney_requires_a_prime_basis() {
        let basis = Basis::builder("not-prime")
            .rational_generator("3/2", PositiveQ::new(Q::new(3.into(), 2.into())).unwrap())
            .build()
            .unwrap();
        assert_eq!(
            LogWeightedL1::tenney(&basis).unwrap_err(),
            ComplexityError::NotPrimeBasis
        );
        // The generic constructor still works: it just does not claim to be
        // Tenney height.
        assert!(LogWeightedL1::from_log2_valuations(&basis).is_ok());
    }

    /// F5: a generator below 1 has a negative logarithm, which must not be
    /// used as a norm weight.
    #[test]
    fn generators_below_one_are_rejected_as_log_weights() {
        let basis = Basis::builder("umt:symbolic:sub-unit")
            .symbolic_real_generator(
                "half",
                RealValuation::new(PositiveFinite::new(0.5).unwrap()),
            )
            .symbolic_real_generator(
                "three",
                RealValuation::new(PositiveFinite::new(3.0).unwrap()),
            )
            .build()
            .unwrap();

        match LogWeightedL1::from_log2_valuations(&basis) {
            Err(ComplexityError::NonPositiveWeight { index, weight }) => {
                assert_eq!(index, 0);
                assert!(weight < 0.0, "log2(0.5) is negative");
            }
            other => panic!("expected a non-positive weight rejection, got {other:?}"),
        }

        // Explicit positive weights are always available.
        let norm = LogWeightedL1::new(
            &basis,
            [
                PositiveFinite::new(1.0).unwrap(),
                PositiveFinite::new(1.585).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(norm.profile(), ComplexityProfile::LatticeNorm);
        assert!(norm.value_f64(&basis.monzo([2, -1]).unwrap()).unwrap() > 0.0);
    }

    #[test]
    fn foreign_monzos_are_rejected() {
        let norm = WeightedL1::uniform(&five_limit());
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        assert!(matches!(
            norm.exact_value(&other.monzo([1, 0, 0]).unwrap()),
            Err(ComplexityError::BasisMismatch { .. })
        ));
    }

    /// F34: `g(m) = sqrt(h1(m))` satisfies the separating group-length laws
    /// but fails integer homogeneity, so it must be declared a `group_length`
    /// and never a `lattice_norm`.
    #[test]
    fn f34_square_root_of_a_norm_is_only_a_group_length() {
        struct SquareRoot(WeightedL1);

        impl Complexity for SquareRoot {
            fn basis(&self) -> &Arc<Basis> {
                self.0.basis()
            }

            fn profile(&self) -> ComplexityProfile {
                ComplexityProfile::GroupLength { separating: true }
            }

            fn value_f64(&self, monzo: &crate::proportion::Monzo) -> Result<f64, ComplexityError> {
                Ok(libm::sqrt(self.0.value_f64(monzo)?))
            }
        }

        let basis = five_limit();
        let root = SquareRoot(WeightedL1::uniform(&basis));

        assert!(!root.profile().claims_homogeneity());
        assert!(root.profile().claims_separation());
        assert!(
            root.exact_value(&basis.monzo([1, 0, 0]).unwrap())
                .unwrap()
                .is_none()
        );

        // Group-length laws hold.
        let m = basis.monzo([-4, 4, -1]).unwrap();
        let n = basis.monzo([2, -1, 1]).unwrap();
        assert_eq!(root.value_f64(&basis.zero()).unwrap(), 0.0);
        assert_eq!(root.value_f64(&m).unwrap(), root.value_f64(&-&m).unwrap());
        assert!(
            root.value_f64(&m.checked_add(&n).unwrap()).unwrap()
                <= root.value_f64(&m).unwrap() + root.value_f64(&n).unwrap() + 1e-12
        );

        // Integer homogeneity fails: g(4m) = 2 g(m), not 4 g(m).
        let quadrupled = m.scale(&Z::from(4));
        let scaled = root.value_f64(&quadrupled).unwrap();
        let expected_if_homogeneous = 4.0 * root.value_f64(&m).unwrap();
        assert!((scaled - 2.0 * root.value_f64(&m).unwrap()).abs() < 1e-12);
        assert!((scaled - expected_if_homogeneous).abs() > 1.0);
    }

    #[test]
    fn octave_equivalent_seminorm_descends_to_the_quotient() {
        // Section 1.3.3: vanishing on the octave subgroup makes the null set a
        // subgroup, so the seminorm descends; two monzos differing by octaves
        // have equal cost.
        let basis = five_limit();
        let seminorm = WeightedL1::new(&basis, [q(0), q(1), q(1)]).unwrap();
        let fifth = basis.monzo([-1, 1, 0]).unwrap();
        let twelfth = basis.monzo([0, 1, 0]).unwrap();
        assert_eq!(
            seminorm.exact_value(&fifth).unwrap(),
            seminorm.exact_value(&twelfth).unwrap()
        );
        assert_eq!(seminorm.exact_value(&fifth).unwrap(), Some(q(1)));
    }

    #[test]
    fn integer_homogeneity_holds_where_it_is_claimed() {
        let basis = five_limit();
        let norm = WeightedL1::new(&basis, [q(1), q(2), q(3)]).unwrap();
        let m = basis.monzo([-4, 4, -1]).unwrap();
        for factor in [-5i64, -1, 0, 1, 7] {
            let scaled = norm
                .exact_value(&m.scale(&Z::from(factor)))
                .unwrap()
                .unwrap();
            let expected =
                Q::new(Z::from(factor.abs()), Z::from(1)) * norm.exact_value(&m).unwrap().unwrap();
            assert_eq!(scaled, expected, "factor {factor}");
        }
    }
}
