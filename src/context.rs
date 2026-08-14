//! The immutable theory context (prompt section 8, UMT-3.2 section 6.3).
//!
//! Definitions that many objects share - bases, ambient lattices, temperament
//! mappings - live once in a frozen registry and are referenced by stable
//! identifier. Section 6.3 requires exactly that: shared context MUST be
//! referenced rather than copied inconsistently into every event.
//!
//! This is also what makes serialization of context-dependent objects
//! possible. A monzo on the wire is a basis identifier plus exponents; loading
//! it means resolving that identifier against a context, which is the only way
//! to get back a monzo whose basis identity is real rather than assumed.
//!
//! The context is immutable once built and is shared as an [`alloc::sync::Arc`]
//! where sharing is wanted, so it is `Send + Sync`. Registries are ordered
//! maps, so iteration order is stable and derived output is reproducible.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::algebra::matrix::IntMatrix;
use crate::error::ContextError;
use crate::proportion::basis::{Basis, BasisId};
use crate::proportion::monzo::Monzo;
use crate::temperament::image::{AmbientLattice, LatticeId};
use crate::temperament::map::{RawTemperamentMap, TemperamentMap};

/// Stable identity of a registered temperament mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct MappingId(alloc::string::String);

impl MappingId {
    /// Wraps a stable mapping identity.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(id.into())
    }

    /// The identity text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for MappingId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A monzo in wire form: a basis reference plus exact exponents.
///
/// UMT layer: L1, exact. This is what a monzo looks like in a document. It
/// carries no basis definition, only the identifier of one, so it is
/// meaningless without the context that defines it - which is the point
/// (UMT-3.2 section 6.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MonzoRef {
    /// Identifier of the basis this monzo is over.
    pub basis: BasisId,
    /// Exact exponents, one per basis generator.
    #[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::vec_z"))]
    pub exponents: Vec<Z>,
}

/// A temperament mapping in wire form: two references plus an exact matrix.
///
/// UMT layer: L2, exact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TemperamentMapRef {
    /// Identifier of the domain basis.
    pub domain: BasisId,
    /// Identifier of the ambient lattice.
    pub ambient: LatticeId,
    /// The exact mapping matrix.
    pub matrix: IntMatrix,
}

/// A frozen registry of shared semantic definitions.
///
/// Built through [`TheoryContextBuilder`] and immutable afterwards. Two
/// contexts are equal when they register the same definitions under the same
/// identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TheoryContext {
    bases: BTreeMap<BasisId, Arc<Basis>>,
    ambients: BTreeMap<LatticeId, Arc<AmbientLattice>>,
    mappings: BTreeMap<MappingId, TemperamentMap>,
}

impl TheoryContext {
    /// Starts building a context.
    #[must_use]
    pub fn builder() -> TheoryContextBuilder {
        TheoryContextBuilder::default()
    }

    /// The registered basis with this identifier.
    #[must_use]
    pub fn basis(&self, id: &BasisId) -> Option<&Arc<Basis>> {
        self.bases.get(id)
    }

    /// The registered ambient lattice with this identifier.
    #[must_use]
    pub fn ambient(&self, id: &LatticeId) -> Option<&Arc<AmbientLattice>> {
        self.ambients.get(id)
    }

    /// The registered mapping with this identifier.
    #[must_use]
    pub fn mapping(&self, id: &MappingId) -> Option<&TemperamentMap> {
        self.mappings.get(id)
    }

    /// Every registered basis, in identifier order.
    pub fn bases(&self) -> impl Iterator<Item = (&BasisId, &Arc<Basis>)> {
        self.bases.iter()
    }

    /// Every registered ambient lattice, in identifier order.
    pub fn ambients(&self) -> impl Iterator<Item = (&LatticeId, &Arc<AmbientLattice>)> {
        self.ambients.iter()
    }

    /// Every registered mapping, in identifier order.
    pub fn mappings(&self) -> impl Iterator<Item = (&MappingId, &TemperamentMap)> {
        self.mappings.iter()
    }

