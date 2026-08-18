//! Realization records and the device-adapter contract (UMT-3.2 sections 7.1,
//! 7.4, and 7.6).
//!
//! # Entering the pipeline somewhere other than the top
//!
//! Section 7.1 draws the L0-to-L4 path and then immediately says it "is not a
//! mandatory route for every object": an empirical tuning may enter at L3,
//! natively tempered notation may target L2, and temporal-constraint objects
//! need not pass through a temperament map at all. What it requires is that a
//! profile entering or bypassing the path "MUST record its entry layer and any
//! interpretation, approximation, or structural information that was omitted".
//!
//! [`RealizationRecord`] is that record. Its entry and exit layers are
//! mandatory fields, and what was omitted is a list rather than an
//! afterthought.
//!
//! # "Lossless given the choice" has exactly two justifications
//!
//! Section 7.4 says an L2-to-L3 realization need not be injective, so the
//! claim is valid only when the map is injective on the represented domain, or
//! when the L2 source is retained alongside the L3 result - and UMT-3.2 uses
//! the second as its default. [`RoundTripBasis`] has those two variants and a
//! third meaning "neither", and [`RealizationRecord::claims_lossless`] is true
//! only for the first two.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::RealizationError;
use crate::realization::provenance::{CanonicalValue, ProvenanceId};
use crate::realization::residual::{ResidualKind, ResidualSet};

/// A layer of the UMT-3.2 pipeline (section 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Layer {
    /// Notation: spelled symbols, ties, tuplet brackets.
    L0Notation,
    /// Exact semantic structure: monzos, exact ratios, rational durations.
    L1Exact,
    /// Structural classes: tempered classes, image lattices, meter.
    L2Quotient,
    /// Real metric realization: log-frequency, tuning curves, tempo maps.
    L3Metric,
    /// Device representation: ticks, control words.
    L4Device,
}

impl Layer {
    /// The layers strictly between two others, in pipeline order.
    ///
    /// Empty when the two are adjacent or out of order.
    #[must_use]
    pub fn between(from: Self, to: Self) -> Vec<Self> {
        const ORDER: [Layer; 5] = [
            Layer::L0Notation,
            Layer::L1Exact,
            Layer::L2Quotient,
            Layer::L3Metric,
            Layer::L4Device,
        ];
        ORDER
            .into_iter()
            .filter(|layer| *layer > from && *layer < to)
            .collect()
    }
}

impl core::fmt::Display for Layer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::L0Notation => "L0",
            Self::L1Exact => "L1",
            Self::L2Quotient => "L2",
            Self::L3Metric => "L3",
            Self::L4Device => "L4",
        })
    }
}

/// What justifies a round-trip claim (UMT-3.2 section 7.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RoundTripBasis {
    /// The realization map is injective on the represented domain, so the
    /// result determines its source.
    ///
    /// A strong claim, and one that has to be true of the *represented*
    /// domain rather than of the map in general.
    InjectiveOnDomain {
        /// What domain injectivity was established on.
        domain: String,
    },
    /// The source object is retained alongside the result.
    ///
    /// UMT-3.2's default design, and the one that needs no argument.
    SourceRetained,
    /// Neither holds, so the realization is not losslessly reversible.
    ///
    /// Perfectly legitimate. Saying so is the requirement.
    NotReversible,
}

/// A record of one realization step (UMT-3.2 section 7.1).
///
/// UMT layer: spans two, and says which.
#[derive(Debug, Clone, PartialEq)]
pub struct RealizationRecord {
    entry: Layer,
    exit: Layer,
    bypassed: Vec<Layer>,
    omitted: Vec<String>,
    round_trip: RoundTripBasis,
    residuals: ResidualSet,
    provenance: Option<ProvenanceId>,
}

impl RealizationRecord {
    /// Records a step from one layer to another.
    ///
    /// The bypassed layers are derived: any layer strictly between the entry
    /// and the exit was not traversed, and section 7.1 wants that recorded.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::BackwardRealization`] if the exit precedes
    /// the entry. Backward paths exist and are type-specific, but they are not
    /// this record: section 7.1 explicitly rejects treating every adjacent
    /// pair as the same kind of lens.
    pub fn new(entry: Layer, exit: Layer) -> Result<Self, RealizationError> {
        if exit < entry {
            return Err(RealizationError::BackwardRealization { entry, exit });
        }
        Ok(Self {
            entry,
            exit,
            bypassed: Layer::between(entry, exit),
            omitted: Vec::new(),
            round_trip: RoundTripBasis::NotReversible,
            residuals: ResidualSet::new(),
            provenance: None,
        })
    }

