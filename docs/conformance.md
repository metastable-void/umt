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
| F3 | 12-EDO quotient | pass | `tests/conformance.rs::f03_twelve_edo_quotient` |
| F4 | 6-EDO image is `2Z` | pass | `tests/conformance.rs::f04_6edo_image` |
| F5 | Height with a generator below 1 | pass | `tests/conformance.rs::f05_height_with_a_generator_below_one` |
| F6 | Nonhomomorphic representative policy | pass | `tests/conformance.rs::f06_nonhomomorphic_representative_policy` |
| F7 | Enharmonic spelling | pass | `tests/conformance.rs::f07_enharmonic_spelling` |
| F8 | Unequal voice count | pass | `tests/conformance.rs::f08_unequal_voice_count` |
| F9 | Tie round trip | pending | needs the score layer |
| F10 | 6/8 versus 3/4 | pass | `tests/conformance.rs::f10_six_eight_versus_three_four` |
| F11 | Additive 2+2+3 | pass | `tests/conformance.rs::f11_additive_two_two_three` |
| F12 | Naive quintuplet floor gives 95 | pass | `tests/conformance.rs::f12_naive_quintuplet_floor` |
| F13 | Endpoint-preserving quintuplet gives 96 | pass | `tests/conformance.rs::f13_endpoint_preserving_quintuplet` |
| F14 | Nested tuplet re-realization at two PPQN | pass | `tests/conformance.rs::f14_nested_tuplet_re_realization` |
| F15 | Strictly increasing but discontinuous tempo map | pass | `tests/conformance.rs::f15_strictly_increasing_but_discontinuous` |
| F16 | STP contradiction | pass | `tests/conformance.rs::f16_stp_contradiction` |
| F17 | Ratio constraint is not a difference edge | pass | `tests/conformance.rs::f17_ratio_constraint_is_not_a_difference_edge` |
| F18 | External temporal predicate | pass | `tests/conformance.rs::f18_external_temporal_predicate` |
| F19 | Inharmonic empirical scale | pending | needs the L3 scale object |
| F20 | Continuous pitch trajectory | pass | `tests/conformance.rs::f20_continuous_pitch` |
| F21 | Scala mixed exact and metric entries | pending | needs the `.scl` adapter |
| F22 | Reachable versus ambient octave classes | pass | `tests/conformance.rs::f22_reachable_versus_ambient_classes` |
| F23 | Unmeasured event without fixed onset | pending | needs the score layer |
| F24 | Global control event without a voice | pending | needs the score layer |
| F25 | Strict ratio positivity, no invented delta | pass | `tests/conformance.rs::f25_strict_ratio_positivity` |
| F26 | Unattained optimization infimum | pass | `tests/conformance.rs::f26_unattained_optimization_infimum` |
| F27 | Infeasible positive-span device allocation | pass | `tests/conformance.rs::f27_infeasible_positive_span_allocation` |
| F28 | Context-dependent realization typing | pass | `tests/conformance.rs::f28_context_dependent_realization_typing` |
| F29 | Direct empirical object without a unit | pending | needs the native container |
| F30 | Markdown math-source and vocabulary lint | pending | source lint, not a library test |
| F31 | Regular interval tuning requires a point reference | pass | `tests/conformance.rs::f31_regular_tuning_requires_a_point_reference` |
| F32 | Reciprocal rate and duration orientation | pass | `tests/conformance.rs::f32_reciprocal_rate_and_duration_orientation` |
| F33 | Saturation excludes the zero multiplier | pass | `tests/conformance.rs::f33_saturation_excludes_the_zero_multiplier` |
| F34 | Group length versus lattice norm | pass | `tests/conformance.rs::f34_group_length_versus_lattice_norm` |
| F35 | Three-note quarter-comma-meantone MOS | pending | needs generated sets |

Twenty-seven of the thirty-five fixtures pass; none is partial. Every remaining
one depends on a layer that does not exist yet - the score layer, the device
layer, the native container, an external adapter, or generated sets - except
F30, which is a lint over the specification source rather than a library test.

Two of them are worth naming, because both are easy to claim loosely.

F20's obligations are discharged in full: the trajectory survives a native
round trip through `PitchTrajectoryRef` *exactly*, not to within a tolerance,
and the device export records an approximation whose bound is derived from the
deviation's Lipschitz constant, then checked against the reconstruction at a
thousand intermediate times rather than asserted.

