//! The Scala `.scl` adapter (UMT-3.2 sections 8.1 and 8.2, fixture F21).
//!
//! # An `.scl` file is not uniformly L3
//!
//! Section 8.2 makes the point that motivates this whole module: a Scala scale
//! file can contain entries written either as exact ratios or as cents values,
//! so the file "is not uniformly `L3 only`". A ratio entry preserves an exact
//! rational interval; a cents entry is a metric real-valued specification.
//! They are different layers, in the same file, line by line.
//!
//! Section 8.2 then requires that "an importer MUST retain whether each entry
//! was exact-rational or metric-decimal when round-trip fidelity matters", so
//! [`ScalaEntry`] is an enum and the distinction survives. Converting to a
//! uniform representation is available - [`ScalaScale::to_empirical_scale`] -
//! and it reports what that costs rather than performing it silently. That is
//! fixture F21.
//!
//! # What this adapter is and is not
//!
//! Section 8.1 requires an adapter to declare what it imports, what it
//! exports, which layers it represents, what it drops, and the exact
//! format profile tested. [`ScalaAdapter::profile`] is those five
//! declarations, as data.
//!
//! An `.scl` file "does not by itself encode a UMT regular-temperament mapping
//! matrix, comma kernel, or full spelling system", and this adapter does not
//! invent any of them. Keyboard mappings are a separate `.kbm` file and a
//! separate object (section 8.3); they are not implemented here.
//!
//! # Examples
//!
//! ```
//! use umt::io::scala::ScalaScale;
//! use umt::pitch::ScaleId;
//!
//! let scale = ScalaScale::parse("\
//! ! mixed.scl
//! !
//! A scale with both kinds of entry
//!  3
//! !
//!  9/8
//!  386.313714
//!  2/1
//! ")?;
//!
//! assert!(scale.is_mixed());
//!
//! // The ratios stayed exact; the cents value did not acquire exactness.
//! assert!(scale.entries()[0].is_exact());
//! assert!(!scale.entries()[1].is_exact());
//! assert_eq!(scale.entries()[1].exact_ratio(), None);
//!
//! // The distinction survives a round trip through the format.
//! assert_eq!(ScalaScale::parse(&scale.to_scl_text())?, scale);
//!
//! // Flattening to a uniform L3 scale is available, and reports its cost.
//! let (_, lost) = scale.to_empirical_scale(ScaleId::new("umt:scale:mixed"))?;
//! assert_eq!(lost.len(), 2, "one notation residual per flattened exact entry");
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use num_traits::Signed;

use crate::algebra::{Q, Z};
use crate::error::IoError;
use crate::pitch::empirical::{EmpiricalDegree, EmpiricalScale, ScaleId};
use crate::pitch::units::{Cents, Octaves};
use crate::realization::record::Layer;
use crate::realization::residual::{Residual, ResidualRecord, ResidualSet};

/// The format profile this adapter supports (UMT-3.2 section 8.7).
///
/// Pinned, because section 8.7 requires it of every evolving interchange
/// standard and the Scala file format has accumulated conventions over
/// decades.
pub const SUPPORTED_PROFILE: &str = "Scala .scl, as documented by the Scala 2.2 distribution";

/// One scale-degree entry of a Scala file (UMT-3.2 section 8.2).
///
/// UMT layer: L1 for a ratio, L3 for a cents value. The two are different
/// variants because they are different layers, and collapsing them would
/// discard exactly what section 8.2 requires be retained.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ScalaEntry {
    /// An exact rational interval, written as `n/d` or as a bare integer.
    Ratio(#[cfg_attr(feature = "serde", serde(with = "crate::io::serde_exact::q"))] Q),
    /// A metric interval in cents, written with a decimal point.
    Cents(Cents),
}

impl ScalaEntry {
    /// The exact rational value, for a ratio entry.
    ///
    /// `None` for a cents entry: a decimal cents value is a real measurement,
    /// and turning it into a rational would fabricate exactness the file never
    /// claimed.
    #[must_use]
    pub fn exact_ratio(&self) -> Option<&Q> {
        match self {
            Self::Ratio(ratio) => Some(ratio),
            Self::Cents(_) => None,
        }
    }

