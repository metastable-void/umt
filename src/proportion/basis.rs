//! Formal bases and basis identity (UMT-3.2 section 1.1).

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::Z;
use crate::error::{BasisError, MonzoError};
use crate::proportion::monzo::Monzo;
use crate::proportion::valuation::{PositiveQ, RealValuation};

/// Stable identity of a formal generator.
///
/// UMT layer: L1 metadata. Generator identity is semantic: it is what a
/// serialized monzo coordinate refers to, so it must be stable across
/// processes and documents (prompt section 8).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct GeneratorId(Arc<str>);

impl GeneratorId {
    /// Wraps a stable generator identity.
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

impl From<String> for GeneratorId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<GeneratorId> for String {
    fn from(value: GeneratorId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for GeneratorId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity of a basis.
///
/// UMT layer: L1 metadata. Two exponent vectors of the same length are not
/// interchangeable unless they are over the same basis, so this identity is
/// what makes a mismatch detectable (prompt section 7).
///
/// Sharing an identity is necessary but not sufficient for compatibility:
/// [`Basis::same_identity`] also compares the declared generators, so a
/// document that reuses an identifier for different content cannot silently
/// corrupt arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct BasisId(Arc<str>);

impl BasisId {
    /// Wraps a stable basis identity.
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

impl From<String> for BasisId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<BasisId> for String {
    fn from(value: BasisId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for BasisId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The valuation attached to a formal generator.
///
/// UMT-3.2 section 1.1 keeps the generator formal and exact at L1 in both
/// profiles; only the valuation differs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum GeneratorValuation {
    /// Exact positive rational valuation (section 1.1.1).
    Rational(PositiveQ),
    /// Symbolic-real valuation attached at L3 (section 1.1.2).
    SymbolicReal(RealValuation),
}

impl GeneratorValuation {
    /// The exact rational valuation, if this generator has one.
    #[must_use]
    pub fn as_rational(&self) -> Option<&PositiveQ> {
        match self {
            Self::Rational(value) => Some(value),
            Self::SymbolicReal(_) => None,
        }
    }

    /// Whether this valuation is exact.
    ///
    /// A structural object derived from a non-exact valuation must record that
    /// fact; see [`crate::temperament::edo::Exactness`].
    #[must_use]
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Rational(_))
    }

    /// L3 real approximation of `log2` of this valuation.
    #[must_use]
    pub fn log2_f64(&self) -> f64 {
        match self {
            Self::Rational(value) => value.log2_f64(),
            Self::SymbolicReal(value) => value.log2_f64(),
        }
    }
}

/// A formal generator together with its valuation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasisGenerator {
    id: GeneratorId,
    valuation: GeneratorValuation,
}

impl BasisGenerator {
    /// Pairs an identity with a valuation.
    #[must_use]
    pub fn new(id: GeneratorId, valuation: GeneratorValuation) -> Self {
        Self { id, valuation }
    }

    /// The generator's stable identity.
    #[must_use]
    pub fn id(&self) -> &GeneratorId {
        &self.id
    }

    /// The generator's valuation.
    #[must_use]
    pub fn valuation(&self) -> &GeneratorValuation {
        &self.valuation
    }
}

/// How multiplicative independence of the generators is established.
///
/// UMT-3.2 section 1.1.2: a symbolic-real basis MUST either declare
/// independence as a modeling assumption or supply an exact algebraic
/// certificate. Independence MUST NOT be inferred from floating-point
/// inequality tests, so this crate never attempts to compute it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum IndependenceContract {
    /// Guaranteed by unique factorization of a prime basis.
    PrimeFactorization,
    /// Asserted by the modeller, with a note recording the justification.
    Declared {
        /// Free-text justification carried for provenance.
        note: String,
    },
    /// Established by an exact algebraic certificate held elsewhere.
    Certified {
        /// Reference to the certificate.
        certificate: String,
    },
}

/// The unvalidated form of a [`Basis`].
///
/// This is what appears on the wire. Loading it goes through
/// `TryFrom<RawBasis>`, so a deserialized basis is subject to exactly the same
/// invariant checks as one built in memory (prompt sections 12 and 39).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawBasis {
    /// Declared basis identity.
    pub id: BasisId,
    /// Declared generators, in order. Order is significant: it fixes the
    /// meaning of every monzo coordinate.
    pub generators: Vec<BasisGenerator>,
    /// Declared independence contract.
    pub independence: IndependenceContract,
}

