//! Supplementary example: modular generated sets, the MOS predicate, and
//! Euclidean rhythms - three things UMT-3.2 part III keeps carefully apart.
//!
//! Run with `cargo run --example generated_scales`.

use umt::generated::{
    EuclideanRhythm, GeneratedSet, GeneratorRatio, MosProfile, RotationConvention,
    quarter_comma_meantone_generator,
};
use umt::pitch::Cents;
use umt::{Q, Z};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let period = Cents::new(1200.0)?;
    let generator = quarter_comma_meantone_generator();

    println!("== Designated data (section 3.1) ==");
    println!("  period    {period}");
    println!("  generator {generator}");
    println!(
        "  g/p is irrational, and that is declared rather than inferred from\n  \
         two doubles: no finite computation on `f64` decides rationality."
    );

    // Fixture F35: three points, two gap sizes.
    let three = GeneratedSet::from_cents(period, generator, 3, GeneratorRatio::Irrational)?;
    println!("\n== Three notes (fixture F35) ==");
    for (index, point) in three.sorted_distinct_points().iter().enumerate() {
        println!("  degree {index}: {}", Cents::from(*point));
    }
    let report = three.gap_report();
    println!("  circular gaps:");
    for gap in report.gaps() {
        println!("    {}", Cents::from(*gap));
    }
    println!(
        "  {} distinct sizes, so cardinality 3 is MOS under the operational\n  \
         predicate of section 3.3",
        report.distinct_sizes().len()
    );

    // Section 3.3's list, computed rather than quoted.
    println!("\n== MOS cardinalities up to 31 (section 3.3) ==");
    let mos = three.mos_cardinalities(31, MosProfile::TwoStepSizes)?;
    println!("  {mos:?}");
    println!(
        "  the intervening cardinalities are still generated scales - they\n  \
         simply have three gap sizes rather than two:"
    );
    for cardinality in [4usize, 6, 8] {
        let scale = three.at_cardinality(cardinality)?;
        println!(
            "    n = {cardinality:>2}: {} distinct points, {} gap sizes",
            scale.sorted_distinct_points().len(),
            scale.gap_report().distinct_sizes().len()
        );
    }

    // The diatonic step word and one of its modes.
    let diatonic = three.at_cardinality(7)?;
    let word = |pattern: &[usize]| {
        pattern
            .iter()
            .map(|step| if *step == 0 { 's' } else { 'L' })
            .collect::<String>()
    };
    println!("\n== Step pattern and modes (section 3.4) ==");
    println!("  pattern  {}", word(&diatonic.step_pattern()));
    for degree in [1usize, 5] {
        println!("  mode {degree}   {}", word(&diatonic.mode(degree)?));
    }
    println!(
        "  whether two rotations count as the same scale is a declared\n  \
         equivalence of the application, which is why this crate reports them\n  \
         and does not identify them."
    );

    // A rational ratio closes its orbit, and the duplicates are reported.
    println!("\n== A closing orbit (sections 3.2 and 9.11) ==");
    let edo = GeneratedSet::from_cents(
        period,
        Cents::new(700.0)?,
        20,
        GeneratorRatio::Rational(Q::new(Z::from(7), Z::from(12))),
    )?;
    let report = edo.gap_report();
    println!(
        "  seven steps of 12-EDO, asked for {} points: {} distinct, {} duplicates",
        report.generated(),
        report.distinct(),
        report.duplicates()
    );
    println!("  orbit closes at {:?}", edo.ratio().orbit_closure());
    println!(
        "  one gap size, so not MOS under the strict predicate and MOS under\n  \
         the one that admits the degenerate equal-step case: {} / {}",
        edo.mos_verdict(MosProfile::TwoStepSizes).is_mos(),
        edo.mos_verdict(MosProfile::TwoStepSizesAllowingEqual)
            .is_mos()
    );

    // Euclidean rhythms: shared arithmetic, different object.
    println!("\n== Euclidean rhythms (section 3.5) ==");
    for (onsets, pulses) in [(3u32, 8u32), (5, 8), (5, 13), (2, 8)] {
        let rhythm = EuclideanRhythm::new(onsets, pulses, RotationConvention::FirstPulseOnset)?;
        let evenness = rhythm.verify_maximal_evenness();
        println!(
            "  {rhythm}  gaps {:?}  maximally even {}  primitive {}",
            rhythm.inter_onset_intervals(),
            evenness.holds(),
            rhythm.is_primitive()
        );
    }
    println!(
        "\n  Evenness is verified against the produced positions by the\n  \
         Clough-Douthett characterisation, not inferred from the formula that\n  \
         produced them. And a finite Euclidean word is not called an infinite\n  \
         Sturmian word: section 3.5 forbids exactly that conflation."
    );

    Ok(())
}
