//! Optimization outcomes (UMT-3.2 section 7.5, prompt section 33).
//!
//! Adaptive pitch realization, expressive time realization, and constrained
//! grid allocation share an engineering interface without sharing a
//! mathematical type. What they do share is that optimization can fail in ways
//! that are not errors:
//!
//! - the admissible set can be empty;
//! - the minimizer can be non-unique;
//! - the infimum can be finite and not attained;
//! - the answer can be approximate with a declared guarantee.
//!
//! `Result<T, E>` cannot express those, so this module does. Section 7.5 is
//! explicit that an optimizer MUST report infeasibility, an unattained
//! infimum, non-uniqueness where it is semantically relevant, and any
//! approximation tolerance, "rather than fabricating a unique exact
//! minimizer".

use alloc::string::String;

/// What an approximate answer is worth.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ApproximationGuarantee<C> {
    /// `J(y) <= J* + epsilon` for the stated `epsilon`.
    AbsoluteGap {
        /// The bound on the excess over the infimum.
        epsilon: C,
    },
    /// The answer is optimal within a searched region, and the region is
    /// stated. Nothing is claimed about admissible points outside it.
    SearchedRegion {
        /// What was searched, in enough detail to reproduce it.
        region: String,
        /// How many candidates were examined.
        examined: usize,
    },
    /// No guarantee is offered. Saying so is better than implying one.
    Unquantified,
}

/// The outcome of a constrained minimization.
///
/// UMT layer: realization policy. `T` is the solution type and `C` the cost
/// type; neither is assumed to be real-valued, so an exact cost stays exact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum OptimizationOutcome<T, C> {
    /// A unique minimizer, proved optimal.
    Exact {
        /// The minimizer.
        solution: T,
        /// Its cost, which is the attained minimum.
        cost: C,
    },
    /// Several minimizers share the minimum cost; one was selected by a
    /// declared deterministic rule.
    Multiple {
        /// The selected minimizer.
        selected: T,
        /// The attained minimum.
        cost: C,
        /// How many *other* minimizers were found, so a caller can tell a
        /// two-way tie from a large plateau.
        others: usize,
    },
    /// A solution with a declared guarantee short of proved optimality.
    Approximate {
        /// The returned solution.
        solution: T,
        /// Its cost.
        cost: C,
        /// What is actually guaranteed about it.
        guarantee: ApproximationGuarantee<C>,
    },
    /// The admissible set is empty.
    Infeasible,
    /// The infimum is finite but no admissible point attains it.
    ///
    /// The classic case is the admissible set `(0, 1)` with `J(y) = y`: the
    /// infimum is 0 and the minimizer set is empty. Returning a fabricated
    /// solution here would be a lie (fixture F26).
    InfimumNotAttained {
        /// The greatest lower bound.
        infimum: C,
    },
}

impl<T, C> OptimizationOutcome<T, C> {
    /// The solution, where one exists.
    ///
    /// `None` for [`OptimizationOutcome::Infeasible`] and
    /// [`OptimizationOutcome::InfimumNotAttained`], because in those cases
    /// there is no solution to return.
    #[must_use]
    pub fn solution(&self) -> Option<&T> {
        match self {
            Self::Exact { solution, .. } | Self::Approximate { solution, .. } => Some(solution),
            Self::Multiple { selected, .. } => Some(selected),
            Self::Infeasible | Self::InfimumNotAttained { .. } => None,
        }
    }

    /// Consumes the outcome, yielding the solution where one exists.
    #[must_use]
    pub fn into_solution(self) -> Option<T> {
        match self {
            Self::Exact { solution, .. } | Self::Approximate { solution, .. } => Some(solution),
            Self::Multiple { selected, .. } => Some(selected),
            Self::Infeasible | Self::InfimumNotAttained { .. } => None,
        }
    }

    /// The cost or infimum, where one is known.
    #[must_use]
    pub fn cost(&self) -> Option<&C> {
        match self {
            Self::Exact { cost, .. }
            | Self::Multiple { cost, .. }
            | Self::Approximate { cost, .. } => Some(cost),
            Self::InfimumNotAttained { infimum } => Some(infimum),
            Self::Infeasible => None,
        }
    }

    /// Whether the solution is proved optimal.
    ///
    /// True for [`OptimizationOutcome::Exact`] and
    /// [`OptimizationOutcome::Multiple`]: both attain the minimum, and
    /// non-uniqueness is not a weaker result, only a different one.
    #[must_use]
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Exact { .. } | Self::Multiple { .. })
    }

    /// Whether a minimizer exists at all.
    #[must_use]
    pub fn has_solution(&self) -> bool {
        self.solution().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{ApproximationGuarantee, OptimizationOutcome};
    use alloc::string::String;

    type Outcome = OptimizationOutcome<f64, f64>;

    #[test]
    fn accessors_agree_with_the_variants() {
        let exact: Outcome = OptimizationOutcome::Exact {
            solution: 2.0,
            cost: 1.0,
        };
        assert_eq!(exact.solution(), Some(&2.0));
        assert_eq!(exact.cost(), Some(&1.0));
        assert!(exact.is_optimal());

        let multiple: Outcome = OptimizationOutcome::Multiple {
            selected: 2.0,
            cost: 1.0,
            others: 3,
        };
        assert!(multiple.is_optimal(), "a tie is still a minimum");
        assert_eq!(multiple.solution(), Some(&2.0));

        let approximate: Outcome = OptimizationOutcome::Approximate {
            solution: 2.0,
            cost: 1.5,
            guarantee: ApproximationGuarantee::AbsoluteGap { epsilon: 0.5 },
        };
        assert!(!approximate.is_optimal());
        assert!(approximate.has_solution());

        let infeasible: Outcome = OptimizationOutcome::Infeasible;
        assert_eq!(infeasible.solution(), None);
        assert_eq!(infeasible.cost(), None);
        assert!(!infeasible.is_optimal());

        let unattained: Outcome = OptimizationOutcome::InfimumNotAttained { infimum: 0.0 };
        assert_eq!(
            unattained.solution(),
            None,
            "no minimizer may be fabricated"
        );
        assert_eq!(unattained.cost(), Some(&0.0), "but the infimum is known");
        assert!(!unattained.is_optimal());
    }

    #[test]
    fn a_searched_region_guarantee_records_what_was_searched() {
        let outcome: Outcome = OptimizationOutcome::Approximate {
            solution: 1.0,
            cost: 1.0,
            guarantee: ApproximationGuarantee::SearchedRegion {
                region: String::from("|c| <= 4"),
                examined: 9,
            },
        };
        match outcome {
            OptimizationOutcome::Approximate {
                guarantee: ApproximationGuarantee::SearchedRegion { region, examined },
                ..
            } => {
                assert_eq!(region, "|c| <= 4");
                assert_eq!(examined, 9);
            }
            other => panic!("unexpected outcome {other:?}"),
        }
    }
}
