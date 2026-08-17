//! Example 4 of the implementation prompt: quantizing a quintuplet to a
//! device grid, three ways, with the residuals reported.
//!
//! UMT-3.2 section 5.7.4 uses this exact case to make a point: five equal
//! children of a 96-tick parent are 19.2 ticks each, so independently flooring
//! them loses a tick and misses the parent endpoint. That is a property of the
//! *method*, not of the grid, and section 5.7.5 gives the method that does not
//! lose it.
//!
//! Run with `cargo run --example quintuplet_quantization`.

use umt::algebra::RoundingConvention;
use umt::time::{AllocationPolicy, BeatSpan, BeatTime, RhythmTree, TickGrid};
use umt::{Q, Z};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The source: five equal children in one beat. Exact, and the source of
    // truth for every realization below.
    let quintuplet = RhythmTree::equal_division(5)?;
    let beat = BeatSpan::new(BeatTime::zero(), BeatTime::ratio(1, 1)?)?;

    println!("== The exact source ==");
    for (index, leaf) in quintuplet.flatten(&beat)?.iter().enumerate() {
        println!(
            "  child {index}: [{}, {}] beats, duration {}",
            leaf.span().start().get(),
            leaf.span().end().get(),
            leaf.span().duration().get()
        );
    }

    let grid = TickGrid::new(96)?;
    let weights = vec![Q::from(Z::from(1)); 5];
    println!("\n  grid: {grid}, so one child is 96/5 = 19.2 ticks");

    // 1. Independent local flooring: drifts (fixture F12).
    let naive = grid.allocate_locally(&weights, &Z::from(96), RoundingConvention::Floor)?;
    println!("\n== 1. Independent local flooring (section 5.7.4) ==");
    println!(
        "  children  {:?}",
        naive
            .child_ticks()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );
    println!(
        "  total     {} of {}",
        naive.total_ticks(),
        naive.parent_ticks()
    );
    println!(
        "  endpoint  {}",
        if naive.endpoint_preserved() {
            "preserved"
        } else {
            "LOST"
        }
    );
    println!("  drift     {} tick(s)", naive.endpoint_drift());

    // 2. Boundary rounding: does not drift (fixture F13).
    let exact = grid
        .allocate_preserving_endpoint(&weights, &Z::from(96), &AllocationPolicy::default())?
        .into_allocation()
        .expect("no minimum span was declared, so this always fits");
    println!("\n== 2. Endpoint-preserving allocation (section 5.7.5) ==");
    println!(
        "  children  {:?}",
        exact
            .child_ticks()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );
    println!(
        "  total     {} of {}",
        exact.total_ticks(),
        exact.parent_ticks()
    );
    println!(
        "  endpoint  {}",
        if exact.endpoint_preserved() {
            "preserved"
        } else {
            "LOST"
        }
    );
    println!("  residuals, in beats, relative to the exact child duration:");
    for (index, child) in exact.children().iter().enumerate() {
        println!(
            "    child {index}: {} ticks, residual {}",
            child.ticks(),
            child.residual().get()
        );
    }

    // 3. The same source at a finer resolution, from the tree (fixture F14).
    println!("\n== 3. Re-realized from the source tree, not from the ticks ==");
    let nested = RhythmTree::division([
        RhythmTree::equal_division(5)?,
        RhythmTree::leaf(1)?,
        RhythmTree::leaf(1)?,
    ])?;
    for resolution in [96u32, 960] {
        let node = TickGrid::new(resolution)?.quantize_tree(
            &nested,
            &beat,
            &AllocationPolicy::default(),
        )?;
        let leaves = node.leaf_ticks();
        println!(
            "  P = {resolution:>3}: first quintuplet note spans ticks {}..{}, \
             seven leaves totalling {} ticks",
            leaves[0].0,
            leaves[0].1,
            node.tick_count()
        );
    }
    println!(
        "\n  6 ticks at P=96 is not 64 ticks at P=960 scaled by ten: the finer\n  \
         realization comes from the exact tree, which is why section 5.7.6\n  \
         requires keeping it."
    );

    // 4. A constraint that cannot be met is reported (fixture F27).
    println!("\n== 4. An infeasible allocation is reported, not fudged ==");
    let cramped = AllocationPolicy::default().with_minimum_ticks(1);
    let outcome =
        grid.allocate_preserving_endpoint(&vec![Q::from(Z::from(1)); 3], &Z::from(2), &cramped)?;
    match outcome.allocation() {
        Some(allocation) => println!("  allocated {:?}", allocation.child_ticks()),
        None => println!("  three children at one tick each do not fit in two ticks"),
    }

    Ok(())
}
