//! Continuous pitch (UMT-3.2 section 4.7).
//!
//! A realized note may carry a pitch trajectory `gamma: [t0, t1] -> P_3`, and
//! section 4.7 gives its shape explicitly:
//!
//! ```text
//! gamma(t) = Phi(x, c(t)) + v(t)
//! ```
//!
//! Three things are being kept apart there, and [`PitchTrajectory`] keeps them
//! apart too:
//!
//! - `x` is a *structural* pitch point, exact, at L2. A trajectory does not
//!   dissolve it into a real number.
//! - `Phi` is the selected contextual realization - the
//!   [`crate::pitch::PitchRealizer`] of section 1.8.2 - so an exact `7/6` lift
//!   and a nominal tempered pitch stay different things.
//! - `v(t)` is a real-valued [`Deviation`] acting on the log-frequency torsor.
//!   Vibrato around a nominal pitch, a continuous portamento, and a stepped
//!   glissando represented as separate nominal events are then genuinely
//!   different objects rather than three names for one curve.
//!
//! # Device export
//!
//! Section 4.7 closes with an obligation: a sampled device encoding at L4
//! "MUST retain the L3 trajectory or a declared approximation record if
//! round-trip reconstruction is required". Both routes exist here.
//! [`PitchTrajectoryRef`] is the exact trajectory in wire form, and
//! [`PitchTrajectory::sample_in_fixed_context`] returns samples *together with*
//! a [`SamplingRecord`] whose error bound is derived from the deviation's
//! Lipschitz constant rather than asserted. That is fixture F20.

use alloc::vec::Vec;

use crate::context::TheoryContext;
use crate::error::{ContextError, PitchError};
use crate::pitch::point::{IntervalGroupElement, PitchPoint, PitchPointRef};
use crate::pitch::tuning::PitchRealizer;
use crate::pitch::units::{FrequencyHz, LogFrequency, Octaves, Radians};
use crate::realization::optimization::ApproximationGuarantee;
use crate::temperament::image::AmbientElem;
use crate::time::span::TimeSpan;
use crate::time::units::{ClockTime, Seconds};

/// The real-valued deviation `v(t)` of UMT-3.2 section 4.7.
///
/// UMT layer: L3. A deviation acts on the log-frequency torsor, so it is an
/// interval-valued function of time and never a pitch by itself.
///
/// The variants are declared shapes rather than arbitrary closures, for two
/// reasons: a document has to be able to carry them, and an error bound for
/// device export has to be derivable from them. A closure could do neither.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Deviation {
    /// No deviation: the note holds its nominal pitch.
    None,
    /// A fixed offset for the whole span.
    Constant {
        /// The offset.
        offset: Octaves,
    },
    /// A linear ramp across the span: continuous portamento or glissando.
    Linear {
        /// The offset at `t0`.
        from: Octaves,
        /// The offset at `t1`.
        to: Octaves,
    },
    /// A sinusoid around the nominal pitch: vibrato.
    Vibrato {
        /// Peak excursion either side of the nominal pitch.
        depth: Octaves,
        /// Oscillations per second.
        rate: FrequencyHz,
        /// Phase at `t0`.
        phase: Radians,
    },
    /// The pointwise sum of several deviations, such as vibrato on a glide.
    Sum(Vec<Deviation>),
}

impl Deviation {
    /// Vibrato with zero phase at the start of the span.
    ///
    /// # Errors
    ///
    /// Propagates validation of the depth and rate.
    pub fn vibrato(depth: Octaves, rate: FrequencyHz) -> Result<Self, PitchError> {
        Ok(Self::Vibrato {
            depth,
            rate,
            phase: Radians::new(0.0)?,
        })
    }

