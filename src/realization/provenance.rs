//! Provenance records and their arena (UMT-3.2 section 7.10, prompt section
//! 36).
//!
//! Section 7.10: every non-exact realization or inference that participates in
//! a conformance decision MUST carry provenance sufficient to identify the
//! semantic profile, algorithm and version, and the parameters that affect the
//! result.
//!
//! Two design constraints follow, and prompt section 36 states both. Records
//! live in an arena and are referenced by [`ProvenanceId`], so a document does
//! not copy the same record into every object it describes. And parameters are
//! [`CanonicalValue`], a small typed tree, rather than an arbitrary JSON blob -
//! which means an exact rational parameter stays exact on the wire instead of
//! becoming a double somewhere between authoring and reproduction.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::algebra::{Q, RoundingConvention, Z};
use crate::error::RealizationError;

/// A stable reference to a provenance record.
///
/// UMT layer: metadata, applicable at every layer. The record itself -
/// algorithm, version, parameters, seed, tolerance, source measurements,
/// parents - is stored once in a [`ProvenanceArena`] and referenced by this
/// identifier rather than copied into every object (prompt section 36).
///
/// The identifier is a stable string, not a process-local counter, so it
/// survives serialization (prompt section 8).
///
/// Equality is presentation equality on the identifier text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct ProvenanceId(Arc<str>);

impl ProvenanceId {
    /// Wraps a stable identifier.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProvenanceId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<ProvenanceId> for String {
    fn from(value: ProvenanceId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for ProvenanceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity of an algorithm.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct AlgorithmId(Arc<str>);

impl AlgorithmId {
    /// Wraps a stable algorithm identity.
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

impl From<String> for AlgorithmId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<AlgorithmId> for String {
    fn from(value: AlgorithmId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for AlgorithmId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parameter value, typed and canonically serializable.
///
/// UMT layer: metadata, but with the crate's exactness discipline intact. An
/// exact rational tolerance is [`CanonicalValue::Rational`] and stays exact
/// through a round trip; a measured one is [`CanonicalValue::Real`] and is
/// honest about being a double. An untyped blob could not tell the difference,
/// which is why prompt section 36 rules one out.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum CanonicalValue {
    /// Text.
    Text(String),
    /// An exact arbitrary-precision integer.
    Integer(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::z"))] Z),
    /// An exact arbitrary-precision rational.
    Rational(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))] Q),
    /// A real value, which is a measurement or a tolerance rather than an
    /// exact quantity.
    Real(f64),
    /// A flag.
    Boolean(bool),
    /// An ordered list.
    List(Vec<CanonicalValue>),
    /// An ordered map, so serialized output is reproducible.
    Map(BTreeMap<String, CanonicalValue>),
}

impl CanonicalValue {
    /// Whether this value is exact, that is, free of floating point
    /// throughout.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        match self {
            Self::Real(_) => false,
            Self::Text(_) | Self::Integer(_) | Self::Rational(_) | Self::Boolean(_) => true,
            Self::List(values) => values.iter().all(Self::is_exact),
            Self::Map(values) => values.values().all(Self::is_exact),
        }
    }
}

/// The format and version an object was imported from or exported to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormatVersion {
    /// The format's declared identifier.
    pub format: String,
    /// Its version.
    pub version: String,
}

/// A provenance record (UMT-3.2 section 7.10, prompt section 36).
///
/// UMT layer: metadata.
///
/// The algorithm and its version are mandatory, because section 7.10 requires
/// provenance "sufficient to identify the semantic profile, algorithm/version,
/// and parameters that affect the result" and a record without them cannot.
/// Everything else is optional and named for what it is, so a reader can tell
/// an absent seed from a seed of zero.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenanceRecord {
    /// Which algorithm produced the result.
    pub algorithm: AlgorithmId,
    /// Its version.
    pub version: String,
    /// Parameters that affect the result, in a reproducible order.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: BTreeMap<String, CanonicalValue>,
    /// The records this one derives from.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parents: Vec<ProvenanceId>,
    /// The random seed, where the algorithm uses one.
    #[cfg_attr(feature = "serde", serde(default))]
    pub seed: Option<u64>,
    /// The optimization tolerance, where the algorithm has one.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tolerance: Option<CanonicalValue>,
    /// The rounding mode, where one applies.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rounding: Option<RoundingConvention>,
    /// Identifiers of the source measurements this rests on.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sources: Vec<String>,
    /// The uncertainty model, where the result carries uncertainty.
    #[cfg_attr(feature = "serde", serde(default))]
    pub uncertainty_model: Option<String>,
    /// The format this came from or went to.
    #[cfg_attr(feature = "serde", serde(default))]
    pub format: Option<FormatVersion>,
}

impl Default for AlgorithmId {
    fn default() -> Self {
        Self::new("")
    }
}

