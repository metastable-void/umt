//! Example 2 of the implementation prompt, and UMT-3.2 fixture F4: the image
//! of a patent val is not the ambient step lattice.
//!
//! The 5-limit patent val for 6-EDO is `[6, 10, 14]`. Its ambient lattice is
//! `Gamma = Z`, one element per EDO step, but its image is `H = 2Z`. An odd
//! ambient step exists and is perfectly meaningful as a step; it is simply not
//! reached, so it has no automatic detempering.
//!
//! Run with `cargo run --example temperament_6edo_image`.

use umt::temperament::{HomomorphicSplit, LinearSplit};
use umt::{Basis, PatentVal, RoundingConvention, Z};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    let val = PatentVal::new(&basis, 6, RoundingConvention::NearestHalfAwayFromZero)?;

    println!("patent val {val} over {}", basis.id());
    println!(
        "entries computed {:?}, no floating point involved",
        val.exactness()
    );
    println!("ambient Gamma = Z, one coordinate per step");
    println!(
        "image H = {}Z, rank {}, surjective {}",
        val.image_generator(),
        val.image_rank(),
        val.is_surjective()
    );

    println!("\nstep  in image?  image coordinate");
    for step in -2i64..=6 {
        let step = Z::from(step);
        match val.image_coordinate(&step) {
            Ok(coordinate) => println!("{step:>4}  yes        {coordinate}"),
            Err(error) => println!("{step:>4}  no         {error}"),
        }
    }

    // Detempering is defined on H. An odd step never becomes a class, so no
    // representative policy is ever consulted for it.
    let map = val.map();
    let split = LinearSplit::of(map)?;
    let class = map.image().from_ambient(&map.ambient().element([4i64])?)?;
    let lift = split.split(&class)?;
    println!(
        "\nstep 4 lifts to {lift} = {}, which maps back to {} steps",
        lift.exact_ratio()?,
        map.apply(&lift)?.coordinates()[0]
    );

    let odd = map.ambient().element([3i64])?;
    match map.image().from_ambient(&odd) {
        Ok(_) => println!("step 3 unexpectedly became a class"),
        Err(error) => println!("step 3 cannot become a class at all: {error}"),
    }

    Ok(())
}