/// A validated formal basis `B = (beta_1, ..., beta_k)` spanning the exact
/// proportion lattice `Lambda_B = Z^k` (UMT-3.2 section 1.1).
///
/// UMT layer: L1, exact.
///
/// Invariants established at construction:
///
/// - generator identities are unique within the basis;
/// - every exact valuation is in `Q_{>0}`;
/// - a [`IndependenceContract::PrimeFactorization`] contract built through
///   [`Basis::primes`] really is over primes.
///
/// Equality is presentation equality over the identity, the ordered
/// generators, and the independence contract - see [`Basis::same_identity`].
/// A basis is immutable once built and is shared through `Arc`, so monzos
/// carry a cheap handle rather than a copy of the definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "RawBasis", try_from = "RawBasis"))]
pub struct Basis {
    id: BasisId,
    generators: Vec<BasisGenerator>,
    independence: IndependenceContract,
}

impl Basis {
    /// Starts building a basis.
    #[must_use]
    pub fn builder(id: &str) -> BasisBuilder {
        BasisBuilder {
            id: BasisId::new(id),
            generators: Vec::new(),
            independence: IndependenceContract::Declared {
                note: String::from("independence not declared"),
            },
        }
    }

    /// Builds a rational prime basis such as `(2, 3, 5)`.
    ///
    /// Generator identities are the decimal forms of the primes, and the
    /// independence contract is [`IndependenceContract::PrimeFactorization`].
    ///
    /// # Errors
    ///
    /// Returns [`BasisError::NotPrime`] if any entry is not prime, and
    /// [`BasisError::DuplicateGeneratorId`] if an entry is repeated. Primality
    /// is checked because unique factorization is the whole justification for
    /// the independence claim (UMT-3.2 section 1.1.1).
    ///
    /// # Examples
    ///
    /// ```
    /// use umt::Basis;
    ///
    /// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    /// assert_eq!(basis.rank(), 3);
    /// assert!(Basis::primes("bad", &[2, 4]).is_err());
    /// # Ok::<(), umt::error::BasisError>(())
    /// ```
    pub fn primes(id: &str, primes: &[u32]) -> Result<Arc<Self>, BasisError> {
        let mut builder = Self::builder(id).independence(IndependenceContract::PrimeFactorization);
        for prime in primes {
            if !is_prime(*prime) {
                return Err(BasisError::NotPrime { value: *prime });
            }
            let id = alloc::format!("{prime}");
            builder = builder.rational_generator(&id, PositiveQ::integer(*prime)?);
        }
        builder.build()
    }

    /// The basis identity.
    #[must_use]
    pub fn id(&self) -> &BasisId {
        &self.id
    }

    /// The rank `k` of `Lambda_B`.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.generators.len()
    }

    /// The ordered generators.
    #[must_use]
    pub fn generators(&self) -> &[BasisGenerator] {
        &self.generators
    }

    /// The generator at `index`, if any.
    #[must_use]
    pub fn generator(&self, index: usize) -> Option<&BasisGenerator> {
        self.generators.get(index)
    }

    /// The declared independence contract.
    #[must_use]
    pub fn independence(&self) -> &IndependenceContract {
        &self.independence
    }

    /// Whether every generator has an exact rational valuation, so that the
    /// exact ratio map `r(m)` of section 1.1.1 is defined on the whole lattice.
    #[must_use]
    pub fn is_rational_profile(&self) -> bool {
        self.generators.iter().all(|g| g.valuation().is_exact())
    }

    /// Whether two bases are the same declared object.
    ///
    /// This compares the identity, the ordered generators, and the
    /// independence contract. Sharing an identifier alone is not enough: a
    /// document that reuses an identifier for different generators must not
    /// silently pass a compatibility check.
    #[must_use]
    pub fn same_identity(&self, other: &Self) -> bool {
        self == other
    }

    /// Builds a monzo over this basis.
    ///
    /// # Errors
    ///
    /// Returns [`MonzoError::RankMismatch`] if the number of exponents differs
    /// from the rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use umt::Basis;
    ///
    /// let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    /// // The syntonic comma 81/80 = 2^-4 * 3^4 * 5^-1.
    /// let comma = basis.monzo([-4, 4, -1])?;
    /// assert_eq!(comma.exact_ratio()?.to_string(), "81/80");
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn monzo<I, T>(self: &Arc<Self>, exponents: I) -> Result<Monzo, MonzoError>
    where
        I: IntoIterator<Item = T>,
        T: Into<Z>,
    {
        Monzo::new(
            Arc::clone(self),
            exponents.into_iter().map(Into::into).collect(),
        )
    }

    /// The zero monzo over this basis, representing the proportion 1.
    #[must_use]
    pub fn zero(self: &Arc<Self>) -> Monzo {
        Monzo::zero(Arc::clone(self))
    }
}

