//! Hierarchical and cyclic rhythm (UMT-3.2 section 5.3, prompt section 27).
//!
//! A [`RhythmTree`] is a rooted ordered tree whose internal nodes carry
//! positive exact child weights. Children divide the parent span in proportion
//! to those weights. That one structure covers equal divisive subdivision,
//! tuplets, additive grouping such as `2+2+3`, and arbitrarily nested mixtures
//! of the three - and it covers them *exactly*, because the weights are
//! rationals and the division is rational arithmetic.
//!
//! # Flattening is lossy, and the tree stays the source
//!
//! [`RhythmTree::flatten`] produces the leaf spans in order. Section 5.3.3 is
//! explicit that this loses information: different trees flatten to the same
//! boundaries, so a round trip must retain the tree rather than expect to
//! reconstruct it. This crate does not offer a function that infers a tree
//! from onset times, and [`crate::time::quantize`] takes the *tree* rather
//! than a flattened tick list for exactly that reason (fixture F14).
//!
//! # Examples
//!
//! An additive `2+2+3` grouping flattens to exact boundaries (fixture F11):
//!
//! ```
//! use umt::time::{BeatSpan, BeatTime, Beats, RhythmTree};
//!
//! let bar = RhythmTree::division([
//!     RhythmTree::leaf(2)?,
//!     RhythmTree::leaf(2)?,
//!     RhythmTree::leaf(3)?,
//! ])?;
//!
//! // Seven units of span, so one weight unit is one unit of time.
//! let span = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(7, 1)?)?;
//! let onsets: Vec<Beats> = bar
//!     .flatten(&span)?
//!     .iter()
//!     .map(|leaf| BeatTime::zero().interval_to(leaf.span().start()))
//!     .collect();
//!
//! assert_eq!(onsets, [
//!     Beats::ratio(0, 1)?,
//!     Beats::ratio(2, 1)?,
//!     Beats::ratio(4, 1)?,
//! ]);
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

use alloc::vec::Vec;
use num_traits::Zero;

use crate::algebra::{Q, Z};
use crate::error::TimeError;
use crate::time::beat::{BeatSpan, BeatTime, Beats};

/// A node of a weighted ordered rhythm tree (UMT-3.2 section 5.3.1).
///
/// UMT layer: L1, exact.
///
/// Every node carries its weight *relative to its siblings*. A leaf is a node
/// with no children. The root's weight is not used when flattening, since the
/// root's span is supplied; it is still recorded so a subtree can be lifted
/// out and reused without losing what it was worth.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawRhythmTree", into = "RawRhythmTree")
)]
pub struct RhythmTree {
    weight: Q,
    children: Vec<RhythmTree>,
}

/// A rhythm tree in wire form, revalidated on the way in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawRhythmTree {
    /// Weight relative to this node's siblings.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))]
    pub weight: Q,
    /// Ordered children; empty for a leaf.
    #[cfg_attr(feature = "serde", serde(default))]
    pub children: Vec<RawRhythmTree>,
}

impl RhythmTree {
    /// A leaf of the given integer weight.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveWeight`] for a weight that is not
    /// strictly positive (UMT-3.2 section 5.3.1).
    pub fn leaf(weight: i64) -> Result<Self, TimeError> {
        Self::weighted_leaf(Q::from(Z::from(weight)))
    }

    /// A leaf of the given exact weight.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveWeight`] for a non-positive weight.
    pub fn weighted_leaf(weight: Q) -> Result<Self, TimeError> {
        if weight <= Q::zero() {
            return Err(TimeError::NonPositiveWeight);
        }
        Ok(Self {
            weight,
            children: Vec::new(),
        })
    }

    /// An internal node of unit weight with the given ordered children.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyDivision`] if there are no children.
    pub fn division<I>(children: I) -> Result<Self, TimeError>
    where
        I: IntoIterator<Item = RhythmTree>,
    {
        Self::weighted_division(Q::from(Z::from(1)), children)
    }

    /// An internal node of the given weight with the given ordered children.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveWeight`] for a non-positive weight and
    /// [`TimeError::EmptyDivision`] if there are no children.
    pub fn weighted_division<I>(weight: Q, children: I) -> Result<Self, TimeError>
    where
        I: IntoIterator<Item = RhythmTree>,
    {
        if weight <= Q::zero() {
            return Err(TimeError::NonPositiveWeight);
        }
        let children: Vec<RhythmTree> = children.into_iter().collect();
        if children.is_empty() {
            return Err(TimeError::EmptyDivision);
        }
        Ok(Self { weight, children })
    }

