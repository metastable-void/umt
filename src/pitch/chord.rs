//! Chords and voices (UMT-3.2 sections 4.3 and 4.6, prompt section 21).
//!
//! A registered chord is a *function* `c: V_c -> P` from voice identities to
//! pitch points, not a set of pitches. That distinction is the whole content
//! of section 4.3: keeping the labels preserves unisons, doublings, and later
//! voice continuity, and a chord modelled as a set of pitch classes has thrown
//! all three away before anyone can ask for them.
//!
//! Views that forget information exist, because analysis needs them. They are
//! named for what they discard - [`Chord::forget_voice_labels`],
//! [`PitchMultiset::forget_multiplicity`] - and each returns a *different
//! type*, so a caller cannot lose multiplicity by accident and cannot pass the
//! lossy view back where the chord was wanted (prompt section 57).
//!
//! # Examples
//!
//! One C and two doubled Cs are different objects, and stay different:
//!
//! ```
//! use umt::pitch::{Chord, PitchOrigin, PitchPoint, VoiceId};
//! use umt::temperament::AmbientLattice;
//!
//! let steps = AmbientLattice::new("umt:edo:12", 1);
//! let middle_c = PitchPoint::new(PitchOrigin::new("umt:origin:c4"), steps.zero());
//!
//! let single = Chord::from_voices([(VoiceId::new("soprano"), middle_c.clone())])?;
//! let doubled = Chord::from_voices([
//!     (VoiceId::new("soprano"), middle_c.clone()),
//!     (VoiceId::new("alto"), middle_c.clone()),
//! ])?;
//!
//! assert_ne!(single, doubled);
//! assert_eq!(single.forget_voice_labels().total_len(), 1);
//! assert_eq!(doubled.forget_voice_labels().total_len(), 2);
//!
//! // Both have one distinct pitch: the doubling is a multiplicity, not a
//! // second pitch.
//! assert_eq!(doubled.forget_voice_labels().distinct_len(), 1);
//! assert!(doubled.has_doubling());
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::error::PitchError;
use crate::pitch::point::{IntervalGroupElement, PitchOrigin, PitchPoint};
use crate::realization::provenance::ProvenanceId;

/// Stable identity of a voice (prompt section 24).
///
/// UMT layer: metadata. Voice identity is semantic: two voices sounding the
/// same pitch are still two voices, and a voice that moves between chords is
/// the same voice because it carries the same identity, not because it happens
/// to occupy the same index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", from = "String"))]
pub struct VoiceId(Arc<str>);

impl VoiceId {
    /// Wraps a stable voice identity.
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

impl From<String> for VoiceId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<VoiceId> for String {
    fn from(value: VoiceId) -> Self {
        value.as_str().into()
    }
}

impl core::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A finite set of voice identities, `V_c` (UMT-3.2 section 4.3).
///
/// UMT layer: metadata. Ordered, so iteration and any derived output are
/// reproducible.
///
/// Parallel juxtaposition of independent voice collections is
/// [`VoiceSet::disjoint_union`], and the empty set is its neutral element -
/// which is a law worth having rather than a triviality, because it is what
/// makes an empty part composable with a full score.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct VoiceSet(BTreeSet<VoiceId>);

impl VoiceSet {
    /// The empty voice set, the neutral object for disjoint union.
    #[must_use]
    pub fn empty() -> Self {
        Self(BTreeSet::new())
    }

    /// Collects voice identities, rejecting repeats.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::DuplicateVoice`] if an identity appears twice.
    /// Silently deduplicating would turn a two-voice doubling into a single
    /// voice, which is exactly the loss section 4.4.4 forbids.
    pub fn new<I>(voices: I) -> Result<Self, PitchError>
    where
        I: IntoIterator<Item = VoiceId>,
    {
        let mut set = BTreeSet::new();
        for voice in voices {
            if !set.insert(voice.clone()) {
                return Err(PitchError::DuplicateVoice { voice });
            }
        }
        Ok(Self(set))
    }

