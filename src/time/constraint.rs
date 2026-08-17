//! Temporal constraint networks (UMT-3.2 section 5.10, prompt sections 31 and
//! 32).
//!
//! Unmeasured and partially measured time is relations among temporal
//! variables, not a compulsory projection onto a grid. Section 5.10
//! distinguishes three solver profiles because *not all temporal constraints
//! reduce to shortest paths*, and prompt section 31 forbids "one universal
//! solver that pretends all constraints are STP edges". So there are three
//! types:
//!
//! - [`StpProblem`] - difference bounds `l <= t_j - t_i <= u`. These really do
//!   reduce to shortest paths, and this is the only profile that gets the
//!   unconditional consistency claim (section 5.10.1).
//! - [`LinearTemporalProblem`] - general linear inequalities, including the
//!   cross-multiplied ratio constraints of section 5.10.2. A three-variable
//!   ratio bound is *not* a difference edge, and there is no way to hand one
//!   to the STP solver: the types do not connect (fixture F17).
//! - [`HybridTemporalProblem`] - the above plus external predicates, which are
//!   typed data with a declared evaluation contract and no decidability claim
//!   (section 5.10.3, fixture F18).
//!
//! # Strict positivity is preserved, not approximated
//!
//! A ratio constraint needs `t_j - t_i > 0`. Section 5.10.2 permits replacing
//! that by `t_j - t_i >= delta` *only* when a positive lower bound is
//! justified by the model or the source, and states plainly that inventing one
//! for solver convenience changes the feasible set.
//!
//! This crate therefore solves strict inequalities directly, by exact
//! Fourier-Motzkin elimination over the rationals, which carries strictness
//! through the arithmetic. A `delta` can still be declared - through
//! [`PositivityHandling::JustifiedDelta`], which requires the justification
//! text alongside it - but nothing here will supply one on its own. That is
//! fixture F25.
//!
//! # Results carry their status
//!
//! Prompt section 32 forbids returning "only a map of `TimeVarId -> f64`".
//! [`TemporalOutcome`] distinguishes exactly solved, approximately solved,
//! partially solved with residual external conditions, inconsistent, and
//! unsupported by the selected profile - and the solved variants carry exact
//! rational assignments, since every solver here is exact.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use num_traits::{Signed, Zero};

use crate::algebra::{Q, Z};
use crate::error::TimeError;

/// Stable identity of a temporal variable, typically an event onset or offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct TimeVarId(Arc<str>);

impl TimeVarId {
    /// Wraps a stable variable identity.
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

impl From<String> for TimeVarId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<TimeVarId> for String {
    fn from(value: TimeVarId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for TimeVarId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which solver profile produced a result (UMT-3.2 section 5.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SolverProfile {
    /// Difference bounds, solved by shortest paths. The only profile with an
    /// unconditional consistency claim.
    SimpleTemporal,
    /// General linear inequalities, solved by exact elimination.
    LinearRatio,
    /// Linear constraints plus external predicates.
    Hybrid,
}

/// How exact a reported temporal solution is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Exactness {
    /// The assignment is exact rational data.
    Exact,
    /// The assignment is a real approximation with a declared tolerance.
    Approximate,
}

/// A difference constraint `lower <= t_to - t_from <= upper`
/// (UMT-3.2 section 5.10.1).
///
/// UMT layer: L1, exact. Either bound may be absent, meaning unbounded on that
/// side.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DifferenceConstraint {
    /// The variable subtracted.
    pub from: TimeVarId,
    /// The variable subtracted from.
    pub to: TimeVarId,
    /// The lower bound, if any.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::io::serde_exact::option_q")
    )]
    pub lower: Option<Q>,
    /// The upper bound, if any.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::io::serde_exact::option_q")
    )]
    pub upper: Option<Q>,
}

impl DifferenceConstraint {
    /// `lower <= t_to - t_from <= upper`.
    #[must_use]
    pub fn between(from: &TimeVarId, to: &TimeVarId, lower: Option<Q>, upper: Option<Q>) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
            lower,
            upper,
        }
    }

    /// `t_to - t_from <= upper`.
    #[must_use]
    pub fn at_most(from: &TimeVarId, to: &TimeVarId, upper: Q) -> Self {
        Self::between(from, to, None, Some(upper))
    }

    /// `t_to - t_from >= lower`.
    #[must_use]
    pub fn at_least(from: &TimeVarId, to: &TimeVarId, lower: Q) -> Self {
        Self::between(from, to, Some(lower), None)
    }
}

/// A Simple Temporal Problem: difference bounds only
/// (UMT-3.2 section 5.10.1).
///
/// UMT layer: L1, exact.
///
/// This profile, and only this profile, gets the unconditional shortest-path
/// consistency claim. Everything it accepts is a difference bound, so there is
/// no way to smuggle a ratio constraint in and have it silently treated as one
/// graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StpProblem {
    variables: Vec<TimeVarId>,
    constraints: Vec<DifferenceConstraint>,
}

impl StpProblem {
    /// An empty problem.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a variable, returning its identity.
    pub fn variable(&mut self, id: &str) -> TimeVarId {
        let variable = TimeVarId::new(id);
        if !self.variables.contains(&variable) {
            self.variables.push(variable.clone());
        }
        variable
    }

    /// The declared variables, in declaration order.
    #[must_use]
    pub fn variables(&self) -> &[TimeVarId] {
        &self.variables
    }

    /// The declared constraints.
    #[must_use]
    pub fn constraints(&self) -> &[DifferenceConstraint] {
        &self.constraints
    }

    /// Adds a difference constraint.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::UnknownTimeVariable`] if either endpoint has not
    /// been declared.
    pub fn constrain(&mut self, constraint: DifferenceConstraint) -> Result<(), TimeError> {
        for variable in [&constraint.from, &constraint.to] {
            if !self.variables.contains(variable) {
                return Err(TimeError::UnknownTimeVariable {
                    variable: variable.to_string(),
                });
            }
        }
        self.constraints.push(constraint);
        Ok(())
    }

