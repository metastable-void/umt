# Conformance matrix

UMT-3.2 section 9.13 fixtures mapped to tests (prompt section 46).

**This crate does not claim UMT-3.2 conformance.** Conformance is claimed only
when the applicable mandatory fixture suite passes in full. Fixtures whose
machinery does not exist yet are marked `pending` and have no test, rather than
a stub that could be mistaken for a pass.

Status values:

- `pass` - the fixture's obligations are all asserted;
- `partial` - the implemented layers decide part of it; remaining obligations
  are listed;
- `pending` - not implemented.

| Fixture | Subject | Status | Test |
|---|---|---|---|
| F1 | Kernel saturation versus surjectivity, `V = [2]` | pass | `tests/conformance.rs::f01_kernel_saturation_versus_surjectivity` |
| F2 | Unsaturated direct comma subgroup | pass | `tests/conformance.rs::f02_unsaturated_direct_comma_subgroup` |
| F3 | 12-EDO quotient | partial | `tests/conformance.rs::f03_twelve_edo_quotient_partial` |
| F4 | 6-EDO image is `2Z` | pass | `tests/conformance.rs::f04_6edo_image` |
| F5 | Height with a generator below 1 | pending | needs complexity profiles |
| F6 | Nonhomomorphic representative policy | pass | `tests/conformance.rs::f06_nonhomomorphic_representative_policy` |
| F7 | Enharmonic spelling | pass | `tests/conformance.rs::f07_enharmonic_spelling` |
| F8 | Unequal voice count | pending | needs chords and voice leading |
| F9 | Tie round trip | pending | needs the score layer |
| F10 | 6/8 versus 3/4 | pending | needs meter |
| F11 | Additive 2+2+3 | pending | needs rhythm trees |
| F12 | Naive quintuplet floor gives 95 | pending | needs quantization |
| F13 | Endpoint-preserving quintuplet gives 96 | pending | needs quantization |
| F14 | Nested tuplet re-realization at two PPQN | pending | needs quantization |
| F15 | Strictly increasing but discontinuous tempo map | pending | needs tempo maps |
| F16 | STP contradiction | pending | needs the temporal solver |
| F17 | Ratio constraint is not a difference edge | pending | needs the temporal solver |
| F18 | External temporal predicate | pending | needs temporal constraints |
| F19 | Inharmonic empirical scale | pending | needs the L3 scale object |
| F20 | Continuous pitch trajectory | pending | needs trajectories |
| F21 | Scala mixed exact and metric entries | pending | needs the `.scl` adapter |
| F22 | Reachable versus ambient octave classes | partial | `tests/conformance.rs::f22_reachable_versus_ambient_classes_partial` |
| F23 | Unmeasured event without fixed onset | pending | needs the score layer |
| F24 | Global control event without a voice | pending | needs the score layer |
| F25 | Strict ratio positivity, no invented delta | pending | needs the temporal solver |
| F26 | Unattained optimization infimum | pending | needs optimization outcomes |
| F27 | Infeasible positive-span device allocation | pending | needs quantization |
| F28 | Context-dependent realization typing | pending | needs realization traits |
| F29 | Direct empirical object without a unit | pending | needs the native container |
| F30 | Markdown math-source and vocabulary lint | pending | source lint, not a library test |
| F31 | Regular interval tuning requires a point reference | pending | needs tuning and references |
| F32 | Reciprocal rate and duration orientation | pending | needs the rate-continuum interface |
| F33 | Saturation excludes the zero multiplier | pass | `tests/conformance.rs::f33_saturation_excludes_the_zero_multiplier` |
| F34 | Group length versus lattice norm | pending | needs complexity profiles |
| F35 | Three-note quarter-comma-meantone MOS | pending | needs generated sets |

## Outstanding obligations for the partial fixtures

**F3.** The kernel, its rank, and the image are now all constructed and
asserted, so the reachable quotient is pinned down as free of rank 1 by the
first isomorphism theorem. What remains is unit equivalence as a constructed
quotient object: `Z/12Z` is currently asserted as an index computation
(UMT-3.2 section 1.9).

**F22.** The class counts are derived from the represented image and octave
image rather than asserted as constants, but the quotient groups themselves
are not constructed. Same dependency as F3.

## Law coverage

UMT-3.2 section 9.1 and prompt section 47 laws currently exercised, in
`tests/properties.rs`:

| Law | Test |
|---|---|
| P1 free-lattice arithmetic | `p1_addition_is_associative`, `p1_zero_and_inverse` |
| P2 exact rational valuation | `p2_valuation_is_multiplicative` |
| P3 mapping homomorphism | `p3_mapping_is_a_homomorphism` |
| P4 kernel correctness | `p4_kernel_membership_iff_mapped_to_zero` |
| P5 map-derived kernel saturation | `p5_kernel_saturation_for_nonzero_multiples` |
| P6 direct-comma validation | `tests/conformance.rs::f02_unsaturated_direct_comma_subgroup` |
| P7 image distinction | `p7_image_membership_and_round_trip`, `p7_general_image_round_trip` |
| P8 right-inverse law | `p8_right_inverse_law` |
| P9 residue law | `p9_residue_is_in_the_kernel` |
| P10 set-level lens laws | `p10_lens_laws` |
| P11 linear-splitting declaration | `p11_homomorphism_only_when_claimed` |
| Basis-mismatch rejection | `basis_mismatch_is_always_rejected` |
| Normal-form invariants (prompt section 10) | `smith_normal_form_invariants`, `hermite_normal_form_is_canonical`, `sublattice_coordinates_round_trip`, plus the unit tests in `src/algebra/normal_form.rs` |
| Section 1.6 entry definition | `nearest_entry_satisfies_its_defining_inequality`, `floor_entry_satisfies_its_defining_inequality`, `conventions_are_ordered`, `octave_entry_is_fixed_to_n` |

P8 to P11 run against four mapping shapes - surjective, non-surjective, the
zero map, and rank 2 - so the degenerate cases are covered rather than assumed
away.

The complexity, tuning, torsor, voice-leading, notation, rhythm-tree,
quantization, tempo-map, and temporal-constraint laws are not yet applicable:
the structures they constrain do not exist.

## Examples

Prompt section 49 requires executable examples. Three of the six are possible
at this stage:

| Example | Subject | File |
|---|---|---|
| 1 | 12-EDO temperament end to end | `examples/temperament_12edo.rs` |
| 2 | 6-EDO image distinction | `examples/temperament_6edo_image.rs` |
| 3 | Adaptive lift selection | `examples/adaptive_lift.rs` |
| 4 | Quintuplet quantization | pending, needs the time layer |
| 5 | Unmeasured event | pending, needs temporal constraints |
| 6 | Performance compilation | pending, needs the device layer |

Example 1 stops short of the regular tuning its prompt text mentions, because
tuning is an L3 map and belongs to a later stage; everything else in it is
exact.