    /// Whether this entry is exact.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Ratio(_))
    }

    /// The entry's size in cents.
    ///
    /// For a ratio entry this crosses from L1 to L3 and is therefore lossy in
    /// the direction that matters: the result no longer knows it came from an
    /// exact value.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::MalformedInput`] if the ratio cannot be evaluated as
    /// a finite real.
    pub fn to_cents(&self) -> Result<Cents, IoError> {
        match self {
            Self::Cents(cents) => Ok(*cents),
            Self::Ratio(ratio) => {
                let log2 = crate::algebra::rational::log2_q_f64(ratio).ok_or_else(|| {
                    IoError::MalformedInput {
                        format: String::from("scl"),
                        line: 0,
                        reason: alloc::format!("ratio {ratio} has no finite logarithm"),
                    }
                })?;
                Octaves::new(log2)
                    .map(Cents::from)
                    .map_err(|_| IoError::MalformedInput {
                        format: String::from("scl"),
                        line: 0,
                        reason: alloc::format!("ratio {ratio} is out of range"),
                    })
            }
        }
    }

    /// The entry as it appears in a Scala file.
    #[must_use]
    pub fn to_scl_text(&self) -> String {
        match self {
            Self::Ratio(ratio) => {
                if ratio.denom() == &Z::from(1) {
                    ratio.numer().to_string()
                } else {
                    alloc::format!("{}/{}", ratio.numer(), ratio.denom())
                }
            }
            // A cents entry must contain a decimal point, or a reader would
            // take it for a ratio.
            Self::Cents(cents) => alloc::format!("{:.6}", cents.get()),
        }
    }
}

/// A Scala scale file (UMT-3.2 section 8.2).
///
/// UMT layer: mixed, per entry.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScalaScale {
    description: String,
    entries: Vec<ScalaEntry>,
}

impl ScalaScale {
    /// Builds a scale from a description and its entries.
    #[must_use]
    pub fn new(description: &str, entries: Vec<ScalaEntry>) -> Self {
        Self {
            description: description.into(),
            entries,
        }
    }

    /// The file's description line.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The scale-degree entries, in order.
    #[must_use]
    pub fn entries(&self) -> &[ScalaEntry] {
        &self.entries
    }