    /// Solves the network by all-pairs shortest paths on the difference graph.
    ///
    /// Consistency is exact: the weights are rationals and no floating point
    /// is involved, so a negative cycle is detected as one rather than as a
    /// rounding artefact.
    ///
    /// The returned assignment is *one* consistent assignment, not a canonical
    /// one. A difference network fixes distances, not positions, so the whole
    /// solution can be translated freely along the timeline and individual
    /// values may well be negative. The tight implied bounds are the
    /// translation-invariant part, and are usually what a caller wants.
    #[must_use]
    pub fn solve(&self) -> TemporalOutcome {
        let count = self.variables.len();
        if count == 0 {
            return TemporalOutcome::Solved {
                profile: SolverProfile::SimpleTemporal,
                exactness: Exactness::Exact,
                assignment: BTreeMap::new(),
                tight_bounds: Vec::new(),
            };
        }

        // A virtual source at index `count`, with a zero-weight edge to every
        // variable, guarantees the distance vector is finite and feasible.
        let size = count + 1;
        let mut distance: Vec<Vec<Option<Q>>> = alloc::vec![alloc::vec![None; size]; size];
        for (index, row) in distance.iter_mut().enumerate() {
            row[index] = Some(Q::zero());
        }
        for slot in distance[count].iter_mut().take(count) {
            *slot = Some(Q::zero());
        }

        // `t_to - t_from <= u` becomes an edge from -> to of weight u.
        // `l <= t_to - t_from` becomes an edge to -> from of weight -l.
        for constraint in &self.constraints {
            let from = self.index_of(&constraint.from);
            let to = self.index_of(&constraint.to);
            if let (Some(from), Some(to)) = (from, to) {
                if let Some(upper) = &constraint.upper {
                    relax(&mut distance[from][to], upper.clone());
                }
                if let Some(lower) = &constraint.lower {
                    relax(&mut distance[to][from], -lower.clone());
                }
            }
        }

        for middle in 0..size {
            let through = distance[middle].clone();
            for row in distance.iter_mut() {
                let Some(left) = row[middle].clone() else {
                    continue;
                };
                for (end, right) in through.iter().enumerate() {
                    let Some(right) = right else {
                        continue;
                    };
                    relax(&mut row[end], &left + right);
                }
            }
        }

        for index in 0..size {
            if let Some(cycle) = &distance[index][index]
                && cycle.is_negative()
            {
                return TemporalOutcome::Inconsistent {
                    profile: SolverProfile::SimpleTemporal,
                    witness: self.witness(index, &distance),
                };
            }
        }

        let assignment = (0..count)
            .map(|index| {
                (
                    self.variables[index].clone(),
                    distance[count][index].clone().unwrap_or_else(Q::zero),
                )
            })
            .collect();

        let mut tight_bounds = Vec::new();
        for (from, row) in distance.iter().enumerate().take(count) {
            for (to, upper) in row.iter().enumerate().take(count) {
                if from == to {
                    continue;
                }
                let lower = distance[to][from].clone().map(|value| -value);
                if upper.is_some() || lower.is_some() {
                    tight_bounds.push(DifferenceConstraint::between(
                        &self.variables[from],
                        &self.variables[to],
                        lower,
                        upper.clone(),
                    ));
                }
            }
        }

        TemporalOutcome::Solved {
            profile: SolverProfile::SimpleTemporal,
            exactness: Exactness::Exact,
            assignment,
            tight_bounds,
        }
    }

    fn index_of(&self, variable: &TimeVarId) -> Option<usize> {
        self.variables.iter().position(|known| known == variable)
    }

    fn witness(&self, index: usize, distance: &[Vec<Option<Q>>]) -> String {
        if index < self.variables.len() {
            alloc::format!(
                "negative cycle through `{}`: implied {} <= 0",
                self.variables[index],
                distance[index][index]
                    .as_ref()
                    .map_or_else(|| "-inf".to_string(), ToString::to_string)
            )
        } else {
            "negative cycle in the difference graph".to_string()
        }
    }
}

fn relax(slot: &mut Option<Q>, candidate: Q) {
    match slot {
        Some(existing) if *existing <= candidate => {}
        _ => *slot = Some(candidate),
    }
}

/// A general linear constraint `sum a_i t_i <= b`, strict or not.
///
/// UMT layer: L1, exact. This is deliberately *not* a difference bound: the
/// coefficients are arbitrary rationals over arbitrarily many variables, which
/// is exactly what a cross-multiplied ratio constraint needs and what a
/// shortest-path algorithm cannot represent.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearConstraint {
    /// Non-zero coefficients, keyed by variable.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::map_q"))]
    pub coefficients: BTreeMap<TimeVarId, Q>,
    /// The right-hand side.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
    pub bound: Q,
    /// Whether the inequality is strict.
    pub strict: bool,
}

impl LinearConstraint {
    /// `sum a_i t_i <= b`.
    #[must_use]
    pub fn at_most<I>(terms: I, bound: Q) -> Self
    where
        I: IntoIterator<Item = (TimeVarId, Q)>,
    {
        Self {
            coefficients: terms.into_iter().filter(|(_, a)| !a.is_zero()).collect(),
            bound,
            strict: false,
        }
    }

    /// `sum a_i t_i < b`.
    #[must_use]
    pub fn less_than<I>(terms: I, bound: Q) -> Self
    where
        I: IntoIterator<Item = (TimeVarId, Q)>,
    {
        let mut constraint = Self::at_most(terms, bound);
        constraint.strict = true;
        constraint
    }
}

