//! Additive grids and quantization (UMT-3.2 section 5.7, prompt section 34).
//!
//! A device grid is `G_P = (1/P) Z` in a declared unit; here the unit is the
//! beat, so `P` is the familiar PPQN. Everything in this module is exact:
//! source coordinates are rationals, allocated positions are integers, and
//! residuals are rationals. No `f64` appears, so a residual reported here is
//! the true residual and not a rounding of one.
//!
//! # Quantizers return evidence, not numbers
//!
//! Prompt section 34 forbids `fn quantize(x: Q) -> i64`, and for a good
//! reason: the discarded part is the interesting part. Every operation here
//! returns a [`Quantized`] carrying the represented value *and* the exact
//! residual `e = x - i(q(x))`.
//!
//! # Four profiles, with different laws
//!
//! - **Floor** and **ceiling** are the order adjunctions `i |- q_down` and
//!   `q_up |- i` of section 5.7.1. They are monotone, they fix grid values,
//!   and their residuals are one-signed.
//! - **Nearest** is *not* one of those adjunctions. It fixes grid values and
//!   its residual is bounded by half a grid step, but no universal one-sided
//!   inequality holds, and section 5.7.2 requires its tie policy to be
//!   declared - which is why every entry point takes a
//!   [`RoundingConvention`].
//! - **Endpoint-preserving allocation** (section 5.7.5) rounds cumulative
//!   boundaries rather than individual durations, so the children sum to the
//!   parent exactly.
//!
//! The difference between the last two is fixtures F12 and F13. Five equal
//! children of a 96-tick parent are `96/5 = 19.2` ticks each; flooring each
//! independently gives five 19s and loses a tick, while rounding the
//! boundaries gives `19, 19, 20, 19, 19` and loses nothing.
//!
//! # Examples
//!
//! ```
//! use umt::algebra::{Q, RoundingConvention, Z};
//! use umt::time::{AllocationPolicy, TickGrid};
//!
//! let grid = TickGrid::new(96)?;
//! let weights = vec![Q::from(Z::from(1)); 5];
//!
//! // Independent local flooring drifts (fixture F12).
//! let naive = grid.allocate_locally(&weights, &Z::from(96), RoundingConvention::Floor)?;
//! assert_eq!(naive.child_ticks(), [19, 19, 19, 19, 19].map(Z::from));
//! assert_eq!(naive.total_ticks(), Z::from(95));
//! assert!(!naive.endpoint_preserved());
//!
//! // Rounding the boundaries does not (fixture F13).
//! let exact = grid
//!     .allocate_preserving_endpoint(&weights, &Z::from(96), &AllocationPolicy::default())?
//!     .into_allocation()
//!     .expect("feasible");
//! assert_eq!(exact.child_ticks(), [19, 19, 20, 19, 19].map(Z::from));
//! assert_eq!(exact.total_ticks(), Z::from(96));
//! assert!(exact.endpoint_preserved());
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

use alloc::vec::Vec;
use num_traits::{One, Signed, Zero};

use crate::algebra::{Q, RoundingConvention, Z};
use crate::error::TimeError;
use crate::time::beat::{BeatSpan, BeatTime, Beats};
use crate::time::rhythm::RhythmTree;

/// A quantized value together with the exact evidence of what was lost
/// (prompt section 34).
///
/// UMT layer: L4 value, L1 residual. The residual convention throughout this
/// crate is `e = x - i(q(x))`, matching UMT-3.2 section 5.7.1: positive when
/// the source lay above the represented value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quantized<T, R> {
    /// The represented value.
    pub value: T,
    /// The exact residual `x - i(q(x))`.
    pub residual: R,
    /// The convention that produced it.
    pub convention: RoundingConvention,
}

/// An additive device grid `G_P = (1/P) Z`, in beats
/// (UMT-3.2 section 5.7).
///
/// UMT layer: L4 structure, exact arithmetic. `P` is pulses per quarter note,
/// since the declared beat unit is the quarter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u32", into = "u32"))]
pub struct TickGrid {
    ticks_per_beat: u32,
}

impl TickGrid {
    /// Declares a grid of `ticks_per_beat` pulses to the beat.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyCycle`] for a resolution of zero, which is
    /// not a grid.
    pub fn new(ticks_per_beat: u32) -> Result<Self, TimeError> {
        if ticks_per_beat == 0 {
            return Err(TimeError::EmptyCycle);
        }
        Ok(Self { ticks_per_beat })
    }

