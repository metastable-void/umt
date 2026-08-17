//! Example 5 of the implementation prompt: an unmeasured event, held by
//! temporal constraints rather than by a fabricated onset.
//!
//! UMT-3.2 section 5.10.5 is direct about this: "a rest, fermata, breath,
//! free-time span, or culturally specific timing instruction MUST NOT be
//! represented merely as a rounding residual unless that is actually the
//! intended semantics". Unmeasured instructions are positive temporal data.
//!
//! This example shows three profiles on one passage, and what each can and
//! cannot decide.
//!
//! Run with `cargo run --example unmeasured_event`.

use std::collections::BTreeMap;

use umt::time::{
    DifferenceConstraint, ExternalPredicate, HybridTemporalProblem, LinearConstraint,
    LinearTemporalProblem, PositivityHandling, PredicateEvaluator, RatioConstraint, StpProblem,
    TemporalOutcome,
};
use umt::{Q, Z};

fn q(value: i64) -> Q {
    Q::from(Z::from(value))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Bounded but unmeasured: the STP profile.
    //
    // "Hold the fermata between two and five seconds, then enter within one
    // second." No grid, no fabricated onset, and the implied bounds tighten.
    println!("== 1. Difference bounds: the STP profile (section 5.10.1) ==");
    let mut stp = StpProblem::new();
    let fermata = stp.variable("fermata-start");
    let release = stp.variable("fermata-release");
    let entry = stp.variable("entry");

    stp.constrain(DifferenceConstraint::between(
        &fermata,
        &release,
        Some(q(2)),
        Some(q(5)),
    ))?;
    stp.constrain(DifferenceConstraint::between(
        &release,
        &entry,
        Some(q(0)),
        Some(q(1)),
    ))?;

    match stp.solve() {
        TemporalOutcome::Solved {
            assignment,
            tight_bounds,
            exactness,
            ..
        } => {
            println!("  solved, {exactness:?}");
            println!(
                "  one consistent assignment - a difference network fixes\n  \
                 distances, not positions, so this may sit anywhere on the line:"
            );
            for (variable, value) in &assignment {
                println!("    {variable:>16} = {value}");
            }
            let implied = tight_bounds
                .iter()
                .find(|bound| bound.from == fermata && bound.to == entry)
                .expect("an implied bound between the outer events");
            println!(
                "  implied fermata-start to entry: [{}, {}]",
                implied.lower.as_ref().unwrap(),
                implied.upper.as_ref().unwrap()
            );
        }
        other => println!("  {other:?}"),
    }

    // Contradict it and the solver says so rather than picking a side.
    let mut contradictory = StpProblem::new();
    let a = contradictory.variable("a");
    let b = contradictory.variable("b");
    contradictory.constrain(DifferenceConstraint::at_most(&a, &b, q(1)))?;
    contradictory.constrain(DifferenceConstraint::at_least(&a, &b, q(2)))?;
    println!(
        "  a contradictory network: consistent = {}",
        contradictory.solve().is_consistent()
    );

    // 2. A proportional instruction: the linear-ratio profile.
    //
    // "The second silence is between one and two times the first." Three
    // events, non-unit coefficients: not a difference edge, and not solvable
    // by shortest paths.
    println!("\n== 2. A ratio constraint: the linear profile (section 5.10.2) ==");
    let mut linear = LinearTemporalProblem::new();
    let first = linear.variable("first");
    let second = linear.variable("second");
    let third = linear.variable("third");

    linear.constrain_ratio(&RatioConstraint {
        earlier: first.clone(),
        middle: second.clone(),
        later: third.clone(),
        lower: q(1),
        upper: q(2),
    })?;
    println!(
        "  cross-multiplied into {} linear constraints, {} of them strict",
        linear.constraints().len(),
        linear
            .constraints()
            .iter()
            .filter(|constraint| constraint.strict)
            .count()
    );
    println!(
        "  positivity handling: {:?}",
        match linear.positivity() {
            PositivityHandling::StrictInequality => "strict inequality, no delta invented",
            PositivityHandling::JustifiedDelta { .. } => "a justified delta",
            other => panic!("unhandled positivity declaration {other:?}"),
        }
    );

    // Pin the first gap and read off the second.
    linear.constrain(LinearConstraint::at_most([(first.clone(), q(1))], q(0)))?;
    linear.constrain(LinearConstraint::at_most([(first, q(-1))], q(0)))?;
    linear.constrain(LinearConstraint::at_most([(second.clone(), q(1))], q(3)))?;
    linear.constrain(LinearConstraint::at_most([(second.clone(), q(-1))], q(-3)))?;

    let outcome = linear.solve()?;
    if let Some(assignment) = outcome.assignment() {
        let gap_one = &assignment[&second] - q(0);
        let gap_two = &assignment[&third] - &assignment[&second];
        println!("  first gap  {gap_one}");
        println!("  second gap {gap_two}");
        println!(
            "  ratio      {} (within the declared [1, 2])",
            &gap_two / &gap_one
        );
    }

    // 3. A cue that no amount of arithmetic decides: the external-predicate
    //    profile.
    println!("\n== 3. An external predicate (section 5.10.3) ==");
    let mut hybrid = HybridTemporalProblem::new(LinearTemporalProblem::new());
    let cue = hybrid.linear_mut().variable("entry");
    hybrid
        .linear_mut()
        .constrain(LinearConstraint::at_most([(cue, q(1))], q(30)))?;
    hybrid.add_predicate(ExternalPredicate {
        id: String::from("after-decay"),
        predicate_type: String::from("umt:predicate:acoustic-decay-threshold"),
        contract: String::from(
            "true once the measured level of the referenced sound falls below the stated \
             threshold, as reported by a configured detector",
        ),
        parameters: BTreeMap::from([
            (String::from("threshold_db"), String::from("-40")),
            (String::from("source"), String::from("voice:1")),
        ]),
    });

    let outcome = hybrid.solve(None)?;
    println!(
        "  claims static decidability: {}",
        hybrid.claims_static_decidability()
    );
    println!("  outstanding predicates:     {:?}", outcome.unresolved());
    println!("  static part consistent:     {}", outcome.is_consistent());

    // Configure a detector and it resolves. The predicate itself never
    // travelled as code: it is a type name, a contract, and parameters.
    struct DecayDetector;
    impl PredicateEvaluator for DecayDetector {
        fn evaluate(&self, predicate: &ExternalPredicate) -> Option<bool> {
            (predicate.predicate_type == "umt:predicate:acoustic-decay-threshold").then_some(true)
        }
    }
    let resolved = hybrid.solve(Some(&DecayDetector))?;
    println!(
        "  with a detector configured: outstanding {:?}, solved = {}",
        resolved.unresolved(),
        matches!(resolved, TemporalOutcome::Solved { .. })
    );

    println!(
        "\n  Nothing above invented an onset. Each profile reports what it can\n  \
         decide, and section 5.10.6 declines to reduce the remaining freedom\n  \
         to one scalar."
    );

    Ok(())
}