/// A bounded ratio constraint on three events (UMT-3.2 section 5.10.2).
///
/// ```text
/// lower <= (t_later - t_middle) / (t_middle - t_earlier) <= upper,
/// ```
///
/// under the semantic condition `t_middle - t_earlier > 0`.
///
/// UMT layer: L1, exact. Three variables with non-unit coefficients after
/// cross-multiplication, so this is never a difference-bound graph edge -
/// which is the whole content of fixture F17.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RatioConstraint {
    /// The first event.
    pub earlier: TimeVarId,
    /// The middle event, whose distance from `earlier` is the denominator.
    pub middle: TimeVarId,
    /// The last event.
    pub later: TimeVarId,
    /// The lower bound on the ratio.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
    pub lower: Q,
    /// The upper bound on the ratio.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
    pub upper: Q,
}

/// How the positive-denominator condition of a ratio constraint is handled
/// (UMT-3.2 section 5.10.2, fixture F25).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
#[non_exhaustive]
pub enum PositivityHandling {
    /// Keep `t_middle - t_earlier > 0` as a strict inequality.
    ///
    /// The default, and the only choice that does not change the feasible set.
    #[default]
    StrictInequality,
    /// Replace it by `t_middle - t_earlier >= delta`, with the justification
    /// that licenses the substitution.
    ///
    /// The justification is a mandatory field, not an optional note: section
    /// 5.10.2 permits this only "when a positive lower bound `delta` is
    /// justified by the model or source data", and a `delta` with no stated
    /// justification is exactly the invented one the specification forbids.
    JustifiedDelta {
        /// The lower bound.
        #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
        delta: Q,
        /// Why the model or source licenses it.
        justification: String,
    },
}

/// A reference to an external temporal predicate
/// (UMT-3.2 section 5.10.3, fixture F18).
///
/// UMT layer: declared metadata.
///
/// Typed data with a declared evaluation contract - never executable code.
/// Section 5.10.3 is explicit that "a native interchange format MUST NOT
/// require recipients to deserialize or execute arbitrary code received from
/// the file", so there is no callback field here and no way to add one through
/// serialization. An application supplies behaviour by implementing
/// [`PredicateEvaluator`] in its own process.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExternalPredicate {
    /// Stable identity of this predicate instance.
    pub id: String,
    /// The predicate type, which names its declared semantics.
    pub predicate_type: String,
    /// The evaluation contract: what an evaluator must guarantee.
    pub contract: String,
    /// Declared parameters, as data.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: BTreeMap<String, String>,
}

/// Something able to decide an external predicate at solve time.
///
/// Implemented by an application, never deserialized from a document. There is
/// no universal decidability guarantee (section 5.10.3), so an evaluator is
/// free to return `None` meaning "cannot decide now", and the solver reports
/// that as a residual condition rather than guessing.
pub trait PredicateEvaluator {
    /// Decides a predicate, or declines to.
    fn evaluate(&self, predicate: &ExternalPredicate) -> Option<bool>;
}

/// A linear temporal problem (UMT-3.2 section 5.10.2).
///
/// UMT layer: L1, exact.
///
/// Solved by Fourier-Motzkin elimination over the rationals, which is exact
/// and handles strict inequalities natively - the first of the two options
/// section 5.10.2 offers, and the one that does not change the feasible set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearTemporalProblem {
    variables: Vec<TimeVarId>,
    constraints: Vec<LinearConstraint>,
    positivity: PositivityHandling,
    budget: usize,
}

/// The default number of intermediate constraints elimination may produce.
pub const DEFAULT_ELIMINATION_BUDGET: usize = 20_000;

impl Default for LinearTemporalProblem {
    /// An empty problem with the default elimination budget.
    fn default() -> Self {
        Self::new()
    }
}