    /// The resolution `P`.
    #[must_use]
    pub fn ticks_per_beat(self) -> u32 {
        self.ticks_per_beat
    }

    /// The exact duration of one tick.
    #[must_use]
    pub fn tick_duration(self) -> Beats {
        Beats::new(Q::new(Z::one(), Z::from(self.ticks_per_beat)))
    }

    /// The inclusion `i: G_P -> T_b`: the exact structural time of a tick.
    #[must_use]
    pub fn tick_time(self, ticks: &Z) -> BeatTime {
        BeatTime::new(Q::new(ticks.clone(), Z::from(self.ticks_per_beat)))
    }

    /// The exact grid coordinate of a structural position, before rounding.
    #[must_use]
    pub fn coordinate(self, at: &BeatTime) -> Q {
        at.get() * Q::from(Z::from(self.ticks_per_beat))
    }

    /// The exact grid coordinate of a structural duration, before rounding.
    #[must_use]
    pub fn duration_coordinate(self, duration: &Beats) -> Q {
        duration.get() * Q::from(Z::from(self.ticks_per_beat))
    }

    /// Whether a position is exactly representable on this grid.
    #[must_use]
    pub fn represents(self, at: &BeatTime) -> bool {
        self.coordinate(at).is_integer()
    }

    /// Quantizes a structural position to the grid.
    ///
    /// The residual is `x - i(q(x))` in beats, exactly.
    #[must_use]
    pub fn quantize(self, at: &BeatTime, convention: RoundingConvention) -> Quantized<Z, Beats> {
        let coordinate = self.coordinate(at);
        let ticks = convention.apply_q(&coordinate);
        let residual = self.tick_time(&ticks).interval_to(at);
        Quantized {
            value: ticks,
            residual,
            convention,
        }
    }

    /// Quantizes a structural duration to a whole number of ticks.
    #[must_use]
    pub fn quantize_duration(
        self,
        duration: &Beats,
        convention: RoundingConvention,
    ) -> Quantized<Z, Beats> {
        let coordinate = self.duration_coordinate(duration);
        let ticks = convention.apply_q(&coordinate);
        let represented = Beats::new(Q::new(ticks.clone(), Z::from(self.ticks_per_beat)));
        Quantized {
            value: ticks,
            residual: duration - &represented,
            convention,
        }
    }

    /// Independent per-child rounding, which can drift
    /// (UMT-3.2 section 5.7.4, fixture F12).
    ///
    /// Each child duration is rounded on its own, so the children need not sum
    /// to the parent. That is not a defect of the grid but of the *method*,
    /// and this constructor exists so the difference can be demonstrated
    /// rather than described.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyDivision`] for no children,
    /// [`TimeError::NonPositiveWeight`] for a non-positive weight, and
    /// [`TimeError::NegativeSpan`] for a parent of negative length.
    pub fn allocate_locally(
        self,
        weights: &[Q],
        parent_ticks: &Z,
        convention: RoundingConvention,
    ) -> Result<GridAllocation, TimeError> {
        let total = validate_weights(weights)?;
        if parent_ticks.is_negative() {
            return Err(TimeError::NegativeSpan);
        }
        let parent = Q::from(parent_ticks.clone());

        let children = weights
            .iter()
            .map(|weight| {
                let exact = &parent * weight / &total;
                let ticks = convention.apply_q(&exact);
                AllocatedChild {
                    residual: self.residual_beats(&exact, &ticks),
                    exact_ticks: exact,
                    ticks,
                }
            })
            .collect();

        Ok(GridAllocation {
            grid: self,
            parent_ticks: parent_ticks.clone(),
            children,
            convention,
            endpoint_preserving: false,
            collapsed: Vec::new(),
        })
    }