    /// Adds a voice, reporting whether it was new.
    pub fn insert(&mut self, voice: VoiceId) -> bool {
        self.0.insert(voice)
    }

    /// Whether this set contains a voice.
    #[must_use]
    pub fn contains(&self, voice: &VoiceId) -> bool {
        self.0.contains(voice)
    }

    /// The number of voices.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The voices, in identifier order.
    pub fn iter(&self) -> impl Iterator<Item = &VoiceId> {
        self.0.iter()
    }

    /// Parallel juxtaposition of two independent voice collections
    /// (UMT-3.2 section 4.3).
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::DuplicateVoice`] if the two sets share an
    /// identity. The operation is *disjoint* union: two parts that both call a
    /// voice "tenor" have not been shown to be independent, and merging them
    /// would silently identify two different voices.
    pub fn disjoint_union(&self, other: &Self) -> Result<Self, PitchError> {
        let mut union = self.0.clone();
        for voice in &other.0 {
            if !union.insert(voice.clone()) {
                return Err(PitchError::DuplicateVoice {
                    voice: voice.clone(),
                });
            }
        }
        Ok(Self(union))
    }
}

impl<'a> IntoIterator for &'a VoiceSet {
    type Item = &'a VoiceId;
    type IntoIter = alloc::collections::btree_set::Iter<'a, VoiceId>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// An analytical annotation attached to a chord (UMT-3.2 section 4.6).
///
/// UMT layer: analysis metadata. A designated root, an inversion label, a
/// pitch-class set, or a virtual-pitch estimate is the output of some model,
/// not a primitive truth of the chord, so an annotation MUST identify the
/// model that produced it. Both fields are mandatory here for that reason:
/// there is no way to attach an anonymous claim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChordAnnotation {
    /// Which analytical model produced this annotation.
    pub model: String,
    /// The claim itself, in the model's own vocabulary.
    pub claim: String,
    /// Where the claim came from.
    pub provenance: ProvenanceId,
}

/// A registered chord: a function from voice identities to pitch points.
///
/// UMT layer: L1 or L2, following the interval type of its points.
///
/// Every point in a chord is measured from the same origin. That is not a
/// convenience: a chord whose members had different origins would have no
/// well-defined interval between its own voices, and voice-leading
/// displacement would be undefined too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord<E> {
    origin: Option<PitchOrigin>,
    voices: BTreeMap<VoiceId, PitchPoint<E>>,
    annotations: Vec<ChordAnnotation>,
}

impl<E> Default for Chord<E> {
    fn default() -> Self {
        Self {
            origin: None,
            voices: BTreeMap::new(),
            annotations: Vec::new(),
        }
    }
}

impl<E: IntervalGroupElement> Chord<E> {
    /// The empty chord, the neutral object for [`Chord::juxtapose`].
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds a chord from voice-point pairs.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::DuplicateVoice`] if a voice is assigned twice,
    /// and [`PitchError::OriginMismatch`] if the points are not all measured
    /// from the same origin.
    pub fn from_voices<I>(voices: I) -> Result<Self, PitchError>
    where
        I: IntoIterator<Item = (VoiceId, PitchPoint<E>)>,
    {
        let mut chord = Self::empty();
        for (voice, point) in voices {
            if chord.voices.contains_key(&voice) {
                return Err(PitchError::DuplicateVoice { voice });
            }
            chord.assign(voice, point)?;
        }
        Ok(chord)
    }

    /// Assigns a point to a voice, replacing any previous assignment.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::OriginMismatch`] if the point is measured from a
    /// different origin than the rest of the chord.
    pub fn assign(
        &mut self,
        voice: VoiceId,
        point: PitchPoint<E>,
    ) -> Result<Option<PitchPoint<E>>, PitchError> {
        match &self.origin {
            Some(origin) if origin != point.origin() => {
                return Err(PitchError::OriginMismatch {
                    left: origin.clone(),
                    right: point.origin().clone(),
                });
            }
            Some(_) => {}
            None => self.origin = Some(point.origin().clone()),
        }
        Ok(self.voices.insert(voice, point))
    }