impl LinearTemporalProblem {
    /// An empty problem, preserving strict inequalities.
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
            positivity: PositivityHandling::StrictInequality,
            budget: DEFAULT_ELIMINATION_BUDGET,
        }
    }

    /// Declares how the positive-denominator condition is handled.
    #[must_use]
    pub fn with_positivity(mut self, positivity: PositivityHandling) -> Self {
        self.positivity = positivity;
        self
    }

    /// Sets the elimination budget.
    #[must_use]
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    /// How the positive-denominator condition is handled.
    #[must_use]
    pub fn positivity(&self) -> &PositivityHandling {
        &self.positivity
    }

    /// Declares a variable.
    pub fn variable(&mut self, id: &str) -> TimeVarId {
        let variable = TimeVarId::new(id);
        if !self.variables.contains(&variable) {
            self.variables.push(variable.clone());
        }
        variable
    }

    /// The declared variables.
    #[must_use]
    pub fn variables(&self) -> &[TimeVarId] {
        &self.variables
    }

    /// The accumulated linear constraints.
    #[must_use]
    pub fn constraints(&self) -> &[LinearConstraint] {
        &self.constraints
    }

    /// Adds a linear constraint.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::UnknownTimeVariable`] for an undeclared variable.
    pub fn constrain(&mut self, constraint: LinearConstraint) -> Result<(), TimeError> {
        for variable in constraint.coefficients.keys() {
            if !self.variables.contains(variable) {
                return Err(TimeError::UnknownTimeVariable {
                    variable: variable.to_string(),
                });
            }
        }
        self.constraints.push(constraint);
        Ok(())
    }

    /// Adds a difference bound, which is a linear constraint like any other
    /// once it is in this profile.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::UnknownTimeVariable`] for an undeclared variable.
    pub fn constrain_difference(
        &mut self,
        constraint: &DifferenceConstraint,
    ) -> Result<(), TimeError> {
        let one = Q::from(Z::from(1));
        if let Some(upper) = &constraint.upper {
            self.constrain(LinearConstraint::at_most(
                [
                    (constraint.to.clone(), one.clone()),
                    (constraint.from.clone(), -one.clone()),
                ],
                upper.clone(),
            ))?;
        }
        if let Some(lower) = &constraint.lower {
            self.constrain(LinearConstraint::at_most(
                [
                    (constraint.from.clone(), one.clone()),
                    (constraint.to.clone(), -one.clone()),
                ],
                -lower.clone(),
            ))?;
        }
        Ok(())
    }

    /// Cross-multiplies a ratio constraint into linear inequalities
    /// (UMT-3.2 section 5.10.2).
    ///
    /// Produces three constraints: the two bounds, and the positive-denominator
    /// condition in whatever form [`LinearTemporalProblem::positivity`]
    /// declares.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::UnknownTimeVariable`] for an undeclared variable.
    pub fn constrain_ratio(&mut self, ratio: &RatioConstraint) -> Result<(), TimeError> {
        let one = Q::from(Z::from(1));
        let zero = Q::zero();

        // lower * (t_j - t_i) <= t_k - t_j
        //   =>  lower*t_j - lower*t_i - t_k + t_j <= 0
        self.constrain(LinearConstraint::at_most(
            [
                (ratio.earlier.clone(), -ratio.lower.clone()),
                (ratio.middle.clone(), &ratio.lower + &one),
                (ratio.later.clone(), -one.clone()),
            ],
            zero.clone(),
        ))?;

        // t_k - t_j <= upper * (t_j - t_i)
        //   =>  t_k - t_j - upper*t_j + upper*t_i <= 0
        self.constrain(LinearConstraint::at_most(
            [
                (ratio.earlier.clone(), ratio.upper.clone()),
                (ratio.middle.clone(), -(&ratio.upper + &one)),
                (ratio.later.clone(), one.clone()),
            ],
            zero.clone(),
        ))?;

        // The denominator condition.
        match self.positivity.clone() {
            PositivityHandling::StrictInequality => {
                // t_i - t_j < 0, that is, t_j - t_i > 0.
                self.constrain(LinearConstraint::less_than(
                    [
                        (ratio.earlier.clone(), one.clone()),
                        (ratio.middle.clone(), -one),
                    ],
                    zero,
                ))?;
            }
            PositivityHandling::JustifiedDelta { delta, .. } => {
                // t_i - t_j <= -delta.
                self.constrain(LinearConstraint::at_most(
                    [
                        (ratio.earlier.clone(), one.clone()),
                        (ratio.middle.clone(), -one),
                    ],
                    -delta,
                ))?;
            }
        }
        Ok(())
    }

    /// Decides feasibility by exact Fourier-Motzkin elimination.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EliminationBudgetExceeded`] when the intermediate
    /// system outgrows the declared budget. Elimination can blow up
    /// combinatorially, and reporting that is better than returning a
    /// feasibility answer that was never computed.
    pub fn solve(&self) -> Result<TemporalOutcome, TimeError> {
        let rows: Vec<Row> = self
            .constraints
            .iter()
            .map(|constraint| Row::from_constraint(constraint, &self.variables))
            .collect();

        let mut history = Vec::new();
        let mut current = rows;
        for index in (0..self.variables.len()).rev() {
            history.push(current.clone());
            current = eliminate(&current, index, self.budget)?;
        }

        // Everything is eliminated: what remains is `0 <= b` or `0 < b`.
        for row in &current {
            let feasible = if row.strict {
                row.bound.is_positive()
            } else {
                !row.bound.is_negative()
            };
            if !feasible {
                return Ok(TemporalOutcome::Inconsistent {
                    profile: SolverProfile::LinearRatio,
                    witness: alloc::format!(
                        "elimination reduced the system to the false statement 0 {} {}",
                        if row.strict { "<" } else { "<=" },
                        row.bound
                    ),
                });
            }
        }

        let assignment = back_substitute(&history, &self.variables);
        Ok(TemporalOutcome::Solved {
            profile: SolverProfile::LinearRatio,
            exactness: Exactness::Exact,
            assignment,
            tight_bounds: Vec::new(),
        })
    }
}

/// A linear problem plus external predicates
/// (UMT-3.2 section 5.10.3, fixture F18).
///
/// UMT layer: L1 constraints, declared metadata predicates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HybridTemporalProblem {
    linear: LinearTemporalProblem,
    predicates: Vec<ExternalPredicate>,
}

impl HybridTemporalProblem {
    /// Wraps a linear problem.
    #[must_use]
    pub fn new(linear: LinearTemporalProblem) -> Self {
        Self {
            linear,
            predicates: Vec::new(),
        }
    }

    /// The underlying linear problem.
    #[must_use]
    pub fn linear(&self) -> &LinearTemporalProblem {
        &self.linear
    }

    /// The underlying linear problem, mutably.
    pub fn linear_mut(&mut self) -> &mut LinearTemporalProblem {
        &mut self.linear
    }

    /// The declared external predicates.
    #[must_use]
    pub fn predicates(&self) -> &[ExternalPredicate] {
        &self.predicates
    }

    /// Declares an external predicate.
    pub fn add_predicate(&mut self, predicate: ExternalPredicate) {
        self.predicates.push(predicate);
    }