    /// Declares what was omitted.
    #[must_use]
    pub fn omitting(mut self, what: &str) -> Self {
        self.omitted.push(what.into());
        self
    }

    /// Declares what justifies a round-trip claim.
    #[must_use]
    pub fn with_round_trip(mut self, basis: RoundTripBasis) -> Self {
        self.round_trip = basis;
        self
    }

    /// Attaches the residuals this step produced.
    #[must_use]
    pub fn with_residuals(mut self, residuals: ResidualSet) -> Self {
        self.residuals = residuals;
        self
    }

    /// Attaches provenance.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// The layer the object entered at.
    #[must_use]
    pub fn entry(&self) -> Layer {
        self.entry
    }

    /// The layer it left at.
    #[must_use]
    pub fn exit(&self) -> Layer {
        self.exit
    }

    /// The layers it did not pass through.
    #[must_use]
    pub fn bypassed(&self) -> &[Layer] {
        &self.bypassed
    }

    /// What was omitted.
    #[must_use]
    pub fn omitted(&self) -> &[String] {
        &self.omitted
    }

    /// What justifies a round-trip claim.
    #[must_use]
    pub fn round_trip(&self) -> &RoundTripBasis {
        &self.round_trip
    }

    /// The residuals produced.
    #[must_use]
    pub fn residuals(&self) -> &ResidualSet {
        &self.residuals
    }

    /// The provenance of the step.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// Whether the result can be taken back to its source.
    ///
    /// True only under the two conditions section 7.4 admits. A record with
    /// residuals may still be reversible - a retained source makes it so - and
    /// a record with none may not be, if the map was non-injective and nothing
    /// was kept.
    #[must_use]
    pub fn claims_lossless(&self) -> bool {
        !matches!(self.round_trip, RoundTripBasis::NotReversible)
    }

    /// Whether this record is fit to participate in a conformance decision
    /// (UMT-3.2 section 7.10).
    ///
    /// A step that produced residuals needs provenance for them and for
    /// itself; a step that produced none needs neither. This reports which
    /// case the record is in rather than assuming.
    #[must_use]
    pub fn is_attributable(&self) -> bool {
        if self.residuals.is_empty() {
            return true;
        }
        self.provenance.is_some() && self.residuals.is_fully_attributed()
    }

    /// Whether any residual of a given kind was recorded.
    #[must_use]
    pub fn produced(&self, kind: ResidualKind) -> bool {
        self.residuals.of_kind(kind).next().is_some()
    }
}

/// How a device adapter behaves at the edge of its representable range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SaturationBehaviour {
    /// Values outside the range are clamped to the nearest representable one,
    /// and the difference is a device-control residual.
    Clamp,
    /// Values outside the range are refused.
    Reject,
    /// Values outside the range wrap around.
    Wrap,
    /// Something else, declared by the adapter.
    Declared(String),
}

/// The contract a device adapter must declare (UMT-3.2 section 7.6).
///
/// UMT layer: L3 to L4.
///
/// Section 7.6 lists six things a device adapter MUST declare, so all six are
/// mandatory fields. The seventh entry is not from the list but from the
/// sentence after it: "Not every device map is a Galois connection", so
/// whether this one is an order adjunction is a declared fact rather than an
/// assumption a caller may make.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceAdapterProfile {
    /// What the device can represent.
    pub representable_domain: String,
    /// How values are quantized or encoded.
    pub encoding_policy: String,
    /// What happens at the edge of the range.
    pub saturation: SaturationBehaviour,
    /// The device's resolution.
    pub resolution: CanonicalValue,
    /// How the residual is modelled.
    pub residual_model: String,
    /// Whether the encoding depends on prior state.
    pub stateful: bool,
    /// Whether the map is one of the order adjunctions of section 5.7.
    ///
    /// Floor and ceiling lattice quantizers are; nearest rounding and stateful
    /// encodings generally are not, and claiming otherwise would license laws
    /// that do not hold.
    pub is_order_adjunction: bool,
}

