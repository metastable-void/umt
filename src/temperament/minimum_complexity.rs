//! Minimum-complexity representative selection (UMT-3.2 sections 1.7.2 and
//! 7.5).
//!
//! Given a declared complexity `h` and a tempered class `x`, this searches the
//! fiber `V^-1(x) = sigma(x) + K` for a lift of least complexity. That is the
//! spelling policy a musician wants: not merely *a* lift, and not merely a
//! small one, but the simplest one under a complexity the caller declared.
//!
//! It is a representative policy, not a splitting. A linear section is forced
//! to send `n x` to `n` times the lift of `x`, so it cannot minimize anything
//! class by class; this policy minimizes each class independently and does not
//! claim additivity.
//!
//! # Why the search is bounded, and provably so
//!
//! For a `lattice_norm` `h` and any kernel element `k`, the triangle
//! inequality gives `h(m0 + k) >= h(k) - h(m0)`. So if `h(k) > 2 h(m0)` then
//! `h(m0 + k) > h(m0)`, and such a `k` cannot improve on the base lift `m0`.
//! Every minimizer therefore satisfies `h(k) <= 2 h(m0)`.
//!
//! That bound is turned into bounds on kernel *coordinates* using the echelon
//! structure of the canonical kernel basis. With pivots `(r_j, j)` and pivot
//! values `v_j > 0`, column `j'` has a zero at row `r_j` for every `j' > j`,
//! so
//!
//! ```text
//! (B c)_{r_j} = v_j c_j + sum_{j' < j} B[r_j][j'] c_{j'}
//! ```
//!
//! and `w_{r_j} |(B c)_{r_j}| <= h(B c) <= 2 h(m0)` bounds `|c_j|` in terms of
//! the earlier bounds. The region is computed in floating point and rounded
//! *outward*, so it can only ever be too large; the selection inside it uses
//! the complexity's own comparison, which is exact when the complexity is.

use alloc::format;
use alloc::sync::Arc;
use alloc::vec::Vec;

use num_traits::Signed;

use crate::algebra::Z;
use crate::algebra::integer::ratio_to_f64;
use crate::error::TemperamentError;
use crate::proportion::complexity::{ComplexityProfile, ComplexityValue, CoordinateWeighted};
use crate::proportion::monzo::{Monzo, compatible};
use crate::realization::optimization::{ApproximationGuarantee, OptimizationOutcome};
use crate::temperament::image::ImageElem;
use crate::temperament::map::TemperamentMap;
use crate::temperament::representative::{LiftDecision, RepresentativePolicy, no_residue};

/// How many candidate lifts a single search may examine before it stops
/// claiming optimality.
pub const DEFAULT_SEARCH_BUDGET: usize = 200_000;

/// Selects the lift of least declared complexity from each fiber.
///
/// UMT layer: L2 to L1 selection policy.
///
/// Construction requires the complexity to be a
/// [`ComplexityProfile::LatticeNorm`]: a seminorm has nonzero elements of zero
/// cost, so its minimizer set over a coset can be infinite, and the bounded
/// search that makes this policy provable does not exist.
///
/// # Examples
///
/// ```
/// use umt::proportion::WeightedL1;
/// use umt::temperament::{
///     AmbientLattice, MinimumComplexityPolicy, RepresentativePolicy, TemperamentMap,
/// };
/// use umt::Basis;
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let steps = AmbientLattice::new("umt:edo:12", 1);
/// let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;
/// let policy = MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis))?;
///
/// // The simplest 5-limit interval sounding as seven 12-EDO steps is 3/2.
/// let class = map.apply_to_image(&basis.monzo([-1, 1, 0])?)?;
/// let chosen = policy.choose(&class, &())?;
/// assert_eq!(chosen.lift.exact_ratio()?.to_string(), "3/2");
///
/// // It minimizes rather than merely reduces, and says so.
/// assert!(policy.best_lift(&class)?.is_optimal());
/// assert!(!RepresentativePolicy::<()>::claims_homomorphic(&policy));
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct MinimumComplexityPolicy<H> {
    map: TemperamentMap,
    complexity: H,
    budget: usize,
}