    /// Solves the linear part and reports the predicate status.
    ///
    /// With no evaluator, every predicate is unresolved and the outcome is
    /// [`TemporalOutcome::PartiallySolved`]. Section 5.10.3 requires an
    /// implementation to report whether a network is statically solved,
    /// partially solved with residual external conditions, validated only at
    /// performance time, or unsupported - and this is how it says the second.
    ///
    /// # Errors
    ///
    /// Propagates elimination failure from the linear part.
    pub fn solve(
        &self,
        evaluator: Option<&dyn PredicateEvaluator>,
    ) -> Result<TemporalOutcome, TimeError> {
        let mut unresolved = Vec::new();
        let mut refuted = Vec::new();
        for predicate in &self.predicates {
            match evaluator.and_then(|evaluator| evaluator.evaluate(predicate)) {
                Some(true) => {}
                Some(false) => refuted.push(predicate.id.clone()),
                None => unresolved.push(predicate.id.clone()),
            }
        }

        if !refuted.is_empty() {
            return Ok(TemporalOutcome::Inconsistent {
                profile: SolverProfile::Hybrid,
                witness: alloc::format!("external predicates refuted: {}", refuted.join(", ")),
            });
        }

        let inner = self.linear.solve()?;
        if unresolved.is_empty() {
            return Ok(match inner {
                TemporalOutcome::Solved {
                    exactness,
                    assignment,
                    tight_bounds,
                    ..
                } => TemporalOutcome::Solved {
                    profile: SolverProfile::Hybrid,
                    exactness,
                    assignment,
                    tight_bounds,
                },
                other => other,
            });
        }

        match inner {
            TemporalOutcome::Solved { assignment, .. } => Ok(TemporalOutcome::PartiallySolved {
                profile: SolverProfile::Hybrid,
                assignment,
                unresolved,
            }),
            other => Ok(other),
        }
    }

    /// Whether any static decidability claim is being made.
    ///
    /// Always `false` while predicates are present: section 5.10.3 states that
    /// "general external predicates do not carry a universal decidability
    /// guarantee", and this method exists so that fact is queryable rather
    /// than merely documented.
    #[must_use]
    pub fn claims_static_decidability(&self) -> bool {
        self.predicates.is_empty()
    }
}

/// The result of a temporal solve (prompt section 32).
///
/// UMT layer: L1 assignments, declared status.
///
/// A bare `TimeVarId -> f64` map cannot say whether the answer is exact, which
/// profile produced it, or what external conditions are still outstanding.
/// Each of those changes what a caller may do with the result, so each is a
/// field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemporalOutcome {
    /// A consistent network with an exact or approximate assignment.
    Solved {
        /// Which profile produced it.
        profile: SolverProfile,
        /// Whether the assignment is exact.
        exactness: Exactness,
        /// One consistent assignment. Not the only one, in general.
        assignment: BTreeMap<TimeVarId, Q>,
        /// Tight implied pairwise bounds, where the profile computes them.
        tight_bounds: Vec<DifferenceConstraint>,
    },
    /// Consistent as far as the static constraints go, with external
    /// conditions still outstanding.
    PartiallySolved {
        /// Which profile produced it.
        profile: SolverProfile,
        /// An assignment satisfying the static constraints.
        assignment: BTreeMap<TimeVarId, Q>,
        /// Identities of the predicates that remain undecided.
        unresolved: Vec<String>,
    },
    /// The constraints contradict each other.
    Inconsistent {
        /// Which profile detected it.
        profile: SolverProfile,
        /// A human-readable witness to the contradiction.
        witness: String,
    },
    /// The selected profile cannot express these constraints.
    ///
    /// Reported rather than worked around: section 5.10.2 forbids inventing a
    /// `delta` to make a strict constraint fit a solver that cannot take one.
    UnsupportedByProfile {
        /// Which profile was asked.
        profile: SolverProfile,
        /// What it cannot express.
        reason: String,
    },
}

impl TemporalOutcome {
    /// The assignment, where there is one.
    #[must_use]
    pub fn assignment(&self) -> Option<&BTreeMap<TimeVarId, Q>> {
        match self {
            Self::Solved { assignment, .. } | Self::PartiallySolved { assignment, .. } => {
                Some(assignment)
            }
            Self::Inconsistent { .. } | Self::UnsupportedByProfile { .. } => None,
        }
    }

    /// Whether the network is consistent as far as this solver determined.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        matches!(self, Self::Solved { .. } | Self::PartiallySolved { .. })
    }

    /// Predicates still outstanding.
    #[must_use]
    pub fn unresolved(&self) -> &[String] {
        match self {
            Self::PartiallySolved { unresolved, .. } => unresolved,
            _ => &[],
        }
    }

    /// Which profile produced this result.
    #[must_use]
    pub fn profile(&self) -> SolverProfile {
        match self {
            Self::Solved { profile, .. }
            | Self::PartiallySolved { profile, .. }
            | Self::Inconsistent { profile, .. }
            | Self::UnsupportedByProfile { profile, .. } => *profile,
        }
    }
}

/// One row of the elimination system: `sum a_i x_i <= b`, or `<` if strict.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    coefficients: Vec<Q>,
    bound: Q,
    strict: bool,
}

impl Row {
    fn from_constraint(constraint: &LinearConstraint, variables: &[TimeVarId]) -> Self {
        let coefficients = variables
            .iter()
            .map(|variable| {
                constraint
                    .coefficients
                    .get(variable)
                    .cloned()
                    .unwrap_or_else(Q::zero)
            })
            .collect();
        Self {
            coefficients,
            bound: constraint.bound.clone(),
            strict: constraint.strict,
        }
    }
}

fn eliminate(rows: &[Row], index: usize, budget: usize) -> Result<Vec<Row>, TimeError> {
    let mut kept = Vec::new();
    let mut uppers = Vec::new();
    let mut lowers = Vec::new();
    for row in rows {
        let coefficient = &row.coefficients[index];
        if coefficient.is_zero() {
            kept.push(row.clone());
        } else if coefficient.is_positive() {
            uppers.push(row);
        } else {
            lowers.push(row);
        }
    }

    if kept.len() + uppers.len() * lowers.len() > budget {
        return Err(TimeError::EliminationBudgetExceeded { budget });
    }

    for upper in &uppers {
        for lower in &lowers {
            let a = upper.coefficients[index].clone();
            let c = lower.coefficients[index].clone();
            // (-c) * upper + a * lower cancels the variable, and both
            // multipliers are positive so the inequality directions hold.
            let scale_upper = -c;
            let scale_lower = a;
            let coefficients = upper
                .coefficients
                .iter()
                .zip(&lower.coefficients)
                .map(|(u, l)| &scale_upper * u + &scale_lower * l)
                .collect();
            kept.push(Row {
                coefficients,
                bound: &scale_upper * &upper.bound + &scale_lower * &lower.bound,
                strict: upper.strict || lower.strict,
            });
        }
    }
    Ok(kept)
}