impl DeviceAdapterProfile {
    /// Whether this profile makes every declaration section 7.6 requires.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.representable_domain.is_empty()
            && !self.encoding_policy.is_empty()
            && !self.residual_model.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceAdapterProfile, Layer, RealizationRecord, RoundTripBasis, SaturationBehaviour,
    };
    use crate::algebra::{Q, Z};
    use crate::error::RealizationError;
    use crate::realization::provenance::{CanonicalValue, ProvenanceId};
    use crate::realization::residual::{Residual, ResidualKind, ResidualRecord, ResidualSet};
    use crate::time::beat::Beats;
    use alloc::string::String;

    #[test]
    fn a_record_derives_the_layers_it_skipped() {
        // An empirical tuning entering at L3 and going straight to a device.
        let record = RealizationRecord::new(Layer::L3Metric, Layer::L4Device).unwrap();
        assert!(record.bypassed().is_empty(), "adjacent layers skip nothing");

        // Notation compiled all the way down bypasses the middle two.
        let full = RealizationRecord::new(Layer::L0Notation, Layer::L4Device).unwrap();
        assert_eq!(
            full.bypassed(),
            &[Layer::L1Exact, Layer::L2Quotient, Layer::L3Metric]
        );

        // Natively tempered notation targeting L2 skips L1.
        let tempered = RealizationRecord::new(Layer::L0Notation, Layer::L2Quotient).unwrap();
        assert_eq!(tempered.bypassed(), &[Layer::L1Exact]);
        assert_eq!(tempered.entry(), Layer::L0Notation);
        assert_eq!(tempered.exit(), Layer::L2Quotient);
        assert_eq!(Layer::L1Exact.to_string(), "L1");
    }

    #[test]
    fn a_backward_step_is_not_this_record() {
        assert!(matches!(
            RealizationRecord::new(Layer::L3Metric, Layer::L1Exact),
            Err(RealizationError::BackwardRealization { .. })
        ));
    }

    #[test]
    fn losslessness_needs_one_of_exactly_two_justifications() {
        let base = RealizationRecord::new(Layer::L2Quotient, Layer::L3Metric).unwrap();
        assert!(
            !base.claims_lossless(),
            "the default claim is the modest one"
        );

        let retained = base.clone().with_round_trip(RoundTripBasis::SourceRetained);
        assert!(retained.claims_lossless());

        let injective = base.with_round_trip(RoundTripBasis::InjectiveOnDomain {
            domain: String::from("the 12 pitch classes of this score"),
        });
        assert!(injective.claims_lossless());
    }

    #[test]
    fn a_record_with_residuals_needs_them_attributed() {
        let provenance = ProvenanceId::new("umt:prov:quantize-1");
        let attributed = ResidualSet::new().with(
            ResidualRecord::new(Residual::Grid {
                deviation: Beats::new(Q::new(Z::from(1), Z::from(480))),
            })
            .with_provenance(provenance.clone()),
        );
        let anonymous = ResidualSet::new().with(ResidualRecord::new(Residual::Grid {
            deviation: Beats::new(Q::new(Z::from(1), Z::from(480))),
        }));

        let clean = RealizationRecord::new(Layer::L3Metric, Layer::L4Device).unwrap();
        assert!(
            clean.is_attributable(),
            "a step that lost nothing needs no attribution"
        );

        let good = clean
            .clone()
            .with_residuals(attributed)
            .with_provenance(provenance);
        assert!(good.is_attributable());
        assert!(good.produced(ResidualKind::Grid));
        assert!(!good.produced(ResidualKind::Structural));

        let bad = clean.with_residuals(anonymous);
        assert!(
            !bad.is_attributable(),
            "residuals without provenance cannot support a conformance decision"
        );
    }

    #[test]
    fn omissions_are_listed_rather_than_implied() {
        let record = RealizationRecord::new(Layer::L3Metric, Layer::L4Device)
            .unwrap()
            .omitting("the L2 source object")
            .omitting("the exact rhythm tree");
        assert_eq!(record.omitted().len(), 2);
        assert!(record.omitted().iter().any(|what| what.contains("tree")));
    }

    #[test]
    fn a_device_profile_declares_all_six_and_the_seventh() {
        let profile = DeviceAdapterProfile {
            representable_domain: String::from("MIDI note 0-127 with 14-bit bend, +/- 2 semitones"),
            encoding_policy: String::from("nearest note, remainder as bend, halves away from zero"),
            saturation: SaturationBehaviour::Clamp,
            resolution: CanonicalValue::Rational(Q::new(Z::from(4), Z::from(8192))),
            residual_model: String::from("device-control residual in semitones"),
            stateful: true,
            is_order_adjunction: false,
        };
        assert!(profile.is_complete());
        assert!(
            !profile.is_order_adjunction,
            "nearest rounding with state is not a Galois connection"
        );
        assert_eq!(profile.saturation, SaturationBehaviour::Clamp);

        let incomplete = DeviceAdapterProfile {
            representable_domain: String::new(),
            ..profile
        };
        assert!(!incomplete.is_complete());
    }
}