    /// `count` equal children, the ordinary divisive subdivision.
    ///
    /// A tuplet is this with a `count` the parent span does not divide evenly;
    /// nothing special is needed, because the arithmetic is exact.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyDivision`] for a count of zero.
    pub fn equal_division(count: usize) -> Result<Self, TimeError> {
        Self::division(
            (0..count)
                .map(|_| Self::leaf(1))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    /// An additive grouping such as `2+2+3`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NonPositiveWeight`] or
    /// [`TimeError::EmptyDivision`].
    pub fn additive(weights: &[i64]) -> Result<Self, TimeError> {
        Self::division(
            weights
                .iter()
                .map(|weight| Self::leaf(*weight))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    /// This node's weight relative to its siblings.
    #[must_use]
    pub fn weight(&self) -> &Q {
        &self.weight
    }

    /// The ordered children; empty for a leaf.
    #[must_use]
    pub fn children(&self) -> &[RhythmTree] {
        &self.children
    }

    /// Whether this node has no children.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// The sum of the children's weights, or zero for a leaf.
    #[must_use]
    pub fn child_weight_total(&self) -> Q {
        self.children
            .iter()
            .fold(Q::zero(), |total, child| total + &child.weight)
    }

    /// The number of leaves, that is, of sounding or resting slots.
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        if self.is_leaf() {
            1
        } else {
            self.children.iter().map(Self::leaf_count).sum()
        }
    }

    /// The depth of the deepest leaf, counting the root as zero.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.children
            .iter()
            .map(|child| 1 + child.depth())
            .max()
            .unwrap_or(0)
    }

    /// The leaf spans, in order, under a given parent span
    /// (UMT-3.2 section 5.3.3).
    ///
    /// Exact throughout: the boundaries are rational whenever the span and the
    /// weights are, which section 5.3.3 requires.
    ///
    /// This operation is **lossy**. Two different trees can produce the same
    /// leaf spans, so the tree must be retained as the source of truth rather
    /// than reconstructed from the result.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedBeatSpan`] only if the parent span is
    /// malformed, which its type prevents.
    pub fn flatten(&self, span: &BeatSpan) -> Result<Vec<FlatLeaf>, TimeError> {
        let mut leaves = Vec::with_capacity(self.leaf_count());
        let mut path = Vec::new();
        self.collect(span, &mut path, &mut leaves)?;
        Ok(leaves)
    }

    /// The spans of this node's immediate children under a parent span.
    ///
    /// The children exactly partition the parent: the first starts where the
    /// parent starts, each begins where the previous ended, and the last ends
    /// where the parent ends. That is law 2 of UMT-3.2 section 9.7, and it
    /// holds by construction because the boundaries are computed from
    /// cumulative weights rather than by accumulating rounded durations.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedBeatSpan`] only for a malformed span.
    pub fn child_spans(&self, span: &BeatSpan) -> Result<Vec<BeatSpan>, TimeError> {
        if self.is_leaf() {
            return Ok(Vec::new());
        }
        let total = self.child_weight_total();
        let length = span.duration();
        let mut spans = Vec::with_capacity(self.children.len());
        let mut cumulative = Q::zero();
        let mut start = span.start().clone();
        for (index, child) in self.children.iter().enumerate() {
            cumulative += &child.weight;
            let end = if index + 1 == self.children.len() {
                // Take the parent's own endpoint rather than recomputing it,
                // so the partition closes exactly even in the presence of any
                // future non-field weight domain.
                span.end().clone()
            } else {
                BeatTime::new(span.start().get() + length.get() * (&cumulative / &total))
            };
            spans.push(BeatSpan::new(start, end.clone())?);
            start = end;
        }
        Ok(spans)
    }

    fn collect(
        &self,
        span: &BeatSpan,
        path: &mut Vec<usize>,
        out: &mut Vec<FlatLeaf>,
    ) -> Result<(), TimeError> {
        if self.is_leaf() {
            out.push(FlatLeaf {
                span: span.clone(),
                path: path.clone(),
            });
            return Ok(());
        }
        for (index, (child, child_span)) in self
            .children
            .iter()
            .zip(self.child_spans(span)?)
            .enumerate()
        {
            path.push(index);
            child.collect(&child_span, path, out)?;
            path.pop();
        }
        Ok(())
    }
}

impl TryFrom<RawRhythmTree> for RhythmTree {
    type Error = TimeError;

    fn try_from(value: RawRhythmTree) -> Result<Self, Self::Error> {
        let children = value
            .children
            .into_iter()
            .map(RhythmTree::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if children.is_empty() {
            Self::weighted_leaf(value.weight)
        } else {
            Self::weighted_division(value.weight, children)
        }
    }
}

impl From<RhythmTree> for RawRhythmTree {
    fn from(value: RhythmTree) -> Self {
        Self {
            weight: value.weight,
            children: value
                .children
                .into_iter()
                .map(RawRhythmTree::from)
                .collect(),
        }
    }
}

/// One leaf of a flattened rhythm tree.
///
/// UMT layer: L1, exact. The `path` records which child index was taken at
/// each level, which is the only part of the tree structure that survives
/// flattening - and it survives precisely so a caller can find its way back
/// into the source tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlatLeaf {
    span: BeatSpan,
    path: Vec<usize>,
}

impl FlatLeaf {
    /// The exact structural span of this leaf.
    #[must_use]
    pub fn span(&self) -> &BeatSpan {
        &self.span
    }

    /// The child index taken at each level, from the root down.
    #[must_use]
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// How deep in the tree this leaf sits.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.path.len()
    }
}

/// A cyclic pulse pattern (UMT-3.2 section 5.3.2).
///
/// UMT layer: L1, exact.
///
/// A cycle length, a pulse resolution, the onset positions within one cycle,
/// and an optional designated rotation reference. Cyclic patterns need not
/// imply hierarchy, which is why this is not a [`RhythmTree`] with a flag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawCyclicRhythm", into = "RawCyclicRhythm")
)]
pub struct CyclicRhythm {
    pulses: u32,
    onsets: Vec<u32>,
    rotation: u32,
}

