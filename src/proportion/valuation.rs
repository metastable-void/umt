//! Generator valuations: exact rational (L1) and symbolic-real (L3).
//!
//! UMT-3.2 sections 1.1.1 and 1.1.2. A generator's *formal* identity is exact
//! in both profiles; what differs is whether its valuation is an exact
//! positive rational or a real observation carrying uncertainty and
//! provenance.

use alloc::string::{String, ToString};

use crate::algebra::rational::{Q, log2_q_f64};
use crate::error::ValuationError;
use crate::io::text::q_from_str;
use crate::realization::provenance::ProvenanceId;

/// An exact valuation in `Q_{>0}` (UMT-3.2 section 1.1.1).
///
/// UMT layer: L1, exact. Construction is fallible: zero and negative values are
/// rejected, because a proportion lattice generator must act on positive rates.
///
/// Equality is presentation equality on the reduced rational.
///
/// Serialized as a canonical `"numerator/denominator"` string, never as a
/// floating-point number (UMT-3.2 section 8.9, prompt section 39).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct PositiveQ(Q);

impl PositiveQ {
    /// Accepts a strictly positive exact rational.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::NonPositiveRational`] if `value <= 0`.
    pub fn new(value: Q) -> Result<Self, ValuationError> {
        if value <= Q::new(0.into(), 1.into()) {
            return Err(ValuationError::NonPositiveRational);
        }
        Ok(Self(value))
    }

    /// Accepts a positive integer valuation such as a prime.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::NonPositiveRational`] if `value` is zero.
    pub fn integer(value: u32) -> Result<Self, ValuationError> {
        Self::new(Q::new(value.into(), 1.into()))
    }

    /// The exact value.
    #[must_use]
    pub fn value(&self) -> &Q {
        &self.0
    }

    /// L3 real approximation of `log2` of this valuation.
    ///
    /// UMT layer: L3. Provided for display and for metric realization; exact
    /// structural decisions never route through it.
    #[must_use]
    pub fn log2_f64(&self) -> f64 {
        log2_q_f64(&self.0).expect("invariant: PositiveQ holds a positive value")
    }
}

impl From<PositiveQ> for String {
    fn from(value: PositiveQ) -> Self {
        crate::io::text::q_to_string(&value.0)
    }
}

impl TryFrom<String> for PositiveQ {
    type Error = ValuationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = q_from_str(&value).ok_or(ValuationError::MalformedRational {
            text: value.clone(),
        })?;
        Self::new(parsed)
    }
}

/// A strictly positive, finite real number.
///
/// UMT layer: L3. Construction is fallible so that NaN, infinities, zero, and
/// negative values cannot enter a metric valuation (prompt section 18).
///
/// Equality and hashing are presentation equality on the bit pattern. That is
/// total here because NaN is excluded by construction and `-0.0` cannot occur.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
pub struct PositiveFinite(f64);

impl PositiveFinite {
    /// Accepts a positive finite real.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::NonPositiveReal`] for zero, negative,
    /// infinite, and NaN inputs.
    pub fn new(value: f64) -> Result<Self, ValuationError> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(ValuationError::NonPositiveReal)
        }
    }

    /// The underlying value.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for PositiveFinite {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for PositiveFinite {}

impl core::hash::Hash for PositiveFinite {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl From<PositiveFinite> for f64 {
    fn from(value: PositiveFinite) -> Self {
        value.0
    }
}

impl TryFrom<f64> for PositiveFinite {
    type Error = ValuationError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A non-negative, finite real number, used for uncertainties.
///
/// UMT layer: L3. Equality and hashing are presentation equality on the bit
/// pattern; `-0.0` is normalized to `0.0` on construction so that equality is
/// consistent with numeric equality.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
pub struct NonNegativeFinite(f64);

impl NonNegativeFinite {
    /// Accepts a non-negative finite real.
    ///
    /// # Errors
    ///
    /// Returns [`ValuationError::InvalidUncertainty`] for negative, infinite,
    /// and NaN inputs.
    pub fn new(value: f64) -> Result<Self, ValuationError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        } else {
            Err(ValuationError::InvalidUncertainty)
        }
    }