    /// Builder form of [`Chord::assign`].
    ///
    /// # Errors
    ///
    /// As [`Chord::assign`].
    pub fn with_voice(mut self, voice: VoiceId, point: PitchPoint<E>) -> Result<Self, PitchError> {
        self.assign(voice, point)?;
        Ok(self)
    }

    /// Removes a voice, returning what it held.
    pub fn remove(&mut self, voice: &VoiceId) -> Option<PitchPoint<E>> {
        let removed = self.voices.remove(voice);
        if self.voices.is_empty() {
            self.origin = None;
        }
        removed
    }

    /// The point assigned to a voice.
    #[must_use]
    pub fn get(&self, voice: &VoiceId) -> Option<&PitchPoint<E>> {
        self.voices.get(voice)
    }

    /// The point assigned to a voice, as a required lookup.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::UnknownVoice`] if the chord has no such voice.
    pub fn require(&self, voice: &VoiceId) -> Result<&PitchPoint<E>, PitchError> {
        self.voices
            .get(voice)
            .ok_or_else(|| PitchError::UnknownVoice {
                voice: voice.clone(),
            })
    }

    /// The chord's voice set.
    #[must_use]
    pub fn voice_set(&self) -> VoiceSet {
        VoiceSet(self.voices.keys().cloned().collect())
    }

    /// The shared origin of every point, if the chord is not empty.
    #[must_use]
    pub fn origin(&self) -> Option<&PitchOrigin> {
        self.origin.as_ref()
    }

