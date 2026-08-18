//! The native UMT container (UMT-3.2 section 8.8, prompt section 39).
//!
//! Section 8.8 gives the container a shape and one structural rule: "Domain
//! sections are present only when required by the represented objects; this is
//! necessary because UMT permits, for example, direct empirical L3 scales with
//! no L1 basis and domains with no distinguished periodic unit."
//!
//! So every domain section here is optional, and absence is meaningful rather
//! than a defect. A document holding nothing but a measured scale has no
//! `basis` and no `unit`, and is valid - which is fixture F29.
//!
//! # Versioning and profiles
//!
//! A container carries a schema version and a declared profile set. The
//! schema version governs whether this build can read the *encoding*
//! ([`crate::io::version::UmtSchemaVersion::can_read`]); the profiles govern
//! whether it understands the *semantics*.
//!
//! Prompt section 39 asks to "allow unknown future extension fields where
//! feasible without silently treating them as understood", and profiles are
//! how that is done. A document may declare `umt.future`; this build will load
//! it, and [`UmtDocument::unsupported_profiles`] will name it, and
//! [`UmtDocument::is_fully_understood`] will be `false`. Nothing pretends
//! otherwise.
//!
//! Extension *data* goes in [`UmtDocument::extensions`], as
//! [`CanonicalValue`], so it keeps its exactness on the wire rather than
//! becoming a float on the way through.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::context::{MonzoRef, TemperamentMapRef};
use crate::error::IoError;
use crate::io::version::{NATIVE_SCHEMA_VERSION, UmtSchemaVersion};
use crate::pitch::empirical::EmpiricalScale;
use crate::pitch::point::PitchPointRef;
use crate::proportion::basis::RawBasis;
use crate::realization::provenance::{CanonicalValue, ProvenanceArena};
use crate::realization::record::Layer;
use crate::score::container::ScoreRef;
use crate::time::constraint::{ExternalPredicate, StpProblem};
use crate::time::meter::{Grouping, Meter};
use crate::time::rhythm::RhythmTree;
use crate::time::tempo::TempoMap;

/// The semantic profiles this build implements.
///
/// A document declaring a profile outside this list is loadable but not fully
/// understood, and says so.
pub const SUPPORTED_PROFILES: &[&str] = &[
    "umt.core",
    "umt.pitch",
    "umt.time",
    "umt.score",
    "umt.realization",
];

/// How a representative policy is identified on the wire
/// (UMT-3.2 section 8.8).
///
/// Section 8.8 ends with a requirement that is easy to miss: "A custom or
/// adaptive representative policy that cannot be reproduced from a stable
/// policy identifier, version, and parameters MUST serialize the selected
/// lifts/residues actually used by the score when those choices are required
/// for round trip."
///
/// So there are two ways to be reproducible, and a policy has to be one of
/// them. [`PolicyDeclaration::is_reproducible`] reports which.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyDeclaration {
    /// What kind of policy it is: homomorphic splitting, canonical lift,
    /// adaptive, or an application's own.
    pub kind: String,
    /// A stable identifier, where the policy has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub policy_id: Option<String>,
    /// The algorithm version, where the policy has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub algorithm_version: Option<String>,
    /// Parameters that affect the choice.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: BTreeMap<String, CanonicalValue>,
    /// Whether the policy claims to be a homomorphism.
    ///
    /// Section 8.9 requires a round trip to preserve "whether a
    /// representative policy is homomorphic or merely set-theoretic".
    pub homomorphic: bool,
    /// The lifts actually selected, where the policy cannot be reproduced
    /// from its identifier alone.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub resolved_lifts: Vec<MonzoRef>,
}

impl PolicyDeclaration {
    /// Whether this declaration is enough to reproduce the policy's choices.
    ///
    /// True when the policy has a stable identifier and version, or when it
    /// serialized the lifts it actually used. Section 8.8 requires one or the
    /// other.
    #[must_use]
    pub fn is_reproducible(&self) -> bool {
        (self.policy_id.is_some() && self.algorithm_version.is_some())
            || !self.resolved_lifts.is_empty()
    }
}

