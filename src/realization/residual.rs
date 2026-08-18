//! The residual taxonomy (UMT-3.2 section 7.9, prompt section 35).
//!
//! Section 7.9 opens with a rule rather than a definition: "UMT-3.2 never
//! stores one undifferentiated `error` field." The seven residual kinds it
//! tabulates live in genuinely different spaces - one is an exact lattice
//! element, one is a real interval, one carries an uncertainty, one is
//! symbolic - and collapsing them into a number would make every downstream
//! question unanswerable.
//!
//! So [`Residual`] is an enum whose variants carry their own units, and there
//! is deliberately **no `Add` implementation**. Two residuals combine only
//! through [`Residual::try_add`], which succeeds within a kind that is
//! genuinely additive and refuses otherwise.
//!
//! # What is and is not additive
//!
//! Grid residuals over consecutive children add: that is how endpoint drift
//! accumulates, and summing them is exactly the right question. Structural
//! residues add, because the kernel is a group. Tuning and temporal deviations
//! add as real intervals.
//!
//! Empirical-fit residuals do not, because their uncertainties would have to
//! combine under a model this crate has not been told. Device-control and
//! notation residuals do not either: one is a pair of encoded values and the
//! other is symbolic. Refusing is better than picking a convention nobody
//! declared.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::RealizationError;
use crate::pitch::units::Octaves;
use crate::realization::provenance::ProvenanceId;
use crate::temperament::kernel::KernelElem;
use crate::time::beat::Beats;
use crate::time::units::Seconds;

/// Which row of the section 7.9 table a residual belongs to.
///
/// Present as a value so a caller can group and filter residuals without
/// matching on the payload, and so a report can say *what kind* of loss it is
/// describing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ResidualKind {
    /// An exact element of the kernel `K`.
    Structural,
    /// A real interval on the log-frequency line.
    TuningDeviation,
    /// A real measurement difference, with an uncertainty.
    EmpiricalFit,
    /// A real duration on the physical timeline.
    TemporalRealization,
    /// An exact structural duration lost to a device grid.
    Grid,
    /// A real control value, requested against encoded.
    DeviceControl,
    /// Symbolic notation information with no numeric value at all.
    Notation,
}

impl ResidualKind {
    /// Whether two residuals of this kind may be added.
    ///
    /// See the module documentation for why the answer is `false` for three
    /// of the seven.
    #[must_use]
    pub fn is_additive(self) -> bool {
        matches!(
            self,
            Self::Structural | Self::TuningDeviation | Self::TemporalRealization | Self::Grid
        )
    }
}

/// A typed realization residual (UMT-3.2 section 7.9).
///
/// UMT layer: whichever layer the residual arose at; every variant says so in
/// its own type. Units are intrinsic where the specification fixes them and
/// declared where it does not.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Residual {
    /// Exact structural information about which lift of a tempered class was
    /// present: the comma the temperament discarded.
    ///
    /// This is not a floating error and never becomes one (UMT-3.2 section
    /// 7.3).
    Structural {
        /// The exact kernel element.
        comma: KernelElem,
    },
    /// A realized interval against a selected target, in octaves.
    TuningDeviation {
        /// Realized minus target.
        deviation: Octaves,
    },
    /// A measured value against a fitted model, with the uncertainty of the
    /// measurement and the unit both declared.
    EmpiricalFit {
        /// Measured minus fitted.
        deviation: f64,
        /// The measurement uncertainty, which is never negative.
        uncertainty: f64,
        /// The declared unit both numbers are in.
        unit: String,
    },
    /// A realized duration against an affine reference, in seconds.
    TemporalRealization {
        /// Realized minus reference.
        deviation: Seconds,
    },
    /// An exact structural onset or duration against its represented tick.
    ///
    /// Exact rational, following UMT-3.2 section 5.7.5: when the source
    /// boundaries and the grid are exact rationals, the residual stays exact.
    Grid {
        /// Exact source minus represented.
        deviation: Beats,
    },
    /// A requested control value against the code a device could encode.
    DeviceControl {
        /// What was asked for.
        requested: f64,
        /// What the device could represent.
        encoded: f64,
        /// The declared unit both numbers are in.
        unit: String,
    },
    /// Symbolic notation information the semantic object does not retain: a
    /// courtesy accidental, a tuplet-bracket choice, an enharmonic layout
    /// convention (UMT-3.2 section 7.2).
    Notation {
        /// What was discarded, in the notation system's own vocabulary.
        detail: String,
    },
}