impl<H: CoordinateWeighted> MinimumComplexityPolicy<H> {
    /// Builds the policy.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the complexity is
    /// defined over a different basis than the mapping's domain, and
    /// [`TemperamentError::UnboundedComplexity`] if the complexity is not a
    /// lattice norm, since only a norm bounds the search.
    pub fn new(map: TemperamentMap, complexity: H) -> Result<Self, TemperamentError> {
        if !compatible(complexity.basis(), map.domain()) {
            return Err(TemperamentError::BasisMismatch {
                expected: map.domain().id().clone(),
                found: complexity.basis().id().clone(),
            });
        }
        if complexity.profile() != ComplexityProfile::LatticeNorm {
            return Err(TemperamentError::UnboundedComplexity);
        }
        Ok(Self {
            map,
            complexity,
            budget: DEFAULT_SEARCH_BUDGET,
        })
    }

    /// Sets the candidate budget.
    #[must_use]
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    /// The mapping.
    #[must_use]
    pub fn temperament(&self) -> &TemperamentMap {
        &self.map
    }

    /// The declared complexity.
    #[must_use]
    pub fn complexity(&self) -> &H {
        &self.complexity
    }

    /// Searches the fiber of `class` for a lift of least complexity.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::ImageMismatch`] if the class belongs to a
    /// different image lattice, and propagates complexity evaluation failures
    /// as [`TemperamentError::Complexity`].
    pub fn best_lift(
        &self,
        class: &ImageElem,
    ) -> Result<OptimizationOutcome<Monzo, ComplexityValue>, TemperamentError> {
        let ambient = self.map.image().embed(class)?;
        let base = self.map.preimage(&ambient)?;
        let kernel = self.map.kernel();
        let rank = kernel.rank();

        let base_cost = self.complexity.value(&base)?;
        if rank == 0 {
            // The fiber is a single point.
            return Ok(OptimizationOutcome::Exact {
                solution: base,
                cost: base_cost,
            });
        }

        let (bounds, complete) = self.search_bounds(&base_cost)?;
        let basis = kernel.basis();

        let mut best = base.clone();
        let mut best_cost = base_cost;
        let mut ties = 0usize;
        let mut examined = 0usize;

        let mut coefficients = bounds.iter().map(|bound| -bound).collect::<Vec<i64>>();
        loop {
            examined += 1;
            let mut candidate = base.clone();
            for (column, coefficient) in coefficients.iter().enumerate() {
                if *coefficient == 0 {
                    continue;
                }
                let offset = basis
                    .column(column)
                    .expect("invariant: kernel basis column index is in range");
                let scaled: Vec<Z> = offset
                    .iter()
                    .map(|entry| entry * Z::from(*coefficient))
                    .collect();
                let shift = Monzo::new(Arc::clone(self.map.domain()), scaled)
                    .expect("invariant: kernel columns have domain rank");
                candidate = candidate.checked_add(&shift)?;
            }

            let cost = self.complexity.value(&candidate)?;
            match cost.partial_cmp(&best_cost) {
                Some(core::cmp::Ordering::Less) => {
                    best = candidate;
                    best_cost = cost;
                    ties = 0;
                }
                Some(core::cmp::Ordering::Equal) if candidate != best => {
                    ties += 1;
                    // Deterministic tie-break: the lexicographically smallest
                    // exponent vector, so the same input always gives the same
                    // spelling.
                    if candidate.exponents() < best.exponents() {
                        best = candidate;
                    }
                }
                _ => {}
            }

            if !advance(&mut coefficients, &bounds) {
                break;
            }
        }

        Ok(if complete {
            if ties == 0 {
                OptimizationOutcome::Exact {
                    solution: best,
                    cost: best_cost,
                }
            } else {
                OptimizationOutcome::Multiple {
                    selected: best,
                    cost: best_cost,
                    others: ties,
                }
            }
        } else {
            OptimizationOutcome::Approximate {
                solution: best,
                cost: best_cost,
                guarantee: ApproximationGuarantee::SearchedRegion {
                    region: format!("kernel coefficients bounded by {bounds:?}"),
                    examined,
                },
            }
        })
    }