    /// Resolves a wire-form monzo against this context.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::UnknownBasis`] if the basis is not registered,
    /// and [`ContextError::Monzo`] if the exponent count does not match its
    /// rank.
    pub fn resolve_monzo(&self, reference: &MonzoRef) -> Result<Monzo, ContextError> {
        let basis = self
            .basis(&reference.basis)
            .ok_or_else(|| ContextError::UnknownBasis {
                id: reference.basis.clone(),
            })?;
        Ok(Monzo::new(Arc::clone(basis), reference.exponents.clone())?)
    }

    /// Produces the wire form of a monzo.
    ///
    /// The basis need not be registered here; the reference records which
    /// basis identifier a reader must resolve.
    #[must_use]
    pub fn monzo_ref(monzo: &Monzo) -> MonzoRef {
        MonzoRef {
            basis: monzo.basis().id().clone(),
            exponents: monzo.exponents().to_vec(),
        }
    }

    /// Resolves a wire-form temperament mapping against this context.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::UnknownBasis`] or
    /// [`ContextError::UnknownAmbient`] if a reference is unregistered, and
    /// [`ContextError::Temperament`] if the matrix shape does not match.
    pub fn resolve_mapping(
        &self,
        reference: &TemperamentMapRef,
    ) -> Result<TemperamentMap, ContextError> {
        let domain = self
            .basis(&reference.domain)
            .ok_or_else(|| ContextError::UnknownBasis {
                id: reference.domain.clone(),
            })?;
        let ambient =
            self.ambient(&reference.ambient)
                .ok_or_else(|| ContextError::UnknownAmbient {
                    id: reference.ambient.clone(),
                })?;
        Ok(TemperamentMap::new(RawTemperamentMap {
            domain: Arc::clone(domain),
            ambient: Arc::clone(ambient),
            matrix: reference.matrix.clone(),
        })?)
    }

    /// Produces the wire form of a temperament mapping.
    #[must_use]
    pub fn mapping_ref(map: &TemperamentMap) -> TemperamentMapRef {
        TemperamentMapRef {
            domain: map.domain().id().clone(),
            ambient: map.ambient().id().clone(),
            matrix: map.matrix().clone(),
        }
    }
}

/// Mutable builder for a [`TheoryContext`] (prompt section 52).
///
/// Registration is rejected rather than silently overwritten when an
/// identifier is reused for different content: a document that redefines an
/// identifier is malformed, and quietly taking the last definition would make
/// every monzo referring to the earlier one wrong.
#[derive(Debug, Clone, Default)]
pub struct TheoryContextBuilder {
    context: TheoryContext,
}

impl TheoryContextBuilder {
    /// Registers a basis under its own identifier.
    ///
    /// Registering the identical definition twice is accepted; registering a
    /// different definition under the same identifier is not.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::ConflictingBasis`] on a redefinition.
    pub fn basis(mut self, basis: &Arc<Basis>) -> Result<Self, ContextError> {
        let id = basis.id().clone();
        match self.context.bases.get(&id) {
            Some(existing) if !existing.same_identity(basis) => {
                return Err(ContextError::ConflictingBasis { id });
            }
            Some(_) => {}
            None => {
                self.context.bases.insert(id, Arc::clone(basis));
            }
        }
        Ok(self)
    }

    /// Registers an ambient lattice under its own identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::ConflictingAmbient`] on a redefinition.
    pub fn ambient(mut self, ambient: &Arc<AmbientLattice>) -> Result<Self, ContextError> {
        let id = ambient.id().clone();
        match self.context.ambients.get(&id) {
            Some(existing) if !existing.same_identity(ambient) => {
                return Err(ContextError::ConflictingAmbient { id });
            }
            Some(_) => {}
            None => {
                self.context.ambients.insert(id, Arc::clone(ambient));
            }
        }
        Ok(self)
    }

    /// Registers a temperament mapping, along with its domain basis and
    /// ambient lattice.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::ConflictingMapping`] on a redefinition, or a
    /// conflict from the basis or ambient lattice it pulls in.
    pub fn mapping(mut self, id: &str, map: &TemperamentMap) -> Result<Self, ContextError> {
        let key = MappingId::new(id);
        if let Some(existing) = self.context.mappings.get(&key)
            && existing != map
        {
            return Err(ContextError::ConflictingMapping { id: key });
        }
        self = self.basis(map.domain())?.ambient(map.ambient())?;
        self.context.mappings.insert(key, map.clone());
        Ok(self)
    }