    /// The deviation at a time within the span.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::Time`] wrapping
    /// [`crate::error::TimeError::OutsideSpan`] for a time outside the domain:
    /// a trajectory is defined on `[t0, t1]` and nowhere else, so
    /// extrapolation is refused rather than guessed.
    pub fn evaluate(&self, domain: TimeSpan, at: ClockTime) -> Result<Octaves, PitchError> {
        match self {
            Self::None => {
                domain.elapsed(at)?;
                Ok(Octaves::ZERO)
            }
            Self::Constant { offset } => {
                domain.elapsed(at)?;
                Ok(*offset)
            }
            Self::Linear { from, to } => {
                let position = domain.position(at)?;
                Ok(Octaves::new(
                    from.get() + (to.get() - from.get()) * position,
                )?)
            }
            Self::Vibrato { depth, rate, phase } => {
                let elapsed = domain.elapsed(at)?.get();
                let angle = core::f64::consts::TAU * rate.get() * elapsed + phase.get();
                Ok(Octaves::new(depth.get() * libm::sin(angle))?)
            }
            Self::Sum(parts) => {
                let mut total = 0.0;
                for part in parts {
                    total += part.evaluate(domain, at)?.get();
                }
                Ok(Octaves::new(total)?)
            }
        }
    }

    /// A Lipschitz bound in octaves per second, where one is derivable.
    ///
    /// This is what turns a sampling rate into a *proved* error bound rather
    /// than a hopeful one. Every variant listed above has an analytic bound;
    /// `None` is returned only where one genuinely does not exist, and the
    /// resulting guarantee then says [`ApproximationGuarantee::Unquantified`]
    /// instead of inventing a number.
    #[must_use]
    pub fn lipschitz_bound(&self, domain: TimeSpan) -> Option<f64> {
        match self {
            Self::None | Self::Constant { .. } => Some(0.0),
            Self::Linear { from, to } => {
                let rise = (to.get() - from.get()).abs();
                let run = domain.duration().get();
                if rise == 0.0 {
                    Some(0.0)
                } else if run > 0.0 {
                    Some(rise / run)
                } else {
                    // A step change over zero time has no finite bound.
                    None
                }
            }
            Self::Vibrato { depth, rate, .. } => {
                Some(depth.get().abs() * core::f64::consts::TAU * rate.get())
            }
            Self::Sum(parts) => parts.iter().try_fold(0.0, |total, part| {
                Some(total + part.lipschitz_bound(domain)?)
            }),
        }
    }
}

/// A pitch trajectory `gamma: [t0, t1] -> P_3` (UMT-3.2 section 4.7).
///
/// UMT layer: L2 nominal pitch, L3 deviation and result.
///
/// The nominal point stays structural and exact. Nothing in this type turns it
/// into a real number; that only happens when a realization is supplied, and
/// which realization was supplied is then visible at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PitchTrajectory<E> {
    nominal: PitchPoint<E>,
    domain: TimeSpan,
    deviation: Deviation,
}

impl<E: IntervalGroupElement> PitchTrajectory<E> {
    /// Builds a trajectory around a nominal structural pitch.
    #[must_use]
    pub fn new(nominal: PitchPoint<E>, domain: TimeSpan, deviation: Deviation) -> Self {
        Self {
            nominal,
            domain,
            deviation,
        }
    }

    /// A note that holds its nominal pitch for the whole span.
    #[must_use]
    pub fn steady(nominal: PitchPoint<E>, domain: TimeSpan) -> Self {
        Self::new(nominal, domain, Deviation::None)
    }

    /// The nominal structural pitch `x`.
    #[must_use]
    pub fn nominal(&self) -> &PitchPoint<E> {
        &self.nominal
    }

    /// The domain `[t0, t1]`.
    #[must_use]
    pub fn domain(&self) -> TimeSpan {
        self.domain
    }

    /// The deviation `v`.
    #[must_use]
    pub fn deviation(&self) -> &Deviation {
        &self.deviation
    }