    /// The number of voices, which is the number of *notes*, doublings
    /// included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.voices.len()
    }

    /// Whether the chord has no voices.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }

    /// The voice-point pairs, in voice-identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&VoiceId, &PitchPoint<E>)> {
        self.voices.iter()
    }

    /// The analytical annotations attached to this chord.
    #[must_use]
    pub fn annotations(&self) -> &[ChordAnnotation] {
        &self.annotations
    }

    /// Attaches an analytical annotation (UMT-3.2 section 4.6).
    pub fn annotate(&mut self, annotation: ChordAnnotation) {
        self.annotations.push(annotation);
    }

    /// Parallel juxtaposition of two independent voice collections
    /// (UMT-3.2 section 4.3).
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::DuplicateVoice`] on a shared voice identity, and
    /// [`PitchError::OriginMismatch`] if the two chords are measured from
    /// different origins.
    pub fn juxtapose(&self, other: &Self) -> Result<Self, PitchError> {
        let mut combined = self.clone();
        for (voice, point) in &other.voices {
            if combined.voices.contains_key(voice) {
                return Err(PitchError::DuplicateVoice {
                    voice: voice.clone(),
                });
            }
            combined.assign(voice.clone(), point.clone())?;
        }
        combined
            .annotations
            .extend(other.annotations.iter().cloned());
        Ok(combined)
    }

    /// The interval between two of this chord's voices.
    ///
    /// # Errors
    ///
    /// Returns [`PitchError::UnknownVoice`] if either voice is absent.
    pub fn interval(&self, from: &VoiceId, to: &VoiceId) -> Result<E, PitchError> {
        self.require(from)?.interval_to(self.require(to)?)
    }

    /// **Lossy view.** Discards voice identity, keeping multiplicity.
    ///
    /// This is the finite multiset of pitch points of UMT-3.2 section 4.3.
    /// Voice continuity cannot be recovered from it, so voice leading must be
    /// computed before this is applied, not after.
    #[must_use]
    pub fn forget_voice_labels(&self) -> PitchMultiset<E> {
        let mut entries: Vec<(PitchPoint<E>, usize)> = Vec::new();
        for point in self.voices.values() {
            match entries.iter_mut().find(|(seen, _)| seen == point) {
                Some((_, count)) => *count += 1,
                None => entries.push((point.clone(), 1)),
            }
        }
        PitchMultiset { entries }
    }

    /// **Lossy view.** Applies a declared projection to every point, keeping
    /// voice identity.
    ///
    /// Octave-class reduction is the motivating case: UMT-3.2 section 4.6
    /// insists register equivalence is an explicit choice rather than an
    /// implicit erasure, so the projection is supplied by the caller and named
    /// at the call site instead of being built in.
    ///
    /// # Errors
    ///
    /// Propagates whatever the projection reports, and returns
    /// [`PitchError::OriginMismatch`] if the projection does not send every
    /// point to a common origin.
    pub fn project_points<F, T>(&self, project: F) -> Result<Chord<T>, PitchError>
    where
        F: Fn(&PitchPoint<E>) -> Result<PitchPoint<T>, PitchError>,
        T: IntervalGroupElement,
    {
        let mut projected = Chord::<T>::empty();
        for (voice, point) in &self.voices {
            projected.assign(voice.clone(), project(point)?)?;
        }
        projected.annotations = self.annotations.clone();
        Ok(projected)
    }

    /// Whether any pitch point is held by more than one voice.
    #[must_use]
    pub fn has_doubling(&self) -> bool {
        self.forget_voice_labels()
            .entries
            .iter()
            .any(|(_, count)| *count > 1)
    }
}

/// **A lossy view of a chord.** A finite multiset of pitch points, with voice
/// identity discarded and multiplicity kept.
///
/// UMT layer: as the chord it came from.
///
/// Equality is multiset equality, so two chords with the same notes under
/// different voice labels compare equal *here* while remaining different
/// chords. That is the whole reason this is a separate type.
#[derive(Debug, Clone)]
pub struct PitchMultiset<E> {
    entries: Vec<(PitchPoint<E>, usize)>,
}

impl<E: IntervalGroupElement> PitchMultiset<E> {
    /// The number of notes, doublings counted separately.
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.entries.iter().map(|(_, count)| count).sum()
    }

    /// The number of distinct pitch points.
    #[must_use]
    pub fn distinct_len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the multiset is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many voices held a given point.
    #[must_use]
    pub fn multiplicity(&self, point: &PitchPoint<E>) -> usize {
        self.entries
            .iter()
            .find(|(seen, _)| seen == point)
            .map_or(0, |(_, count)| *count)
    }

    /// The distinct points with their multiplicities.
    pub fn iter(&self) -> impl Iterator<Item = (&PitchPoint<E>, usize)> {
        self.entries.iter().map(|(point, count)| (point, *count))
    }

    /// Every note as a separate entry, doublings repeated.
    #[must_use]
    pub fn expand(&self) -> Vec<PitchPoint<E>> {
        let mut points = Vec::with_capacity(self.total_len());
        for (point, count) in &self.entries {
            for _ in 0..*count {
                points.push(point.clone());
            }
        }
        points
    }

    /// **Lossy view.** Discards multiplicity as well, leaving a set.
    ///
    /// After this, one C and two doubled Cs are genuinely indistinguishable.
    /// UMT-3.2 section 4.4.4 does not forbid that; it forbids it happening
    /// *without being asked for*, which is why it takes a second named call.
    #[must_use]
    pub fn forget_multiplicity(&self) -> Vec<PitchPoint<E>> {
        self.entries
            .iter()
            .map(|(point, _)| point.clone())
            .collect()
    }
}

impl<E: IntervalGroupElement> PartialEq for PitchMultiset<E> {
    fn eq(&self, other: &Self) -> bool {
        self.entries.len() == other.entries.len()
            && self
                .entries
                .iter()
                .all(|(point, count)| other.multiplicity(point) == *count)
    }
}

impl<E: IntervalGroupElement> Eq for PitchMultiset<E> {}

#[cfg(test)]
mod tests {
    use super::{Chord, ChordAnnotation, VoiceId, VoiceSet};
    use crate::error::PitchError;
    use crate::pitch::point::{PitchOrigin, PitchPoint};
    use crate::realization::provenance::ProvenanceId;
    use crate::temperament::image::{AmbientElem, AmbientLattice};
    use alloc::string::String;
    use alloc::sync::Arc;

    fn steps() -> Arc<AmbientLattice> {
        AmbientLattice::new("umt:edo:12", 1)
    }

    fn point(lattice: &Arc<AmbientLattice>, step: i64) -> PitchPoint<AmbientElem> {
        PitchPoint::new(
            PitchOrigin::new("umt:origin:c4"),
            lattice.element([step]).unwrap(),
        )
    }

    fn triad() -> Chord<AmbientElem> {
        let lattice = steps();
        Chord::from_voices([
            (VoiceId::new("bass"), point(&lattice, 0)),
            (VoiceId::new("tenor"), point(&lattice, 4)),
            (VoiceId::new("soprano"), point(&lattice, 7)),
        ])
        .unwrap()
    }

    #[test]
    fn a_chord_is_a_function_from_voices_to_points() {
        let chord = triad();
        assert_eq!(chord.len(), 3);
        assert_eq!(chord.voice_set().len(), 3);
        assert!(chord.voice_set().contains(&VoiceId::new("tenor")));
        assert_eq!(chord.get(&VoiceId::new("tenor")), Some(&point(&steps(), 4)));
        assert!(matches!(
            chord.require(&VoiceId::new("descant")),
            Err(PitchError::UnknownVoice { .. })
        ));
    }

    #[test]
    fn doublings_and_unisons_survive() {
        let lattice = steps();
        let doubled = Chord::from_voices([
            (VoiceId::new("soprano"), point(&lattice, 0)),
            (VoiceId::new("alto"), point(&lattice, 0)),
        ])
        .unwrap();
        let single = Chord::from_voices([(VoiceId::new("soprano"), point(&lattice, 0))]).unwrap();

        assert_ne!(single, doubled);
        assert!(doubled.has_doubling());
        assert!(!single.has_doubling());

        let multiset = doubled.forget_voice_labels();
        assert_eq!(multiset.total_len(), 2);
        assert_eq!(multiset.distinct_len(), 1);
        assert_eq!(multiset.multiplicity(&point(&lattice, 0)), 2);
        assert_eq!(multiset.expand().len(), 2);

        // Only the second, separately named step erases the multiplicity.
        assert_eq!(multiset.forget_multiplicity().len(), 1);
        assert_ne!(multiset, single.forget_voice_labels());
    }

    #[test]
    fn forgetting_labels_identifies_relabelled_chords_but_not_the_chords() {
        let lattice = steps();
        let one = Chord::from_voices([
            (VoiceId::new("a"), point(&lattice, 0)),
            (VoiceId::new("b"), point(&lattice, 7)),
        ])
        .unwrap();
        let other = Chord::from_voices([
            (VoiceId::new("b"), point(&lattice, 0)),
            (VoiceId::new("a"), point(&lattice, 7)),
        ])
        .unwrap();

        assert_ne!(one, other, "the voices moved");
        assert_eq!(
            one.forget_voice_labels(),
            other.forget_voice_labels(),
            "but the sounding multiset did not"
        );
    }

    #[test]
    fn every_point_shares_one_origin() {
        let lattice = steps();
        let elsewhere = PitchPoint::new(PitchOrigin::new("umt:origin:a4"), lattice.zero());
        let mut chord = Chord::from_voices([(VoiceId::new("a"), point(&lattice, 0))]).unwrap();
        assert!(matches!(
            chord.assign(VoiceId::new("b"), elsewhere),
            Err(PitchError::OriginMismatch { .. })
        ));

        // An emptied chord forgets its origin and can be reused.
        assert!(chord.remove(&VoiceId::new("a")).is_some());
        assert!(chord.origin().is_none());
        assert!(
            chord
                .assign(
                    VoiceId::new("b"),
                    PitchPoint::new(PitchOrigin::new("umt:origin:a4"), lattice.zero())
                )
                .is_ok()
        );
    }

    #[test]
    fn juxtaposition_is_disjoint_and_the_empty_chord_is_neutral() {
        let lattice = steps();
        let lower = Chord::from_voices([(VoiceId::new("bass"), point(&lattice, 0))]).unwrap();
        let upper = Chord::from_voices([(VoiceId::new("soprano"), point(&lattice, 7))]).unwrap();

        let together = lower.juxtapose(&upper).unwrap();
        assert_eq!(together.len(), 2);

        // Neutral element.
        assert_eq!(lower.juxtapose(&Chord::empty()).unwrap(), lower);
        assert_eq!(Chord::empty().juxtapose(&lower).unwrap(), lower);

        // A shared voice identity is a defect, not a merge.
        assert!(matches!(
            lower.juxtapose(&lower),
            Err(PitchError::DuplicateVoice { .. })
        ));
    }

    #[test]
    fn voice_sets_reject_repeats_and_compose_disjointly() {
        assert!(matches!(
            VoiceSet::new([VoiceId::new("a"), VoiceId::new("a")]),
            Err(PitchError::DuplicateVoice { .. })
        ));

        let left = VoiceSet::new([VoiceId::new("a")]).unwrap();
        let right = VoiceSet::new([VoiceId::new("b")]).unwrap();
        assert_eq!(left.disjoint_union(&right).unwrap().len(), 2);
        assert!(matches!(
            left.disjoint_union(&left),
            Err(PitchError::DuplicateVoice { .. })
        ));

        // The empty set is neutral.
        assert_eq!(left.disjoint_union(&VoiceSet::empty()).unwrap(), left);
        assert_eq!(VoiceSet::empty().disjoint_union(&left).unwrap(), left);
    }

    #[test]
    fn a_declared_projection_keeps_voices_and_loses_register() {
        let lattice = steps();
        let chord = Chord::from_voices([
            (VoiceId::new("bass"), point(&lattice, 0)),
            (VoiceId::new("soprano"), point(&lattice, 12)),
        ])
        .unwrap();
        assert_eq!(chord.forget_voice_labels().distinct_len(), 2);

        // Octave reduction is asked for by name, not applied behind the scenes.
        let classes = chord
            .project_points(|p| {
                let step = p.offset().coordinates()[0].clone();
                let reduced = ((step % 12) + 12) % 12;
                Ok(PitchPoint::new(
                    p.origin().clone(),
                    lattice.element([reduced])?,
                ))
            })
            .unwrap();
        assert_eq!(classes.len(), 2, "both voices survive");
        assert_eq!(
            classes.forget_voice_labels().distinct_len(),
            1,
            "but they now sound the same class"
        );
        assert!(classes.has_doubling());
    }

    #[test]
    fn intervals_within_a_chord_are_available() {
        let chord = triad();
        let third = chord
            .interval(&VoiceId::new("bass"), &VoiceId::new("tenor"))
            .unwrap();
        assert_eq!(third, steps().element([4i64]).unwrap());
    }

    #[test]
    fn annotations_carry_their_model_and_provenance() {
        let mut chord = triad();
        assert!(chord.annotations().is_empty());
        chord.annotate(ChordAnnotation {
            model: String::from("umt:analysis:root-position-triad"),
            claim: String::from("root = bass"),
            provenance: ProvenanceId::new("umt:prov:test"),
        });
        assert_eq!(chord.annotations().len(), 1);
        assert_eq!(
            chord.annotations()[0].model,
            "umt:analysis:root-position-triad"
        );
    }
}