impl From<Basis> for RawBasis {
    fn from(value: Basis) -> Self {
        Self {
            id: value.id,
            generators: value.generators,
            independence: value.independence,
        }
    }
}

impl TryFrom<RawBasis> for Basis {
    type Error = BasisError;

    fn try_from(value: RawBasis) -> Result<Self, Self::Error> {
        for (index, generator) in value.generators.iter().enumerate() {
            if value.generators[..index]
                .iter()
                .any(|earlier| earlier.id() == generator.id())
            {
                return Err(BasisError::DuplicateGeneratorId {
                    id: generator.id().clone(),
                });
            }
        }
        Ok(Self {
            id: value.id,
            generators: value.generators,
            independence: value.independence,
        })
    }
}

/// Incremental builder for a [`Basis`] (prompt section 52).
///
/// Convenience is allowed here; every invariant is checked in
/// [`BasisBuilder::build`].
#[derive(Debug, Clone)]
pub struct BasisBuilder {
    id: BasisId,
    generators: Vec<BasisGenerator>,
    independence: IndependenceContract,
}

impl BasisBuilder {
    /// Appends a generator with an exact rational valuation.
    #[must_use]
    pub fn rational_generator(mut self, id: &str, value: PositiveQ) -> Self {
        self.generators.push(BasisGenerator::new(
            GeneratorId::new(id),
            GeneratorValuation::Rational(value),
        ));
        self
    }

    /// Appends a generator with a symbolic-real valuation.
    #[must_use]
    pub fn symbolic_real_generator(mut self, id: &str, value: RealValuation) -> Self {
        self.generators.push(BasisGenerator::new(
            GeneratorId::new(id),
            GeneratorValuation::SymbolicReal(value),
        ));
        self
    }

    /// Declares how independence is established.
    #[must_use]
    pub fn independence(mut self, contract: IndependenceContract) -> Self {
        self.independence = contract;
        self
    }

    /// Validates and freezes the basis.
    ///
    /// # Errors
    ///
    /// Returns [`BasisError::DuplicateGeneratorId`] if two generators share an
    /// identity.
    pub fn build(self) -> Result<Arc<Basis>, BasisError> {
        Basis::try_from(RawBasis {
            id: self.id,
            generators: self.generators,
            independence: self.independence,
        })
        .map(Arc::new)
    }
}

fn is_prime(value: u32) -> bool {
    if value < 2 {
        return false;
    }
    if value % 2 == 0 {
        return value == 2;
    }
    let mut divisor = 3u32;
    while let Some(square) = divisor.checked_mul(divisor) {
        if square > value {
            break;
        }
        if value % divisor == 0 {
            return false;
        }
        divisor += 2;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{Basis, IndependenceContract, PositiveQ};
    use crate::algebra::Q;
    use crate::error::BasisError;

    #[test]
    fn prime_basis_validates_primality() {
        assert!(Basis::primes("p", &[2, 3, 5, 7, 11, 13]).is_ok());
        assert_eq!(
            Basis::primes("p", &[2, 3, 4]).unwrap_err(),
            BasisError::NotPrime { value: 4 }
        );
        assert_eq!(
            Basis::primes("p", &[1]).unwrap_err(),
            BasisError::NotPrime { value: 1 }
        );
        assert_eq!(
            Basis::primes("p", &[2, 2]).unwrap_err(),
            BasisError::DuplicateGeneratorId {
                id: super::GeneratorId::new("2")
            }
        );
    }

    #[test]
    fn duplicate_generator_identities_are_rejected() {
        let value = PositiveQ::new(Q::new(3.into(), 2.into())).unwrap();
        let result = Basis::builder("b")
            .rational_generator("g", value.clone())
            .rational_generator("g", value)
            .build();
        assert!(matches!(
            result,
            Err(BasisError::DuplicateGeneratorId { .. })
        ));
    }

    #[test]
    fn rational_profile_detection() {
        let basis = Basis::primes("p", &[2, 3]).unwrap();
        assert!(basis.is_rational_profile());
        assert_eq!(
            basis.independence(),
            &IndependenceContract::PrimeFactorization
        );
    }

    #[test]
    fn bases_with_the_same_id_but_different_generators_are_not_compatible() {
        let a = Basis::primes("shared-id", &[2, 3, 5]).unwrap();
        let b = Basis::primes("shared-id", &[2, 3, 7]).unwrap();
        assert!(!a.same_identity(&b));
    }
}
