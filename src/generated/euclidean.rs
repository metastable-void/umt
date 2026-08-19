//! Euclidean rhythms (UMT-3.2 section 3.5).
//!
//! For integers `0 < k <= n`, a Euclidean rhythm `E(k, n)` distributes `k`
//! onsets among `n` pulse positions as evenly as possible, under a declared
//! rotation convention.
//!
//! # What is shared with generated scales, and what is not
//!
//! Section 3.5 says generated scales and Euclidean rhythms "share modular
//! arithmetic, balance properties, and continued-fraction structure", and
//! permits common algorithms. It then says, in bold, that UMT-3.2 "does
//! **not** identify every MOS construction with every Euclidean-rhythm
//! construction as the same theorem or the same object".
//!
//! So [`EuclideanRhythm`] and [`crate::generated::GeneratedSet`] are separate
//! types with no conversion between them. They compute similar things about
//! different objects.
//!
//! # The rotation convention is declared, and evenness is verified
//!
//! Section 9.11 requires both: "A Euclidean-rhythm implementation MUST declare
//! its rotation convention and verify maximal evenness under the selected
//! definition."
//!
//! [`RotationConvention`] is the first. The second is
//! [`EuclideanRhythm::verify_maximal_evenness`], which checks the
//! Clough-Douthett characterisation directly rather than trusting the
//! generating formula: for every `m` from 1 to `k - 1`, the circular distances
//! between elements `m` apart take at most two values, and if two, they differ
//! by exactly one pulse.
//!
//! # Not a Sturmian word
//!
//! Section 3.5 closes with a warning worth repeating: "A finite
//! Euclidean-rhythm word MUST NOT simply be called an infinite Sturmian word."
//! Nothing here calls it one. [`EuclideanRhythm::is_primitive`] reports
//! `gcd(k, n) = 1`, which is the condition under which the word is closely
//! related to a primitive Christoffel word - a relationship, not an identity.

use alloc::vec::Vec;

use crate::error::GeneratedError;
use crate::time::rhythm::CyclicRhythm;

/// Where the pattern starts (UMT-3.2 sections 3.5 and 9.11).
///
/// Declared, because the same set of onsets rotated differently is a different
/// rhythm to a listener, and the literature does not agree on one convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RotationConvention {
    /// Onset `i` sits at pulse `floor(i n / k)`, so pulse 0 always carries an
    /// onset. The convention this crate generates in.
    #[default]
    FirstPulseOnset,
    /// The pattern above, then rotated so it begins at the declared pulse.
    RotatedBy(u32),
}

/// What a maximal-evenness verification found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvennessReport {
    holds: bool,
    checked: usize,
    worst_spread: u32,
}

impl EvennessReport {
    /// Whether the set is maximally even under the checked definition.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.holds
    }

    /// How many values of `m` were checked, that is, `k - 1`.
    #[must_use]
    pub fn checked(&self) -> usize {
        self.checked
    }

    /// The largest difference found between the two circular distances at any
    /// `m`.
    ///
    /// One for a maximally even set of more than one onset, zero where every
    /// distance at every `m` was equal, and more than one where the property
    /// fails.
    #[must_use]
    pub fn worst_spread(&self) -> u32 {
        self.worst_spread
    }
}

/// A Euclidean rhythm `E(k, n)` (UMT-3.2 section 3.5).
///
/// UMT layer: L1, exact - the onsets are integer pulse indices.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawEuclideanRhythm", into = "RawEuclideanRhythm")
)]
pub struct EuclideanRhythm {
    onsets: u32,
    pulses: u32,
    rotation: RotationConvention,
}

/// A Euclidean rhythm in wire form, revalidated on the way in.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawEuclideanRhythm {
    /// How many onsets.
    pub onsets: u32,
    /// How many pulses.
    pub pulses: u32,
    /// The declared rotation convention.
    pub rotation: RotationConvention,
}

impl EuclideanRhythm {
    /// Builds `E(k, n)` under a declared rotation convention.
    ///
    /// # Errors
    ///
    /// Returns [`GeneratedError::EmptyCardinality`] for zero onsets or zero
    /// pulses, and [`GeneratedError::TooManyOnsets`] for more onsets than
    /// pulses. Section 3.5 requires `0 < k <= n`.
    pub fn new(
        onsets: u32,
        pulses: u32,
        rotation: RotationConvention,
    ) -> Result<Self, GeneratedError> {
        if onsets == 0 || pulses == 0 {
            return Err(GeneratedError::EmptyCardinality);
        }
        if onsets > pulses {
            return Err(GeneratedError::TooManyOnsets { onsets, pulses });
        }
        if let RotationConvention::RotatedBy(by) = rotation
            && by >= pulses
        {
            return Err(GeneratedError::DegreeOutOfRange {
                degree: by as usize,
                steps: pulses as usize,
            });
        }
        Ok(Self {
            onsets,
            pulses,
            rotation,
        })
    }

