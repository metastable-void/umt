//! Representative policies and the structural quotient lens (UMT-3.2 sections
//! 1.7.2, 1.7.3, and 7.3).
//!
//! A representative policy is an arbitrary set-theoretic right inverse
//! `sigma: H -> Lambda_B` with `V . sigma = id_H`. It need not preserve
//! addition, and most useful ones do not: minimum-complexity spelling,
//! context-sensitive detempering, and adaptive lift selection are all
//! naturally non-homomorphic.
//!
//! For any right inverse the residue `rho_sigma(m) = m - sigma(V(m))` is an
//! exact element of the kernel, and the set-level lens laws hold. None of that
//! requires linearity, and none of it implies it: a direct-sum decomposition
//! `Lambda_B ~ H (+) K` follows only from a
//! [`crate::temperament::HomomorphicSplit`].

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::algebra::Z;
use crate::error::TemperamentError;
use crate::proportion::monzo::Monzo;
use crate::realization::provenance::ProvenanceId;
use crate::temperament::image::ImageElem;
use crate::temperament::kernel::KernelElem;
use crate::temperament::map::TemperamentMap;
use crate::temperament::splitting::HomomorphicSplit;

/// The outcome of choosing one exact lift for a tempered class.
///
/// UMT layer: L1/L2, exact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiftDecision {
    /// The chosen exact lift, which satisfies `V(lift) = class`.
    pub lift: Monzo,
    /// How this lift differs from the policy's own reference lift for the same
    /// class, as an exact kernel element.
    ///
    /// UMT-3.2 section 4.8 requires an adaptive optimizer to report the
    /// selected lifts *and* the comma residues that distinguish them from the
    /// configured canonical policy; this is that residue. A policy that is its
    /// own reference - a homomorphic splitting, for instance - reports zero.
    ///
    /// This is not the same quantity as the residue of a *monzo* under the
    /// policy, which is [`StructuralLens::residue`].
    pub residue: KernelElem,
    /// Reference to the provenance record for this decision, where the policy
    /// keeps one.
    pub provenance: Option<ProvenanceId>,
}

impl LiftDecision {
    /// Records a decision.
    #[must_use]
    pub fn new(lift: Monzo, residue: KernelElem, provenance: Option<ProvenanceId>) -> Self {
        Self {
            lift,
            residue,
            provenance,
        }
    }
}

/// An arbitrary right inverse `sigma: H -> Lambda_B`, possibly depending on a
/// context (UMT-3.2 section 1.7.2).
///
/// UMT layer: L2 to L1 selection policy.
///
/// The context type `C` is explicit because a policy that depends on register,
/// harmonic context, instrument state, or performance time must say so in its
/// type rather than hide the dependency behind a unary function (UMT-3.2
/// section 1.8.2, fixture F28).
///
/// # Contract
///
/// - `V(choose(x, c).lift) == x` for every class `x` and context `c`. This is
///   law P8 and it is not optional.
/// - `residue` reports the difference from the policy's own reference lift.
/// - [`RepresentativePolicy::claims_homomorphic`] may return `true` only if
///   `sigma_c` really is additive for every fixed `c`, which law P11 requires
///   to be tested whenever it is claimed.
pub trait RepresentativePolicy<C> {
    /// What can go wrong while choosing.
    type Error;

    /// The mapping whose classes this policy lifts.
    fn map(&self) -> &TemperamentMap;

    /// Chooses a lift for a tempered class in a context.
    ///
    /// # Errors
    ///
    /// Policy-defined; a policy that cannot decide must say so rather than
    /// return an arbitrary lift.
    fn choose(&self, class: &ImageElem, context: &C) -> Result<LiftDecision, Self::Error>;

    /// Whether this policy claims to be a group homomorphism.
    ///
    /// Defaults to `false`, which is the safe answer: assuming additivity of a
    /// minimum-cost or adaptive policy is exactly the error UMT-3.2 section
    /// 1.7.2 warns against.
    fn claims_homomorphic(&self) -> bool {
        false
    }
}

/// Adapts a homomorphic splitting into a representative policy.
///
/// This direction is always valid; the reverse is not, which is why no
/// conversion the other way exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPolicy<S> {
    split: S,
}