    /// How many degrees the scale has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the scale has no degrees.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries were written as exact ratios.
    #[must_use]
    pub fn exact_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_exact()).count()
    }

    /// Whether the file mixes exact and metric entries.
    ///
    /// The case fixture F21 is about, and the case a uniform representation
    /// would silently flatten.
    #[must_use]
    pub fn is_mixed(&self) -> bool {
        let exact = self.exact_count();
        exact > 0 && exact < self.entries.len()
    }

    /// Parses a Scala `.scl` file.
    ///
    /// The format: lines beginning with `!` are comments; the first
    /// non-comment line is the description, which may be empty; the second is
    /// the degree count; the remainder are the entries. An entry containing a
    /// `.` is a cents value, and anything else is a ratio.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::MalformedInput`] with the offending line number for
    /// a missing count, a count that disagrees with the entries, a
    /// non-positive ratio, a zero denominator, or an unparseable entry.
    pub fn parse(text: &str) -> Result<Self, IoError> {
        let malformed = |line: usize, reason: &str| IoError::MalformedInput {
            format: String::from("scl"),
            line,
            reason: reason.into(),
        };

        let mut description: Option<String> = None;
        let mut expected: Option<usize> = None;
        let mut entries = Vec::new();

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.starts_with('!') {
                continue;
            }
            if description.is_none() {
                // The description may legitimately be empty, so a blank line
                // here is the description rather than a skipped line.
                description = Some(trimmed.to_string());
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }
            if expected.is_none() {
                expected = Some(
                    trimmed
                        .split_whitespace()
                        .next()
                        .and_then(|token| token.parse::<usize>().ok())
                        .ok_or_else(|| malformed(line, "expected a degree count"))?,
                );
                continue;
            }
            // Anything after the first token on an entry line is a comment.
            let token = trimmed
                .split_whitespace()
                .next()
                .ok_or_else(|| malformed(line, "empty entry"))?;
            entries.push(parse_entry(token, line)?);
        }

        let description = description.ok_or_else(|| malformed(1, "empty file"))?;
        let expected = expected.ok_or_else(|| malformed(1, "no degree count"))?;
        if entries.len() != expected {
            return Err(malformed(
                0,
                &alloc::format!(
                    "the file declares {expected} degrees and contains {}",
                    entries.len()
                ),
            ));
        }
        Ok(Self {
            description,
            entries,
        })
    }

    /// Writes the scale back out as a Scala file.
    ///
    /// Exact entries are written as ratios and metric entries with a decimal
    /// point, so a round trip through this pair preserves the distinction
    /// section 8.2 requires be retained.
    #[must_use]
    pub fn to_scl_text(&self) -> String {
        let mut out = String::from("! Written by umt.\n!\n");
        out.push_str(&self.description);
        out.push('\n');
        out.push_str(&alloc::format!(" {}\n", self.entries.len()));
        out.push_str("!\n");
        for entry in &self.entries {
            out.push(' ');
            out.push_str(&entry.to_scl_text());
            out.push('\n');
        }
        out
    }

    /// **Lossy view.** Every entry as a measured L3 degree.
    ///
    /// Uniform, and therefore no longer able to say which entries were exact.
    /// The returned [`ResidualSet`] records one notation residual per exact
    /// entry that was flattened, so the loss is reported rather than
    /// performed quietly (UMT-3.2 section 9.12, law 2).
    ///
    /// # Errors
    ///
    /// Propagates a ratio that cannot be evaluated as a finite real.
    pub fn to_empirical_scale(
        &self,
        id: ScaleId,
    ) -> Result<(EmpiricalScale, ResidualSet), IoError> {
        let mut degrees = Vec::with_capacity(self.entries.len());
        let mut lost = ResidualSet::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let cents = entry.to_cents()?;
            degrees.push(
                EmpiricalDegree::from_cents(cents, Cents::new(0.0).expect("zero is finite"))
                    .expect("a zero uncertainty is non-negative"),
            );
            if entry.is_exact() {
                lost.push(ResidualRecord::new(Residual::Notation {
                    detail: alloc::format!(
                        "degree {} was written as the exact ratio {}",
                        index + 1,
                        entry.to_scl_text()
                    ),
                }));
            }
        }
        Ok((EmpiricalScale::new(id, degrees), lost))
    }
}

fn parse_entry(token: &str, line: usize) -> Result<ScalaEntry, IoError> {
    let malformed = |reason: String| IoError::MalformedInput {
        format: String::from("scl"),
        line,
        reason,
    };

    if token.contains('.') {
        let value: f64 = token
            .parse()
            .map_err(|_| malformed(alloc::format!("`{token}` is not a cents value")))?;
        return Cents::new(value)
            .map(ScalaEntry::Cents)
            .map_err(|_| malformed(alloc::format!("`{token}` is not finite")));
    }

    let (numerator, denominator) = match token.split_once('/') {
        Some((numerator, denominator)) => (numerator, denominator),
        None => (token, "1"),
    };
    let numerator: i64 = numerator
        .trim()
        .parse()
        .map_err(|_| malformed(alloc::format!("`{token}` is not a ratio")))?;
    let denominator: i64 = denominator
        .trim()
        .parse()
        .map_err(|_| malformed(alloc::format!("`{token}` is not a ratio")))?;
    if denominator == 0 {
        return Err(malformed(alloc::format!(
            "`{token}` has a zero denominator"
        )));
    }
    let ratio = Q::new(Z::from(numerator), Z::from(denominator));
    if !ratio.is_positive() {
        return Err(malformed(alloc::format!(
            "`{token}` is not a positive interval"
        )));
    }
    Ok(ScalaEntry::Ratio(ratio))
}