/// A cyclic pattern in wire form, revalidated on the way in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawCyclicRhythm {
    /// Pulses in one cycle.
    pub pulses: u32,
    /// Pulse indices carrying an onset.
    pub onsets: Vec<u32>,
    /// The designated rotation reference.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rotation: u32,
}

impl CyclicRhythm {
    /// Builds a pattern of `pulses` positions with onsets at the given
    /// indices.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::EmptyCycle`] for a zero-length cycle and
    /// [`TimeError::OnsetOutsideCycle`] for an index at or beyond the cycle
    /// length. Repeated indices are collapsed, since a pulse either carries an
    /// onset or does not.
    pub fn new(pulses: u32, onsets: &[u32], rotation: u32) -> Result<Self, TimeError> {
        if pulses == 0 {
            return Err(TimeError::EmptyCycle);
        }
        for onset in onsets.iter().chain(core::iter::once(&rotation)) {
            if *onset >= pulses {
                return Err(TimeError::OnsetOutsideCycle {
                    index: *onset,
                    pulses,
                });
            }
        }
        let mut sorted: Vec<u32> = onsets.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        Ok(Self {
            pulses,
            onsets: sorted,
            rotation,
        })
    }

    /// Pulses in one cycle.
    #[must_use]
    pub fn pulses(&self) -> u32 {
        self.pulses
    }

    /// The onset indices, ascending and distinct.
    #[must_use]
    pub fn onsets(&self) -> &[u32] {
        &self.onsets
    }

    /// The designated rotation reference.
    #[must_use]
    pub fn rotation(&self) -> u32 {
        self.rotation
    }

    /// Whether a pulse index carries an onset.
    #[must_use]
    pub fn is_onset(&self, index: u32) -> bool {
        self.onsets.binary_search(&index).is_ok()
    }

    /// The pattern rotated so that pulse `by` becomes pulse zero.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OnsetOutsideCycle`] if `by` is not a pulse of this
    /// cycle.
    pub fn rotated(&self, by: u32) -> Result<Self, TimeError> {
        if by >= self.pulses {
            return Err(TimeError::OnsetOutsideCycle {
                index: by,
                pulses: self.pulses,
            });
        }
        let onsets: Vec<u32> = self
            .onsets
            .iter()
            .map(|index| (index + self.pulses - by) % self.pulses)
            .collect();
        Self::new(
            self.pulses,
            &onsets,
            (self.rotation + self.pulses - by) % self.pulses,
        )
    }