F25 is discharged by construction rather than by care. The linear profile
solves strict inequalities directly, by exact Fourier-Motzkin elimination over
the rationals, so there is no point at which a `delta` would be convenient. One
can still be declared - but `PositivityHandling::JustifiedDelta` has a
mandatory `justification` field, so a `delta` with no stated justification is
not a value this crate can represent.

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
| Section 9.4 point-space laws | `point_space_laws`, `realized_point_space_laws` |
| Law T1 regular tuning homomorphism | `t1_regular_tuning_is_a_homomorphism` |
| Law T2 comma error | `t2_comma_error_is_minus_the_just_size` |
| Section 4.3 chord views and voice sets | `chord_views_lose_exactly_what_they_say_they_lose`, `voice_sets_form_a_partial_commutative_monoid` |
| Section 4.4.1 span composition by pullback | `voice_leading_composition_is_associative` |
| Section 4.4.2 declared-span cost | `declared_span_cost_is_the_sum_of_its_terms` |
| Section 4.4.5 declared versus minimum | `the_family_minimum_is_no_worse_than_any_member` |
| Section 9.5 voice-leading metric laws | `the_edit_profile_obeys_the_metric_laws`, plus the unit tests in `src/pitch/voice_leading.rs` |
| Section 9.7 rhythm-tree laws | `rhythm_tree_flattening_partitions_the_parent`; law 1 is a construction condition, law 5 is `tests/serialization.rs::time_layer_objects_round_trip_and_revalidate` |
| Section 9.8 floor and ceiling profiles | `floor_and_ceiling_are_order_adjunctions` |
| Section 9.8 nearest profile | `nearest_quantization_is_bounded_but_not_one_sided` |
| Section 9.8 endpoint-preserving profile | `endpoint_preserving_allocation_sums_to_the_parent` |
| Section 9.9 tempo-map laws | `a_tempo_map_is_strictly_increasing_and_invertible` |
| Section 9.10 STP profile | `a_consistent_stp_assignment_satisfies_every_constraint` |
| Section 9.10 linear-ratio profile | `a_feasible_linear_system_yields_a_satisfying_assignment` |
| Section 2.1 rate and duration orientation | `an_oriented_ratio_inverts_across_the_reciprocal` |

P8 to P11 run against four mapping shapes - surjective, non-surjective, the
zero map, and rank 2 - so the degenerate cases are covered rather than assumed
away. The point-space laws run on both the exact structural torsor and the L3
log-frequency torsor.

The complexity laws of section 9.2 are exercised in
`src/proportion/complexity.rs`: group-length laws, integer homogeneity where
it is claimed, the seminorm null subgroup, and the Tenney-height identity
`h_T(m) = log2(n d)` against an independently computed right-hand side.

Law T3 - that a context-dependent realization is not advertised as a regular
homomorphism - is structural here: `PitchRealizer::is_regular` defaults to
`false`, and fixture F28 checks both answers.

Section 9.5 deserves a note, since it is the one law group that constrains what
an implementation is allowed to *say* rather than what it must compute.
`ChordDistance::metric_claim` returns a value naming the state space each
profile claims its laws on, and the claims are tested there rather than
inherited:

- balanced per-voice transport is a metric on multisets of one fixed
  cardinality, and only a *pseudometric* on labelled chords, since relabelling
  a chord does not move it - the claim says so, and a unit test demonstrates a
  relabelled pair at distance zero;
- the assignment/edit profile is a metric across cardinalities under the
  truncated ground cost, tested over three of them;
- a zero boundary cost withdraws the claim entirely, and the test shows the
  failure it would otherwise hide.

The declared-span cost of section 4.4.2 is never called a chord metric, which
section 9.5 forbids without proof.

Section 9.10 also deserves a note, for the same reason. The STP profile is the
only one given the unconditional shortest-path consistency claim, and it is the
only one whose solver runs Floyd-Warshall. A ratio constraint cannot reach that
solver: `StpProblem::constrain` accepts a `DifferenceConstraint` and nothing
else, and `RatioConstraint` cross-multiplies into `LinearConstraint`, which
`StpProblem` has no method for. The separation is in the types, not in a
runtime check that could be bypassed.

The notation and generated-structure laws are not yet applicable: the
structures they constrain do not exist.

## Examples

Prompt section 49 requires executable examples. Five of the six are possible at
this stage:

| Example | Subject | File |
|---|---|---|
| 1 | 12-EDO temperament end to end | `examples/temperament_12edo.rs` |
| 2 | 6-EDO image distinction | `examples/temperament_6edo_image.rs` |
| 3 | Adaptive lift selection | `examples/adaptive_lift.rs` |
| 4 | Quintuplet quantization | `examples/quintuplet_quantization.rs` |
| 5 | Unmeasured event | `examples/unmeasured_event.rs` |
| 6 | Performance compilation | pending, needs the device layer |

Example 1 stops short of the regular tuning its prompt text mentions, because
tuning is an L3 map and belongs to a later stage; everything else in it is
exact.

One supplementary example beyond the prompt's list:

| Example | Subject | File |
|---|---|---|
| - | Voice leading: declared cost, family minimum, unequal counts | `examples/voice_leading.rs` |

It takes a voice exchange, where the declared leading moves two voices by a
fifth each and the minimum over relabellings is zero. Both numbers are
correct, and section 4.4.5 is about never confusing them.
