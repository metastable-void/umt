# Specification issues

Findings about UMT-3.2 itself, recorded rather than silently coded around
(prompt sections 1.5 and 62). Each entry gives the section, the problem, a
counterexample or argument, a proposed correction, and the behaviour
implemented in the meantime.

Status legend: **open** - not resolved upstream; **editorial** - does not
affect semantics.

---

## S1. Patent-val construction routes an L2 structural object through floating point

**Status:** open. **Sections:** 0.6.1, 1.6, 9.13 fixture F4.

### Problem

Section 0.6.1 is unambiguous:

> L0--L2 MUST NOT require binary floating point for identity, equality,
> quotient membership, or conformance decisions.

Section 1.6 then defines the patent-val entry as

> `v_i = round(N log2 nu_3(beta_i))`

where `nu_3` is the L3 real valuation. The resulting mapping `V_N` is an L2
structural object: its kernel decides comma equivalence, and its image decides
quotient membership - exactly the decisions section 0.6.1 places off limits to
floating point. Fixture F4 makes this concrete, since whether an ambient step
is detemperable depends on `gcd(v_1, ..., v_k)`, which depends on every entry
being right.

A naive implementation therefore has a conformance defect that only shows up
at large `N`, where `N log2(p)` lands close enough to a half-integer that
double precision picks the wrong side. Nothing in the text warns the
implementer, and section 1.1.2's rule - that in the rational profile the
default real valuation is just the embedding of the exact ratio - is what makes
the exact computation possible, but it is stated two sections earlier and not
connected to 1.6.

### Argument

For a rational-profile generator the quantity being rounded is not
floating-point data. `nu_3(beta_i)` is the embedding of an exact rational
`p/q`, so `N log2(p/q)` is a definite real number determined by two integers,
and every rounding decision about it is an exact integer comparison:

- `floor(N log2(p/q)) = k` iff `2^k q^N <= p^N < 2^(k+1) q^N`;
- nearest rounding compares `p^(2N)` with `2^(2k+1) q^(2N)`.

Both are decidable in exact integer arithmetic. Furthermore an exact tie is
impossible: `p^(2N) = 2^(2k+1) q^(2N)` forces `q = 1` and `p = 2^j` with
`2Nj = 2k+1`, and `2Nj` is even. So in the rational profile the declared
tie-breaking convention never changes a nearest-rounded entry, which the text
does not say either.

### Proposed correction

In section 1.6, after the entry formula, add words to the effect of:

> When a generator has an exact rational valuation, the entry MUST be computed
> so that it equals the exact rounding of `N log2 nu(beta_i)`. Because that
> quantity is determined by two integers, the rounding decision is an exact
> integer comparison and MUST NOT depend on floating-point evaluation.
> Nearest-rounding ties cannot occur in the rational profile. When a generator
> has only a symbolic-real valuation, the entry is an L3-derived quantity and
> the mapping MUST record that its entries were decided from a real
> observation.

### Implemented behaviour

`algebra::integer::round_n_log2` decides rational-profile entries by exact
integer comparison; no floating point is involved. Symbolic-real generators use
`f64` and the resulting mapping reports `Exactness::RealValued`, so a caller
can tell whether the structural object satisfies section 0.6.1. This is
strictly more conservative than the literal text and agrees with it on every
input.

---

## S2. The title line labels the revision inconsistently

**Status:** editorial. **Section:** document title.

The first line reads

> UMT-3.2 -- Unified Music Theory, Third Design, Revision 2

while the document is revision 3.2 throughout and Appendix D is titled
"Principal Corrections from UMT-3.1 to UMT-3.2". "Revision 2" is presumably
"the second revision of the third design", but read alone it suggests UMT-2.

Suggested: "Third Design, Revision 3.2", or "Third Design, second revision".

No implementation impact. `UMT_SPEC_VERSION` in this crate is the string
`"UMT-3.2"`.

---

## Checked and found correct

Claims that looked like candidate errors during implementation and were
verified against the specification's own arithmetic:

- section 1.6.1, `[6, 10, 14]` and image `2Z` - correct;
- section 1.9, `H/6Z ≅ Z/3Z` versus `Gamma/6Z ≅ Z/6Z` for the 6-EDO fixture -
  correct;
- section 5.7.5, endpoint-preserving boundaries `0, 19, 38, 58, 77, 96` and
  children `19, 19, 20, 19, 19` - correct, and consistent with nearest-rounding
  the exact boundaries `19.2, 38.4, 57.6, 76.8`;
- fixture F11, additive `2+2+3` flattening to `0, 2, 4, 7` - correct;
- fixture F34, `g(m) = sqrt(h_1(m))` satisfying the separating group-length
  laws while failing integer homogeneity, with `g(4m) = 2g(m)` - correct;
- fixture F35, quarter-comma-meantone generator `300 log2 5`, three-note gaps
  approximately `193.157` and `503.422` cents - correct; the two large gaps are
  exactly equal, and the printed difference in the specification is rounding of
  the displayed values only;
- section 1.4.1, the exclusion of the zero multiplier from the saturation
  implication - correct and necessary, as fixture F33 tests.
