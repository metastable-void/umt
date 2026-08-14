//! Example 3 of the implementation prompt: a context-dependent representative
//! policy chooses different exact lifts for the same tempered class.
//!
//! This is the shape of adaptive just intonation (UMT-3.2 section 4.8). The
//! sounding class is fixed by the temperament; which exact interval realizes
//! it is a policy decision that depends on context, and the difference from
//! the canonical lift is reported as an exact comma rather than as a
//! floating-point deviation.
//!
//! Run with `cargo run --example adaptive_lift`.

use umt::Basis;
use umt::temperament::{
    AmbientLattice, CanonicalLiftPolicy, ImageElem, KernelElem, OffsetPolicy, RepresentativePolicy,
    TemperamentMap,
};

/// Where in a phrase the interval is being realized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Harmony {
    /// Sounding against the tonic: prefer the plain lift.
    Tonic,
    /// Sounding against the subdominant: prefer the comma-shifted lift, so
    /// the vertical interval stays just.
    Subdominant,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let basis = Basis::primes("umt:prime:2.3.5", &[2, 3, 5])?;
    let steps = AmbientLattice::new("umt:edo:12", 1);
    let map = TemperamentMap::from_rows(&basis, &steps, [[12i64, 19, 28]])?;

    // The policy: a base lift, adjusted by an exact comma in one context.
    // Because the adjustment comes from the kernel, the right-inverse law
    // survives in every context; additivity does not, and is not claimed.
    let syntonic = basis.monzo([-4, 4, -1])?;
    let offset = map
        .kernel()
        .coordinates(&syntonic)?
        .expect("the syntonic comma is tempered out by 12-EDO");
    let policy = OffsetPolicy::new(
        CanonicalLiftPolicy::new(map.clone()),
        move |_class: &ImageElem, harmony: &Harmony| -> Option<KernelElem> {
            match harmony {
                Harmony::Tonic => None,
                Harmony::Subdominant => Some(offset.clone()),
            }
        },
    );

    println!(
        "policy claims homomorphism: {}",
        policy.claims_homomorphic()
    );
    println!("(it must not: an adaptive policy is not additive)\n");

    // One structural class, realized differently in two contexts.
    let written = basis.monzo([-1, 1, 0])?;
    let class = map.apply_to_image(&written)?;
    println!(
        "written interval {written} = {}, sounding class {} steps\n",
        written.exact_ratio()?,
        map.apply(&written)?.coordinates()[0]
    );

    for harmony in [Harmony::Tonic, Harmony::Subdominant] {
        let decision = policy.choose(&class, &harmony)?;
        let residue = map.kernel().embed(&decision.residue)?;
        println!("context {harmony:?}");
        println!(
            "  lift    {} = {}",
            decision.lift,
            decision.lift.exact_ratio()?
        );
        println!(
            "  size    {:.4} cents",
            decision.lift.log2_valuation_f64()? * 1200.0
        );
        println!("  residue {residue} relative to the canonical lift");

        // The right-inverse law holds regardless of context.
        assert_eq!(map.apply_to_image(&decision.lift)?, class);
    }

    println!("\nsame class, different exact lifts, exact comma between them.");
    Ok(())
}
