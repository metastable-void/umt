//! Pitch points as torsors (UMT-3.2 sections 1.10 and 9.4).
//!
//! Intervals form groups; pitch points do not. A point space `P` over an
//! abelian group `G` is a set with a simply transitive action, so:
//!
//! ```text
//! point + interval -> point
//! point - point    -> interval
//! ```
//!
//! and there is no `point + point`. This module provides that structure once,
//! generically over the interval type, so the same discipline holds for exact
//! L1 monzo intervals, L2 ambient or image intervals, and - through
//! [`crate::pitch::units::LogFrequency`] - L3 real intervals.
//!
//! A structural point also carries a declared *origin identity*. Two points
//! are only comparable when they were measured from the same designated
//! reference, because "a fifth above C" and "a fifth above D" are not the same
//! pitch and nothing in an exponent vector says which is meant.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::context::TheoryContext;
use crate::error::{ContextError, PitchError};
use crate::proportion::monzo::Monzo;
use crate::temperament::image::{AmbientElem, AmbientLattice, ImageElem, LatticeId};

/// Stable identity of a designated reference point.
///
/// UMT layer: metadata. Changing concert pitch or a transposing-instrument
/// reference changes reference *data*; it does not mutate the interval lattice
/// (UMT-3.2 section 4.2), and this identity is what those data attach to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct PitchOrigin(Arc<str>);

impl PitchOrigin {
    /// Wraps a stable origin identity.
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

impl From<String> for PitchOrigin {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<PitchOrigin> for String {
    fn from(value: PitchOrigin) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for PitchOrigin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An interval type that can act on a pitch-point torsor.
///
/// Implemented for exact L1 monzos and for L2 ambient and image elements, so
/// the same point type serves every structural layer.
pub trait IntervalGroupElement: Sized + Clone + PartialEq {
    /// Group addition.
    ///
    /// # Errors
    ///
    /// Fails when the operands belong to different declared groups.
    fn checked_add(&self, other: &Self) -> Result<Self, PitchError>;

    /// Group subtraction.
    ///
    /// # Errors
    ///
    /// Fails when the operands belong to different declared groups.
    fn checked_sub(&self, other: &Self) -> Result<Self, PitchError>;
}

impl IntervalGroupElement for Monzo {
    fn checked_add(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(Monzo::checked_add(self, other)?)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(Monzo::checked_sub(self, other)?)
    }
}

impl IntervalGroupElement for AmbientElem {
    fn checked_add(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(AmbientElem::checked_add(self, other)?)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(AmbientElem::checked_sub(self, other)?)
    }
}

impl IntervalGroupElement for ImageElem {
    fn checked_add(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(ImageElem::checked_add(self, other)?)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, PitchError> {
        Ok(ImageElem::checked_sub(self, other)?)
    }
}

/// A pitch position, expressed as an interval from a designated origin.
///
/// UMT layer: L1 with a [`Monzo`] interval, L2 with an ambient or image
/// element. Exact in both cases.
///
/// Equality is presentation equality: same origin identity and same offset.
/// There is no `Add` implementation between points, by design.
///
/// # Examples
///
/// ```
/// use umt::pitch::{PitchOrigin, PitchPoint};
/// use umt::Basis;
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let middle_c = PitchOrigin::new("umt:origin:c4");
/// let tonic = PitchPoint::new(middle_c.clone(), basis.zero());
///
/// let fifth = basis.monzo([-1, 1, 0])?;
/// let dominant = tonic.translate(&fifth)?;
///
/// // point + int(point, other) = other
/// assert_eq!(tonic.interval_to(&dominant)?, fifth);
/// assert_eq!(tonic.translate(&tonic.interval_to(&dominant)?)?, dominant);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PitchPoint<E> {
    origin: PitchOrigin,
    offset: E,
}

impl<E: IntervalGroupElement> PitchPoint<E> {
    /// Builds a point at `offset` from `origin`.
    #[must_use]
    pub fn new(origin: PitchOrigin, offset: E) -> Self {
        Self { origin, offset }
    }