/// The five declarations UMT-3.2 section 8.1 requires of an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdapterProfile {
    /// What the adapter imports.
    pub imports: Vec<String>,
    /// What it exports.
    pub exports: Vec<String>,
    /// Which UMT layers it represents.
    pub layers: Vec<Layer>,
    /// What it drops, approximates, or reconstructs.
    pub dropped: Vec<String>,
    /// The exact format profile tested.
    pub tested_profile: String,
}

/// The Scala `.scl` adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalaAdapter;

impl ScalaAdapter {
    /// What this adapter declares about itself (UMT-3.2 section 8.1).
    #[must_use]
    pub fn profile() -> AdapterProfile {
        AdapterProfile {
            imports: alloc::vec![
                String::from("scale description"),
                String::from("degree count"),
                String::from("exact rational degree entries"),
                String::from("metric cents degree entries"),
            ],
            exports: alloc::vec![
                String::from("scale description"),
                String::from("exact rational degree entries"),
                String::from("metric cents degree entries"),
            ],
            layers: alloc::vec![Layer::L1Exact, Layer::L3Metric],
            dropped: alloc::vec![
                String::from("comment lines other than the description"),
                String::from("trailing comments on entry lines"),
                String::from(
                    "everything the format does not carry: no temperament mapping matrix, \
                     no comma kernel, no spelling system, no keyboard mapping"
                ),
                String::from("measurement uncertainty, which .scl has no field for"),
            ],
            tested_profile: String::from(SUPPORTED_PROFILE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScalaAdapter, ScalaEntry, ScalaScale};
    use crate::algebra::{Q, Z};
    use crate::error::IoError;
    use crate::pitch::empirical::ScaleId;
    use crate::pitch::units::Cents;
    use crate::realization::record::Layer;
    use crate::realization::residual::ResidualKind;

    /// A file mixing exact ratios and cents values, which is the case
    /// fixture F21 is about.
    const MIXED: &str = "\
! mixed.scl
!
A scale with both kinds of entry
 4
!
 9/8
 386.313714
 3/2
 2
";

    #[test]
    fn f21_a_mixed_file_keeps_each_entry_in_its_own_layer() {
        let scale = ScalaScale::parse(MIXED).unwrap();
        assert_eq!(scale.description(), "A scale with both kinds of entry");
        assert_eq!(scale.len(), 4);
        assert!(scale.is_mixed());
        assert_eq!(scale.exact_count(), 3);

        // The ratios stayed exact.
        assert_eq!(
            scale.entries()[0].exact_ratio(),
            Some(&Q::new(Z::from(9), Z::from(8)))
        );
        assert_eq!(
            scale.entries()[3].exact_ratio(),
            Some(&Q::from(Z::from(2))),
            "a bare integer is the ratio n/1"
        );

        // The cents value did not become one.
        assert!(!scale.entries()[1].is_exact());
        assert_eq!(scale.entries()[1].exact_ratio(), None);
        match &scale.entries()[1] {
            ScalaEntry::Cents(cents) => assert!((cents.get() - 386.313_714).abs() < 1e-9),
            other => panic!("expected a cents entry, got {other:?}"),
        }
    }

    #[test]
    fn the_distinction_survives_a_round_trip_through_the_format() {
        let scale = ScalaScale::parse(MIXED).unwrap();
        let text = scale.to_scl_text();
        let reparsed = ScalaScale::parse(&text).unwrap();
        assert_eq!(reparsed, scale);
        assert_eq!(reparsed.exact_count(), 3);

        // The written ratios are ratios, and the written cents has a point.
        assert!(text.contains(" 9/8\n"));
        assert!(text.contains(" 2\n"));
        assert!(text.contains("386.313714"));
    }

    #[test]
    fn flattening_to_a_uniform_scale_reports_what_it_cost() {
        let scale = ScalaScale::parse(MIXED).unwrap();
        let (empirical, lost) = scale
            .to_empirical_scale(ScaleId::new("umt:scale:mixed"))
            .unwrap();

        assert_eq!(empirical.len(), 4);
        assert!(!empirical.has_period(), "no period is invented");

        // Three exact entries were flattened, and three residuals say so.
        assert_eq!(lost.len(), 3);
        assert_eq!(lost.of_kind(ResidualKind::Notation).count(), 3);
        assert!(lost.records()[0].residual().kind() == ResidualKind::Notation);

        // The just major third and the tempered one are now indistinguishable
        // in kind, which is exactly the loss that had to be reported.
        let third = empirical.degrees()[1].cents().get();
        assert!((third - 386.313_714).abs() < 1e-9);
    }

    #[test]
    fn a_ratio_converts_to_cents_and_says_it_is_leaving_l1() {
        let fifth = ScalaEntry::Ratio(Q::new(Z::from(3), Z::from(2)));
        assert!((fifth.to_cents().unwrap().get() - 701.955_000_9).abs() < 1e-6);
        assert!(fifth.is_exact());

        let measured = ScalaEntry::Cents(Cents::new(700.0).unwrap());
        assert_eq!(measured.to_cents().unwrap(), Cents::new(700.0).unwrap());
        assert!(!measured.is_exact());
        assert_ne!(
            fifth, measured,
            "701.955 cents and 3/2 are different objects"
        );
    }

    #[test]
    fn malformed_input_is_reported_with_its_line() {
        let bad_count = "! x\ndesc\n 3\n 9/8\n 2\n";
        assert!(matches!(
            ScalaScale::parse(bad_count),
            Err(IoError::MalformedInput { .. })
        ));

        let zero_denominator = "! x\ndesc\n 1\n 3/0\n";
        match ScalaScale::parse(zero_denominator) {
            Err(IoError::MalformedInput { line, reason, .. }) => {
                assert_eq!(line, 4);
                assert!(reason.contains("zero denominator"), "{reason}");
            }
            other => panic!("unexpected {other:?}"),
        }

        let negative = "! x\ndesc\n 1\n -3/2\n";
        assert!(matches!(
            ScalaScale::parse(negative),
            Err(IoError::MalformedInput { .. })
        ));

        let nonsense = "! x\ndesc\n 1\n banana\n";
        assert!(ScalaScale::parse(nonsense).is_err());
    }

    #[test]
    fn comments_and_trailing_text_are_handled_as_the_format_says() {
        let text = "\
! twelve.scl
! a comment
An empty description follows this line
 2
! another comment
 100.0 trailing comment
 2/1 also a comment
";
        let scale = ScalaScale::parse(text).unwrap();
        assert_eq!(scale.len(), 2);
        assert!(!scale.entries()[0].is_exact());
        assert!(scale.entries()[1].is_exact());
    }

    #[test]
    fn an_empty_description_is_a_description() {
        let text = "!\n\n 1\n 2/1\n";
        let scale = ScalaScale::parse(text).unwrap();
        assert_eq!(scale.description(), "");
        assert_eq!(scale.len(), 1);
    }

    #[test]
    fn the_adapter_declares_all_five_things_section_8_1_asks_for() {
        let profile = ScalaAdapter::profile();
        assert!(!profile.imports.is_empty());
        assert!(!profile.exports.is_empty());
        assert_eq!(profile.layers, [Layer::L1Exact, Layer::L3Metric]);
        assert!(!profile.dropped.is_empty());
        assert!(profile.tested_profile.contains("Scala"));

        // What it drops is named, including the things the format simply does
        // not carry.
        assert!(
            profile
                .dropped
                .iter()
                .any(|what| what.contains("temperament mapping matrix"))
        );
        assert!(
            profile
                .dropped
                .iter()
                .any(|what| what.contains("uncertainty"))
        );
    }
}