/// A pitch reference: a structural point and the frequency that realizes it
/// (UMT-3.2 section 4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PitchReferenceSection {
    /// The designated structural point.
    pub point: PitchPointRef,
    /// The frequency in hertz that realizes it.
    pub frequency_hz: crate::pitch::units::FrequencyHz,
}

/// A declared tuning or realization (UMT-3.2 section 8.8).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TuningSection {
    /// Which interval group the tuning is defined on: the ambient lattice or
    /// the reachable image (UMT-3.2 section 1.9).
    pub interval_group: String,
    /// The size of each generator, in octaves.
    pub generator_sizes: Vec<crate::pitch::units::Octaves>,
    /// Whether the realization claims to be a regular homomorphism.
    ///
    /// Law T3 forbids advertising a context-dependent realization as one.
    pub regular: bool,
}

/// The temporal-constraint section (UMT-3.2 section 8.8).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TemporalSection {
    /// The difference-bound network, where the document has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub stp: Option<StpProblem>,
    /// Which solver profile was selected.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub solver_profile: Option<String>,
    /// External predicates, as typed data and never as code.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub predicates: Vec<ExternalPredicate>,
}

/// The metrical section (UMT-3.2 section 8.8).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeterSection {
    /// The metrical hierarchies in force.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub meters: Vec<Meter>,
    /// Grouping structures, which need not agree with the meters.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub groupings: Vec<Grouping>,
}

/// A native UMT document (UMT-3.2 section 8.8).
///
/// UMT layer: whichever layers its sections represent, which
/// [`UmtDocument::represented_layers`] reports.
///
/// Every domain section is optional. That is not laxity: section 8.8 requires
/// it, because UMT-3.2 admits objects that have no basis, no unit, no mapping,
/// and no events.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmtDocument {
    /// The specification version whose semantics this document uses.
    pub umt_version: String,
    /// The encoding schema version.
    pub schema: UmtSchemaVersion,
    /// The semantic profiles the document declares.
    #[cfg_attr(feature = "serde", serde(default))]
    pub profiles: Vec<String>,

    /// The proportion basis, where the document has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub basis: Option<RawBasis>,
    /// The distinguished periodic unit, where the document has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub unit: Option<MonzoRef>,
    /// The temperament mapping, where the document has one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub mapping: Option<TemperamentMapRef>,
    /// The representative policy in force.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub representative_policy: Option<PolicyDeclaration>,
    /// The pitch reference.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pitch_reference: Option<PitchReferenceSection>,
    /// The tuning or realization.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub tuning: Option<TuningSection>,
    /// Meter and grouping.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub meter_and_grouping: Option<MeterSection>,
    /// Rhythm trees.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub rhythm_trees: Vec<RhythmTree>,
    /// The tempo map.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub tempo: Option<TempoMap>,
    /// Temporal constraints.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub temporal_constraints: Option<TemporalSection>,
    /// The event-indexed score.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub events: Option<ScoreRef>,
    /// Directly measured L3 scales, which have no basis and need none.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub empirical_scales: Vec<EmpiricalScale>,

    /// Provenance records, referenced by identifier from everywhere else.
    #[cfg_attr(feature = "serde", serde(default))]
    pub provenance: ProvenanceArena,
    /// Extension data, in canonical form.
    ///
    /// A producer puts data this specification does not describe here. It
    /// keeps its exactness, and a consumer that does not recognize a key can
    /// say so rather than guessing.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub extensions: BTreeMap<String, CanonicalValue>,
}

impl Default for UmtDocument {
    /// An empty document at the native schema version, declaring the core
    /// profile and carrying no domain section at all.
    fn default() -> Self {
        Self {
            umt_version: crate::UMT_SPEC_VERSION.into(),
            schema: NATIVE_SCHEMA_VERSION,
            profiles: alloc::vec![String::from("umt.core")],
            basis: None,
            unit: None,
            mapping: None,
            representative_policy: None,
            pitch_reference: None,
            tuning: None,
            meter_and_grouping: None,
            rhythm_trees: Vec::new(),
            tempo: None,
            temporal_constraints: None,
            events: None,
            empirical_scales: Vec::new(),
            provenance: ProvenanceArena::new(),
            extensions: BTreeMap::new(),
        }
    }
}