/// Recovers one satisfying assignment from the elimination history.
fn back_substitute(history: &[Vec<Row>], variables: &[TimeVarId]) -> BTreeMap<TimeVarId, Q> {
    let mut values: Vec<Q> = alloc::vec![Q::zero(); variables.len()];
    let mut assigned = BTreeSet::new();

    // `history[k]` is the system before eliminating variable
    // `variables.len() - 1 - k`, so walking it backwards reintroduces
    // variables from the first to the last.
    for (step, rows) in history.iter().enumerate().rev() {
        let index = variables.len() - 1 - step;
        let mut lower: Option<(Q, bool)> = None;
        let mut upper: Option<(Q, bool)> = None;

        for row in rows {
            let coefficient = &row.coefficients[index];
            if coefficient.is_zero() {
                continue;
            }
            // Move every already-assigned term to the right-hand side.
            let mut rest = row.bound.clone();
            let mut pending = false;
            for (other, value) in row.coefficients.iter().enumerate() {
                if other == index || value.is_zero() {
                    continue;
                }
                if assigned.contains(&other) {
                    rest -= value * &values[other];
                } else {
                    pending = true;
                }
            }
            if pending {
                continue;
            }
            let limit = rest / coefficient;
            // On a tie the strictness accumulates: a bound met by both a
            // strict and a non-strict row is strict, and picking the endpoint
            // would violate the strict one.
            if coefficient.is_positive() {
                match &mut upper {
                    Some((best, best_strict)) if limit == *best => {
                        *best_strict = *best_strict || row.strict;
                    }
                    Some((best, best_strict)) if limit < *best => {
                        *best = limit;
                        *best_strict = row.strict;
                    }
                    Some(_) => {}
                    None => upper = Some((limit, row.strict)),
                }
            } else {
                match &mut lower {
                    Some((best, best_strict)) if limit == *best => {
                        *best_strict = *best_strict || row.strict;
                    }
                    Some((best, best_strict)) if limit > *best => {
                        *best = limit;
                        *best_strict = row.strict;
                    }
                    Some(_) => {}
                    None => lower = Some((limit, row.strict)),
                }
            }
        }

        values[index] = choose(lower, upper);
        assigned.insert(index);
    }

    variables.iter().cloned().zip(values).collect()
}