    /// Boundary-rounding allocation, which does not drift
    /// (UMT-3.2 section 5.7.5, fixture F13).
    ///
    /// Cumulative boundaries are rounded rather than individual durations, so
    /// the integer children sum to the parent exactly. Every child residual is
    /// still reported relative to its exact structural duration, as section
    /// 9.8 requires.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyDivision`] for no children,
    /// [`TimeError::NonPositiveWeight`] for a non-positive weight, and
    /// [`TimeError::NegativeSpan`] for a parent of negative length.
    pub fn allocate_preserving_endpoint(
        self,
        weights: &[Q],
        parent_ticks: &Z,
        policy: &AllocationPolicy,
    ) -> Result<AllocationOutcome, TimeError> {
        let total = validate_weights(weights)?;
        if parent_ticks.is_negative() {
            return Err(TimeError::NegativeSpan);
        }
        let parent = Q::from(parent_ticks.clone());
        let convention = policy.convention;

        // Feasibility first: the declared minimum cannot be met if the parent
        // is too short, and no rounding policy can rescue that. A minimum of
        // zero declares no minimum, so it can never be the reason for an
        // infeasible allocation.
        let required = &policy.minimum_ticks * Z::from(weights.len());
        if policy.minimum_ticks.is_positive() && required > *parent_ticks {
            let infeasibility = AllocationInfeasibility::MinimumSpan {
                children: weights.len(),
                required_ticks: required,
                available_ticks: parent_ticks.clone(),
            };
            if policy.on_infeasible == CollisionPolicy::Report {
                return Ok(AllocationOutcome::Infeasible {
                    reason: infeasibility,
                });
            }
        }

        let mut children = Vec::with_capacity(weights.len());
        let mut cumulative = Q::zero();
        let mut previous_boundary = Z::zero();
        for (index, weight) in weights.iter().enumerate() {
            let exact_start = &parent * &cumulative / &total;
            cumulative += weight;
            let exact_end = &parent * &cumulative / &total;
            let boundary = if index + 1 == weights.len() {
                parent_ticks.clone()
            } else {
                convention.apply_q(&exact_end)
            };
            let exact_ticks = &exact_end - &exact_start;
            let ticks = &boundary - &previous_boundary;
            children.push(AllocatedChild {
                residual: self.residual_beats(&exact_ticks, &ticks),
                exact_ticks,
                ticks,
            });
            previous_boundary = boundary;
        }

        // Whatever the rounding produced, check it against the declared
        // minimum rather than assuming boundary rounding respected it.
        let collapsed: Vec<usize> = children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.ticks < policy.minimum_ticks)
            .map(|(index, _)| index)
            .collect();

        if !collapsed.is_empty() && policy.on_infeasible == CollisionPolicy::Report {
            return Ok(AllocationOutcome::Infeasible {
                reason: AllocationInfeasibility::MinimumSpan {
                    children: weights.len(),
                    required_ticks: &policy.minimum_ticks * Z::from(weights.len()),
                    available_ticks: parent_ticks.clone(),
                },
            });
        }

        Ok(AllocationOutcome::Allocated(GridAllocation {
            grid: self,
            parent_ticks: parent_ticks.clone(),
            children,
            convention,
            endpoint_preserving: true,
            collapsed,
        }))
    }

    /// Hierarchical constrained quantization of a rhythm tree
    /// (UMT-3.2 section 5.7.6, fixture F14).
    ///
    /// The parent endpoints are fixed first, integer child spans are
    /// distributed inside them, and the procedure recurses. The *tree* is the
    /// input, not a flattened tick sequence, so re-realizing at a different
    /// resolution starts from the exact source rather than compounding a
    /// previous rounding.
    ///
    /// # Errors
    ///
    /// Propagates weight validation.
    pub fn quantize_tree(
        self,
        tree: &RhythmTree,
        span: &BeatSpan,
        policy: &AllocationPolicy,
    ) -> Result<QuantizedNode, TimeError> {
        let start = self.quantize(span.start(), policy.convention);
        let end = self.quantize(span.end(), policy.convention);
        self.quantize_subtree(tree, span, &start.value, &end.value, policy)
    }

    fn quantize_subtree(
        self,
        tree: &RhythmTree,
        span: &BeatSpan,
        start_tick: &Z,
        end_tick: &Z,
        policy: &AllocationPolicy,
    ) -> Result<QuantizedNode, TimeError> {
        let exact_ticks = self.duration_coordinate(&span.duration());
        let allocated = end_tick - start_tick;
        let mut node = QuantizedNode {
            start_tick: start_tick.clone(),
            end_tick: end_tick.clone(),
            source: span.clone(),
            residual: self.residual_beats(&exact_ticks, &allocated),
            children: Vec::new(),
            issue: None,
        };

        if tree.is_leaf() {
            return Ok(node);
        }

        let weights: Vec<Q> = tree
            .children()
            .iter()
            .map(|child| child.weight().clone())
            .collect();
        let outcome = self.allocate_preserving_endpoint(&weights, &allocated, policy)?;
        let allocation = match outcome {
            AllocationOutcome::Allocated(allocation) => allocation,
            AllocationOutcome::Infeasible { reason } => {
                // Stop here rather than descend into a span that cannot hold
                // its children. Section 5.7.6 requires the report; it does not
                // permit silently violating the constraint below it.
                node.issue = Some(reason);
                return Ok(node);
            }
        };

        let child_spans = tree.child_spans(span)?;
        let mut boundary = start_tick.clone();
        for ((child, child_span), ticks) in tree
            .children()
            .iter()
            .zip(child_spans)
            .zip(allocation.child_ticks())
        {
            let child_end = &boundary + &ticks;
            node.children.push(self.quantize_subtree(
                child,
                &child_span,
                &boundary,
                &child_end,
                policy,
            )?);
            boundary = child_end;
        }
        Ok(node)
    }

    fn residual_beats(self, exact_ticks: &Q, allocated: &Z) -> Beats {
        Beats::new(
            (exact_ticks - Q::from(allocated.clone())) / Q::from(Z::from(self.ticks_per_beat)),
        )
    }
}