    /// Evaluates `gamma(t) = Phi(x, c) + v(t)` in a given context.
    ///
    /// The context is the caller's `c(t)`: UMT-3.2 writes the realization
    /// context as a function of time, and this crate does not presume to know
    /// how an application indexes it. Passing a context that does not vary is
    /// the fixed-context case, and is what
    /// [`PitchTrajectory::sample_in_fixed_context`] assumes.
    ///
    /// # Errors
    ///
    /// Propagates the realizer's error, and reports a time outside the domain.
    pub fn evaluate<R, C>(
        &self,
        realizer: &R,
        context: &C,
        at: ClockTime,
    ) -> Result<LogFrequency, R::Error>
    where
        R: PitchRealizer<E, C>,
        R::Error: From<PitchError>,
    {
        let deviation = self.deviation.evaluate(self.domain, at)?;
        let nominal = realizer.realize(&self.nominal, context)?;
        Ok(nominal.translate(deviation))
    }

    /// Evaluates `gamma(t)` with a context that varies over time.
    ///
    /// # Errors
    ///
    /// Propagates the realizer's error, and reports a time outside the domain.
    pub fn evaluate_with<R, C, F>(
        &self,
        realizer: &R,
        context_at: F,
        at: ClockTime,
    ) -> Result<LogFrequency, R::Error>
    where
        R: PitchRealizer<E, C>,
        R::Error: From<PitchError>,
        F: Fn(ClockTime) -> C,
    {
        self.evaluate(realizer, &context_at(at), at)
    }

    /// Samples the trajectory for device export, in a context that does not
    /// vary over the span.
    ///
    /// The returned [`SamplingRecord`] carries the error bound implied by the
    /// declared sampling rate and interpolation, computed from the deviation's
    /// Lipschitz constant. Because `Phi(x, c)` is constant here, that constant
    /// bounds the whole trajectory - which is exactly why this method names
    /// the assumption instead of hiding it. For a varying context, sample with
    /// [`PitchTrajectory::evaluate_with`] and state your own guarantee.
    ///
    /// Sample times always include both endpoints; the actual step is at most
    /// `1 / rate`.
    ///
    /// # Errors
    ///
    /// Propagates the realizer's error.
    pub fn sample_in_fixed_context<R, C>(
        &self,
        realizer: &R,
        context: &C,
        rate: FrequencyHz,
        interpolation: Interpolation,
    ) -> Result<SampledTrajectory, R::Error>
    where
        R: PitchRealizer<E, C>,
        R::Error: From<PitchError>,
    {
        let duration = self.domain.duration().get();
        let intervals = if duration > 0.0 {
            let wanted = libm::ceil(duration * rate.get());
            // `wanted` is finite and positive here, and a span long enough to
            // overflow `usize` is not a span any device will export.
            (wanted as usize).max(1)
        } else {
            0
        };

        let mut samples = Vec::with_capacity(intervals + 1);
        for index in 0..=intervals {
            let at = if intervals == 0 {
                self.domain.start()
            } else if index == intervals {
                self.domain.end()
            } else {
                let offset = Seconds::new(duration * index as f64 / intervals as f64)
                    .map_err(PitchError::from)?;
                self.domain.start().translate(offset)
            };
            samples.push(TrajectorySample {
                at,
                pitch: self.evaluate(realizer, context, at)?,
            });
        }

        let step = if intervals == 0 {
            Seconds::ZERO
        } else {
            Seconds::new(duration / intervals as f64).map_err(PitchError::from)?
        };
        let guarantee = match self.deviation.lipschitz_bound(self.domain) {
            Some(lipschitz) => {
                // Linear interpolation of an L-Lipschitz function at spacing h
                // errs by at most L*h/2; a zero-order hold by at most L*h.
                let factor = match interpolation {
                    Interpolation::Linear => 0.5,
                    Interpolation::ZeroOrderHold => 1.0,
                };
                ApproximationGuarantee::AbsoluteGap {
                    epsilon: Octaves::new(lipschitz * step.get() * factor)?,
                }
            }
            None => ApproximationGuarantee::Unquantified,
        };

        Ok(SampledTrajectory {
            samples,
            record: SamplingRecord {
                rate,
                step,
                interpolation,
                guarantee,
            },
        })
    }
}