fn choose(lower: Option<(Q, bool)>, upper: Option<(Q, bool)>) -> Q {
    let one = Q::from(Z::from(1));
    let two = Q::from(Z::from(2));
    match (lower, upper) {
        (Some((low, _)), Some((high, _))) => {
            if low == high {
                low
            } else {
                (&low + &high) / &two
            }
        }
        (Some((low, strict)), None) => {
            if strict {
                low + one
            } else {
                low
            }
        }
        (None, Some((high, strict))) => {
            if strict {
                high - one
            } else {
                high
            }
        }
        (None, None) => Q::zero(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DifferenceConstraint, Exactness, ExternalPredicate, HybridTemporalProblem,
        LinearConstraint, LinearTemporalProblem, PositivityHandling, PredicateEvaluator,
        RatioConstraint, SolverProfile, StpProblem, TemporalOutcome, TimeVarId,
    };
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;
    use alloc::collections::BTreeMap;
    use alloc::string::{String, ToString};

    fn q(value: i64) -> Q {
        Q::from(Z::from(value))
    }

    #[test]
    fn f16_an_stp_contradiction_is_reported() {
        let mut problem = StpProblem::new();
        let t1 = problem.variable("t1");
        let t2 = problem.variable("t2");

        // t2 - t1 <= 1 and t2 - t1 >= 2.
        problem
            .constrain(DifferenceConstraint::at_most(&t1, &t2, q(1)))
            .unwrap();
        problem
            .constrain(DifferenceConstraint::at_least(&t1, &t2, q(2)))
            .unwrap();

        let outcome = problem.solve();
        assert!(!outcome.is_consistent());
        assert_eq!(outcome.profile(), SolverProfile::SimpleTemporal);
        assert!(matches!(outcome, TemporalOutcome::Inconsistent { .. }));
        assert!(outcome.assignment().is_none());
    }

    #[test]
    fn a_consistent_stp_is_solved_exactly_with_tight_bounds() {
        let mut problem = StpProblem::new();
        let start = problem.variable("start");
        let middle = problem.variable("middle");
        let end = problem.variable("end");

        problem
            .constrain(DifferenceConstraint::between(
                &start,
                &middle,
                Some(q(1)),
                Some(q(3)),
            ))
            .unwrap();
        problem
            .constrain(DifferenceConstraint::between(
                &middle,
                &end,
                Some(q(2)),
                Some(q(4)),
            ))
            .unwrap();

        let outcome = problem.solve();
        assert!(outcome.is_consistent());
        let TemporalOutcome::Solved {
            exactness,
            assignment,
            tight_bounds,
            ..
        } = &outcome
        else {
            panic!("expected a solution, got {outcome:?}");
        };
        assert_eq!(*exactness, Exactness::Exact);

        // The assignment really does satisfy the constraints.
        let elapsed = |a: &TimeVarId, b: &TimeVarId| &assignment[b] - &assignment[a];
        assert!(elapsed(&start, &middle) >= q(1) && elapsed(&start, &middle) <= q(3));
        assert!(elapsed(&middle, &end) >= q(2) && elapsed(&middle, &end) <= q(4));

        // And the implied start-to-end bound is tightened to [3, 7].
        let implied = tight_bounds
            .iter()
            .find(|bound| bound.from == start && bound.to == end)
            .expect("an implied bound between the endpoints");
        assert_eq!(implied.lower, Some(q(3)));
        assert_eq!(implied.upper, Some(q(7)));
    }

    #[test]
    fn undeclared_variables_are_rejected() {
        let mut problem = StpProblem::new();
        let known = problem.variable("known");
        let stranger = TimeVarId::new("stranger");
        assert!(matches!(
            problem.constrain(DifferenceConstraint::at_most(&known, &stranger, q(1))),
            Err(TimeError::UnknownTimeVariable { .. })
        ));
    }

    #[test]
    fn f17_a_ratio_constraint_is_not_a_difference_edge() {
        // "The second gap is between one and two times the first."
        let mut problem = LinearTemporalProblem::new();
        let a = problem.variable("a");
        let b = problem.variable("b");
        let c = problem.variable("c");

        let ratio = RatioConstraint {
            earlier: a.clone(),
            middle: b.clone(),
            later: c.clone(),
            lower: q(1),
            upper: q(2),
        };
        problem.constrain_ratio(&ratio).unwrap();

        // Cross-multiplication produces three-variable constraints with
        // non-unit coefficients. There is no difference bound among them, and
        // no API accepting one into `StpProblem`.
        assert_eq!(problem.constraints().len(), 3);
        let three_variable = problem
            .constraints()
            .iter()
            .filter(|constraint| constraint.coefficients.len() == 3)
            .count();
        assert_eq!(three_variable, 2, "the two ratio bounds touch all three");
        assert!(
            problem
                .constraints()
                .iter()
                .any(|constraint| constraint.strict),
            "the positive-denominator condition stays strict"
        );

        // Pin the first gap and the answer follows.
        problem
            .constrain(LinearConstraint::at_most([(a.clone(), q(1))], q(0)))
            .unwrap();
        problem
            .constrain(LinearConstraint::at_most([(a.clone(), q(-1))], q(0)))
            .unwrap();
        problem
            .constrain(LinearConstraint::at_most([(b.clone(), q(1))], q(2)))
            .unwrap();
        problem
            .constrain(LinearConstraint::at_most([(b.clone(), q(-1))], q(-2)))
            .unwrap();

        let outcome = problem.solve().unwrap();
        assert_eq!(outcome.profile(), SolverProfile::LinearRatio);
        assert!(outcome.is_consistent());
        let assignment = outcome.assignment().unwrap();
        let (first, second) = (
            &assignment[&b] - &assignment[&a],
            &assignment[&c] - &assignment[&b],
        );
        assert_eq!(first, q(2));
        assert!(
            second >= &first * q(1) && second <= &first * q(2),
            "{second}"
        );
    }

    #[test]
    fn f25_a_strict_positivity_condition_is_preserved_not_replaced() {
        let ratio = RatioConstraint {
            earlier: TimeVarId::new("a"),
            middle: TimeVarId::new("b"),
            later: TimeVarId::new("c"),
            lower: q(1),
            upper: q(2),
        };

        // The default handling keeps the strict inequality, so no positive
        // lower bound is invented.
        let mut strict = LinearTemporalProblem::new();
        for id in ["a", "b", "c"] {
            strict.variable(id);
        }
        assert_eq!(*strict.positivity(), PositivityHandling::StrictInequality);
        strict.constrain_ratio(&ratio).unwrap();

        let denominator = strict
            .constraints()
            .iter()
            .find(|constraint| constraint.strict)
            .expect("a strict constraint");
        assert_eq!(denominator.bound, q(0), "no delta was introduced anywhere");

        // The strict system is feasible, and it admits gaps as small as one
        // likes: nothing bounds the denominator away from zero.
        assert!(strict.solve().unwrap().is_consistent());

        // A delta may be declared, but only with the justification that
        // licenses it, and it does change the feasible set.
        let mut justified =
            LinearTemporalProblem::new().with_positivity(PositivityHandling::JustifiedDelta {
                delta: q(1),
                justification: String::from("the source specifies a minimum notated eighth"),
            });
        for id in ["a", "b", "c"] {
            justified.variable(id);
        }
        justified.constrain_ratio(&ratio).unwrap();
        assert!(
            justified
                .constraints()
                .iter()
                .all(|constraint| !constraint.strict),
            "the substitution replaced the strict constraint"
        );

        // The two systems differ: a half-unit denominator is feasible under
        // the strict reading and infeasible under the delta.
        let pin = |problem: &mut LinearTemporalProblem, gap: Q| {
            let a = TimeVarId::new("a");
            let b = TimeVarId::new("b");
            problem
                .constrain(LinearConstraint::at_most([(a.clone(), q(1))], q(0)))
                .unwrap();
            problem
                .constrain(LinearConstraint::at_most([(a, q(-1))], q(0)))
                .unwrap();
            problem
                .constrain(LinearConstraint::at_most([(b.clone(), q(1))], gap.clone()))
                .unwrap();
            problem
                .constrain(LinearConstraint::at_most([(b, q(-1))], -gap))
                .unwrap();
        };
        let half = Q::new(Z::from(1), Z::from(2));
        pin(&mut strict, half.clone());
        pin(&mut justified, half);
        assert!(strict.solve().unwrap().is_consistent());
        assert!(
            !justified.solve().unwrap().is_consistent(),
            "inventing a delta would have changed this answer"
        );
    }

    #[test]
    fn the_linear_profile_detects_an_impossible_system() {
        let mut problem = LinearTemporalProblem::new();
        let x = problem.variable("x");
        problem
            .constrain(LinearConstraint::at_most([(x.clone(), q(1))], q(1)))
            .unwrap();
        problem
            .constrain(LinearConstraint::at_most([(x, q(-1))], q(-3)))
            .unwrap();

        let outcome = problem.solve().unwrap();
        assert!(!outcome.is_consistent());
        assert_eq!(outcome.profile(), SolverProfile::LinearRatio);
        assert!(matches!(outcome, TemporalOutcome::Inconsistent { .. }));
    }

    #[test]
    fn a_strict_inequality_alone_can_make_a_system_infeasible() {
        // x < 0 and x >= 0 is infeasible only if strictness is respected.
        let mut problem = LinearTemporalProblem::new();
        let x = problem.variable("x");
        problem
            .constrain(LinearConstraint::less_than([(x.clone(), q(1))], q(0)))
            .unwrap();
        problem
            .constrain(LinearConstraint::at_most([(x, q(-1))], q(0)))
            .unwrap();
        assert!(!problem.solve().unwrap().is_consistent());
    }

    #[test]
    fn f18_an_external_predicate_stays_external() {
        let mut linear = LinearTemporalProblem::new();
        let entry = linear.variable("entry");
        linear
            .constrain(LinearConstraint::at_most([(entry, q(1))], q(30)))
            .unwrap();

        let mut problem = HybridTemporalProblem::new(linear);
        problem.add_predicate(ExternalPredicate {
            id: String::from("decay-1"),
            predicate_type: String::from("umt:predicate:acoustic-decay-threshold"),
            contract: String::from(
                "true once the measured level of the previous sound falls below the stated \
                 threshold, as reported by a configured detector",
            ),
            parameters: BTreeMap::from([
                ("threshold_db".to_string(), "-40".to_string()),
                ("source".to_string(), "voice:1".to_string()),
            ]),
        });

        // No detector configured: the predicate stays unresolved, and no
        // static decidability is claimed.
        let outcome = problem.solve(None).unwrap();
        assert!(!problem.claims_static_decidability());
        assert_eq!(outcome.unresolved(), ["decay-1"]);
        assert_eq!(outcome.profile(), SolverProfile::Hybrid);
        assert!(matches!(outcome, TemporalOutcome::PartiallySolved { .. }));
        assert!(
            outcome.is_consistent(),
            "the static part is consistent; the predicate is simply outstanding"
        );

        // The predicate carries data, never code.
        let predicate = &problem.predicates()[0];
        assert!(predicate.contract.contains("configured detector"));
        assert_eq!(predicate.parameters["threshold_db"], "-40");

        // With a detector configured it resolves.
        struct Detector;
        impl PredicateEvaluator for Detector {
            fn evaluate(&self, predicate: &ExternalPredicate) -> Option<bool> {
                (predicate.predicate_type == "umt:predicate:acoustic-decay-threshold")
                    .then_some(true)
            }
        }
        let resolved = problem.solve(Some(&Detector)).unwrap();
        assert!(resolved.unresolved().is_empty());
        assert!(matches!(resolved, TemporalOutcome::Solved { .. }));

        // And a detector that refuses to decide leaves it outstanding rather
        // than guessing.
        struct Undecided;
        impl PredicateEvaluator for Undecided {
            fn evaluate(&self, _: &ExternalPredicate) -> Option<bool> {
                None
            }
        }
        assert_eq!(
            problem.solve(Some(&Undecided)).unwrap().unresolved(),
            ["decay-1"]
        );
    }

    #[test]
    fn a_refuted_predicate_makes_the_network_inconsistent() {
        let mut problem = HybridTemporalProblem::new(LinearTemporalProblem::new());
        problem.add_predicate(ExternalPredicate {
            id: String::from("cue"),
            predicate_type: String::from("umt:predicate:conductor-cue"),
            contract: String::from("true once the conductor cues the entry"),
            parameters: BTreeMap::new(),
        });

        struct Never;
        impl PredicateEvaluator for Never {
            fn evaluate(&self, _: &ExternalPredicate) -> Option<bool> {
                Some(false)
            }
        }
        let outcome = problem.solve(Some(&Never)).unwrap();
        assert!(!outcome.is_consistent());
        assert!(matches!(outcome, TemporalOutcome::Inconsistent { .. }));
    }

    #[test]
    fn the_elimination_budget_is_reported_rather_than_exceeded_silently() {
        let mut problem = LinearTemporalProblem::new().with_budget(2);
        let x = problem.variable("x");
        let y = problem.variable("y");
        for bound in 0..4 {
            problem
                .constrain(LinearConstraint::at_most(
                    [(x.clone(), q(1)), (y.clone(), q(1))],
                    q(bound),
                ))
                .unwrap();
            // Opposite signs on `y`, so elimination has to pair every upper
            // bound with every lower bound and the system grows.
            problem
                .constrain(LinearConstraint::at_most(
                    [(x.clone(), q(-1)), (y.clone(), q(-2))],
                    q(bound),
                ))
                .unwrap();
        }
        assert!(matches!(
            problem.solve(),
            Err(TimeError::EliminationBudgetExceeded { budget: 2 })
        ));
    }

    #[test]
    fn a_difference_bound_can_also_be_stated_in_the_linear_profile() {
        // The linear profile subsumes STP; the point of keeping them apart is
        // that the STP profile refuses everything else, not that it is weaker.
        let mut problem = LinearTemporalProblem::new();
        let a = problem.variable("a");
        let b = problem.variable("b");
        problem
            .constrain_difference(&DifferenceConstraint::between(
                &a,
                &b,
                Some(q(2)),
                Some(q(5)),
            ))
            .unwrap();
        let outcome = problem.solve().unwrap();
        assert!(outcome.is_consistent());
        let assignment = outcome.assignment().unwrap();
        let gap = &assignment[&b] - &assignment[&a];
        assert!(gap >= q(2) && gap <= q(5), "{gap}");
    }
}