impl Residual {
    /// Which row of the section 7.9 table this residual is.
    #[must_use]
    pub fn kind(&self) -> ResidualKind {
        match self {
            Self::Structural { .. } => ResidualKind::Structural,
            Self::TuningDeviation { .. } => ResidualKind::TuningDeviation,
            Self::EmpiricalFit { .. } => ResidualKind::EmpiricalFit,
            Self::TemporalRealization { .. } => ResidualKind::TemporalRealization,
            Self::Grid { .. } => ResidualKind::Grid,
            Self::DeviceControl { .. } => ResidualKind::DeviceControl,
            Self::Notation { .. } => ResidualKind::Notation,
        }
    }

    /// An empirical-fit residual, validating its uncertainty.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::InvalidUncertainty`] for a negative or
    /// non-finite uncertainty, and [`RealizationError::NonFiniteResidual`] for
    /// a non-finite deviation.
    pub fn empirical_fit(
        deviation: f64,
        uncertainty: f64,
        unit: &str,
    ) -> Result<Self, RealizationError> {
        if !deviation.is_finite() {
            return Err(RealizationError::NonFiniteResidual);
        }
        if !uncertainty.is_finite() || uncertainty < 0.0 {
            return Err(RealizationError::InvalidUncertainty);
        }
        Ok(Self::EmpiricalFit {
            deviation,
            uncertainty,
            unit: unit.into(),
        })
    }

    /// A device-control residual.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::NonFiniteResidual`] if either value is not
    /// finite.
    pub fn device_control(
        requested: f64,
        encoded: f64,
        unit: &str,
    ) -> Result<Self, RealizationError> {
        if !requested.is_finite() || !encoded.is_finite() {
            return Err(RealizationError::NonFiniteResidual);
        }
        Ok(Self::DeviceControl {
            requested,
            encoded,
            unit: unit.into(),
        })
    }