    /// The declared origin.
    #[must_use]
    pub fn origin(&self) -> &PitchOrigin {
        &self.origin
    }

    /// The offset from the origin.
    ///
    /// This is an interval, and it is only meaningful together with the
    /// origin.
    #[must_use]
    pub fn offset(&self) -> &E {
        &self.offset
    }

    /// The torsor action: `p + g`.
    ///
    /// # Errors
    ///
    /// Fails if the interval belongs to a different declared group.
    pub fn translate(&self, interval: &E) -> Result<Self, PitchError> {
        Ok(Self {
            origin: self.origin.clone(),
            offset: self.offset.checked_add(interval)?,
        })
    }

    /// The unique interval `int(p, q)` with `p + int(p, q) = q`.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::OriginMismatch`] if the two points are measured
    /// from different origins, since no interval between them is defined.
    pub fn interval_to(&self, other: &Self) -> Result<E, PitchError> {
        if self.origin != other.origin {
            return Err(PitchError::OriginMismatch {
                left: self.origin.clone(),
                right: other.origin.clone(),
            });
        }
        other.offset.checked_sub(&self.offset)
    }

    /// Re-expresses this point from a different origin, given the interval
    /// from the new origin to the old one.
    ///
    /// This is what a change of concert pitch or transposing reference does:
    /// it changes reference data, leaving the interval lattice untouched
    /// (UMT-3.2 section 4.2).
    ///
    /// # Errors
    ///
    /// Fails if the interval belongs to a different declared group.
    pub fn rebase(
        &self,
        origin: PitchOrigin,
        offset_of_old_origin: &E,
    ) -> Result<Self, PitchError> {
        Ok(Self {
            origin,
            offset: offset_of_old_origin.checked_add(&self.offset)?,
        })
    }
}

/// A pitch point in wire form: an origin, a lattice reference, and exact
/// coordinates.
///
/// UMT layer: L2, exact. Like [`crate::context::MonzoRef`], this carries no
/// lattice *definition*, only the identifier of one, so it is meaningless
/// without the [`TheoryContext`] that defines it (UMT-3.2 section 6.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PitchPointRef {
    /// Identifier of the designated reference point.
    pub origin: PitchOrigin,
    /// Identifier of the lattice the offset lives in.
    pub lattice: LatticeId,
    /// Exact offset coordinates, one per lattice generator.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::vec_z"))]
    pub coordinates: Vec<Z>,
}

impl PitchPointRef {
    /// Produces the wire form of a point over an ambient lattice.
    #[must_use]
    pub fn of_ambient(point: &PitchPoint<AmbientElem>) -> Self {
        Self {
            origin: point.origin().clone(),
            lattice: point.offset().lattice().id().clone(),
            coordinates: point.offset().coordinates().to_vec(),
        }
    }

    /// Resolves this reference against a context.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::UnknownAmbient`] if the lattice is not
    /// registered, and [`ContextError::Temperament`] if the coordinate count
    /// does not match its rank.
    pub fn resolve_ambient(
        &self,
        context: &TheoryContext,
    ) -> Result<PitchPoint<AmbientElem>, ContextError> {
        let lattice: &Arc<AmbientLattice> =
            context
                .ambient(&self.lattice)
                .ok_or_else(|| ContextError::UnknownAmbient {
                    id: self.lattice.clone(),
                })?;
        let offset = lattice.element(self.coordinates.clone())?;
        Ok(PitchPoint::new(self.origin.clone(), offset))
    }
}

#[cfg(test)]
mod tests {
    use super::{PitchOrigin, PitchPoint, PitchPointRef};
    use crate::context::TheoryContext;
    use crate::error::PitchError;
    use crate::proportion::Basis;
    use crate::temperament::image::AmbientLattice;
    use alloc::sync::Arc;

    fn five_limit() -> Arc<Basis> {
        Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap()
    }