impl PitchTrajectory<AmbientElem> {
    /// Produces the wire form of this trajectory.
    #[must_use]
    pub fn to_ref(&self) -> PitchTrajectoryRef {
        PitchTrajectoryRef {
            nominal: PitchPointRef::of_ambient(&self.nominal),
            domain: self.domain,
            deviation: self.deviation.clone(),
        }
    }
}

/// A pitch trajectory in wire form (UMT-3.2 section 4.7, fixture F20).
///
/// UMT layer: L2 nominal reference, L3 deviation. This is the "retain the L3
/// trajectory" half of the section 4.7 device-export obligation: an exporter
/// that keeps this alongside its samples can reconstruct the source exactly,
/// with no approximation record needed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PitchTrajectoryRef {
    /// The nominal structural pitch, as a context reference.
    pub nominal: PitchPointRef,
    /// The domain `[t0, t1]`.
    pub domain: TimeSpan,
    /// The deviation `v`.
    pub deviation: Deviation,
}

impl PitchTrajectoryRef {
    /// Resolves this reference against a context.
    ///
    /// # Errors
    ///
    /// Propagates an unresolved lattice or a coordinate-rank mismatch.
    pub fn resolve_ambient(
        &self,
        context: &TheoryContext,
    ) -> Result<PitchTrajectory<AmbientElem>, ContextError> {
        Ok(PitchTrajectory::new(
            self.nominal.resolve_ambient(context)?,
            self.domain,
            self.deviation.clone(),
        ))
    }
}

/// How a consumer is expected to read between samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Interpolation {
    /// Each sample holds until the next one. The usual behaviour of a
    /// control-change stream.
    ZeroOrderHold,
    /// Straight-line interpolation between adjacent samples, in log-frequency.
    Linear,
}

/// One sampled point of a trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrajectorySample {
    /// When it was taken.
    pub at: ClockTime,
    /// The realized pitch there.
    pub pitch: LogFrequency,
}

/// What a sampling did, and what it costs in accuracy (UMT-3.2 sections 4.7
/// and 7.9).
///
/// UMT layer: L3 to L4 boundary. The guarantee is derived, not declared by
/// hand: for a deviation with a Lipschitz bound it is a genuine worst case over
/// the whole span, and where no bound exists it says
/// [`ApproximationGuarantee::Unquantified`] rather than quoting a number
/// nobody proved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SamplingRecord {
    /// The requested sampling rate.
    pub rate: FrequencyHz,
    /// The actual step used, which is at most `1 / rate`.
    pub step: Seconds,
    /// How a consumer is expected to read between samples.
    pub interpolation: Interpolation,
    /// The worst-case reconstruction error, in octaves.
    pub guarantee: ApproximationGuarantee<Octaves>,
}

/// A sampled trajectory together with the record of what the sampling cost.
///
/// UMT layer: L4. The two travel together on purpose: samples without the
/// record would be an approximation presented as data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SampledTrajectory {
    samples: Vec<TrajectorySample>,
    record: SamplingRecord,
}

impl SampledTrajectory {
    /// The samples, in time order.
    #[must_use]
    pub fn samples(&self) -> &[TrajectorySample] {
        &self.samples
    }

    /// What the sampling cost.
    #[must_use]
    pub fn record(&self) -> &SamplingRecord {
        &self.record
    }