    /// Whether this residual is zero, that is, nothing was actually lost.
    ///
    /// A symbolic residual is never zero: it either exists or it does not.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Structural { comma } => comma.is_zero(),
            Self::TuningDeviation { deviation } => *deviation == Octaves::ZERO,
            Self::EmpiricalFit { deviation, .. } => *deviation == 0.0,
            Self::TemporalRealization { deviation } => *deviation == Seconds::ZERO,
            Self::Grid { deviation } => deviation.is_zero(),
            Self::DeviceControl {
                requested, encoded, ..
            } => requested == encoded,
            Self::Notation { .. } => false,
        }
    }

    /// Adds two residuals of a compatible, genuinely additive kind.
    ///
    /// There is no `Add` implementation, by design. Prompt section 35 forbids
    /// "a generic arithmetic `Add` across residual variants", and section 7.9
    /// requires that residuals "MUST NOT be added numerically unless they live
    /// in compatible spaces and the addition is mathematically meaningful".
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::IncompatibleResiduals`] for different
    /// kinds, and [`RealizationError::NonAdditiveResidual`] for a kind whose
    /// addition this crate declines to define.
    pub fn try_add(&self, other: &Self) -> Result<Self, RealizationError> {
        if self.kind() != other.kind() {
            return Err(RealizationError::IncompatibleResiduals {
                left: self.kind(),
                right: other.kind(),
            });
        }
        match (self, other) {
            (Self::Structural { comma: left }, Self::Structural { comma: right }) => {
                Ok(Self::Structural {
                    comma: left.checked_add(right)?,
                })
            }
            (
                Self::TuningDeviation { deviation: left },
                Self::TuningDeviation { deviation: right },
            ) => Ok(Self::TuningDeviation {
                deviation: *left + *right,
            }),
            (
                Self::TemporalRealization { deviation: left },
                Self::TemporalRealization { deviation: right },
            ) => Ok(Self::TemporalRealization {
                deviation: *left + *right,
            }),
            (Self::Grid { deviation: left }, Self::Grid { deviation: right }) => Ok(Self::Grid {
                deviation: left + right,
            }),
            _ => Err(RealizationError::NonAdditiveResidual { kind: self.kind() }),
        }
    }
}

/// A residual together with the provenance of the operation that produced it
/// (UMT-3.2 section 7.9).
///
/// UMT layer: as the residual. Section 7.9 requires residuals to preserve
/// units *and* provenance; the units live in the residual's own type and the
/// provenance lives here.
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualRecord {
    residual: Residual,
    provenance: Option<ProvenanceId>,
    note: Option<String>,
}

impl ResidualRecord {
    /// Records a residual with no provenance yet.
    ///
    /// Legitimate while a value is being assembled; a residual that
    /// participates in a conformance decision needs provenance under section
    /// 7.10, and [`ResidualRecord::has_provenance`] is how a caller checks.
    #[must_use]
    pub fn new(residual: Residual) -> Self {
        Self {
            residual,
            provenance: None,
            note: None,
        }
    }

    /// Attaches provenance.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Attaches a human-readable note.
    #[must_use]
    pub fn with_note(mut self, note: &str) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The residual itself.
    #[must_use]
    pub fn residual(&self) -> &Residual {
        &self.residual
    }

    /// Its kind.
    #[must_use]
    pub fn kind(&self) -> ResidualKind {
        self.residual.kind()
    }

    /// The provenance of the operation that produced it.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// Whether provenance was recorded.
    #[must_use]
    pub fn has_provenance(&self) -> bool {
        self.provenance.is_some()
    }

    /// The attached note.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// A collection of residuals, kept separate by kind.
///
/// UMT layer: mixed, by construction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResidualSet {
    records: Vec<ResidualRecord>,
}

impl ResidualSet {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a residual.
    pub fn push(&mut self, record: ResidualRecord) {
        self.records.push(record);
    }

    /// Builder form of [`ResidualSet::push`].
    #[must_use]
    pub fn with(mut self, record: ResidualRecord) -> Self {
        self.records.push(record);
        self
    }

    /// Every record, in the order they were added.
    #[must_use]
    pub fn records(&self) -> &[ResidualRecord] {
        &self.records
    }

    /// How many residuals are recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether nothing was lost, or at least nothing was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The records of one kind.
    pub fn of_kind(&self, kind: ResidualKind) -> impl Iterator<Item = &ResidualRecord> {
        self.records
            .iter()
            .filter(move |record| record.kind() == kind)
    }

    /// Which kinds are present, ascending.
    #[must_use]
    pub fn kinds(&self) -> Vec<ResidualKind> {
        let mut kinds: Vec<ResidualKind> = self.records.iter().map(ResidualRecord::kind).collect();
        kinds.sort_unstable();
        kinds.dedup();
        kinds
    }

    /// Whether every recorded residual carries provenance.
    ///
    /// Section 7.10 requires it of anything participating in a conformance
    /// decision, so this is the question to ask before making one.
    #[must_use]
    pub fn is_fully_attributed(&self) -> bool {
        self.records.iter().all(ResidualRecord::has_provenance)
    }

    /// The sum of every residual of one kind.
    ///
    /// Returns `Ok(None)` when no residual of that kind is present.
    ///
    /// # Errors
    ///
    /// Returns [`RealizationError::NonAdditiveResidual`] for a kind this crate
    /// declines to add, and propagates a kernel mismatch when structural
    /// residues come from different temperaments.
    pub fn total_of_kind(&self, kind: ResidualKind) -> Result<Option<Residual>, RealizationError> {
        if !kind.is_additive() {
            return Err(RealizationError::NonAdditiveResidual { kind });
        }
        let mut total: Option<Residual> = None;
        for record in self.of_kind(kind) {
            total = Some(match total {
                None => record.residual().clone(),
                Some(running) => running.try_add(record.residual())?,
            });
        }
        Ok(total)
    }
}

impl FromIterator<ResidualRecord> for ResidualSet {
    fn from_iter<I: IntoIterator<Item = ResidualRecord>>(iter: I) -> Self {
        Self {
            records: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Residual, ResidualKind, ResidualRecord, ResidualSet};
    use crate::algebra::{Q, Z};
    use crate::error::RealizationError;
    use crate::pitch::units::Octaves;
    use crate::proportion::Basis;
    use crate::realization::provenance::ProvenanceId;
    use crate::temperament::image::AmbientLattice;
    use crate::temperament::map::TemperamentMap;
    use crate::time::beat::Beats;
    use crate::time::units::Seconds;
    use alloc::string::String;

    fn syntonic_residue() -> Residual {
        let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5]).unwrap();
        let steps = AmbientLattice::new("umt:edo:12", 1);
        let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]]).unwrap();
        let comma = map
            .kernel()
            .coordinates(&basis.monzo([-4, 4, -1]).unwrap())
            .unwrap()
            .unwrap();
        Residual::Structural { comma }
    }

    #[test]
    fn every_kind_is_its_own_space() {
        let residuals = [
            syntonic_residue(),
            Residual::TuningDeviation {
                deviation: Octaves::new(-0.0018).unwrap(),
            },
            Residual::empirical_fit(3.2, 0.5, "cents").unwrap(),
            Residual::TemporalRealization {
                deviation: Seconds::new(0.04).unwrap(),
            },
            Residual::Grid {
                deviation: Beats::new(Q::new(Z::from(1), Z::from(480))),
            },
            Residual::device_control(8192.4, 8192.0, "bend code").unwrap(),
            Residual::Notation {
                detail: String::from("courtesy accidental"),
            },
        ];
        let kinds: alloc::vec::Vec<ResidualKind> = residuals.iter().map(Residual::kind).collect();
        let mut sorted = kinds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 7, "seven distinct kinds, as in the 7.9 table");

        // Cross-kind addition is refused, whatever the pair.
        for left in &residuals {
            for right in &residuals {
                if left.kind() != right.kind() {
                    assert!(matches!(
                        left.try_add(right),
                        Err(RealizationError::IncompatibleResiduals { .. })
                    ));
                }
            }
        }
    }

    #[test]
    fn additive_kinds_add_and_the_others_refuse() {
        let grid = |numerator: i64| Residual::Grid {
            deviation: Beats::new(Q::new(Z::from(numerator), Z::from(480))),
        };
        // Grid residuals accumulate, exactly. That is how endpoint drift is
        // measured, and the sum is a rational rather than a rounded double.
        let total = grid(1)
            .try_add(&grid(1))
            .unwrap()
            .try_add(&grid(-3))
            .unwrap();
        assert_eq!(
            total,
            Residual::Grid {
                deviation: Beats::new(Q::new(Z::from(-1), Z::from(480)))
            }
        );

        // Structural residues add in the kernel.
        let doubled = syntonic_residue().try_add(&syntonic_residue()).unwrap();
        assert!(!doubled.is_zero());
        assert_eq!(doubled.kind(), ResidualKind::Structural);

        // An empirical fit does not: its uncertainties would have to combine
        // under a model nobody declared.
        let fit = Residual::empirical_fit(1.0, 0.1, "cents").unwrap();
        assert!(matches!(
            fit.try_add(&fit),
            Err(RealizationError::NonAdditiveResidual {
                kind: ResidualKind::EmpiricalFit
            })
        ));
        assert!(!ResidualKind::EmpiricalFit.is_additive());
        assert!(!ResidualKind::DeviceControl.is_additive());
        assert!(!ResidualKind::Notation.is_additive());
    }

    #[test]
    fn residual_construction_validates_its_numbers() {
        assert!(matches!(
            Residual::empirical_fit(1.0, -0.5, "cents"),
            Err(RealizationError::InvalidUncertainty)
        ));
        assert!(matches!(
            Residual::empirical_fit(f64::NAN, 0.5, "cents"),
            Err(RealizationError::NonFiniteResidual)
        ));
        assert!(Residual::device_control(1.0, f64::INFINITY, "code").is_err());
    }

    #[test]
    fn a_set_keeps_kinds_apart_and_reports_attribution() {
        let provenance = ProvenanceId::new("umt:prov:realize-1");
        let mut set = ResidualSet::new();
        set.push(
            ResidualRecord::new(syntonic_residue())
                .with_provenance(provenance.clone())
                .with_note("spelling comma lost by 12-EDO"),
        );
        set.push(
            ResidualRecord::new(Residual::Grid {
                deviation: Beats::new(Q::new(Z::from(1), Z::from(480))),
            })
            .with_provenance(provenance),
        );
        set.push(ResidualRecord::new(Residual::Notation {
            detail: String::from("tuplet bracket"),
        }));

        assert_eq!(set.len(), 3);
        assert_eq!(set.kinds().len(), 3);
        assert_eq!(set.of_kind(ResidualKind::Grid).count(), 1);
        assert!(!set.is_fully_attributed(), "the notation residual has none");
        assert_eq!(
            set.records()[0].note(),
            Some("spelling comma lost by 12-EDO")
        );

        // A total is only offered where addition is meaningful.
        assert!(set.total_of_kind(ResidualKind::Grid).unwrap().is_some());
        assert!(
            set.total_of_kind(ResidualKind::TuningDeviation)
                .unwrap()
                .is_none(),
            "no residual of that kind is present"
        );
        assert!(set.total_of_kind(ResidualKind::Notation).is_err());
    }

    #[test]
    fn a_zero_residual_says_nothing_was_lost_except_symbolically() {
        assert!(
            Residual::TuningDeviation {
                deviation: Octaves::ZERO
            }
            .is_zero()
        );
        assert!(
            Residual::Grid {
                deviation: Beats::zero()
            }
            .is_zero()
        );
        assert!(
            !Residual::Notation {
                detail: String::from("courtesy accidental")
            }
            .is_zero(),
            "a symbolic residual either exists or does not"
        );
    }
}
