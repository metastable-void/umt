//! Empirical scales and optional lattice inference (UMT-3.2 section 4.9).
//!
//! Section 4.9 opens by rejecting a premise: "Arithmetic ratio simplicity is
//! not a universal sensory-consonance law." A measured or traditional tuning
//! may therefore be stored *directly* as real interval values with uncertainty
//! and provenance, and no just-intonation lattice explanation is required
//! (section 4.9.1).
//!
//! [`EmpiricalScale`] is that representation. It enters the pipeline at L3 and
//! has no basis, no monzos, and no temperament mapping. It need not even have
//! a distinguished periodic unit - many tunings do not have an octave, and
//! fixture F29 requires a document containing one to remain valid with both
//! the `basis` and `unit` sections absent.
//!
//! # Fitting a lattice is optional, separate, and heavily qualified
//!
//! Section 4.9.3 permits inferring a lattice model, and then lists six things
//! such an inference MUST declare. [`FitDeclaration`] has six mandatory fields
//! for exactly that reason, and [`LatticeFit`] is stored *alongside* the
//! measurements rather than replacing them - so the scale is still the scale
//! after someone has had a theory about it.
//!
//! Section 4.9.3 also warns that "there is no canonical instruction to take a
//! maximally independent subset of local minima": different maximal subsets
//! exist, numerical minima are unstable under measurement error, and
//! approximate real independence is not an exact observable. This crate
//! therefore infers nothing on its own. A fit is something a caller supplies,
//! with its declaration.
//!
//! # Not a special case of Tenney height
//!
//! Section 4.9.4: Tenney height and spectrum-derived dissonance "may correlate
//! in restricted contexts, but neither is defined as a special case of the
//! other". Nothing here computes one from the other.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::error::PitchError;
use crate::pitch::units::{Cents, Octaves};
use crate::proportion::basis::BasisId;
use crate::realization::provenance::ProvenanceId;
use crate::realization::residual::{Residual, ResidualRecord};

/// Stable identity of an empirical scale.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct ScaleId(Arc<str>);

impl ScaleId {
    /// Wraps a stable identity.
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

impl From<String> for ScaleId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<ScaleId> for String {
    fn from(value: ScaleId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for ScaleId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One measured scale degree (UMT-3.2 section 4.9.1).
///
/// UMT layer: L3. An interval from the scale's reference, its measurement
/// uncertainty, and where the measurement came from. Section 0.6.1 requires
/// every real-valued observation to carry provenance, which is why the field
/// is here rather than only on the scale.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmpiricalDegree {
    interval: Octaves,
    uncertainty: Octaves,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    provenance: Option<ProvenanceId>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    source: Option<String>,
}

impl EmpiricalDegree {
    /// A measured degree.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NegativeUncertainty`] for a negative
    /// uncertainty. Zero is legal and means the measurement was reported
    /// without one, which is different from claiming it is exact.
    pub fn new(interval: Octaves, uncertainty: Octaves) -> Result<Self, PitchError> {
        if uncertainty < Octaves::ZERO {
            return Err(PitchError::NegativeUncertainty);
        }
        Ok(Self {
            interval,
            uncertainty,
            provenance: None,
            source: None,
        })
    }

    /// A degree measured in cents.
    ///
    /// # Errors
    ///
    /// As [`EmpiricalDegree::new`].
    pub fn from_cents(interval: Cents, uncertainty: Cents) -> Result<Self, PitchError> {
        Self::new(Octaves::from(interval), Octaves::from(uncertainty))
    }

    /// Attaches provenance.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Names the source measurement.
    #[must_use]
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.into());
        self
    }

    /// The measured interval from the scale's reference.
    #[must_use]
    pub fn interval(&self) -> Octaves {
        self.interval
    }

    /// The interval in cents.
    #[must_use]
    pub fn cents(&self) -> Cents {
        Cents::from(self.interval)
    }

    /// The measurement uncertainty, which is never negative.
    #[must_use]
    pub fn uncertainty(&self) -> Octaves {
        self.uncertainty
    }

    /// Where the measurement came from.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// The source measurement's identifier.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Whether another degree lies within the two measurements' combined
    /// uncertainty.
    ///
    /// A convenience for comparing measurements, not a claim that the two are
    /// the same degree. Section 4.9.3 warns that numerical minima are unstable
    /// under measurement error, and this is the arithmetic behind that
    /// warning rather than a way around it.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        let separation = (self.interval.get() - other.interval.get()).abs();
        separation <= self.uncertainty.get() + other.uncertainty.get()
    }
}