    /// Derives per-coordinate search bounds, and reports whether they are the
    /// provable ones or a budget-limited truncation.
    fn search_bounds(
        &self,
        base_cost: &ComplexityValue,
    ) -> Result<(Vec<i64>, bool), TemperamentError> {
        let kernel = self.map.kernel();
        let basis = kernel.basis();
        let threshold = 2.0 * base_cost.as_f64();

        let mut bounds: Vec<i64> = Vec::with_capacity(kernel.rank());
        for (column, (row, _)) in kernel.sublattice().pivots().iter().enumerate() {
            let weight = self
                .complexity
                .coordinate_weight_f64(*row)
                .ok_or(TemperamentError::UnboundedComplexity)?;
            let mut allowance = threshold / weight;
            for (earlier, bound) in bounds.iter().enumerate() {
                let entry = ratio_to_f64(&basis.at(*row, earlier).abs(), &Z::from(1))
                    .ok_or(TemperamentError::UnboundedComplexity)?;
                allowance += entry * (*bound as f64);
            }
            let pivot = ratio_to_f64(basis.at(*row, column), &Z::from(1))
                .ok_or(TemperamentError::UnboundedComplexity)?;
            // Round outward: the region may be too large, never too small.
            let bound = libm::ceil(allowance / pivot * (1.0 + 1e-9)) + 1.0;
            if !bound.is_finite() || bound > 1.0e9 {
                return Err(TemperamentError::UnboundedComplexity);
            }
            bounds.push(bound as i64);
        }

        // Shrink uniformly if the provable region exceeds the budget.
        let size = |bounds: &[i64]| -> u128 {
            bounds
                .iter()
                .try_fold(1u128, |total, bound| {
                    total.checked_mul((2 * bound + 1) as u128)
                })
                .unwrap_or(u128::MAX)
        };
        if size(&bounds) <= self.budget as u128 {
            return Ok((bounds, true));
        }
        let mut truncated = bounds.clone();
        while size(&truncated) > self.budget as u128 {
            let largest = truncated
                .iter()
                .enumerate()
                .max_by_key(|(_, bound)| **bound)
                .map(|(index, _)| index)
                .expect("invariant: a nonzero-rank kernel has at least one bound");
            if truncated[largest] == 0 {
                break;
            }
            truncated[largest] -= 1;
        }
        Ok((truncated, false))
    }
}

/// Advances an odometer over `[-bound, bound]` per digit. Returns false when
/// it wraps around, that is, when enumeration is complete.
fn advance(coefficients: &mut [i64], bounds: &[i64]) -> bool {
    for (digit, bound) in coefficients.iter_mut().zip(bounds) {
        if *digit < *bound {
            *digit += 1;
            return true;
        }
        *digit = -bound;
    }
    false
}

impl<C, H: CoordinateWeighted> RepresentativePolicy<C> for MinimumComplexityPolicy<H> {
    type Error = TemperamentError;

    fn map(&self) -> &TemperamentMap {
        &self.map
    }

    fn choose(&self, class: &ImageElem, _context: &C) -> Result<LiftDecision, Self::Error> {
        let outcome = self.best_lift(class)?;
        let lift = outcome
            .into_solution()
            .expect("invariant: the search region always contains the base lift");
        Ok(LiftDecision::new(lift, no_residue(&self.map)?, None))
    }
}

#[cfg(test)]
mod tests {
    use super::MinimumComplexityPolicy;
    use crate::algebra::{Q, Z};
    use crate::error::TemperamentError;
    use crate::proportion::Basis;
    use crate::proportion::complexity::{LogWeightedL1, WeightedL1};
    use crate::realization::optimization::OptimizationOutcome;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use crate::temperament::representative::RepresentativePolicy;
    use alloc::string::ToString;
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    fn twelve_edo() -> TemperamentMap {
        TemperamentMap::from_rows(
            &five_limit(),
            &AmbientLattice::new("umt:edo:12", 1),
            [[12i64, 19, 28]],
        )
        .unwrap()
    }