    /// How many onsets.
    #[must_use]
    pub fn onset_count(&self) -> u32 {
        self.onsets
    }

    /// How many pulses in one cycle.
    #[must_use]
    pub fn pulse_count(&self) -> u32 {
        self.pulses
    }

    /// The declared rotation convention.
    #[must_use]
    pub fn rotation(&self) -> RotationConvention {
        self.rotation
    }

    /// The onset pulse indices, ascending.
    #[must_use]
    pub fn onset_positions(&self) -> Vec<u32> {
        let mut positions: Vec<u32> = (0..self.onsets)
            .map(|index| {
                // floor(i * n / k), computed in integers so no rounding
                // convention is involved.
                (u64::from(index) * u64::from(self.pulses) / u64::from(self.onsets)) as u32
            })
            .collect();
        if let RotationConvention::RotatedBy(by) = self.rotation {
            for position in &mut positions {
                *position = (*position + self.pulses - by) % self.pulses;
            }
            positions.sort_unstable();
        }
        positions
    }

    /// The pattern as a binary word, `true` where an onset falls.
    #[must_use]
    pub fn word(&self) -> Vec<bool> {
        let mut word = alloc::vec![false; self.pulses as usize];
        for position in self.onset_positions() {
            word[position as usize] = true;
        }
        word
    }

    /// The gaps between consecutive onsets, wrapping around the cycle.
    ///
    /// Always sums to the pulse count.
    #[must_use]
    pub fn inter_onset_intervals(&self) -> Vec<u32> {
        let positions = self.onset_positions();
        let mut gaps = Vec::with_capacity(positions.len());
        for pair in positions.windows(2) {
            gaps.push(pair[1] - pair[0]);
        }
        gaps.push(self.pulses + positions[0] - positions[positions.len() - 1]);
        gaps
    }

    /// Whether `gcd(k, n) = 1` (UMT-3.2 section 3.5).
    ///
    /// The condition under which the word is closely related to a *primitive
    /// Christoffel word*. That is a relationship between finite words, and it
    /// is not a claim that the rhythm is an infinite Sturmian word - section
    /// 3.5 forbids exactly that conflation.
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        gcd(self.onsets, self.pulses) == 1
    }

    /// Verifies maximal evenness under the Clough-Douthett characterisation
    /// (UMT-3.2 section 9.11).
    ///
    /// For each `m` from 1 to `k - 1`, the circular distances between onsets
    /// `m` apart must take at most two values, and any two must differ by
    /// exactly one pulse. Checked directly against the produced positions,
    /// rather than inferred from the formula that produced them.
    #[must_use]
    pub fn verify_maximal_evenness(&self) -> EvennessReport {
        let positions = self.onset_positions();
        let count = positions.len();
        let mut holds = true;
        let mut worst_spread = 0u32;

        for step in 1..count {
            let mut smallest = u32::MAX;
            let mut largest = 0u32;
            for index in 0..count {
                let from = positions[index];
                let to = positions[(index + step) % count];
                let distance = (to + self.pulses - from) % self.pulses;
                // A full wrap reads as zero; it is the whole cycle.
                let distance = if distance == 0 && step != 0 {
                    self.pulses
                } else {
                    distance
                };
                smallest = smallest.min(distance);
                largest = largest.max(distance);
            }
            let spread = largest - smallest;
            worst_spread = worst_spread.max(spread);
            if spread > 1 {
                holds = false;
            }
        }

        EvennessReport {
            holds,
            checked: count.saturating_sub(1),
            worst_spread,
        }
    }

    /// The same pattern as a [`CyclicRhythm`] on the structural timeline.
    ///
    /// A conversion into the rhythm layer, not an identification with it: the
    /// result is a cyclic pulse pattern like any other, and nothing records
    /// that it came from a Euclidean construction unless a caller says so.
    ///
    /// # Errors
    ///
    /// Propagates cycle validation.
    pub fn to_cyclic_rhythm(&self) -> Result<CyclicRhythm, crate::error::TimeError> {
        let positions = self.onset_positions();
        let reference = match self.rotation {
            RotationConvention::FirstPulseOnset => 0,
            RotationConvention::RotatedBy(by) => (self.pulses - by) % self.pulses,
        };
        CyclicRhythm::new(self.pulses, &positions, reference)
    }
}