impl TryFrom<u32> for TickGrid {
    type Error = TimeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TickGrid> for u32 {
    fn from(value: TickGrid) -> Self {
        value.ticks_per_beat
    }
}

impl core::fmt::Display for TickGrid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ticks/beat", self.ticks_per_beat)
    }
}

fn validate_weights(weights: &[Q]) -> Result<Q, TimeError> {
    if weights.is_empty() {
        return Err(TimeError::EmptyDivision);
    }
    let mut total = Q::zero();
    for weight in weights {
        if !weight.is_positive() {
            return Err(TimeError::NonPositiveWeight);
        }
        total += weight;
    }
    Ok(total)
}

/// What to do when a declared minimum span cannot be met
/// (UMT-3.2 section 5.7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CollisionPolicy {
    /// Report infeasibility and allocate nothing. The default, because
    /// section 5.7.6 forbids silently violating the constraint.
    #[default]
    Report,
    /// Allocate anyway, recording which children fell below the minimum.
    ///
    /// Legitimate only when the application has explicitly decided that a
    /// collision or collapse is acceptable.
    AllowCollapse,
}

/// The declared policy for a constrained allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AllocationPolicy {
    /// The rounding convention, including its tie policy
    /// (UMT-3.2 section 5.7.2).
    pub convention: RoundingConvention,
    /// The minimum number of ticks each child must receive.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::z"))]
    pub minimum_ticks: Z,
    /// What to do when the minimum cannot be met.
    pub on_infeasible: CollisionPolicy,
}

impl Default for AllocationPolicy {
    /// Nearest rounding with halves away from zero, no minimum span, and
    /// infeasibility reported rather than collapsed.
    fn default() -> Self {
        Self {
            convention: RoundingConvention::NearestHalfAwayFromZero,
            minimum_ticks: Z::zero(),
            on_infeasible: CollisionPolicy::Report,
        }
    }
}

impl AllocationPolicy {
    /// Requires each child to receive at least this many ticks.
    #[must_use]
    pub fn with_minimum_ticks(mut self, ticks: u32) -> Self {
        self.minimum_ticks = Z::from(ticks);
        self
    }

    /// Permits a collapse rather than reporting infeasibility.
    #[must_use]
    pub fn allowing_collapse(mut self) -> Self {
        self.on_infeasible = CollisionPolicy::AllowCollapse;
        self
    }
}

/// Why a constrained allocation could not be satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AllocationInfeasibility {
    /// The children cannot all reach the declared minimum span.
    MinimumSpan {
        /// How many children were to be placed.
        children: usize,
        /// Ticks the minimum requires in total.
        #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::z"))]
        required_ticks: Z,
        /// Ticks the parent actually has.
        #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::z"))]
        available_ticks: Z,
    },
}

/// The result of a constrained allocation.
///
/// Infeasibility is an outcome, not an error: three children that each need a
/// tick genuinely do not fit in a two-tick parent, and the caller asked a
/// well-formed question (fixture F27).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AllocationOutcome {
    /// Every child received a span.
    Allocated(GridAllocation),
    /// The declared constraints cannot be met.
    Infeasible {
        /// What could not be satisfied.
        reason: AllocationInfeasibility,
    },
}

impl AllocationOutcome {
    /// The allocation, where there is one.
    #[must_use]
    pub fn allocation(&self) -> Option<&GridAllocation> {
        match self {
            Self::Allocated(allocation) => Some(allocation),
            Self::Infeasible { .. } => None,
        }
    }

    /// Consumes the outcome, yielding the allocation where there is one.
    #[must_use]
    pub fn into_allocation(self) -> Option<GridAllocation> {
        match self {
            Self::Allocated(allocation) => Some(allocation),
            Self::Infeasible { .. } => None,
        }
    }