    /// The underlying value.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for NonNegativeFinite {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for NonNegativeFinite {}

impl core::hash::Hash for NonNegativeFinite {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl From<NonNegativeFinite> for f64 {
    fn from(value: NonNegativeFinite) -> Self {
        value.0
    }
}

impl TryFrom<f64> for NonNegativeFinite {
    type Error = ValuationError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A symbolic-real generator valuation (UMT-3.2 section 1.1.2).
///
/// UMT layer: L3. The generator stays formal and exact at L1; this attaches the
/// metric size. Section 0.6.1 requires a real-valued observation to carry a
/// value, an uncertainty where applicable, and provenance, so both are
/// modelled explicitly rather than being optional afterthoughts on a bare
/// `f64`.
///
/// Equality is presentation equality, including the uncertainty and provenance
/// slots: two valuations that agree numerically but came from different
/// measurements are different objects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealValuation {
    value: PositiveFinite,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    uncertainty: Option<NonNegativeFinite>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    provenance: Option<ProvenanceId>,
}

impl RealValuation {
    /// A real valuation with no declared uncertainty or provenance.
    ///
    /// Prefer [`RealValuation::with_uncertainty`] and
    /// [`RealValuation::with_provenance`]: a measured valuation that carries
    /// neither cannot satisfy the section 0.6.1 contract for values that
    /// participate in conformance decisions.
    #[must_use]
    pub fn new(value: PositiveFinite) -> Self {
        Self {
            value,
            uncertainty: None,
            provenance: None,
        }
    }

    /// Attaches an uncertainty.
    #[must_use]
    pub fn with_uncertainty(mut self, uncertainty: NonNegativeFinite) -> Self {
        self.uncertainty = Some(uncertainty);
        self
    }

    /// Attaches a provenance reference.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceId) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// The metric value.
    #[must_use]
    pub fn value(&self) -> PositiveFinite {
        self.value
    }

    /// The declared uncertainty, if any.
    #[must_use]
    pub fn uncertainty(&self) -> Option<NonNegativeFinite> {
        self.uncertainty
    }

    /// The provenance reference, if any.
    #[must_use]
    pub fn provenance(&self) -> Option<&ProvenanceId> {
        self.provenance.as_ref()
    }

    /// L3 real approximation of `log2` of this valuation.
    #[must_use]
    pub fn log2_f64(&self) -> f64 {
        libm::log2(self.value.get())
    }
}

impl core::fmt::Display for PositiveQ {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&crate::io::text::q_to_string(&self.0))
    }
}

impl core::fmt::Display for PositiveFinite {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{NonNegativeFinite, PositiveFinite, PositiveQ, RealValuation};
    use crate::algebra::rational::Q;
    use crate::error::ValuationError;

    #[test]
    fn positive_rational_validation() {
        assert!(PositiveQ::new(Q::new(3.into(), 2.into())).is_ok());
        assert_eq!(
            PositiveQ::new(Q::new(0.into(), 1.into())),
            Err(ValuationError::NonPositiveRational)
        );
        assert_eq!(
            PositiveQ::new(Q::new((-3).into(), 2.into())),
            Err(ValuationError::NonPositiveRational)
        );
    }

    #[test]
    fn positive_finite_validation() {
        assert!(PositiveFinite::new(1.5).is_ok());
        assert!(PositiveFinite::new(0.0).is_err());
        assert!(PositiveFinite::new(-1.0).is_err());
        assert!(PositiveFinite::new(f64::NAN).is_err());
        assert!(PositiveFinite::new(f64::INFINITY).is_err());
    }

    #[test]
    fn uncertainty_allows_zero_but_not_negatives() {
        assert!(NonNegativeFinite::new(0.0).is_ok());
        assert!(NonNegativeFinite::new(-0.0).is_ok());
        assert_eq!(NonNegativeFinite::new(-0.0), NonNegativeFinite::new(0.0));
        assert!(NonNegativeFinite::new(-1e-9).is_err());
    }

    #[test]
    fn log2_paths_agree_on_a_rational_generator() {
        let exact = PositiveQ::integer(3).unwrap();
        let real = RealValuation::new(PositiveFinite::new(3.0).unwrap());
        assert!((exact.log2_f64() - real.log2_f64()).abs() < 1e-15);
    }
}