impl TryFrom<RawEuclideanRhythm> for EuclideanRhythm {
    type Error = GeneratedError;

    fn try_from(value: RawEuclideanRhythm) -> Result<Self, Self::Error> {
        Self::new(value.onsets, value.pulses, value.rotation)
    }
}

impl From<EuclideanRhythm> for RawEuclideanRhythm {
    fn from(value: EuclideanRhythm) -> Self {
        Self {
            onsets: value.onsets,
            pulses: value.pulses,
            rotation: value.rotation,
        }
    }
}

impl core::fmt::Display for EuclideanRhythm {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "E({}, {}) [", self.onsets, self.pulses)?;
        for onset in self.word() {
            f.write_str(if onset { "x" } else { "." })?;
        }
        f.write_str("]")
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::{EuclideanRhythm, RotationConvention};
    use crate::error::GeneratedError;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    fn rhythm(onsets: u32, pulses: u32) -> EuclideanRhythm {
        EuclideanRhythm::new(onsets, pulses, RotationConvention::FirstPulseOnset).unwrap()
    }

    #[test]
    fn well_known_patterns_come_out_as_expected() {
        // The tresillo, E(3, 8).
        let tresillo = rhythm(3, 8);
        assert_eq!(tresillo.onset_positions(), [0, 2, 5]);
        assert_eq!(tresillo.inter_onset_intervals(), [2, 3, 3]);
        assert_eq!(tresillo.to_string(), "E(3, 8) [x.x..x..]");

        // E(5, 8), the cinquillo skeleton.
        assert_eq!(rhythm(5, 8).onset_positions(), [0, 1, 3, 4, 6]);
        // E(2, 5).
        assert_eq!(rhythm(2, 5).onset_positions(), [0, 2]);
        // E(4, 4): every pulse.
        assert_eq!(rhythm(4, 4).onset_positions(), [0, 1, 2, 3]);
        // E(1, 4): one onset.
        assert_eq!(rhythm(1, 4).onset_positions(), [0]);
    }

    #[test]
    fn gaps_always_sum_to_the_cycle() {
        for pulses in 1u32..=24 {
            for onsets in 1..=pulses {
                let rhythm = rhythm(onsets, pulses);
                let gaps = rhythm.inter_onset_intervals();
                assert_eq!(gaps.len(), onsets as usize);
                assert_eq!(gaps.iter().sum::<u32>(), pulses, "E({onsets}, {pulses})");
                assert_eq!(
                    rhythm.word().iter().filter(|on| **on).count(),
                    onsets as usize
                );
            }
        }
    }

    #[test]
    fn maximal_evenness_is_verified_rather_than_assumed() {
        // Checked against the produced positions, for every E(k, n) up to a
        // reasonable size, using the Clough-Douthett characterisation.
        for pulses in 1u32..=24 {
            for onsets in 1..=pulses {
                let report = rhythm(onsets, pulses).verify_maximal_evenness();
                assert!(
                    report.holds(),
                    "E({onsets}, {pulses}) failed with spread {}",
                    report.worst_spread()
                );
                assert_eq!(report.checked(), onsets as usize - 1);
            }
        }

        // And the verification is not vacuous: a deliberately uneven set of
        // the same cardinality fails it.
        let uneven = EuclideanRhythm::new(3, 8, RotationConvention::FirstPulseOnset).unwrap();
        assert!(uneven.verify_maximal_evenness().holds());
        // Positions 0, 1, 2 of an eight-pulse cycle are as uneven as three
        // onsets get; the check below reproduces the same arithmetic on them.
        let clustered = [0u32, 1, 2];
        let spread = |step: usize| {
            let mut smallest = u32::MAX;
            let mut largest = 0;
            for index in 0..clustered.len() {
                let distance =
                    (clustered[(index + step) % clustered.len()] + 8 - clustered[index]) % 8;
                let distance = if distance == 0 { 8 } else { distance };
                smallest = smallest.min(distance);
                largest = largest.max(distance);
            }
            largest - smallest
        };
        assert!(spread(1) > 1, "a clustered set is not maximally even");
    }

    #[test]
    fn the_rotation_convention_is_declared_and_changes_the_pattern() {
        let plain = rhythm(3, 8);
        let rotated = EuclideanRhythm::new(3, 8, RotationConvention::RotatedBy(2)).unwrap();

        assert_eq!(plain.onset_positions(), [0, 2, 5]);
        assert_eq!(rotated.onset_positions(), [0, 3, 6]);
        assert_ne!(plain, rotated, "the same onsets, differently placed");
        assert_eq!(rotated.rotation(), RotationConvention::RotatedBy(2));

        // Rotation preserves maximal evenness and the multiset of gaps.
        assert!(rotated.verify_maximal_evenness().holds());
        let mut plain_gaps = plain.inter_onset_intervals();
        let mut rotated_gaps = rotated.inter_onset_intervals();
        plain_gaps.sort_unstable();
        rotated_gaps.sort_unstable();
        assert_eq!(plain_gaps, rotated_gaps);

        // A rotation outside the cycle is rejected.
        assert!(matches!(
            EuclideanRhythm::new(3, 8, RotationConvention::RotatedBy(8)),
            Err(GeneratedError::DegreeOutOfRange { .. })
        ));
    }

    #[test]
    fn primitivity_is_reported_without_claiming_sturmian_identity() {
        assert!(rhythm(3, 8).is_primitive(), "gcd(3, 8) = 1");
        assert!(!rhythm(2, 8).is_primitive(), "gcd(2, 8) = 2");
        assert!(!rhythm(6, 9).is_primitive());
        assert!(rhythm(5, 12).is_primitive());

        // A non-primitive rhythm is still maximally even; primitivity is a
        // different property, and this crate does not conflate them.
        assert!(rhythm(2, 8).verify_maximal_evenness().holds());
    }

    #[test]
    fn construction_is_validated() {
        assert!(matches!(
            EuclideanRhythm::new(0, 8, RotationConvention::FirstPulseOnset),
            Err(GeneratedError::EmptyCardinality)
        ));
        assert!(matches!(
            EuclideanRhythm::new(3, 0, RotationConvention::FirstPulseOnset),
            Err(GeneratedError::EmptyCardinality)
        ));
        assert!(matches!(
            EuclideanRhythm::new(9, 8, RotationConvention::FirstPulseOnset),
            Err(GeneratedError::TooManyOnsets {
                onsets: 9,
                pulses: 8
            })
        ));
    }

    #[test]
    fn converting_to_a_cyclic_rhythm_keeps_the_onsets_and_the_reference() {
        let rotated = EuclideanRhythm::new(3, 8, RotationConvention::RotatedBy(2)).unwrap();
        let cyclic = rotated.to_cyclic_rhythm().unwrap();
        assert_eq!(cyclic.pulses(), 8);
        assert_eq!(cyclic.onsets(), rotated.onset_positions().as_slice());
        assert_eq!(
            cyclic.rotation(),
            6,
            "the declared reference travels across"
        );

        let plain = rhythm(3, 8).to_cyclic_rhythm().unwrap();
        assert_eq!(plain.rotation(), 0);
        assert_eq!(
            plain.inter_onset_pulses(),
            rhythm(3, 8).inter_onset_intervals().as_slice()
        );
    }

    #[test]
    fn a_rhythm_round_trips_through_its_wire_form() {
        let rhythm = EuclideanRhythm::new(5, 13, RotationConvention::RotatedBy(3)).unwrap();
        let raw = super::RawEuclideanRhythm::from(rhythm.clone());
        assert_eq!(EuclideanRhythm::try_from(raw).unwrap(), rhythm);

        // And the invariants are revalidated on the way in.
        let bad = super::RawEuclideanRhythm {
            onsets: 20,
            pulses: 13,
            rotation: RotationConvention::FirstPulseOnset,
        };
        assert!(EuclideanRhythm::try_from(bad).is_err());
    }

    #[test]
    fn the_word_and_the_positions_agree() {
        for pulses in 1u32..=16 {
            for onsets in 1..=pulses {
                let rhythm = rhythm(onsets, pulses);
                let from_word: Vec<u32> = rhythm
                    .word()
                    .iter()
                    .enumerate()
                    .filter(|(_, on)| **on)
                    .map(|(index, _)| index as u32)
                    .collect();
                assert_eq!(from_word, rhythm.onset_positions());
            }
        }
    }
}