    /// Freezes the context.
    #[must_use]
    pub fn build(self) -> Arc<TheoryContext> {
        Arc::new(self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::{MappingId, MonzoRef, TheoryContext};
    use crate::algebra::Z;
    use crate::error::ContextError;
    use crate::proportion::Basis;
    use crate::proportion::basis::BasisId;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use alloc::vec;

    fn context() -> alloc::sync::Arc<TheoryContext> {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        TheoryContext::builder()
            .mapping("umt:map:12edo-5limit", &map)
            .unwrap()
            .build()
    }

    #[test]
    fn registering_a_mapping_registers_what_it_depends_on() {
        let context = context();
        assert!(context.basis(&BasisId::new("umt:prime:2.3.5")).is_some());
        assert!(
            context
                .ambient(&crate::temperament::LatticeId::new("umt:edo:12"))
                .is_some()
        );
        assert!(
            context
                .mapping(&MappingId::new("umt:map:12edo-5limit"))
                .is_some()
        );
        assert_eq!(context.bases().count(), 1);
        assert_eq!(context.mappings().count(), 1);
    }

    #[test]
    fn monzos_round_trip_through_their_reference_form() {
        let context = context();
        let basis = context.basis(&BasisId::new("umt:prime:2.3.5")).unwrap();
        let comma = basis.monzo([-4, 4, -1]).unwrap();

        let reference = TheoryContext::monzo_ref(&comma);
        assert_eq!(reference.basis, BasisId::new("umt:prime:2.3.5"));
        assert_eq!(context.resolve_monzo(&reference).unwrap(), comma);
    }

    #[test]
    fn mappings_round_trip_through_their_reference_form() {
        let context = context();
        let map = context
            .mapping(&MappingId::new("umt:map:12edo-5limit"))
            .unwrap();
        let reference = TheoryContext::mapping_ref(map);
        assert_eq!(&context.resolve_mapping(&reference).unwrap(), map);
    }

    #[test]
    fn unresolvable_references_are_rejected() {
        let context = context();
        let reference = MonzoRef {
            basis: BasisId::new("umt:prime:2.3.7"),
            exponents: vec![Z::from(1), Z::from(0), Z::from(0)],
        };
        assert_eq!(
            context.resolve_monzo(&reference).unwrap_err(),
            ContextError::UnknownBasis {
                id: BasisId::new("umt:prime:2.3.7")
            }
        );

        // Right basis, wrong rank.
        let reference = MonzoRef {
            basis: BasisId::new("umt:prime:2.3.5"),
            exponents: vec![Z::from(1)],
        };
        assert!(matches!(
            context.resolve_monzo(&reference),
            Err(ContextError::Monzo(_))
        ));
    }

    #[test]
    fn redefining_an_identifier_is_rejected() {
        let five = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let impostor = Basis::primes("umt:prime:2.3.5", &[2, 3, 7]).unwrap();

        // The same definition twice is fine.
        assert!(
            TheoryContext::builder()
                .basis(&five)
                .unwrap()
                .basis(&five)
                .is_ok()
        );

        // A different definition under the same identifier is not.
        assert_eq!(
            TheoryContext::builder()
                .basis(&five)
                .unwrap()
                .basis(&impostor)
                .unwrap_err(),
            ContextError::ConflictingBasis {
                id: BasisId::new("umt:prime:2.3.5")
            }
        );
    }

    #[test]
    fn a_resolved_monzo_is_compatible_with_context_objects() {
        let context = context();
        let map = context
            .mapping(&MappingId::new("umt:map:12edo-5limit"))
            .unwrap();
        let reference = MonzoRef {
            basis: BasisId::new("umt:prime:2.3.5"),
            exponents: vec![Z::from(-4), Z::from(4), Z::from(-1)],
        };
        // The resolved monzo carries the registered basis handle, so it works
        // with everything else the context holds.
        let comma = context.resolve_monzo(&reference).unwrap();
        assert!(map.kills(&comma).unwrap());
    }
}