    /// Reconstructs the pitch at a time, using the declared interpolation.
    ///
    /// The result is within the record's guarantee of the original trajectory
    /// wherever that guarantee is quantified, which is a claim worth being
    /// able to test rather than merely assert.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::NoSamples`] for an empty sampling, and
    /// [`PitchError::Time`] for a time outside the sampled range.
    pub fn reconstruct(&self, at: ClockTime) -> Result<LogFrequency, PitchError> {
        let (Some(first), Some(last)) = (self.samples.first(), self.samples.last()) else {
            return Err(PitchError::NoSamples);
        };
        if at < first.at || at > last.at {
            return Err(PitchError::Time(crate::error::TimeError::OutsideSpan {
                time: at.get(),
                start: first.at.get(),
                end: last.at.get(),
            }));
        }

        let upper = self
            .samples
            .iter()
            .position(|sample| sample.at >= at)
            .unwrap_or(self.samples.len() - 1);
        if upper == 0 || self.samples[upper].at == at {
            return Ok(self.samples[upper].pitch);
        }
        let before = &self.samples[upper - 1];
        let after = &self.samples[upper];

        match self.record.interpolation {
            Interpolation::ZeroOrderHold => Ok(before.pitch),
            Interpolation::Linear => {
                let width = before.at.interval_to(after.at).get();
                let position = if width == 0.0 {
                    0.0
                } else {
                    before.at.interval_to(at).get() / width
                };
                Ok(before
                    .pitch
                    .translate(before.pitch.interval_to(after.pitch).scale(position)?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Deviation, Interpolation, PitchTrajectory, PitchTrajectoryRef};
    use crate::context::TheoryContext;
    use crate::error::PitchError;
    use crate::pitch::point::{PitchOrigin, PitchPoint};
    use crate::pitch::tuning::{PitchRealization, RegularTuning};
    use crate::pitch::units::{Cents, FrequencyHz, Octaves, Radians};
    use crate::realization::optimization::ApproximationGuarantee;
    use crate::temperament::image::{AmbientElem, AmbientLattice};
    use crate::time::span::TimeSpan;
    use crate::time::units::{ClockTime, Seconds};
    use alloc::sync::Arc;
    use alloc::vec;

    fn steps() -> Arc<AmbientLattice> {
        AmbientLattice::new("umt:edo:12", 1)
    }

    fn realization() -> PitchRealization<AmbientLattice> {
        let lattice = steps();
        PitchRealization::new(
            RegularTuning::equal_divisions(&lattice, 12).unwrap(),
            PitchPoint::new(PitchOrigin::new("umt:origin:a4"), lattice.zero()),
            FrequencyHz::new(440.0).unwrap().log_frequency(),
        )
    }

    fn nominal(step: i64) -> PitchPoint<AmbientElem> {
        PitchPoint::new(
            PitchOrigin::new("umt:origin:a4"),
            steps().element([step]).unwrap(),
        )
    }

    fn span(seconds: f64) -> TimeSpan {
        TimeSpan::from_duration(ClockTime::ZERO, Seconds::new(seconds).unwrap()).unwrap()
    }

    #[test]
    fn a_steady_note_sits_on_its_nominal_pitch() {
        let trajectory = PitchTrajectory::steady(nominal(0), span(1.0));
        let realizer = realization();
        let pitch = trajectory
            .evaluate(&realizer, &(), ClockTime::new(0.5).unwrap())
            .unwrap();
        assert!((pitch.frequency().unwrap().get() - 440.0).abs() < 1e-9);
    }

    #[test]
    fn a_glissando_is_continuous_and_hits_both_endpoints() {
        // A whole-tone glide up, expressed as a deviation from A4.
        let trajectory = PitchTrajectory::new(
            nominal(0),
            span(2.0),
            Deviation::Linear {
                from: Octaves::ZERO,
                to: Octaves::from(Cents::new(200.0).unwrap()),
            },
        );
        let realizer = realization();

        let start = trajectory
            .evaluate(&realizer, &(), ClockTime::ZERO)
            .unwrap();
        let middle = trajectory
            .evaluate(&realizer, &(), ClockTime::new(1.0).unwrap())
            .unwrap();
        let end = trajectory
            .evaluate(&realizer, &(), ClockTime::new(2.0).unwrap())
            .unwrap();

        assert!((start.frequency().unwrap().get() - 440.0).abs() < 1e-9);
        assert!((Cents::from(start.interval_to(middle)).get() - 100.0).abs() < 1e-9);
        assert!((Cents::from(start.interval_to(end)).get() - 200.0).abs() < 1e-9);
    }

    #[test]
    fn vibrato_oscillates_around_the_nominal_pitch() {
        let depth = Octaves::from(Cents::new(30.0).unwrap());
        let trajectory = PitchTrajectory::new(
            nominal(0),
            span(1.0),
            Deviation::vibrato(depth, FrequencyHz::new(5.0).unwrap()).unwrap(),
        );
        let realizer = realization();
        let nominal_pitch = FrequencyHz::new(440.0).unwrap().log_frequency();

        // Zero at t = 0, at the peak a quarter cycle later, back at zero after
        // half a cycle.
        let at_zero = trajectory
            .evaluate(&realizer, &(), ClockTime::ZERO)
            .unwrap();
        assert_eq!(at_zero, nominal_pitch);

        let at_peak = trajectory
            .evaluate(&realizer, &(), ClockTime::new(0.05).unwrap())
            .unwrap();
        assert!((Cents::from(nominal_pitch.interval_to(at_peak)).get() - 30.0).abs() < 1e-9);

        let at_trough = trajectory
            .evaluate(&realizer, &(), ClockTime::new(0.15).unwrap())
            .unwrap();
        assert!((Cents::from(nominal_pitch.interval_to(at_trough)).get() + 30.0).abs() < 1e-9);
    }

    #[test]
    fn deviations_compose_by_summing() {
        let combined = Deviation::Sum(vec![
            Deviation::Linear {
                from: Octaves::ZERO,
                to: Octaves::new(1.0).unwrap(),
            },
            Deviation::Constant {
                offset: Octaves::new(0.25).unwrap(),
            },
        ]);
        let domain = span(4.0);
        let value = combined
            .evaluate(domain, ClockTime::new(2.0).unwrap())
            .unwrap();
        assert!((value.get() - 0.75).abs() < 1e-12);
        assert!((combined.lipschitz_bound(domain).unwrap() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn a_trajectory_is_defined_on_its_domain_and_nowhere_else() {
        let trajectory = PitchTrajectory::steady(nominal(0), span(1.0));
        let realizer = realization();
        assert!(matches!(
            trajectory.evaluate(&realizer, &(), ClockTime::new(1.5).unwrap()),
            Err(PitchError::Time(_))
        ));
        assert!(matches!(
            trajectory.evaluate(&realizer, &(), ClockTime::new(-0.1).unwrap()),
            Err(PitchError::Time(_))
        ));
    }

    #[test]
    fn sampling_records_a_bound_it_actually_meets() {
        let depth = Octaves::from(Cents::new(50.0).unwrap());
        let trajectory = PitchTrajectory::new(
            nominal(0),
            span(1.0),
            Deviation::Vibrato {
                depth,
                rate: FrequencyHz::new(6.0).unwrap(),
                phase: Radians::new(0.7).unwrap(),
            },
        );
        let realizer = realization();
        let sampled = trajectory
            .sample_in_fixed_context(
                &realizer,
                &(),
                FrequencyHz::new(200.0).unwrap(),
                Interpolation::Linear,
            )
            .unwrap();

        let ApproximationGuarantee::AbsoluteGap { epsilon } = sampled.record().guarantee else {
            panic!("a vibrato has a Lipschitz bound");
        };
        assert!(epsilon.get() > 0.0);
        assert_eq!(sampled.samples().len(), 201);

        // The declared bound is met at every point between the samples, which
        // is the claim that makes the record worth serializing.
        for index in 0..2000 {
            let at = ClockTime::new(index as f64 / 2000.0).unwrap();
            let exact = trajectory.evaluate(&realizer, &(), at).unwrap();
            let rebuilt = sampled.reconstruct(at).unwrap();
            let error = exact.interval_to(rebuilt).get().abs();
            assert!(
                error <= epsilon.get() + 1e-15,
                "{error} > {}",
                epsilon.get()
            );
        }
    }

    #[test]
    fn a_zero_order_hold_is_twice_as_loose_as_linear_interpolation() {
        let trajectory = PitchTrajectory::new(
            nominal(0),
            span(1.0),
            Deviation::Linear {
                from: Octaves::ZERO,
                to: Octaves::new(1.0).unwrap(),
            },
        );
        let realizer = realization();
        let rate = FrequencyHz::new(10.0).unwrap();

        let linear = trajectory
            .sample_in_fixed_context(&realizer, &(), rate, Interpolation::Linear)
            .unwrap();
        let held = trajectory
            .sample_in_fixed_context(&realizer, &(), rate, Interpolation::ZeroOrderHold)
            .unwrap();

        let (
            ApproximationGuarantee::AbsoluteGap { epsilon: fine },
            ApproximationGuarantee::AbsoluteGap { epsilon: coarse },
        ) = (&linear.record().guarantee, &held.record().guarantee)
        else {
            panic!("a ramp has a Lipschitz bound");
        };
        assert!((coarse.get() - 2.0 * fine.get()).abs() < 1e-15);

        // A linear ramp is reconstructed exactly by linear interpolation.
        for index in 0..=100 {
            let at = ClockTime::new(index as f64 / 100.0).unwrap();
            let exact = trajectory.evaluate(&realizer, &(), at).unwrap();
            let rebuilt = linear.reconstruct(at).unwrap();
            assert!(exact.interval_to(rebuilt).get().abs() < 1e-12);
        }
    }

    #[test]
    fn an_instant_yields_a_single_sample() {
        let domain = TimeSpan::new(ClockTime::ZERO, ClockTime::ZERO).unwrap();
        let trajectory = PitchTrajectory::steady(nominal(0), domain);
        let realizer = realization();
        let sampled = trajectory
            .sample_in_fixed_context(
                &realizer,
                &(),
                FrequencyHz::new(100.0).unwrap(),
                Interpolation::Linear,
            )
            .unwrap();
        assert_eq!(sampled.samples().len(), 1);
        assert_eq!(sampled.record().step, Seconds::ZERO);
        assert_eq!(
            sampled.reconstruct(ClockTime::ZERO).unwrap(),
            sampled.samples()[0].pitch
        );
        assert!(matches!(
            sampled.reconstruct(ClockTime::new(1.0).unwrap()),
            Err(PitchError::Time(_))
        ));
    }

    #[test]
    fn a_trajectory_round_trips_through_its_reference_form() {
        let lattice = steps();
        let context = TheoryContext::builder().ambient(&lattice).unwrap().build();
        let trajectory = PitchTrajectory::new(
            nominal(7),
            span(1.5),
            Deviation::Sum(vec![
                Deviation::Linear {
                    from: Octaves::ZERO,
                    to: Octaves::new(-0.5).unwrap(),
                },
                Deviation::vibrato(
                    Octaves::from(Cents::new(20.0).unwrap()),
                    FrequencyHz::new(5.5).unwrap(),
                )
                .unwrap(),
            ]),
        );

        let reference = trajectory.to_ref();
        assert_eq!(reference.resolve_ambient(&context).unwrap(), trajectory);
    }

    #[test]
    fn a_step_change_over_zero_time_declines_to_claim_a_bound() {
        let domain = TimeSpan::new(ClockTime::ZERO, ClockTime::ZERO).unwrap();
        let jump = Deviation::Linear {
            from: Octaves::ZERO,
            to: Octaves::new(1.0).unwrap(),
        };
        assert_eq!(jump.lipschitz_bound(domain), None);

        let trajectory = PitchTrajectory::new(nominal(0), domain, jump);
        let realizer = realization();
        let sampled = trajectory
            .sample_in_fixed_context(
                &realizer,
                &(),
                FrequencyHz::new(100.0).unwrap(),
                Interpolation::Linear,
            )
            .unwrap();
        assert_eq!(
            sampled.record().guarantee,
            ApproximationGuarantee::Unquantified
        );
    }

    #[test]
    fn the_wire_form_is_the_exact_trajectory_not_a_sampling() {
        let trajectory = PitchTrajectory::new(
            nominal(0),
            span(1.0),
            Deviation::vibrato(
                Octaves::from(Cents::new(25.0).unwrap()),
                FrequencyHz::new(5.0).unwrap(),
            )
            .unwrap(),
        );
        let reference: PitchTrajectoryRef = trajectory.to_ref();
        assert_eq!(&reference.deviation, trajectory.deviation());
        assert_eq!(reference.domain, trajectory.domain());
    }
}
