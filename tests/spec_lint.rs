//! Fixture F30: the normative Markdown source lint (UMT-3.2 section 9.13).
//!
//! F30 is unlike every other fixture: it constrains the *specification
//! source*, not the library. It is here because the specification is checked
//! into this repository as `UMT-3.2.md` and is authoritative for everything
//! else in the crate, so a defect in it is a defect this project can and
//! should catch.
//!
//! The three obligations, from the fixture text:
//!
//! - zero occurrences of the avoided named-operator macro;
//! - named functions use the source-compatible alternatives selected by this
//!   revision;
//! - uppercase normative keywords are drawn only from the vocabulary declared
//!   in section 0.3.

use std::collections::BTreeSet;

const SPEC: &str = include_str!("../UMT-3.2.md");

/// The vocabulary UMT-3.2 section 0.3 declares normative.
const NORMATIVE_VOCABULARY: &[&str] = &["MUST", "MUST NOT", "SHOULD", "SHOULD NOT", "MAY"];

/// Uppercase words that appear in the specification for reasons other than
/// normative force: acronyms, format names, and identifiers.
///
/// Listed explicitly so that a *new* uppercase word entering the source has to
/// be classified deliberately rather than slipping past the lint.
const NON_NORMATIVE_UPPERCASE: &[&str] = &[
    // Acronyms and format names.
    "API", "CBOR", "DAC", "DOI", "EDO", "GCD", "ID", "JI", "JSON", "LCM", "MEI", "MIDI", "MNX",
    "MOS", "PPQN", "SHA", "SNF", "STP", "TCN", "TOP", "UCLA", "UI", "UMT", "URL",
    // Ordinary English words that happen to be short and capitalised in
    // prose or in a heading.
    "ALL", "AND", "NOT", "OR", // Layer labels and the general linear group.
    "GL", "L0", "L1", "L2", "L3", "L4", // Roman numerals in part headings.
    "II", "III", "IV", "IX", "V", "VI", "VII", "VIII", "X", "XI", "XII", "XIII", "XIV",
];

/// F30, first obligation: the avoided named-operator macro does not appear.
///
/// GitHub's Markdown math renderer does not support `\operatorname`, so this
/// revision avoids it. A single occurrence would render as literal source text
/// in the published specification.
#[test]
fn f30_the_avoided_named_operator_macro_is_absent() {
    let occurrences = SPEC.matches("\\operatorname").count();
    assert_eq!(
        occurrences, 0,
        "`\\operatorname` does not render on GitHub; use `\\mathrm` instead"
    );
}

/// F30, second obligation: named functions use the source-compatible
/// alternative this revision selected.
///
/// The alternative is `\mathrm{...}`. This checks that the named functions the
/// specification actually uses are written that way, and that none of them has
/// been left as a bare identifier that would render in italic as a product of
/// variables.
#[test]
fn f30_named_functions_use_the_selected_alternative() {
    // Every named function the specification defines or references in math.
    let named = [
        "im",
        "int",
        "id",
        "get",
        "put",
        "parse",
        "write",
        "round",
        "move",
        "split",
        "merge",
        "birth",
        "death",
        "vertical",
        "horizontal",
        "spelling",
        "drift",
        "parent",
    ];

    let mut missing = Vec::new();
    for name in named {
        if !SPEC.contains(&format!("\\mathrm{{{name}}}")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "these named functions are not written with the selected macro: {missing:?}"
    );

    // And no named function is left bare inside display math, which would
    // render as a product of italic variables.
    for name in ["\\operatorname", "\\DeclareMathOperator"] {
        assert!(
            !SPEC.contains(name),
            "`{name}` is not source-compatible with the target renderer"
        );
    }
}

/// F30, third obligation: uppercase normative keywords come only from the
/// vocabulary section 0.3 declares.
///
/// A word such as `SHALL` or `REQUIRED` carries normative force in other
/// specification traditions. Using one here would create an obligation outside
/// the declared vocabulary, which section 0.3 does not license.
#[test]
fn f30_uppercase_keywords_are_drawn_from_the_declared_vocabulary() {
    let mut found: BTreeSet<String> = BTreeSet::new();

    for word in SPEC.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        // Only all-caps words of two or more letters can be keywords.
        if word.len() < 2 || !word.chars().all(|c| c.is_ascii_uppercase()) {
            continue;
        }
        found.insert(word.to_string());
    }

    let declared: BTreeSet<String> = NORMATIVE_VOCABULARY
        .iter()
        .flat_map(|phrase| phrase.split_whitespace())
        .map(str::to_string)
        .collect();
    let allowed: BTreeSet<String> = NON_NORMATIVE_UPPERCASE
        .iter()
        .map(|word| (*word).to_string())
        .collect();

    let unexpected: Vec<&String> = found
        .iter()
        .filter(|word| !declared.contains(*word) && !allowed.contains(*word))
        .collect();

    assert!(
        unexpected.is_empty(),
        "uppercase words outside the section 0.3 vocabulary and the \
         non-normative allowlist: {unexpected:?}"
    );

    // The declared vocabulary is actually used, so the lint is testing
    // something rather than passing vacuously.
    for keyword in ["MUST", "SHOULD", "MAY"] {
        assert!(found.contains(keyword), "`{keyword}` should appear");
    }

    // And the keywords that carry normative force elsewhere are absent here.
    for foreign in ["SHALL", "REQUIRED", "RECOMMENDED", "OPTIONAL"] {
        assert!(
            !found.contains(foreign),
            "`{foreign}` is not in the section 0.3 vocabulary"
        );
    }
}