    /// Whether the allocation succeeded.
    #[must_use]
    pub fn is_feasible(&self) -> bool {
        matches!(self, Self::Allocated(_))
    }
}

/// One child of an allocation, with its exact residual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocatedChild {
    ticks: Z,
    exact_ticks: Q,
    residual: Beats,
}

impl AllocatedChild {
    /// The integer ticks this child received.
    #[must_use]
    pub fn ticks(&self) -> &Z {
        &self.ticks
    }

    /// The exact structural duration this child should have had, in ticks.
    #[must_use]
    pub fn exact_ticks(&self) -> &Q {
        &self.exact_ticks
    }

    /// The exact residual, in beats: structural duration minus allocated.
    #[must_use]
    pub fn residual(&self) -> &Beats {
        &self.residual
    }
}

/// An integer allocation of a parent span among weighted children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridAllocation {
    grid: TickGrid,
    parent_ticks: Z,
    children: Vec<AllocatedChild>,
    convention: RoundingConvention,
    endpoint_preserving: bool,
    collapsed: Vec<usize>,
}

impl GridAllocation {
    /// The grid this allocation is on.
    #[must_use]
    pub fn grid(&self) -> TickGrid {
        self.grid
    }

    /// The convention that produced it.
    #[must_use]
    pub fn convention(&self) -> RoundingConvention {
        self.convention
    }

    /// The parent's tick count.
    #[must_use]
    pub fn parent_ticks(&self) -> &Z {
        &self.parent_ticks
    }

    /// The children, with their residuals.
    #[must_use]
    pub fn children(&self) -> &[AllocatedChild] {
        &self.children
    }

    /// Just the integer tick counts, in order.
    #[must_use]
    pub fn child_ticks(&self) -> Vec<Z> {
        self.children
            .iter()
            .map(|child| child.ticks.clone())
            .collect()
    }

    /// The sum of the children's ticks.
    #[must_use]
    pub fn total_ticks(&self) -> Z {
        self.children
            .iter()
            .fold(Z::zero(), |total, child| total + &child.ticks)
    }

    /// Whether the children sum to the parent, so no endpoint was lost.
    #[must_use]
    pub fn endpoint_preserved(&self) -> bool {
        self.total_ticks() == self.parent_ticks
    }

    /// Whether this allocation used a boundary-rounding method.
    ///
    /// Distinct from [`GridAllocation::endpoint_preserved`]: an independent
    /// local rounding can happen to land on the endpoint without having been
    /// constructed to.
    #[must_use]
    pub fn is_endpoint_preserving_method(&self) -> bool {
        self.endpoint_preserving
    }

    /// Ticks by which the children miss the parent's endpoint.
    #[must_use]
    pub fn endpoint_drift(&self) -> Z {
        &self.parent_ticks - self.total_ticks()
    }

    /// Indices of children that fell below the declared minimum span.
    ///
    /// Non-empty only under [`CollisionPolicy::AllowCollapse`], where a
    /// collapse was explicitly permitted.
    #[must_use]
    pub fn collapsed(&self) -> &[usize] {
        &self.collapsed
    }

    /// The largest absolute child residual, in beats.
    #[must_use]
    pub fn worst_residual(&self) -> Beats {
        self.children
            .iter()
            .map(|child| child.residual.abs())
            .max()
            .unwrap_or_else(Beats::zero)
    }
}

/// One node of a hierarchically quantized rhythm tree
/// (UMT-3.2 section 5.7.6).
///
/// UMT layer: L4 tick spans over an L1 exact source. The source span is kept
/// at every node, so a later re-realization at a different resolution can go
/// back to the structure rather than compound this one's rounding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizedNode {
    start_tick: Z,
    end_tick: Z,
    source: BeatSpan,
    residual: Beats,
    children: Vec<QuantizedNode>,
    issue: Option<AllocationInfeasibility>,
}

impl QuantizedNode {
    /// The first tick of this node's span.
    #[must_use]
    pub fn start_tick(&self) -> &Z {
        &self.start_tick
    }

    /// The tick one past the end of this node's span.
    #[must_use]
    pub fn end_tick(&self) -> &Z {
        &self.end_tick
    }

    /// The allocated length in ticks.
    #[must_use]
    pub fn tick_count(&self) -> Z {
        &self.end_tick - &self.start_tick
    }