    /// The exact structural onset times within one cycle of the given span.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ReversedBeatSpan`] only for a malformed span.
    pub fn onset_times(&self, cycle: &BeatSpan) -> Result<Vec<BeatTime>, TimeError> {
        let length = cycle.duration();
        let pulses = Q::from(Z::from(self.pulses));
        Ok(self
            .onsets
            .iter()
            .map(|index| {
                let position = Q::from(Z::from(*index)) / &pulses;
                BeatTime::new(cycle.start().get() + length.get() * position)
            })
            .collect())
    }

    /// The gaps between consecutive onsets, wrapping around the cycle.
    ///
    /// Empty if there are no onsets. The gaps always sum to the cycle length.
    #[must_use]
    pub fn inter_onset_pulses(&self) -> Vec<u32> {
        if self.onsets.is_empty() {
            return Vec::new();
        }
        let mut gaps = Vec::with_capacity(self.onsets.len());
        for window in self.onsets.windows(2) {
            gaps.push(window[1] - window[0]);
        }
        gaps.push(self.pulses + self.onsets[0] - self.onsets[self.onsets.len() - 1]);
        gaps
    }
}

impl TryFrom<RawCyclicRhythm> for CyclicRhythm {
    type Error = TimeError;

    fn try_from(value: RawCyclicRhythm) -> Result<Self, Self::Error> {
        Self::new(value.pulses, &value.onsets, value.rotation)
    }
}

impl From<CyclicRhythm> for RawCyclicRhythm {
    fn from(value: CyclicRhythm) -> Self {
        Self {
            pulses: value.pulses,
            onsets: value.onsets,
            rotation: value.rotation,
        }
    }
}

/// The total structural duration a flattening covers, for law checking.
#[must_use]
pub fn flattened_total(leaves: &[FlatLeaf]) -> Beats {
    leaves.iter().map(|leaf| leaf.span().duration()).sum()
}

#[cfg(test)]
mod tests {
    use super::{CyclicRhythm, RhythmTree, flattened_total};
    use crate::algebra::{Q, Z};
    use crate::error::TimeError;
    use crate::time::beat::{BeatSpan, BeatTime, Beats};
    use alloc::vec::Vec;

    fn span(beats: i64) -> BeatSpan {
        BeatSpan::new(BeatTime::zero(), BeatTime::ratio(beats, 1).unwrap()).unwrap()
    }

    fn onsets(tree: &RhythmTree, over: &BeatSpan) -> Vec<Q> {
        tree.flatten(over)
            .unwrap()
            .iter()
            .map(|leaf| leaf.span().start().get().clone())
            .collect()
    }

    #[test]
    fn f11_additive_two_two_three_flattens_to_exact_boundaries() {
        let bar = RhythmTree::additive(&[2, 2, 3]).unwrap();
        let seven = span(7);
        assert_eq!(
            onsets(&bar, &seven),
            [
                Q::from(Z::from(0)),
                Q::from(Z::from(2)),
                Q::from(Z::from(4))
            ]
        );

        let leaves = bar.flatten(&seven).unwrap();
        assert_eq!(leaves.len(), 3);
        assert_eq!(*leaves[2].span().end().get(), Q::from(Z::from(7)));
        assert_eq!(flattened_total(&leaves), Beats::ratio(7, 1).unwrap());

        // Child order is preserved and the weights are still there.
        assert_eq!(
            bar.children()
                .iter()
                .map(|child| child.weight().clone())
                .collect::<Vec<_>>(),
            [
                Q::from(Z::from(2)),
                Q::from(Z::from(2)),
                Q::from(Z::from(3))
            ]
        );
    }

    #[test]
    fn children_exactly_partition_the_parent() {
        // A quintuplet inside a triplet: boundaries no float represents.
        let tree = RhythmTree::division([
            RhythmTree::equal_division(5).unwrap(),
            RhythmTree::leaf(1).unwrap(),
            RhythmTree::leaf(1).unwrap(),
        ])
        .unwrap();
        let bar = span(1);

        let leaves = tree.flatten(&bar).unwrap();
        assert_eq!(leaves.len(), 7);
        assert_eq!(*leaves[0].span().start(), BeatTime::zero());
        assert_eq!(*leaves[6].span().end().get(), Q::from(Z::from(1)));

        // Each leaf begins where the previous ended.
        for pair in leaves.windows(2) {
            assert_eq!(pair[0].span().end(), pair[1].span().start());
        }
        assert_eq!(flattened_total(&leaves), Beats::ratio(1, 1).unwrap());

        // The first five leaves are fifteenths of a beat.
        assert_eq!(
            leaves[0].span().duration(),
            Beats::ratio(1, 15).unwrap(),
            "a fifth of a third"
        );
        assert_eq!(leaves[0].depth(), 2);
        assert_eq!(leaves[5].depth(), 1);
        assert_eq!(leaves[0].path(), &[0, 0]);
    }