impl<S: HomomorphicSplit> SplitPolicy<S> {
    /// Wraps a splitting.
    #[must_use]
    pub fn new(split: S) -> Self {
        Self { split }
    }

    /// The underlying splitting.
    #[must_use]
    pub fn split(&self) -> &S {
        &self.split
    }
}

impl<C, S: HomomorphicSplit> RepresentativePolicy<C> for SplitPolicy<S> {
    type Error = TemperamentError;

    fn map(&self) -> &TemperamentMap {
        self.split.map()
    }

    fn choose(&self, class: &ImageElem, _context: &C) -> Result<LiftDecision, Self::Error> {
        let lift = self.split.split(class)?;
        Ok(LiftDecision::new(lift, no_residue(self.split.map())?, None))
    }

    fn claims_homomorphic(&self) -> bool {
        true
    }
}

/// The policy that lifts each class to the canonical preimage of its ambient
/// coordinate.
///
/// UMT layer: L2 to L1 selection policy.
///
/// Deterministic, and small: the reduction that
/// [`TemperamentMap::preimage`] applies keeps the high-generator exponents
/// bounded, so the 12-EDO class of seven steps lifts to `3/2` rather than to
/// some twenty-digit member of the same fiber.
///
/// It is *not* a homomorphism, and it does not claim to be. A linear section
/// is forced to send the class `n x` to `n` times the lift of `x`, which is
/// what makes homomorphic splittings unsuitable as spelling policies; this
/// policy reduces every class independently instead.
///
/// It is also not a minimum-complexity policy: that would need a declared
/// complexity function, and "small under this reduction" is not the same
/// claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLiftPolicy {
    map: TemperamentMap,
}

impl CanonicalLiftPolicy {
    /// Builds the policy for a mapping.
    #[must_use]
    pub fn new(map: TemperamentMap) -> Self {
        Self { map }
    }
}

impl<C> RepresentativePolicy<C> for CanonicalLiftPolicy {
    type Error = TemperamentError;

    fn map(&self) -> &TemperamentMap {
        &self.map
    }

    fn choose(&self, class: &ImageElem, _context: &C) -> Result<LiftDecision, Self::Error> {
        let ambient = self.map.image().embed(class)?;
        let lift = self.map.preimage(&ambient)?;
        Ok(LiftDecision::new(lift, no_residue(&self.map)?, None))
    }
}

/// A policy that offsets another policy by a context-dependent comma.
///
/// This is the general shape of adaptive and context-sensitive detempering:
/// the offset is an exact kernel element, so the result is still a right
/// inverse for every context, while additivity is generally lost - which is
/// correct, not a defect (UMT-3.2 sections 1.7.2 and 4.8).
///
/// Because the offset comes from the kernel, the right-inverse law cannot be
/// broken by a caller's choice of offsets, only additivity can.
///
/// # Examples
///
/// ```
/// use umt::temperament::{
///     AmbientLattice, CanonicalLiftPolicy, OffsetPolicy, RepresentativePolicy, TemperamentMap,
/// };
/// use umt::Basis;
///
/// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
/// let steps = AmbientLattice::new("umt:edo:12", 1);
/// let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;
///
/// // In context `true`, shift every lift by the syntonic comma.
/// let comma = map.kernel().coordinates(&basis.monzo([-4, 4, -1])?)?.unwrap();
/// let policy = OffsetPolicy::new(
///     CanonicalLiftPolicy::new(map.clone()),
///     move |_class: &_, shifted: &bool| if *shifted { Some(comma.clone()) } else { None },
/// );
///
/// let class = map.apply_to_image(&basis.monzo([-1, 1, 0])?)?;
/// let plain = policy.choose(&class, &false)?;
/// let shifted = policy.choose(&class, &true)?;
///
/// // Different exact lifts of the same tempered class, in different contexts.
/// assert_ne!(plain.lift, shifted.lift);
/// assert!(plain.residue.is_zero());
/// assert!(!shifted.residue.is_zero());
/// assert!(!policy.claims_homomorphic());
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct OffsetPolicy<P, F> {
    base: P,
    offset: F,
}

impl<P, F> OffsetPolicy<P, F> {
    /// Builds a policy from a base policy and a context-dependent offset.
    ///
    /// Returning `None` from the offset means "use the base lift unchanged".
    #[must_use]
    pub fn new(base: P, offset: F) -> Self {
        Self { base, offset }
    }

