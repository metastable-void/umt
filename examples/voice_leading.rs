//! Supplementary example: chords keep voice identity, and the three numbers
//! you can ask about a voice leading are three different numbers.
//!
//! UMT-3.2 section 4.4.5 requires an implementation to state whether a
//! reported cost is the cost of a declared voice leading or a minimum over
//! some admissible family, "because these answer different questions". This
//! example shows both, side by side, along with the unequal-voice-count case
//! of section 4.4.4 that fixture F8 covers.
//!
//! Run with `cargo run --example voice_leading`.

use umt::pitch::{
    Chord, ChordDistance, CostQuestion, Edge, LogPitchDistance, MassProfile, MetricClaim,
    PitchOrigin, PitchPoint, RegularTuning, SpanCostModel, SpanPenalties, TransportProfile,
    VoiceId, VoiceLeading,
};
use umt::temperament::{AmbientElem, AmbientLattice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let origin = PitchOrigin::new("umt:origin:c4");
    let tuning = RegularTuning::equal_divisions(&steps, 12)?;
    let ground = LogPitchDistance::new(tuning);

    let note = |semitones: i64| -> Result<PitchPoint<AmbientElem>, Box<dyn std::error::Error>> {
        Ok(PitchPoint::new(origin.clone(), steps.element([semitones])?))
    };

    // A C major triad, and the same three pitches after a voice exchange:
    // the bass climbs a fifth to the note the soprano had, and the soprano
    // drops a fifth to the note the bass had.
    let opening = Chord::from_voices([
        (VoiceId::new("bass"), note(0)?),
        (VoiceId::new("tenor"), note(4)?),
        (VoiceId::new("soprano"), note(7)?),
    ])?;
    let exchanged = Chord::from_voices([
        (VoiceId::new("bass"), note(7)?),
        (VoiceId::new("tenor"), note(4)?),
        (VoiceId::new("soprano"), note(0)?),
    ])?;

    println!("== A voice exchange, three voices throughout ==");
    for (voice, point) in opening.iter() {
        println!(
            "  opening   {voice:>8}: {:>3} steps",
            point.offset().coordinates()[0]
        );
    }
    for (voice, point) in exchanged.iter() {
        println!(
            "  exchanged {voice:>8}: {:>3} steps",
            point.offset().coordinates()[0]
        );
    }
    println!(
        "\n  the sounding multiset is unchanged: {}",
        opening.forget_voice_labels() == exchanged.forget_voice_labels()
    );

    // 1. The declared voice leading: every voice keeps its own name, and two
    //    of them really do move a fifth.
    let declared = VoiceLeading::new(
        opening.voice_set(),
        exchanged.voice_set(),
        ["bass", "tenor", "soprano"].map(|name| Edge::new(VoiceId::new(name), VoiceId::new(name))),
    )?;

    let model = SpanCostModel::new(ground.clone(), 1.0, SpanPenalties::uniform(1.0))?;
    let cost = model.declared_cost(&declared, &opening, &exchanged)?;
    println!("\n== 1. Cost of the declared voice leading (section 4.4.2) ==");
    println!("  span      {declared}");
    println!("  question  {:?}", cost.question());
    println!("  movement  {:.4} octaves", cost.movement());
    println!("  total     {:.4}", cost.total());
    println!(
        "  shape     {} moves, {} continuations, no splits or merges",
        cost.shape().moves,
        cost.shape().continuations
    );

    // 2. The minimum over an admissible family: a different question, and
    //    here a strikingly different answer.
    let outcome = model.minimum_over_assignments(&opening, &exchanged)?;
    let best = outcome.solution().expect("a minimizer exists");
    let minimum = outcome.cost().expect("so does its cost");
    println!("\n== 2. Minimum over the admissible family (section 4.4.5) ==");
    println!("  span      {best}");
    println!("  question  {:?}", minimum.question());
    println!("  total     {:.4}", minimum.total());
    assert_eq!(
        minimum.question(),
        &CostQuestion::MinimumOverFamily(umt::pitch::AdmissibleFamily::PartialAssignment)
    );
    println!(
        "\n  the declared leading costs {:.4}; the minimum is {:.4}, because\n  \
         the optimizer is free to say nobody moved and the voices were merely\n  \
         renamed. Both numbers are correct. They are answers to different\n  \
         questions, and section 4.4.5 requires saying which one you have.",
        cost.total(),
        minimum.total()
    );

    // 3. Unequal voice counts: fixture F8.
    println!("\n== 3. Unequal voice counts (section 4.4.4, fixture F8) ==");
    let single = Chord::from_voices([(VoiceId::new("soprano"), note(0)?)])?;
    let doubled = Chord::from_voices([
        (VoiceId::new("soprano"), note(0)?),
        (VoiceId::new("alto"), note(0)?),
    ])?;
    println!(
        "  one C versus two doubled Cs: {} note versus {} notes, {} distinct pitch",
        single.forget_voice_labels().total_len(),
        doubled.forget_voice_labels().total_len(),
        doubled.forget_voice_labels().distinct_len()
    );

    let sensitive = ChordDistance::new(
        ground.clone(),
        2.0,
        TransportProfile::Balanced {
            mass: MassProfile::PerVoice,
        },
    )?;
    match sensitive.distance(&single, &doubled) {
        Ok(value) => println!("  balanced W_2:  {value}"),
        Err(error) => println!("  balanced W_2:  refused - {error}"),
    }

    let normalized = ChordDistance::new(
        ground.clone(),
        2.0,
        TransportProfile::Balanced {
            mass: MassProfile::NormalizedProbability,
        },
    )?;
    println!(
        "  normalized:    {:.4}  <- the documented loss, only ever opt-in",
        normalized.distance(&single, &doubled)?
    );

    let edit = ChordDistance::new(ground, 1.0, TransportProfile::Edit { boundary: 0.75 })?;
    println!(
        "  edit profile:  {:.4}  <- one birth at the configured cost",
        edit.distance(&single, &doubled)?
    );
    match edit.metric_claim() {
        MetricClaim::Metric { state_space } => println!("  claims a metric on {state_space}"),
        other => println!("  claim: {other:?}"),
    }

    // And the chords themselves stay distinguishable regardless of which
    // distance was used, which is the actual obligation F8 imposes.
    assert_ne!(single, doubled);
    println!("\n  the two chords remain different objects under every profile");

    Ok(())
}