/// Whether generator independence was assumed or certified
/// (UMT-3.2 sections 1.1.2 and 4.9.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum IndependenceClaim {
    /// Independence is assumed, and the fit says so rather than implying it.
    Assumed,
    /// Independence is certified, with a reference to the certificate.
    Certified {
        /// Where the certificate lives.
        certificate: String,
    },
}

/// What a lattice inference must declare (UMT-3.2 section 4.9.3).
///
/// UMT layer: declared metadata.
///
/// Six mandatory fields, one for each of the six things section 4.9.3 says an
/// inference MUST declare. None is optional, because a fit that omits any of
/// them cannot be assessed - and section 4.9.3 exists precisely because such
/// fits are easy to produce and hard to argue with.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FitDeclaration {
    /// How candidate intervals were selected.
    pub candidate_selection: String,
    /// The tolerance or uncertainty region used.
    pub tolerance: String,
    /// How many generators were requested.
    pub generators_requested: usize,
    /// How many were selected.
    pub generators_selected: usize,
    /// The optimization criterion.
    pub criterion: String,
    /// Whether independence was assumed or certified.
    pub independence: IndependenceClaim,
}

/// An optional lattice model fitted to an empirical scale
/// (UMT-3.2 section 4.9.3).
///
/// UMT layer: L1 model over L3 measurements.
///
/// Stored alongside the measurements rather than in place of them, so the
/// scale remains what it was measured to be. One residual per fitted degree is
/// mandatory: section 4.9.3 requires "the approximation residual for every
/// fitted interval", and a fit with fewer is rejected at construction.
#[derive(Debug, Clone, PartialEq)]
pub struct LatticeFit {
    basis: BasisId,
    residuals: Vec<ResidualRecord>,
    declaration: FitDeclaration,
    provenance: Option<ProvenanceId>,
}

impl LatticeFit {
    /// Records a fit of `degree_count` degrees.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::IncompleteFit`] if the number of residuals does
    /// not match the number of degrees, or if any residual is not an
    /// empirical-fit residual - the fit's residual is a measurement against a
    /// model, and no other kind of residual describes that.
    pub fn new(
        basis: BasisId,
        degree_count: usize,
        residuals: Vec<ResidualRecord>,
        declaration: FitDeclaration,
    ) -> Result<Self, PitchError> {
        if residuals.len() != degree_count
            || !residuals
                .iter()
                .all(|record| matches!(record.residual(), Residual::EmpiricalFit { .. }))
        {
            return Err(PitchError::IncompleteFit {
                degrees: degree_count,
                residuals: residuals.len(),
            });
        }
        Ok(Self {
            basis,
            residuals,
            declaration,
            provenance: None,
        })
    }

    /// Attaches provenance.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// The basis the fit is over.
    #[must_use]
    pub fn basis(&self) -> &BasisId {
        &self.basis
    }

    /// One residual per fitted degree.
    #[must_use]
    pub fn residuals(&self) -> &[ResidualRecord] {
        &self.residuals
    }

    /// What the inference declared.
    #[must_use]
    pub fn declaration(&self) -> &FitDeclaration {
        &self.declaration
    }

    /// The provenance of the inference.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// The largest absolute fit residual, in the declared unit.
    #[must_use]
    pub fn worst_residual(&self) -> f64 {
        self.residuals
            .iter()
            .filter_map(|record| match record.residual() {
                Residual::EmpiricalFit { deviation, .. } => Some(deviation.abs()),
                _ => None,
            })
            .fold(0.0, f64::max)
    }
}

/// A directly measured scale (UMT-3.2 section 4.9.1).
///
/// UMT layer: L3, entering the pipeline there.
///
/// No basis, no monzos, no mapping. The period is optional because many
/// tunings have no distinguished periodic unit, and a representation that
/// demanded one would force a claim the measurements do not support - which
/// is what fixture F29 checks.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmpiricalScale {
    id: ScaleId,
    degrees: Vec<EmpiricalDegree>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    period: Option<Octaves>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    provenance: Option<ProvenanceId>,
    #[cfg_attr(feature = "serde", serde(skip))]
    fit: Option<LatticeFit>,
}

impl EmpiricalScale {
    /// A scale of measured degrees, with no period and no lattice model.
    #[must_use]
    pub fn new<I>(id: ScaleId, degrees: I) -> Self
    where
        I: IntoIterator<Item = EmpiricalDegree>,
    {
        Self {
            id,
            degrees: degrees.into_iter().collect(),
            period: None,
            provenance: None,
            fit: None,
        }
    }