    /// The base policy.
    #[must_use]
    pub fn base(&self) -> &P {
        &self.base
    }
}

impl<C, P, F> RepresentativePolicy<C> for OffsetPolicy<P, F>
where
    P: RepresentativePolicy<C, Error = TemperamentError>,
    F: Fn(&ImageElem, &C) -> Option<KernelElem>,
{
    type Error = TemperamentError;

    fn map(&self) -> &TemperamentMap {
        self.base.map()
    }

    fn choose(&self, class: &ImageElem, context: &C) -> Result<LiftDecision, Self::Error> {
        let base = self.base.choose(class, context)?;
        match (self.offset)(class, context) {
            None => Ok(base),
            Some(offset) => {
                let comma = self.base.map().kernel().embed(&offset)?;
                let lift = base.lift.checked_add(&comma)?;
                let residue = base.residue.checked_add(&offset)?;
                Ok(LiftDecision::new(lift, residue, base.provenance))
            }
        }
    }
}

/// The structural quotient lens of UMT-3.2 sections 1.7.3 and 7.3.
///
/// UMT layer: L1 to L2, exact.
///
/// Bundles a mapping with a representative policy so that the three lens
/// operations are available:
///
/// ```text
/// get(m)     = V(m)
/// residue(m) = m - sigma(V(m))          in K
/// put(m, x)  = sigma(x) + residue(m)
/// ```
///
/// These satisfy GetPut, PutGet, and PutPut for *any* right inverse, at a
/// fixed context. They do not imply that `sigma` is linear, and this type
/// makes no such assumption.
///
/// The context type is a parameter of the lens rather than of each call, so a
/// lens is pinned to the context a policy actually consumes.
pub struct StructuralLens<P, C> {
    policy: P,
    context: PhantomData<fn(&C)>,
}

impl<P, C> StructuralLens<P, C> {
    /// Builds a lens over a policy.
    #[must_use]
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            context: PhantomData,
        }
    }

    /// The policy.
    #[must_use]
    pub fn policy(&self) -> &P {
        &self.policy
    }
}

// Written out rather than derived so that the context type, of which no value
// is ever held, does not impose bounds on the lens.
impl<P: Clone, C> Clone for StructuralLens<P, C> {
    fn clone(&self) -> Self {
        Self::new(self.policy.clone())
    }
}

impl<P: core::fmt::Debug, C> core::fmt::Debug for StructuralLens<P, C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StructuralLens")
            .field("policy", &self.policy)
            .finish()
    }
}

impl<C, P: RepresentativePolicy<C>> StructuralLens<P, C> {
    /// The mapping.
    pub fn map(&self) -> &TemperamentMap {
        self.policy.map()
    }