    #[test]
    fn the_simplest_lift_of_a_fifth_is_a_just_fifth() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis)).unwrap();

        let class = map
            .apply_to_image(&basis.monzo([-1, 1, 0]).unwrap())
            .unwrap();
        let outcome = policy.best_lift(&class).unwrap();
        assert!(outcome.is_optimal(), "the search region is provable here");
        assert_eq!(
            outcome
                .solution()
                .unwrap()
                .exact_ratio()
                .unwrap()
                .to_string(),
            "3/2"
        );
    }

    #[test]
    fn every_lift_maps_back_and_is_no_worse_than_the_base() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis)).unwrap();

        for coordinate in -12i64..=12 {
            let class = map.image().element([coordinate]).unwrap();
            let outcome = policy.best_lift(&class).unwrap();
            let lift = outcome.solution().unwrap();
            assert_eq!(map.apply_to_image(lift).unwrap(), class);

            // Never worse than the canonical preimage it started from.
            let base = map.preimage(&map.image().embed(&class).unwrap()).unwrap();
            let complexity = WeightedL1::uniform(&basis);
            use crate::proportion::complexity::Complexity;
            assert!(
                complexity.value(lift).unwrap() <= complexity.value(&base).unwrap(),
                "class {coordinate}"
            );
        }
    }

    #[test]
    fn tenney_weighting_prefers_simpler_primes() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), LogWeightedL1::tenney(&basis).unwrap())
                .unwrap();

        // Four steps: the major third. Tenney height prefers 5/4 over the
        // pythagorean 81/64, which the uniform l1 norm would tie differently.
        let class = map.image().element([4i64]).unwrap();
        let outcome = policy.best_lift(&class).unwrap();
        assert_eq!(
            outcome
                .solution()
                .unwrap()
                .exact_ratio()
                .unwrap()
                .to_string(),
            "5/4"
        );
    }

    #[test]
    fn ties_are_reported_and_broken_deterministically() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis)).unwrap();

        // Run the same search twice: the selection must not vary.
        let class = map.image().element([6i64]).unwrap();
        let first = policy.best_lift(&class).unwrap();
        let second = policy.best_lift(&class).unwrap();
        assert_eq!(first.solution(), second.solution());

        if let OptimizationOutcome::Multiple { others, .. } = first {
            assert!(others > 0, "a reported tie must have alternatives");
        }
    }

    #[test]
    fn a_seminorm_is_rejected_because_it_does_not_bound_the_search() {
        let basis = five_limit();
        let zero = Q::new(Z::from(0), Z::from(1));
        let one = Q::new(Z::from(1), Z::from(1));
        let seminorm = WeightedL1::new(&basis, [zero, one.clone(), one]).unwrap();
        assert_eq!(
            MinimumComplexityPolicy::new(twelve_edo(), seminorm).unwrap_err(),
            TemperamentError::UnboundedComplexity
        );
    }

    #[test]
    fn a_complexity_over_another_basis_is_rejected() {
        let other = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        assert!(matches!(
            MinimumComplexityPolicy::new(twelve_edo(), WeightedL1::uniform(&other)),
            Err(TemperamentError::BasisMismatch { .. })
        ));
    }

    #[test]
    fn a_tight_budget_downgrades_the_claim_rather_than_lying() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy = MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis))
            .unwrap()
            .with_budget(3);

        let class = map.image().element([7i64]).unwrap();
        let outcome = policy.best_lift(&class).unwrap();
        assert!(
            !outcome.is_optimal(),
            "a truncated search must not claim optimality"
        );
        assert!(outcome.has_solution());
        // The right inverse law survives regardless.
        assert_eq!(
            map.apply_to_image(outcome.solution().unwrap()).unwrap(),
            class
        );
    }

    #[test]
    fn a_trivial_kernel_leaves_one_lift() {
        let domain = Basis::primes("umt:prime:2", &[2]).unwrap();
        let ambient = AmbientLattice::new("umt:f01-ambient", 1);
        let map = TemperamentMap::from_rows(&domain, &ambient, [[2i64]]).unwrap();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&domain)).unwrap();

        let class = map.image().element([3i64]).unwrap();
        let outcome = policy.best_lift(&class).unwrap();
        assert!(matches!(outcome, OptimizationOutcome::Exact { .. }));
        assert_eq!(outcome.solution().unwrap(), &domain.monzo([3]).unwrap());
    }

    #[test]
    fn the_policy_does_not_claim_homomorphism() {
        let basis = five_limit();
        let map = twelve_edo();
        let policy =
            MinimumComplexityPolicy::new(map.clone(), WeightedL1::uniform(&basis)).unwrap();
        assert!(!RepresentativePolicy::<()>::claims_homomorphic(&policy));

        // And indeed it is not additive.
        let lift = |coordinate: i64| {
            policy
                .choose(&map.image().element([coordinate]).unwrap(), &())
                .unwrap()
                .lift
        };
        let one = lift(1);
        assert_ne!(lift(2), one.checked_add(&one).unwrap());
    }
}