impl UmtDocument {
    /// An empty document at the native schema version, declaring the core
    /// profile.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a semantic profile.
    #[must_use]
    pub fn with_profile(mut self, profile: &str) -> Self {
        let profile = String::from(profile);
        if !self.profiles.contains(&profile) {
            self.profiles.push(profile);
        }
        self
    }

    /// The profiles this build does not implement.
    ///
    /// Empty for a document this build fully understands. Non-empty is not an
    /// error: it is the honest answer, and a consumer decides what to do with
    /// it.
    #[must_use]
    pub fn unsupported_profiles(&self) -> Vec<&str> {
        self.profiles
            .iter()
            .map(String::as_str)
            .filter(|profile| !SUPPORTED_PROFILES.contains(profile))
            .collect()
    }

    /// Whether this build understands every profile the document declares.
    #[must_use]
    pub fn is_fully_understood(&self) -> bool {
        self.unsupported_profiles().is_empty()
    }

    /// Which UMT layers the document's present sections represent.
    ///
    /// Section 8.1 asks an adapter to declare which layers it represents; a
    /// native document can compute it from what is actually there.
    #[must_use]
    pub fn represented_layers(&self) -> Vec<Layer> {
        let mut layers = Vec::new();
        if self.events.is_some() {
            layers.push(Layer::L0Notation);
        }
        if self.basis.is_some() || !self.rhythm_trees.is_empty() {
            layers.push(Layer::L1Exact);
        }
        if self.mapping.is_some() || self.meter_and_grouping.is_some() {
            layers.push(Layer::L2Quotient);
        }
        if self.tuning.is_some()
            || self.tempo.is_some()
            || !self.empirical_scales.is_empty()
            || self.pitch_reference.is_some()
        {
            layers.push(Layer::L3Metric);
        }
        layers.sort_unstable();
        layers.dedup();
        layers
    }

    /// Validates the document's internal consistency.
    ///
    /// This is not a schema check - `serde` has already done that - but a
    /// semantic one, over the invariants section 8.9 requires a round trip to
    /// preserve.
    ///
    /// # Errors
    ///
    /// - [`IoError::UnreadableSchema`] if this build cannot read the declared
    ///   schema version;
    /// - [`IoError::UnitWithoutBasis`] if a distinguished unit is present with
    ///   no basis to interpret its coordinates against;
    /// - [`IoError::IrreproduciblePolicy`] if a representative policy can be
    ///   reproduced neither from an identifier and version nor from the lifts
    ///   it actually used;
    /// - [`IoError::DanglingProvenance`] if a section references a provenance
    ///   record the document does not carry.
    pub fn validate(&self) -> Result<(), IoError> {
        if !NATIVE_SCHEMA_VERSION.can_read(self.schema) {
            return Err(IoError::UnreadableSchema {
                document: self.schema,
                native: NATIVE_SCHEMA_VERSION,
            });
        }
        if self.unit.is_some() && self.basis.is_none() {
            return Err(IoError::UnitWithoutBasis);
        }
        if let Some(policy) = &self.representative_policy
            && !policy.is_reproducible()
        {
            return Err(IoError::IrreproduciblePolicy);
        }
        for scale in &self.empirical_scales {
            for id in scale
                .provenance()
                .into_iter()
                .chain(scale.degrees().iter().filter_map(|d| d.provenance()))
            {
                if self.provenance.get(id).is_none() {
                    return Err(IoError::DanglingProvenance { id: id.clone() });
                }
            }
        }
        Ok(())
    }

    /// Whether the document carries any domain section at all.
    ///
    /// A document with none is a header, which is legal and occasionally
    /// useful.
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.basis.is_some()
            || self.mapping.is_some()
            || self.events.is_some()
            || self.tuning.is_some()
            || self.tempo.is_some()
            || self.meter_and_grouping.is_some()
            || self.temporal_constraints.is_some()
            || !self.rhythm_trees.is_empty()
            || !self.empirical_scales.is_empty()
    }
}