    /// Declares a periodic unit.
    ///
    /// Optional on purpose: a tuning without one is a valid scale, not an
    /// incomplete one.
    #[must_use]
    pub fn with_period(mut self, period: Octaves) -> Self {
        self.period = Some(period);
        self
    }

    /// Attaches provenance for the measurement campaign as a whole.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Attaches an inferred lattice model, alongside the measurements.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::IncompleteFit`] if the fit does not cover every
    /// degree.
    pub fn with_fit(mut self, fit: LatticeFit) -> Result<Self, PitchError> {
        if fit.residuals().len() != self.degrees.len() {
            return Err(PitchError::IncompleteFit {
                degrees: self.degrees.len(),
                residuals: fit.residuals().len(),
            });
        }
        self.fit = Some(fit);
        Ok(self)
    }

    /// The scale's identity.
    #[must_use]
    pub fn id(&self) -> &ScaleId {
        &self.id
    }

    /// The measured degrees, in order.
    #[must_use]
    pub fn degrees(&self) -> &[EmpiricalDegree] {
        &self.degrees
    }

    /// How many degrees the scale has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.degrees.len()
    }

    /// Whether the scale has no degrees.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.degrees.is_empty()
    }

    /// The declared periodic unit, if the scale has one.
    #[must_use]
    pub fn period(&self) -> Option<Octaves> {
        self.period
    }

    /// Whether the scale declares a periodic unit at all.
    #[must_use]
    pub fn has_period(&self) -> bool {
        self.period.is_some()
    }

    /// The provenance of the measurement campaign.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// The inferred lattice model, if one was supplied.
    ///
    /// `None` is the normal case. Section 4.9.1 is explicit that a direct
    /// empirical scale is "the minimum adequate representation for tunings
    /// whose cultural or acoustic basis is not established by a small-integer
    /// model", and no fit is required for it to be one.
    #[must_use]
    pub fn fit(&self) -> Option<&LatticeFit> {
        self.fit.as_ref()
    }

    /// Whether every degree carries provenance.
    ///
    /// Section 0.6.1 requires it of real-valued observations that participate
    /// in a conformance decision.
    #[must_use]
    pub fn is_fully_attributed(&self) -> bool {
        self.degrees
            .iter()
            .all(|degree| degree.provenance().is_some())
    }

    /// Whether the degrees ascend strictly.
    ///
    /// Not enforced: a measured scale may legitimately be reported in any
    /// order, and reordering it would discard the source's own presentation.
    #[must_use]
    pub fn is_ascending(&self) -> bool {
        self.degrees
            .windows(2)
            .all(|pair| pair[0].interval() < pair[1].interval())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmpiricalDegree, EmpiricalScale, FitDeclaration, IndependenceClaim, LatticeFit, ScaleId,
    };
    use crate::error::PitchError;
    use crate::pitch::units::{Cents, Octaves};
    use crate::proportion::basis::BasisId;
    use crate::realization::provenance::ProvenanceId;
    use crate::realization::residual::{Residual, ResidualRecord};
    use alloc::string::String;
    use alloc::vec::Vec;

    /// A measured pelog-like scale: five degrees, none of them a small
    /// integer ratio, with realistic uncertainties.
    fn measured() -> EmpiricalScale {
        let provenance = ProvenanceId::new("umt:prov:field-recording-1975");
        let degrees: Vec<EmpiricalDegree> = [
            (0.0, 0.0),
            (120.0, 8.0),
            (270.0, 9.0),
            (540.0, 7.0),
            (670.0, 11.0),
        ]
        .into_iter()
        .map(|(cents, uncertainty)| {
            EmpiricalDegree::from_cents(
                Cents::new(cents).unwrap(),
                Cents::new(uncertainty).unwrap(),
            )
            .unwrap()
            .with_provenance(provenance.clone())
            .with_source("umt:measurement:gamelan-1")
        })
        .collect();
        EmpiricalScale::new(ScaleId::new("umt:scale:measured-pelog"), degrees)
            .with_provenance(provenance)
    }

    #[test]
    fn a_measured_scale_needs_no_lattice_and_no_period() {
        let scale = measured();
        assert_eq!(scale.len(), 5);
        assert!(scale.fit().is_none(), "no rationalization is forced");
        assert!(!scale.has_period(), "and no periodic unit is invented");
        assert_eq!(scale.period(), None);
        assert!(scale.is_fully_attributed());
        assert!(scale.is_ascending());

        // The values are real intervals with uncertainty, not ratios.
        assert!((scale.degrees()[2].cents().get() - 270.0).abs() < 1e-9);
        assert!((scale.degrees()[2].uncertainty().get() * 1200.0 - 9.0).abs() < 1e-9);
        assert_eq!(
            scale.degrees()[2].source(),
            Some("umt:measurement:gamelan-1")
        );
    }

    #[test]
    fn a_period_can_be_declared_but_is_never_required() {
        let with_octave = measured().with_period(Octaves::new(1.0).unwrap());
        assert!(with_octave.has_period());
        assert_eq!(with_octave.period(), Some(Octaves::new(1.0).unwrap()));

        // A stretched period is just as declarable, which is the point of not
        // assuming one.
        let stretched = measured().with_period(Octaves::new(1.02).unwrap());
        assert_ne!(stretched.period(), with_octave.period());
    }

    #[test]
    fn uncertainties_are_never_negative() {
        assert!(matches!(
            EmpiricalDegree::new(Octaves::ZERO, Octaves::new(-0.001).unwrap()),
            Err(PitchError::NegativeUncertainty)
        ));
        assert!(
            EmpiricalDegree::new(Octaves::ZERO, Octaves::ZERO).is_ok(),
            "zero means no uncertainty was reported, not that it is exact"
        );
    }

    #[test]
    fn overlapping_measurements_are_reported_not_merged() {
        let a = EmpiricalDegree::from_cents(Cents::new(700.0).unwrap(), Cents::new(10.0).unwrap())
            .unwrap();
        let b = EmpiricalDegree::from_cents(Cents::new(706.0).unwrap(), Cents::new(5.0).unwrap())
            .unwrap();
        let c = EmpiricalDegree::from_cents(Cents::new(730.0).unwrap(), Cents::new(5.0).unwrap())
            .unwrap();

        assert!(a.overlaps(&b), "within the combined uncertainty");
        assert!(!a.overlaps(&c));
        assert_ne!(a, b, "and overlapping is not being equal");
    }

    #[test]
    fn a_fit_declares_all_six_things_and_covers_every_degree() {
        let scale = measured();
        let declaration = FitDeclaration {
            candidate_selection: String::from("nearest 7-limit ratio within 15 cents"),
            tolerance: String::from("15 cents, one sigma of the reported uncertainty"),
            generators_requested: 3,
            generators_selected: 3,
            criterion: String::from("minimize the sum of squared cent deviations"),
            independence: IndependenceClaim::Assumed,
        };
        let residuals: Vec<ResidualRecord> = [3.1, -4.0, 8.2, -1.5, 6.0]
            .into_iter()
            .map(|deviation| {
                ResidualRecord::new(Residual::empirical_fit(deviation, 9.0, "cents").unwrap())
                    .with_provenance(ProvenanceId::new("umt:prov:fit-1"))
            })
            .collect();

        let fit = LatticeFit::new(
            BasisId::new("umt:prime:2.3.5.7"),
            scale.len(),
            residuals.clone(),
            declaration.clone(),
        )
        .unwrap();
        assert!((fit.worst_residual() - 8.2).abs() < 1e-9);
        assert_eq!(fit.declaration().generators_selected, 3);
        assert_eq!(fit.declaration().independence, IndependenceClaim::Assumed);

        // The fit lives alongside the measurements, which are unchanged.
        let fitted = scale.clone().with_fit(fit).unwrap();
        assert!(fitted.fit().is_some());
        assert_eq!(fitted.degrees(), scale.degrees());
        assert!(!fitted.has_period(), "fitting a lattice invents no period");

        // A fit that does not cover every degree is refused.
        assert!(matches!(
            LatticeFit::new(
                BasisId::new("umt:prime:2.3.5.7"),
                scale.len(),
                residuals[..2].to_vec(),
                declaration
            ),
            Err(PitchError::IncompleteFit { .. })
        ));
    }

    #[test]
    fn a_fit_residual_must_be_an_empirical_fit_residual() {
        let declaration = FitDeclaration {
            candidate_selection: String::from("x"),
            tolerance: String::from("y"),
            generators_requested: 1,
            generators_selected: 1,
            criterion: String::from("z"),
            independence: IndependenceClaim::Certified {
                certificate: String::from("umt:certificate:1"),
            },
        };
        let wrong_kind = alloc::vec![ResidualRecord::new(Residual::TuningDeviation {
            deviation: Octaves::new(0.01).unwrap()
        })];
        assert!(matches!(
            LatticeFit::new(BasisId::new("umt:prime:2.3"), 1, wrong_kind, declaration),
            Err(PitchError::IncompleteFit { .. })
        ));
    }
}