impl ProvenanceRecord {
    /// A record naming an algorithm and its version.
    #[must_use]
    pub fn new(algorithm: AlgorithmId, version: &str) -> Self {
        Self {
            algorithm,
            version: version.into(),
            ..Self::default()
        }
    }

    /// Adds a parameter.
    #[must_use]
    pub fn with_parameter(mut self, name: &str, value: CanonicalValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    /// Declares a parent record.
    #[must_use]
    pub fn with_parent(mut self, parent: ProvenanceId) -> Self {
        self.parents.push(parent);
        self
    }

    /// Declares the random seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Declares the optimization tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tolerance: CanonicalValue) -> Self {
        self.tolerance = Some(tolerance);
        self
    }

    /// Declares the rounding mode.
    #[must_use]
    pub fn with_rounding(mut self, rounding: RoundingConvention) -> Self {
        self.rounding = Some(rounding);
        self
    }

    /// Whether this record names an algorithm and a version at all.
    ///
    /// A record that does not cannot satisfy section 7.10, and
    /// [`ProvenanceArena::insert`] refuses it.
    #[must_use]
    pub fn identifies_its_algorithm(&self) -> bool {
        !self.algorithm.as_str().is_empty() && !self.version.is_empty()
    }

    /// Whether every parameter is exact.
    ///
    /// Not a requirement - a tolerance is a real number and should be - but
    /// worth being able to ask when reproducibility is in question.
    #[must_use]
    pub fn parameters_are_exact(&self) -> bool {
        self.parameters.values().all(CanonicalValue::is_exact)
    }
}

/// An arena of provenance records, referenced by identifier
/// (prompt section 36).
///
/// UMT layer: metadata.
///
/// Parents must already be present when a record is inserted, which makes the
/// ancestry graph acyclic by construction rather than by a later check. That
/// in turn makes [`ProvenanceArena::ancestors`] terminate for every input.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ProvenanceArena {
    records: BTreeMap<ProvenanceId, ProvenanceRecord>,
}

impl ProvenanceArena {
    /// An empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a record under an identifier.
    ///
    /// # Errors
    ///
    /// - [`RealizationError::AnonymousProvenance`] if the record does not name
    ///   an algorithm and a version, which section 7.10 requires;
    /// - [`RealizationError::DuplicateProvenance`] if the identifier is taken
    ///   by a different record;
    /// - [`RealizationError::UnknownProvenance`] if a declared parent is not
    ///   already in the arena, which is what keeps the graph acyclic.
    pub fn insert(
        &mut self,
        id: ProvenanceId,
        record: ProvenanceRecord,
    ) -> Result<(), RealizationError> {
        if !record.identifies_its_algorithm() {
            return Err(RealizationError::AnonymousProvenance { id });
        }
        for parent in &record.parents {
            if !self.records.contains_key(parent) {
                return Err(RealizationError::UnknownProvenance { id: parent.clone() });
            }
        }
        match self.records.get(&id) {
            Some(existing) if *existing != record => {
                Err(RealizationError::DuplicateProvenance { id })
            }
            Some(_) => Ok(()),
            None => {
                self.records.insert(id, record);
                Ok(())
            }
        }
    }

    /// The record stored under an identifier.
    #[must_use]
    pub fn get(&self, id: &ProvenanceId) -> Option<&ProvenanceRecord> {
        self.records.get(id)
    }

    /// Every record, in identifier order.
    pub fn records(&self) -> impl Iterator<Item = (&ProvenanceId, &ProvenanceRecord)> {
        self.records.iter()
    }