    /// The exact structural span this node came from.
    #[must_use]
    pub fn source(&self) -> &BeatSpan {
        &self.source
    }

    /// The exact residual for this node, in beats.
    #[must_use]
    pub fn residual(&self) -> &Beats {
        &self.residual
    }

    /// The quantized children, in order.
    #[must_use]
    pub fn children(&self) -> &[QuantizedNode] {
        &self.children
    }

    /// Whether this node is a leaf of the quantized tree.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// The infeasibility reported at this node, if any.
    #[must_use]
    pub fn issue(&self) -> Option<&AllocationInfeasibility> {
        self.issue.as_ref()
    }

    /// Whether this node and everything under it allocated successfully.
    #[must_use]
    pub fn is_feasible(&self) -> bool {
        self.issue.is_none() && self.children.iter().all(QuantizedNode::is_feasible)
    }

    /// The tick spans of the leaves, in order.
    #[must_use]
    pub fn leaf_ticks(&self) -> Vec<(Z, Z)> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    /// Every residual in the subtree, in traversal order.
    #[must_use]
    pub fn residuals(&self) -> Vec<Beats> {
        let mut out = alloc::vec![self.residual.clone()];
        for child in &self.children {
            out.extend(child.residuals());
        }
        out
    }

    fn collect_leaves(&self, out: &mut Vec<(Z, Z)>) {
        if self.is_leaf() {
            out.push((self.start_tick.clone(), self.end_tick.clone()));
            return;
        }
        for child in &self.children {
            child.collect_leaves(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AllocationInfeasibility, AllocationPolicy, TickGrid};
    use crate::algebra::RoundingConvention::{
        Ceiling, Floor, NearestHalfAwayFromZero, NearestHalfToEven,
    };
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;
    use crate::time::beat::{BeatSpan, BeatTime, Beats};
    use crate::time::rhythm::RhythmTree;
    use alloc::vec::Vec;
    use num_traits::Zero;

    fn ones(count: usize) -> Vec<Q> {
        alloc::vec![Q::from(Z::from(1)); count]
    }

    fn grid() -> TickGrid {
        TickGrid::new(96).unwrap()
    }

    #[test]
    fn a_quantizer_returns_the_residual_with_the_value() {
        // Three sevenths of a beat on a 96-tick grid: 288/7 = 41.142857...
        let at = BeatTime::ratio(3, 7).unwrap();
        let floored = grid().quantize(&at, Floor);
        assert_eq!(floored.value, Z::from(41));
        assert_eq!(floored.convention, Floor);
        // Residual is positive under floor: the source lies above the tick.
        assert!(floored.residual.is_positive());
        assert_eq!(
            floored.residual,
            Beats::new(Q::new(Z::from(3), Z::from(7)) - Q::new(Z::from(41), Z::from(96)))
        );

        let ceiled = grid().quantize(&at, Ceiling);
        assert_eq!(ceiled.value, Z::from(42));
        assert!(!ceiled.residual.is_positive(), "ceiling residual is <= 0");
    }

    #[test]
    fn quantizers_are_the_identity_on_represented_values() {
        // UMT-3.2 section 5.7.3: q(i(g)) = g for every convention.
        for ticks in [-97i64, -1, 0, 1, 48, 96, 193] {
            let at = grid().tick_time(&Z::from(ticks));
            assert!(grid().represents(&at));
            for convention in [Floor, Ceiling, NearestHalfAwayFromZero, NearestHalfToEven] {
                let quantized = grid().quantize(&at, convention);
                assert_eq!(
                    quantized.value,
                    Z::from(ticks),
                    "{ticks} under {convention:?}"
                );
                assert!(quantized.residual.is_zero());
            }
        }
    }

    #[test]
    fn floor_and_ceiling_bracket_nearest_and_the_tie_policy_matters() {
        // Exactly half a tick past tick 3, on a 2-tick-per-beat grid.
        let coarse = TickGrid::new(2).unwrap();
        let at = BeatTime::ratio(7, 4).unwrap(); // 3.5 ticks
        assert_eq!(coarse.quantize(&at, Floor).value, Z::from(3));
        assert_eq!(coarse.quantize(&at, Ceiling).value, Z::from(4));
        assert_eq!(
            coarse.quantize(&at, NearestHalfAwayFromZero).value,
            Z::from(4)
        );
        assert_eq!(coarse.quantize(&at, NearestHalfToEven).value, Z::from(4));

        // And at 2.5 ticks the two nearest policies disagree, which is why
        // section 5.7.2 requires the policy to be declared.
        let at = BeatTime::ratio(5, 4).unwrap();
        assert_eq!(
            coarse.quantize(&at, NearestHalfAwayFromZero).value,
            Z::from(3)
        );
        assert_eq!(coarse.quantize(&at, NearestHalfToEven).value, Z::from(2));

        // Negative halves go away from zero, not down.
        let at = BeatTime::ratio(-5, 4).unwrap();
        assert_eq!(
            coarse.quantize(&at, NearestHalfAwayFromZero).value,
            Z::from(-3)
        );
        assert_eq!(coarse.quantize(&at, Floor).value, Z::from(-3));
        assert_eq!(coarse.quantize(&at, Ceiling).value, Z::from(-2));
    }

    #[test]
    fn f12_independent_local_flooring_drifts() {
        let allocation = grid()
            .allocate_locally(&ones(5), &Z::from(96), Floor)
            .unwrap();
        assert_eq!(
            allocation.child_ticks(),
            [19, 19, 19, 19, 19].map(Z::from),
            "96/5 = 19.2 ticks, floored"
        );
        assert_eq!(allocation.total_ticks(), Z::from(95));
        assert!(!allocation.endpoint_preserved());
        assert_eq!(allocation.endpoint_drift(), Z::from(1));
        assert!(!allocation.is_endpoint_preserving_method());

        // Every child residual is reported, exactly.
        for child in allocation.children() {
            assert_eq!(*child.exact_ticks(), Q::new(Z::from(96), Z::from(5)));
            assert_eq!(
                *child.residual(),
                Beats::new(Q::new(Z::from(1), Z::from(5)) / Q::from(Z::from(96)))
            );
        }
    }

    #[test]
    fn f13_boundary_rounding_preserves_the_endpoint() {
        let allocation = grid()
            .allocate_preserving_endpoint(&ones(5), &Z::from(96), &AllocationPolicy::default())
            .unwrap()
            .into_allocation()
            .unwrap();

        assert_eq!(
            allocation.child_ticks(),
            [19, 19, 20, 19, 19].map(Z::from),
            "the boundaries 0, 19, 38, 58, 77, 96 of section 5.7.5"
        );
        assert_eq!(allocation.total_ticks(), Z::from(96));
        assert!(allocation.endpoint_preserved());
        assert!(allocation.is_endpoint_preserving_method());
        assert!(allocation.endpoint_drift().is_zero());

        // Residuals are relative to the exact structural child duration, and
        // they are not all equal: the middle child is a tick long.
        let residuals: Vec<Beats> = allocation
            .children()
            .iter()
            .map(|child| child.residual().clone())
            .collect();
        assert!(residuals[0].is_positive(), "19 ticks is short of 19.2");
        assert!(!residuals[2].is_positive(), "20 ticks overshoots 19.2");
        assert_eq!(
            residuals.iter().cloned().sum::<Beats>(),
            Beats::zero(),
            "the residuals cancel exactly, which is what preserving the endpoint means"
        );
    }

    #[test]
    fn f27_an_infeasible_positive_span_allocation_is_reported() {
        // Three children, each requiring at least one tick, into two ticks.
        let policy = AllocationPolicy::default().with_minimum_ticks(1);
        let outcome = grid()
            .allocate_preserving_endpoint(&ones(3), &Z::from(2), &policy)
            .unwrap();

        assert!(!outcome.is_feasible());
        assert!(outcome.allocation().is_none());
        assert_eq!(
            outcome,
            super::AllocationOutcome::Infeasible {
                reason: AllocationInfeasibility::MinimumSpan {
                    children: 3,
                    required_ticks: Z::from(3),
                    available_ticks: Z::from(2),
                }
            }
        );

        // A collapse is available, but only by naming it, and it says which
        // children collapsed.
        let permitted = policy.clone().allowing_collapse();
        let allocation = grid()
            .allocate_preserving_endpoint(&ones(3), &Z::from(2), &permitted)
            .unwrap()
            .into_allocation()
            .unwrap();
        assert_eq!(allocation.total_ticks(), Z::from(2));
        assert!(!allocation.collapsed().is_empty());

        // With one more tick it fits.
        let fits = grid()
            .allocate_preserving_endpoint(&ones(3), &Z::from(3), &policy)
            .unwrap();
        assert!(fits.is_feasible());
        assert_eq!(
            fits.into_allocation().unwrap().child_ticks(),
            [1, 1, 1].map(Z::from)
        );
    }

    #[test]
    fn f14_a_nested_tree_re_realizes_from_the_source_at_any_resolution() {
        // Five inside three: a quintuplet filling the first third of a beat.
        let tree = RhythmTree::division([
            RhythmTree::equal_division(5).unwrap(),
            RhythmTree::leaf(1).unwrap(),
            RhythmTree::leaf(1).unwrap(),
        ])
        .unwrap();
        let beat = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(1, 1).unwrap()).unwrap();
        let policy = AllocationPolicy::default();

        let coarse = TickGrid::new(96)
            .unwrap()
            .quantize_tree(&tree, &beat, &policy)
            .unwrap();
        let fine = TickGrid::new(960)
            .unwrap()
            .quantize_tree(&tree, &beat, &policy)
            .unwrap();

        assert!(coarse.is_feasible() && fine.is_feasible());
        assert_eq!(coarse.tick_count(), Z::from(96));
        assert_eq!(fine.tick_count(), Z::from(960));

        // Both keep the exact source span at every node, so neither derives
        // from the other.
        assert_eq!(coarse.source(), fine.source());
        assert_eq!(coarse.children()[0].source(), fine.children()[0].source());
        assert_eq!(
            coarse.children()[0].source().duration(),
            Beats::ratio(1, 3).unwrap()
        );

        // Children tile the parent exactly at both resolutions.
        for node in [&coarse, &fine] {
            let leaves = node.leaf_ticks();
            assert_eq!(leaves.len(), 7);
            assert_eq!(leaves[0].0, *node.start_tick());
            assert_eq!(leaves[6].1, *node.end_tick());
            for pair in leaves.windows(2) {
                assert_eq!(pair[0].1, pair[1].0);
            }
        }

        // The finer grid is strictly better, and re-quantizing the coarse tick
        // sequence could not have produced it: 32/5 = 6.4 ticks per quintuplet
        // note at P=96, against 64 at P=960.
        assert_eq!(coarse.leaf_ticks()[0], (Z::from(0), Z::from(6)));
        assert_eq!(fine.leaf_ticks()[0], (Z::from(0), Z::from(64)));
        let coarse_worst = coarse
            .residuals()
            .into_iter()
            .map(|r| r.abs())
            .max()
            .unwrap();
        let fine_worst = fine.residuals().into_iter().map(|r| r.abs()).max().unwrap();
        assert!(fine_worst < coarse_worst, "a finer grid loses less");
    }

    #[test]
    fn a_hierarchical_quantization_reports_an_infeasible_subtree() {
        // A three-way division inside a span too short to hold it.
        let tree = RhythmTree::division([
            RhythmTree::equal_division(3).unwrap(),
            RhythmTree::leaf(23).unwrap(),
        ])
        .unwrap();
        let beat = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(1, 1).unwrap()).unwrap();
        let coarse = TickGrid::new(24).unwrap();
        let policy = AllocationPolicy::default().with_minimum_ticks(1);

        let node = coarse.quantize_tree(&tree, &beat, &policy).unwrap();
        assert!(!node.is_feasible());
        let first = &node.children()[0];
        assert_eq!(
            first.tick_count(),
            Z::from(1),
            "one tick for three children"
        );
        assert!(first.issue().is_some());
        assert!(first.is_leaf(), "no descent into an infeasible node");
    }

    #[test]
    fn grids_and_weights_are_validated() {
        assert_eq!(TickGrid::new(0), Err(TimeError::EmptyCycle));
        assert_eq!(
            grid().allocate_locally(&[], &Z::from(96), Floor),
            Err(TimeError::EmptyDivision)
        );
        assert_eq!(
            grid().allocate_locally(&[Q::from(Z::from(0))], &Z::from(96), Floor),
            Err(TimeError::NonPositiveWeight)
        );
        assert_eq!(grid().to_string(), "96 ticks/beat");
    }

    #[test]
    fn unequal_weights_allocate_proportionally() {
        // 2 + 2 + 3 of a 96-tick parent: 27.43, 27.43, 41.14 ticks.
        let weights = [2, 2, 3].map(|w| Q::from(Z::from(w)));
        let allocation = grid()
            .allocate_preserving_endpoint(&weights, &Z::from(96), &AllocationPolicy::default())
            .unwrap()
            .into_allocation()
            .unwrap();
        assert_eq!(allocation.total_ticks(), Z::from(96));
        assert_eq!(allocation.child_ticks(), [27, 28, 41].map(Z::from));
        assert!(allocation.endpoint_preserved());
    }
}