    #[test]
    fn torsor_laws_hold_for_exact_intervals() {
        let basis = five_limit();
        let origin = PitchOrigin::new("umt:origin:c4");
        let p = PitchPoint::new(origin.clone(), basis.zero());
        let g = basis.monzo([-1, 1, 0]).unwrap();
        let h = basis.monzo([-2, 0, 1]).unwrap();

        // (p + g) + h = p + (g + h)
        assert_eq!(
            p.translate(&g).unwrap().translate(&h).unwrap(),
            p.translate(&g.checked_add(&h).unwrap()).unwrap()
        );
        // p + 0 = p
        assert_eq!(p.translate(&basis.zero()).unwrap(), p);
        // p + int(p, q) = q
        let q = p.translate(&g).unwrap();
        assert_eq!(p.translate(&p.interval_to(&q).unwrap()).unwrap(), q);
        // int(p, q) + int(q, r) = int(p, r)
        let r = q.translate(&h).unwrap();
        assert_eq!(
            p.interval_to(&q)
                .unwrap()
                .checked_add(&q.interval_to(&r).unwrap())
                .unwrap(),
            p.interval_to(&r).unwrap()
        );
    }

    #[test]
    fn points_from_different_origins_have_no_interval_between_them() {
        let basis = five_limit();
        let here = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), basis.zero());
        let there = PitchPoint::new(PitchOrigin::new("umt:origin:a4"), basis.zero());
        assert_ne!(here, there);
        assert!(matches!(
            here.interval_to(&there),
            Err(PitchError::OriginMismatch { .. })
        ));
    }

    #[test]
    fn rebasing_changes_reference_data_not_intervals() {
        let basis = five_limit();
        let c4 = PitchOrigin::new("umt:origin:c4");
        let a4 = PitchOrigin::new("umt:origin:a4");
        // A4 is a major sixth above C4: 5/3.
        let sixth = basis.monzo([0, -1, 1]).unwrap();

        let tonic = PitchPoint::new(c4.clone(), basis.zero());
        let dominant = tonic.translate(&basis.monzo([-1, 1, 0]).unwrap()).unwrap();
        let interval = tonic.interval_to(&dominant).unwrap();

        // Re-express both from A4 by supplying the interval from A4 to C4.
        let from_a4 = basis.monzo([0, 1, -1]).unwrap();
        assert_eq!(from_a4, -&sixth);
        let tonic = tonic.rebase(a4.clone(), &from_a4).unwrap();
        let dominant = dominant.rebase(a4, &from_a4).unwrap();

        assert_eq!(
            tonic.interval_to(&dominant).unwrap(),
            interval,
            "the interval between two points is independent of the origin"
        );
    }

    #[test]
    fn structural_points_work_over_ambient_intervals() {
        let ambient = AmbientLattice::new("umt:edo:12", 1);
        let origin = PitchOrigin::new("umt:origin:c4");
        let point = PitchPoint::new(origin, ambient.zero());
        let seven = ambient.element([7i64]).unwrap();
        let fifth_up = point.translate(&seven).unwrap();
        assert_eq!(point.interval_to(&fifth_up).unwrap(), seven);
        assert_eq!(fifth_up.interval_to(&point).unwrap(), -&seven);
    }

    #[test]
    fn intervals_from_unrelated_groups_are_rejected() {
        let five = five_limit();
        let seven = Basis::primes("umt:prime:2.3.7", &[2, 3, 7]).unwrap();
        let point = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), five.zero());
        assert!(point.translate(&seven.monzo([1, 0, 0]).unwrap()).is_err());
    }

    #[test]
    fn points_round_trip_through_their_reference_form() {
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let context = TheoryContext::builder().ambient(&steps).unwrap().build();
        let point = PitchPoint::new(
            PitchOrigin::new("umt:origin:c4"),
            steps.element([7i64]).unwrap(),
        );

        let reference = PitchPointRef::of_ambient(&point);
        assert_eq!(reference.origin, PitchOrigin::new("umt:origin:c4"));
        assert_eq!(reference.resolve_ambient(&context).unwrap(), point);

        // An unregistered lattice does not silently resolve.
        let empty = TheoryContext::builder().build();
        assert!(reference.resolve_ambient(&empty).is_err());
    }
}
