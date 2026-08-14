//! Example 1 of the implementation prompt: a 12-EDO temperament end to end.
//!
//! Builds a 5-limit prime basis, constructs the mapping, inspects its kernel
//! and image, selects a representative lift, and computes an exact comma
//! residue.
//!
//! Regular tuning is deliberately absent: it is an L3 map of intervals to real
//! sizes and belongs to a later stage. Nothing here needs it, which is the
//! point - the whole example is exact.
//!
//! Run with `cargo run --example temperament_12edo`.

use umt::Basis;
use umt::temperament::{
    AmbientLattice, HomomorphicSplit, LinearSplit, SplitPolicy, StructuralLens, TemperamentMap,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // L1: the exact proportion lattice over the 5-limit prime basis.
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    println!("basis rank {}", basis.rank());

    // L2: the mapping into the 12-step ambient lattice.
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;
    println!("mapping matrix {}", map.matrix());
    println!("rank {}, surjective {}", map.rank(), map.is_surjective());

    // The kernel: which commas vanish.
    let kernel = map.kernel();
    println!("kernel rank {}", kernel.rank());
    for comma in kernel.comma_basis_monzos() {
        println!(
            "  comma {comma} = {} ({:.4} cents)",
            comma.exact_ratio()?,
            comma.log2_valuation_f64()? * 1200.0
        );
    }
    println!("kernel saturated: {}", kernel.is_saturated());
    // The canonical basis is the same subgroup, presented for identity rather
    // than for reading.
    println!("canonical basis of the same subgroup: {}", kernel.basis());
    println!(
        "syntonic comma tempered out: {}",
        kernel.contains(&basis.monzo([-4, 4, -1])?)?
    );

    // The image: everything the mapping reaches.
    println!(
        "image rank {}, basis {}, is all of Gamma: {}",
        map.image().rank(),
        map.image().basis(),
        map.image().is_full()
    );

    // Select a representative: one exact lift per tempered class.
    let split = LinearSplit::of(&map)?;
    let fifth = basis.monzo([-1, 1, 0])?;
    let class = map.apply_to_image(&fifth)?;
    let canonical = split.split(&class)?;
    println!(
        "\nthe just fifth {} sounds as {} steps",
        fifth.exact_ratio()?,
        map.apply(&fifth)?.coordinates()[0]
    );
    println!(
        "  canonical preimage of that class: {} = {}",
        map.preimage(&map.apply(&fifth)?)?,
        map.preimage(&map.apply(&fifth)?)?.exact_ratio()?
    );
    println!("  linear splitting of that class:   {canonical}");
    println!("  both map back to 7 steps, as right inverses must, but a linear");
    println!("  section must send class 7n to n times the lift of class 7, so it");
    println!("  cannot also keep every lift simple. That is the whole reason");
    println!("  UMT separates homomorphic splittings from representative policies.");

    // The residue: exactly how the written interval differs from the lift.
    let lens = StructuralLens::new(SplitPolicy::new(split));
    let wide = fifth.checked_add(&basis.monzo([-4, 4, -1])?)?;
    for spelling in [&fifth, &wide] {
        let residue = lens.residue(spelling, &())?;
        let comma = map.kernel().embed(&residue)?;
        println!(
            "  {spelling} = {:>8}  class {:>3}  residue {comma}",
            spelling.exact_ratio()?,
            lens.get(spelling)?.coordinates()[0],
        );
    }
    println!("\nboth spellings share a class; their residues do not.");

    Ok(())
}