    /// How many records are stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Every ancestor of a record, transitively, in identifier order.
    ///
    /// Terminates for every input, because insertion refuses a parent that is
    /// not already present and therefore admits no cycles.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::UnknownProvenance`] if the starting
    /// identifier is not in the arena.
    pub fn ancestors(&self, id: &ProvenanceId) -> Result<Vec<ProvenanceId>, RealizationError> {
        if !self.records.contains_key(id) {
            return Err(RealizationError::UnknownProvenance { id: id.clone() });
        }
        let mut seen = alloc::collections::BTreeSet::new();
        let mut frontier = alloc::vec![id.clone()];
        while let Some(current) = frontier.pop() {
            let Some(record) = self.records.get(&current) else {
                continue;
            };
            for parent in &record.parents {
                if seen.insert(parent.clone()) {
                    frontier.push(parent.clone());
                }
            }
        }
        Ok(seen.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AlgorithmId, CanonicalValue, FormatVersion, ProvenanceArena, ProvenanceId, ProvenanceRecord,
    };
    use crate::algebra::{Q, RoundingConvention, Z};
    use crate::error::RealizationError;
    use alloc::string::String;

    fn record(name: &str) -> ProvenanceRecord {
        ProvenanceRecord::new(AlgorithmId::new(name), "0.1.0")
    }

    #[test]
    fn a_record_must_identify_its_algorithm() {
        let mut arena = ProvenanceArena::new();
        let anonymous = ProvenanceRecord::default();
        assert!(!anonymous.identifies_its_algorithm());
        assert!(matches!(
            arena.insert(ProvenanceId::new("p1"), anonymous),
            Err(RealizationError::AnonymousProvenance { .. })
        ));

        // A version is required too, not just a name.
        let unversioned = ProvenanceRecord::new(AlgorithmId::new("umt:algo:x"), "");
        assert!(!unversioned.identifies_its_algorithm());
        assert!(arena.insert(ProvenanceId::new("p1"), unversioned).is_err());

        assert!(
            arena
                .insert(
                    ProvenanceId::new("p1"),
                    record("umt:algo:minimum-complexity")
                )
                .is_ok()
        );
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn parameters_keep_their_exactness() {
        let exact = record("umt:algo:quantize")
            .with_parameter("ticks_per_beat", CanonicalValue::Integer(Z::from(96)))
            .with_parameter(
                "weight",
                CanonicalValue::Rational(Q::new(Z::from(1), Z::from(5))),
            )
            .with_rounding(RoundingConvention::NearestHalfAwayFromZero);
        assert!(exact.parameters_are_exact());

        // A tolerance is a real number, and saying so is the point.
        let approximate = exact
            .clone()
            .with_tolerance(CanonicalValue::Real(1e-9))
            .with_parameter("epsilon", CanonicalValue::Real(0.5));
        assert!(!approximate.parameters_are_exact());
        assert!(!CanonicalValue::Real(0.5).is_exact());
        assert!(CanonicalValue::Integer(Z::from(3)).is_exact());
        assert!(
            !CanonicalValue::List(alloc::vec![
                CanonicalValue::Integer(Z::from(1)),
                CanonicalValue::Real(0.5)
            ])
            .is_exact(),
            "a list is exact only if every element is"
        );
    }

    #[test]
    fn the_arena_admits_no_cycles_because_parents_must_exist_first() {
        let mut arena = ProvenanceArena::new();
        let root = ProvenanceId::new("p1");
        let child = ProvenanceId::new("p2");

        // A parent that is not there yet is refused, which is what makes a
        // cycle unconstructible.
        assert!(matches!(
            arena.insert(
                child.clone(),
                record("umt:algo:b").with_parent(root.clone())
            ),
            Err(RealizationError::UnknownProvenance { .. })
        ));

        arena.insert(root.clone(), record("umt:algo:a")).unwrap();
        arena
            .insert(
                child.clone(),
                record("umt:algo:b").with_parent(root.clone()),
            )
            .unwrap();
        let grandchild = ProvenanceId::new("p3");
        arena
            .insert(
                grandchild.clone(),
                record("umt:algo:c").with_parent(child.clone()),
            )
            .unwrap();

        assert_eq!(arena.ancestors(&grandchild).unwrap(), [root, child]);
        assert!(arena.ancestors(&ProvenanceId::new("nowhere")).is_err());
    }

    #[test]
    fn re_inserting_the_same_record_is_fine_and_a_different_one_is_not() {
        let mut arena = ProvenanceArena::new();
        let id = ProvenanceId::new("p1");
        arena.insert(id.clone(), record("umt:algo:a")).unwrap();
        arena.insert(id.clone(), record("umt:algo:a")).unwrap();
        assert!(matches!(
            arena.insert(id, record("umt:algo:b")),
            Err(RealizationError::DuplicateProvenance { .. })
        ));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn a_record_carries_what_section_7_10_asks_for() {
        let full = record("umt:algo:adaptive-ji")
            .with_seed(42)
            .with_tolerance(CanonicalValue::Real(1e-6))
            .with_rounding(RoundingConvention::Floor)
            .with_parameter("lambda_v", CanonicalValue::Real(1.0));
        let full = ProvenanceRecord {
            sources: alloc::vec![String::from("umt:measurement:1")],
            uncertainty_model: Some(String::from("gaussian, one sigma")),
            format: Some(FormatVersion {
                format: String::from("scala"),
                version: String::from("1"),
            }),
            ..full
        };

        assert!(full.identifies_its_algorithm());
        assert_eq!(full.seed, Some(42));
        assert_eq!(full.rounding, Some(RoundingConvention::Floor));
        assert_eq!(full.sources.len(), 1);
        assert!(full.uncertainty_model.is_some());
        assert_eq!(full.format.as_ref().unwrap().format, "scala");

        let mut arena = ProvenanceArena::new();
        arena.insert(ProvenanceId::new("p1"), full.clone()).unwrap();
        assert_eq!(arena.get(&ProvenanceId::new("p1")), Some(&full));
        assert_eq!(arena.records().count(), 1);
    }
}