    #[test]
    fn recursive_flattening_preserves_the_root_total() {
        let tree = RhythmTree::division([
            RhythmTree::weighted_division(
                Q::new(Z::from(3), Z::from(2)),
                [RhythmTree::leaf(1).unwrap(), RhythmTree::leaf(2).unwrap()],
            )
            .unwrap(),
            RhythmTree::equal_division(7).unwrap(),
            RhythmTree::leaf(5).unwrap(),
        ])
        .unwrap();
        for beats in [1i64, 3, 7, 12] {
            let bar = span(beats);
            assert_eq!(
                flattened_total(&tree.flatten(&bar).unwrap()),
                Beats::ratio(beats, 1).unwrap(),
                "{beats} beats"
            );
        }
        assert_eq!(tree.leaf_count(), 10);
        assert_eq!(tree.depth(), 2);
    }

    #[test]
    fn weights_must_be_positive_and_divisions_non_empty() {
        assert_eq!(RhythmTree::leaf(0), Err(TimeError::NonPositiveWeight));
        assert_eq!(RhythmTree::leaf(-3), Err(TimeError::NonPositiveWeight));
        assert_eq!(
            RhythmTree::division(core::iter::empty()),
            Err(TimeError::EmptyDivision)
        );
        assert_eq!(RhythmTree::equal_division(0), Err(TimeError::EmptyDivision));
    }

    #[test]
    fn flattening_is_lossy_and_the_tree_is_the_source() {
        // Two structurally different trees with identical leaf boundaries.
        let flat = RhythmTree::additive(&[1, 1, 1, 1]).unwrap();
        let nested = RhythmTree::division([
            RhythmTree::equal_division(2).unwrap(),
            RhythmTree::equal_division(2).unwrap(),
        ])
        .unwrap();
        let bar = span(4);

        assert_eq!(onsets(&flat, &bar), onsets(&nested, &bar));
        assert_ne!(flat, nested, "the trees are different, and stay different");
        assert_ne!(flat.depth(), nested.depth());

        // The path is the only structural trace that survives, and it differs.
        let flat_leaves = flat.flatten(&bar).unwrap();
        let nested_leaves = nested.flatten(&bar).unwrap();
        assert_eq!(flat_leaves[3].path(), &[3]);
        assert_eq!(nested_leaves[3].path(), &[1, 1]);
    }

    #[test]
    fn a_cyclic_pattern_keeps_its_rotation_convention() {
        // The tresillo: onsets at 0, 3, 6 of an eight-pulse cycle.
        let tresillo = CyclicRhythm::new(8, &[0, 3, 6], 0).unwrap();
        assert_eq!(tresillo.onsets(), &[0, 3, 6]);
        assert!(tresillo.is_onset(3));
        assert!(!tresillo.is_onset(4));
        assert_eq!(tresillo.inter_onset_pulses(), [3, 3, 2]);
        assert_eq!(
            tresillo.inter_onset_pulses().iter().sum::<u32>(),
            tresillo.pulses()
        );

        let rotated = tresillo.rotated(3).unwrap();
        assert_eq!(rotated.onsets(), &[0, 3, 5]);
        assert_eq!(rotated.rotation(), 5, "the reference travels with it");

        // Exact onset times over a two-beat cycle.
        let times = tresillo.onset_times(&span(2)).unwrap();
        assert_eq!(*times[1].get(), Q::new(Z::from(3), Z::from(4)));
    }

    #[test]
    fn a_cycle_validates_its_own_bounds() {
        assert_eq!(CyclicRhythm::new(0, &[], 0), Err(TimeError::EmptyCycle));
        assert_eq!(
            CyclicRhythm::new(4, &[4], 0),
            Err(TimeError::OnsetOutsideCycle {
                index: 4,
                pulses: 4
            })
        );
        assert!(CyclicRhythm::new(4, &[0, 0, 2], 0).is_ok());
        assert_eq!(
            CyclicRhythm::new(4, &[0, 0, 2], 0).unwrap().onsets(),
            &[0, 2],
            "a pulse either carries an onset or does not"
        );
    }
}