    /// `get(m) = V(m)`.
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::BasisMismatch`] if the monzo is over an
    /// unrelated basis.
    pub fn get(&self, monzo: &Monzo) -> Result<ImageElem, LensError<P::Error>> {
        self.map().apply_to_image(monzo).map_err(LensError::Lattice)
    }

    /// `sigma_c(x)`, the chosen lift of a class in a context.
    ///
    /// # Errors
    ///
    /// Propagates the policy's error.
    pub fn section(&self, class: &ImageElem, context: &C) -> Result<Monzo, LensError<P::Error>> {
        Ok(self
            .policy
            .choose(class, context)
            .map_err(LensError::Policy)?
            .lift)
    }

    /// `rho_sigma(m) = m - sigma_c(V(m))`, an exact kernel element.
    ///
    /// This is structural information about which lift of the tempered class
    /// was present. It is not a tuning deviation, not a rounding error, and
    /// not a real number (UMT-3.2 sections 1.7.2 and 7.9).
    ///
    /// # Errors
    ///
    /// Returns [`TemperamentError::NotARightInverse`] if the policy returned a
    /// lift that does not map back to the class it was asked about, so that
    /// the difference is not in the kernel. A policy violating its own
    /// contract is reported rather than silently absorbed.
    pub fn residue(&self, monzo: &Monzo, context: &C) -> Result<KernelElem, LensError<P::Error>> {
        let class = self.get(monzo)?;
        let lift = self.section(&class, context)?;
        let difference = monzo
            .checked_sub(&lift)
            .map_err(|error| LensError::Lattice(TemperamentError::from(error)))?;
        self.map()
            .kernel()
            .coordinates(&difference)
            .map_err(LensError::Lattice)?
            .ok_or(LensError::Lattice(TemperamentError::NotARightInverse))
    }

    /// `put(m, x) = sigma_c(x) + rho_sigma(m)`: replaces the tempered class of
    /// `m` with `x` while preserving its comma residue.
    ///
    /// # Errors
    ///
    /// As [`StructuralLens::residue`], plus the policy's own errors.
    pub fn put(
        &self,
        monzo: &Monzo,
        class: &ImageElem,
        context: &C,
    ) -> Result<Monzo, LensError<P::Error>> {
        let residue = self.residue(monzo, context)?;
        let comma = self
            .map()
            .kernel()
            .embed(&residue)
            .map_err(LensError::Lattice)?;
        let lift = self.section(class, context)?;
        lift.checked_add(&comma)
            .map_err(|error| LensError::Lattice(TemperamentError::from(error)))
    }
}

/// A lens operation failed, either structurally or inside the policy.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum LensError<E> {
    /// A structural, lattice-level failure.
    #[error(transparent)]
    Lattice(TemperamentError),
    /// The policy declined or failed.
    #[error("representative policy failed: {0}")]
    Policy(E),
}

/// The zero kernel element of a mapping: no comma residue.
///
/// Useful to policy implementors that are their own reference lift.
///
/// # Errors
///
/// Cannot fail for a validated mapping; the result is fallible only because
/// element construction checks its coordinate count.
pub fn no_residue(map: &TemperamentMap) -> Result<KernelElem, TemperamentError> {
    let zeros: Vec<Z> = (0..map.kernel().rank()).map(|_| Z::from(0)).collect();
    map.kernel().element(zeros)
}

#[cfg(test)]
mod tests {
    use super::{LensError, OffsetPolicy, RepresentativePolicy, SplitPolicy, StructuralLens};
    use crate::error::TemperamentError;
    use crate::proportion::Basis;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use crate::temperament::splitting::LinearSplit;
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
    fn a_split_policy_claims_and_keeps_additivity() {
        let map = twelve_edo();
        let policy = SplitPolicy::new(LinearSplit::of(&map).unwrap());
        assert!(RepresentativePolicy::<()>::claims_homomorphic(&policy));

        let a = map.image().element([5i64]).unwrap();
        let b = map.image().element([-2i64]).unwrap();
        let sum = a.checked_add(&b).unwrap();

        let lift = |class| policy.choose(class, &()).unwrap().lift;
        assert_eq!(
            lift(&sum),
            lift(&a).checked_add(&lift(&b)).unwrap(),
            "a policy claiming homomorphism must actually be additive"
        );
        assert!(policy.choose(&a, &()).unwrap().residue.is_zero());
    }

    #[test]
    fn an_offset_policy_is_a_right_inverse_but_not_additive() {
        let map = twelve_edo();
        let kernel = map.kernel().clone();
        let policy = OffsetPolicy::new(
            SplitPolicy::new(LinearSplit::of(&map).unwrap()),
            move |class: &crate::temperament::ImageElem, _: &()| {
                // Offset only for odd classes: additivity cannot survive this.
                if (&class.coordinates()[0] % 2i32) != 0.into() {
                    kernel.element([1, 0]).ok()
                } else {
                    None
                }
            },
        );

        assert!(!policy.claims_homomorphic());

        // Right-inverse law still holds everywhere.
        for coordinate in [-5i64, -1, 0, 1, 2, 7] {
            let class = map.image().element([coordinate]).unwrap();
            let decision = policy.choose(&class, &()).unwrap();
            assert_eq!(map.apply_to_image(&decision.lift).unwrap(), class);
        }

        // Additivity fails, as it must be allowed to.
        let one = map.image().element([1i64]).unwrap();
        let two = map.image().element([2i64]).unwrap();
        let lift = |class| policy.choose(class, &()).unwrap().lift;
        assert_ne!(lift(&two), lift(&one).checked_add(&lift(&one)).unwrap());
    }

    #[test]
    fn the_canonical_lift_policy_is_small_and_not_additive() {
        use super::CanonicalLiftPolicy;

        let map = twelve_edo();
        let basis = five_limit();
        let policy = CanonicalLiftPolicy::new(map.clone());

        assert!(!RepresentativePolicy::<()>::claims_homomorphic(&policy));

        // The class of the just fifth lifts back to the just fifth.
        let class = map
            .apply_to_image(&basis.monzo([-1, 1, 0]).unwrap())
            .unwrap();
        let decision = policy.choose(&class, &()).unwrap();
        assert_eq!(decision.lift, basis.monzo([-1, 1, 0]).unwrap());
        assert!(decision.residue.is_zero());

        // Right-inverse law holds for every class.
        for coordinate in -12i64..=12 {
            let class = map.image().element([coordinate]).unwrap();
            let decision = policy.choose(&class, &()).unwrap();
            assert_eq!(map.apply_to_image(&decision.lift).unwrap(), class);
        }

        // It is not additive, which is exactly why it is not a splitting. It
        // agrees with a linear section on some pairs and not on others; a
        // splitting has no such freedom.
        let lift = |coordinate: i64| {
            policy
                .choose(&map.image().element([coordinate]).unwrap(), &())
                .unwrap()
                .lift
        };
        assert_eq!(
            lift(14),
            lift(7).checked_add(&lift(7)).unwrap(),
            "additive here by coincidence"
        );
        assert_ne!(
            lift(2),
            lift(1).checked_add(&lift(1)).unwrap(),
            "and not additive here, which a splitting could never do"
        );
    }

    #[test]
    fn lens_laws_hold_for_a_non_homomorphic_policy() {
        let basis = five_limit();
        let map = twelve_edo();
        let kernel = map.kernel().clone();
        let policy = OffsetPolicy::new(
            SplitPolicy::new(LinearSplit::of(&map).unwrap()),
            move |class: &crate::temperament::ImageElem, _: &()| {
                if (&class.coordinates()[0] % 3i32) != 0.into() {
                    kernel.element([1, -1]).ok()
                } else {
                    None
                }
            },
        );
        let lens = StructuralLens::new(policy);

        let monzos = [
            basis.monzo([-1, 1, 0]).unwrap(),
            basis.monzo([-4, 4, -1]).unwrap(),
            basis.monzo([3, -2, 1]).unwrap(),
        ];
        let classes = [
            map.image().element([7i64]).unwrap(),
            map.image().element([0i64]).unwrap(),
            map.image().element([-5i64]).unwrap(),
        ];

        for monzo in &monzos {
            // Residue is an exact kernel element.
            let residue = lens.residue(monzo, &()).unwrap();
            let comma = map.kernel().embed(&residue).unwrap();
            assert!(map.kills(&comma).unwrap());

            // GetPut.
            let class = lens.get(monzo).unwrap();
            assert_eq!(&lens.put(monzo, &class, &()).unwrap(), monzo);

            for target in &classes {
                // PutGet.
                let put = lens.put(monzo, target, &()).unwrap();
                assert_eq!(&lens.get(&put).unwrap(), target);

                // PutPut.
                for second in &classes {
                    assert_eq!(
                        lens.put(&put, second, &()).unwrap(),
                        lens.put(monzo, second, &()).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn a_policy_that_breaks_its_contract_is_reported() {
        struct Broken(TemperamentMap);

        impl RepresentativePolicy<()> for Broken {
            type Error = TemperamentError;

            fn map(&self) -> &TemperamentMap {
                &self.0
            }

            fn choose(
                &self,
                _class: &crate::temperament::ImageElem,
                _context: &(),
            ) -> Result<super::LiftDecision, Self::Error> {
                // Not in the fiber of the requested class at all.
                let lift = self.0.domain().monzo([1, 0, 0]).unwrap();
                Ok(super::LiftDecision::new(
                    lift,
                    super::no_residue(&self.0)?,
                    None,
                ))
            }
        }

        let map = twelve_edo();
        let basis = five_limit();
        let lens = StructuralLens::new(Broken(map));
        let result = lens.residue(&basis.monzo([-1, 1, 0]).unwrap(), &());
        assert!(matches!(
            result,
            Err(LensError::Lattice(TemperamentError::NotARightInverse))
        ));
    }
}
