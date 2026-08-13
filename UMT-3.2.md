# UMT-3.2 -- Unified Music Theory, Third Design, Revision 2

**Exact proportion, structural approximation, temporal organization, and physical realization.**

Status: corrected draft specification, revision 3.2.

Date: 2026-08-13.

UMT-3.2 supersedes UMT-3.1, which superseded UMT-3 and UMT-2. It preserves the useful L0--L4 separation and the exact integer temperament core, while tightening type contracts, temporal semantics, serialization requirements, and GitHub Markdown compatibility. In particular, UMT-3.2 unifies **multiplicative proportion algebra** across pitch and rhythm; it does not claim that pitch and rhythm, as complete musical phenomena, are identical.

---

# Part 0 -- Charter

## 0.1 The claim being made

UMT-3.2 makes one central structural claim:

> **Pitch intervals and tempo/rhythmic proportions can instantiate the same algebra of multiplicative ratios. Their complete musical structures are different enrichments of that common core.**

For example, the proportion $3/2$ may describe a frequency interval, a tempo ratio, a metric modulation, or a subdivision relation. The same exponent-vector arithmetic can therefore be reused across those domains.

This does **not** imply any of the following:

- that a sum of acoustic signals is the same operation as concatenating durations;
- that pitch perception and rhythm perception have a single sharp boundary;
- that every pitch construction has a rhythmic counterpart;
- that every rhythmic construction has a pitch counterpart;
- that psychoacoustic consonance is identical to arithmetic ratio complexity;
- that temperament and time-grid rounding are the same kind of approximation.

UMT-3.2 requires every cross-domain identification to be justified by a typed common construction. Similarity of formulas alone is not enough.

## 0.2 Scope

**Core scope.**

- multiplicative proportion lattices;
- rational and symbolic-real bases;
- regular temperament mappings and comma groups;
- representative selection, spelling residue, and detempering;
- regular and non-regular tuning realization;
- generated scales and modular generated sets;
- pitch points, chords, voices, and voice leading;
- notation and spelling as information-bearing structures;
- continuous pitch trajectories;
- adaptive just-intonation realization;
- rhythmic trees, additive grouping, cyclic timelines, meter, grouping, tempo, rubato, and swing;
- measured and unmeasured temporal constraints;
- event-indexed score structure;
- realization and device quantization;
- interchange and explicit loss accounting.

**Deliberately outside the core.**

- engraving and page layout;
- large-scale musical form;
- style-specific harmonic grammar;
- automatic musical analysis;
- a complete model of timbre;
- a universal model of consonance or musical preference;
- a complete performance-practice model.

Timbre may parameterize a consonance or realization model without becoming a core UMT object.

## 0.3 Normative language

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative in this document.

A UMT-3.2 implementation may implement only a profile of the full specification, but it MUST declare which profiles it implements and MUST NOT silently claim conformance to unimplemented laws.

## 0.4 Adequacy suite

A formalism is assessed against executable tasks. The core conformance corpus MUST include at least the following.

| ID | Task | Primary sections |
|---|---|---|
| A1 | Detect a comma pump and report structural residue plus realized drift | 1.4--1.8, 4.8 |
| A2 | Preserve enharmonic spelling through a temperament that identifies sounding classes | 1.7, 4.5 |
| A3 | Round-trip notation and realization without losing spelling, ties, or nested tuplets when the target format can carry them | 5.2, 7, 8 |
| A4 | Express adaptive JI and rubato through one **optimization interface** while preserving their different mathematical types | 4.8, 5.8, 7.5 |
| A5 | Express transposition and reference change without treating pitch points as intervals | 1.10, 4.2 |
| A6 | Distinguish 6/8 from 3/4, represent 2+2+3 grouping, and represent a cyclic bell timeline | 5.3--5.5 |
| A7 | Represent unmeasured time as positive constraints rather than grid error | 5.10 |
| A8 | Represent an inharmonic or empirically tuned scale without forcing a small-integer JI explanation | 4.9 |
| A9 | Demonstrate both accumulating PPQN error under naive local rounding and endpoint-preserving quantization under a constrained policy | 5.7 |
| A10 | Preserve glissando and vibrato as trajectories rather than sampled nominal pitches | 4.7 |
| A11 | Distinguish the image of a patent val from the ambient EDO step lattice | 1.6 |
| A12 | Distinguish classical equal-mass transport from unequal-voice-count voice-leading costs | 4.4 |

## 0.5 Falsification conditions

The following findings would require changing the architecture rather than patching terminology.

1. If exact temperament cannot be represented independently of real-valued tuning, the L1/L2/L3 split is wrong.
2. If a score cannot preserve event incidence while independently projecting pitch-only and time-only views, the score model is wrong.
3. If unmeasured temporal instructions must be encoded as deviations from a metrical grid, the temporal model is wrong.
4. If notation-preserving round trip requires discarding L0 information before realization, the realization pipeline is wrong.
5. If the same API cannot host regular tuning, context-dependent tuning, and empirical tuning without conflating their types, the realization model is wrong.
6. If a purportedly exact construction depends on floating-point equality, it does not belong above L3.

## 0.6 Layer model

UMT-3.2 uses the following conceptual layers.

| Layer | Name | Typical content | Arithmetic |
|---|---|---|---|
| L0 | Notation | spelled symbols, noteheads, ties, tuplet brackets, grouping marks | symbolic, exact |
| L1 | Exact structure | monzos, exact ratios, rational durations, formal generators | integers/rationals, exact |
| L2 | Structural quotient / organization | tempered classes, image lattices, metric classes, exact meter/grouping structures | integers/rationals, exact |
| L3 | Metric realization | log-frequency, empirical tuning curves, beat-to-clock maps, trajectories | real with declared error |
| L4 | Device realization | ticks, MIDI encodings, sampled control values, bytes | bounded integer/real |

The layer labels describe **information roles**, not a requirement that every object pass through every layer. An imported empirical scale, for example, may begin at L3 and acquire an optional L1 model later.

### 0.6.1 Exactness rule

L0--L2 MUST NOT require binary floating point for identity, equality, quotient membership, or conformance decisions.

Real-valued observations used to rank or realize exact objects MUST carry:

1. a value;
2. an uncertainty or numerical error bound when applicable;
3. provenance identifying the algorithm, parameters, and source data.

## 0.7 Type discipline

UMT-3.2 distinguishes four operations that previous designs blurred together:

1. **Structural maps** -- exact homomorphisms between discrete algebraic objects.
2. **Representative policies** -- right inverses that choose one exact lift from a structural class; they need not be homomorphisms.
3. **Metric realization maps** -- real-valued maps such as tunings and tempo curves; they may be linear or nonlinear.
4. **Device quantizers** -- maps from a metric continuum to a device-representable set; their algebra depends on the chosen rounding policy.

A conformance implementation MUST expose these as different types or otherwise prevent accidental substitution.

## 0.8 Revision 3.1 source-compatibility policy

The normative Markdown source is intended to render on GitHub without relying on the named-operator LaTeX macro that motivated this revision. Named mathematical functions that need upright text use `\mathrm{...}` or are defined by ordinary prose and set notation instead.

Source-level rendering compatibility is not a mathematical law, but the conformance repository SHOULD lint the normative Markdown for unsupported or intentionally avoided macros before publication.

---

# Part I -- Exact Proportion Core

## 1.1 Formal basis and lattice

Fix a finite ordered set of formal generators

$$B=(\beta_1,\ldots,\beta_k).$$

The exact proportion lattice is the free abelian group

$$\Lambda_B = \bigoplus_{i=1}^k \mathbb{Z} \cong \mathbb{Z}^k.$$

An element

$$m=(a_1,\ldots,a_k)$$

is a **monzo**. Group addition corresponds to multiplication of represented proportions.

The formal lattice is exact even when the physical interpretation of a generator is empirical or irrational.

### 1.1.1 Rational basis profile

In the rational profile, each generator has an exact positive rational valuation

$$\nu(\beta_i)\in\mathbb{Q}_{>0}$$

and the generators are multiplicatively independent. The exact ratio map is

$$r(m)=\prod_i \nu(\beta_i)^{a_i}\in\mathbb{Q}_{>0}.$$

For the prime basis, unique factorization guarantees independence.

### 1.1.2 Symbolic-real basis profile

In the symbolic-real profile, the generators remain formal and exact at L1. A metric valuation

$$\nu_3(\beta_i)\in\mathbb{R}_{>0}$$

is attached at L3 with uncertainty and provenance. Extend the selected generator valuation multiplicatively to monzos by

$$\nu_3(m)=\prod_i \nu_3(\beta_i)^{a_i}.$$

In the rational profile, the default real valuation is the ordinary embedding of the exact ratio $r(m)$ into $\mathbb{R}_{>0}$, unless a different L3 realization is explicitly selected.

Multiplicative independence of approximate measured reals is generally not mechanically decidable from finite-precision observations. Therefore a symbolic-real basis MUST either:

- declare independence as a modeling assumption; or
- provide an exact algebraic certificate from which independence is established.

The declaration itself is structural metadata and MUST NOT be inferred from floating-point inequality tests.

## 1.2 Distinguished unit and logarithmic valuation

When a domain has a distinguished periodic unit, UMT-3.2 represents it by a **lattice element**

$$\hat u\in\Lambda_B,$$

not by an unrelated real number.

For the standard order-preserving logarithmic coordinate, choose the orientation of the unit so that its real valuation is

$$u=\nu_3(\hat u)>1.$$

Then define

$$\pi_{\hat u}(m)=\log_u \nu_3(m).$$

If a naturally supplied unit has valuation in $(0,1)$, the implementation SHOULD use the inverse lattice element $-\hat u$ as the positively oriented unit rather than silently reversing the order of the logarithmic coordinate.

For an exact rational basis the ratio $r(m)$ is exact, but the logarithm is an L3 real observation unless represented symbolically.

If the basis valuation is multiplicatively independent, the real ratio map is injective. This fact MUST NOT be confused with temperament: a nontrivial temperament kernel is the kernel of a chosen structural mapping, not the kernel of the pure ratio valuation.

## 1.3 Complexity and height

A **complexity function** is a declared map

$$h:\Lambda_B\to\mathbb{R}_{\ge 0}.$$

UMT-3.2 does not require every complexity function to be a norm. The implementation MUST declare which laws a chosen complexity satisfies. Because the word *norm* is used differently for groups and for modules/vector spaces, UMT-3.2 distinguishes group-length laws from $\mathbb{Z}$-homogeneous lattice norm laws rather than relying on the bare word alone.

### 1.3.1 Weighted lattice norm

For positive weights $w_i>0$,

$$h_{1,w}(m)=\sum_i w_i|a_i|$$

is a norm on $\Lambda_B\otimes\mathbb{R}$ restricted to the lattice.

### 1.3.2 Tenney height

For a prime rational basis $B=(p_1,\ldots,p_k)$,

$$h_T(m)=\sum_i |a_i|\log_2 p_i.$$

If $r(m)=n/d$ is in lowest terms, then

$$h_T(m)=\log_2(nd).$$

This identity is specific to prime-factor coordinates. It MUST NOT be asserted for an arbitrary independent rational basis without transforming to prime coordinates.

### 1.3.3 Octave-equivalent complexity

A lattice seminorm MAY assign zero cost to every element of the octave subgroup. If it does, the octave subgroup lies in its null subgroup. The seminorm descends to the quotient by the octave subgroup; the descended function is a lattice norm only when the null subgroup is exactly the subgroup being quotiented (or when any larger null subgroup is also quotiented).

Merely assigning zero cost to octave powers is not by itself enough to prove the seminorm laws. An implementation MUST test or establish the relevant homogeneity and triangle laws and MUST NOT advertise full-lattice identity of indiscernibles when nonzero octave elements have zero cost.

### 1.3.4 Musical interpretation

Arithmetic complexity is a structural or heuristic quantity. It MAY be used as:

- a representative-selection cost;
- a tuning-optimization weight;
- a ratio-complexity descriptor;
- one feature in a consonance model.

It MUST NOT be identified with sensory dissonance in general.

## 1.4 Temperament mappings

A **regular temperament mapping** is an integer homomorphism

$$V:\Lambda_B\to\Gamma,$$

where $\Gamma\cong\mathbb{Z}^r$ is a declared ambient free abelian group.

The mapping need **not** be surjective.

Define

$$K=\ker V,\qquad H=\mathrm{im}V\le\Gamma.$$

The exact tempered quotient is

$$T=\Lambda_B/K.$$

By the first isomorphism theorem,

$$T\cong H.$$

The ambient group $\Gamma$ and the actually reachable subgroup $H$ MUST be distinguished. This matters for EDO patent vals and other mappings whose image is a proper subgroup of the chosen coordinate lattice.

### 1.4.1 Kernel saturation theorem

Because $\Gamma$ is torsion-free, $K=\ker V$ is automatically saturated. Explicitly, for every $x\in\Lambda_B$ and every nonzero integer $n\in\mathbb{Z}\setminus\{0\}$,

$$nx\in K\Rightarrow nV(x)=0\Rightarrow V(x)=0\Rightarrow x\in K.$$

The exclusion $n\ne0$ is essential: $0x\in K$ holds for every $x$ and carries no information about membership of $x$ in $K$.

Therefore **kernel saturation is not an extra construction-time condition when $K$ is computed as the kernel of a map into a free abelian group**.

### 1.4.2 Image primitivity and surjectivity

These are separate questions.

Given an integer matrix for $V$, the Smith normal form describes the invariant factors of the image/cokernel. If $V$ has full row rank $r$, then $V$ is surjective onto $\mathbb{Z}^r$ iff the gcd of its $r\times r$ minors is $1$.

That gcd condition concerns image primitivity/surjectivity. It is **not** equivalent to saturation of $\ker V$.

Counterexample:

$$V=[2]:\mathbb{Z}\to\mathbb{Z}.$$

Its kernel is $0$ and therefore saturated, but its image is $2\mathbb{Z}$ and the gcd of maximal minors is $2$.

## 1.5 Direct comma-subgroup specification

UMT-3.2 also permits a user to specify a subgroup

$$K_0\le\Lambda_B$$

directly, without first supplying a mapping.

If the intended result is a torsion-free regular temperament quotient, the supplied subgroup MUST be saturated. An implementation MUST either:

- reject a nonsaturated subgroup; or
- explicitly replace it by its saturation and report the change.

For example, specifying only twice the syntonic comma produces a quotient with 2-torsion. A regular real-valued tuning that kills twice a comma necessarily kills the comma itself, so the unsaturated distinction cannot survive realization by a homomorphism into a torsion-free real group.

This validation applies to **direct comma specifications**. It is not a condition that must be redundantly imposed on a kernel already computed from $V:\Lambda_B\to\mathbb{Z}^r$.

## 1.6 Equal divisions and patent vals

For an $N$-EDO pitch model with octave generator $2$, define an ambient step lattice

$$\Gamma_N=\mathbb{Z}$$

whose element $1$ means one EDO step. A patent-val-style mapping on a basis $B=(\beta_1,\ldots,\beta_k)$ with selected positive real generator valuations may be written

$$V_N(a_1,\ldots,a_k)=\sum_i a_i v_i,$$

with

$$v_i=\mathrm{round}\!\left(N\log_2 \nu_3(\beta_i)\right)$$

under a declared rounding convention. When a basis generator has exact valuation $2$, its patent-val entry is fixed to $N$.

The image is

$$H_N=\gcd(v_1,\ldots,v_k)\mathbb{Z}.$$

The convention $\gcd(0,\ldots,0)=0$ gives $H_N=\{0\}$ for the zero mapping.

It may be a proper subgroup of $\Gamma_N$.

### 1.6.1 Required 6-EDO test

On the 5-limit prime basis $(2,3,5)$, the usual patent val for 6-EDO is

$$[6,10,14].$$

Its image is $2\mathbb{Z}$, not all of $\mathbb{Z}$.

A conforming implementation MUST therefore be able to represent both:

- the ambient 6-EDO step lattice $\Gamma_6=\mathbb{Z}$, containing every EDO step; and
- the mapped JI image $H_6=2\mathbb{Z}$, containing the classes reached by the chosen 5-limit patent val.

Detempering is defined automatically only on $H_6$, not on arbitrary odd steps of the ambient EDO lattice.

## 1.7 Splittings, representative policies, and residue

Because $H$ is free abelian, the exact sequence

$$0\to K\to\Lambda_B\xrightarrow{V}H\to0$$

splits as abelian groups.

UMT-3.2 distinguishes two kinds of right inverse.

### 1.7.1 Homomorphic splitting

A **linear splitting** is a group homomorphism

$$s:H\to\Lambda_B$$

with

$$V\circ s=\mathrm{id}_H.$$

Then

$$\Lambda_B\cong H\oplus K$$

as abelian groups, once $s$ is chosen.

### 1.7.2 Representative policy

A **representative policy** is an arbitrary set-theoretic right inverse

$$\sigma:H\to\Lambda_B,\qquad V\circ\sigma=\mathrm{id}_H.$$

It need not preserve addition. Minimum-complexity spelling, context-sensitive detempering, and adaptive lift selection are naturally represented by such policies.

For any representative policy, define the residue

$$\rho_\sigma(m)=m-\sigma(V(m))\in K.$$

Then the map

$$m\mapsto (V(m),\rho_\sigma(m))$$

is a bijection of sets between $\Lambda_B$ and $H\times K$. It is a group isomorphism iff $\sigma$ is a homomorphic splitting.

### 1.7.3 Lens laws

Define

$$\mathrm{get}(m)=V(m)$$

and

$$\mathrm{put}(m,x)=\sigma(x)+\rho_\sigma(m).$$

For any right inverse $\sigma$, the following set-level lens laws hold:

1. GetPut: $\mathrm{put}(m,\mathrm{get}(m))=m$.
2. PutGet: $\mathrm{get}(\mathrm{put}(m,x))=x$.
3. PutPut: $\mathrm{put}(\mathrm{put}(m,x),y)=\mathrm{put}(m,y)$.

The complement is the comma residue $K$. The laws do **not** imply that $\sigma$ is linear.

### 1.7.4 Canonical versus actual lifts

A representative policy selects one canonical or preferred lift. An actual musical spelling or adaptive realization may choose **any** lift in the fiber

$$V^{-1}(x)=\sigma(x)+K.$$

Therefore UMT-3.2 says:

> a section may define a canonical spelling or detempering policy; notation itself is not identical to a section.

## 1.8 Tuning and realization maps

UMT-3.2 distinguishes regular interval tuning from non-regular pitch realization.

### 1.8.1 Regular tuning

A **regular tuning** on the reachable temperament structure is a group homomorphism

$$\tau_H:H\to\mathbb{R}.$$

An application whose ambient lattice $\Gamma$ has independent musical meaning MAY instead or additionally provide an ambient tuning

$$\tau_\Gamma:\Gamma\to\mathbb{R},$$

whose restriction to $H$ supplies $\tau_H$. This is useful, for example, when every EDO step in the ambient lattice must have a realized size even though the chosen JI patent val reaches only a subgroup.

Restricted along $V$, a reachable regular tuning realizes exact monzos by

$$\tau_H\circ V:\Lambda_B\to\mathbb{R}.$$

Given a pure log valuation $\pi$, the regular tuning error functional is

$$\varepsilon=\tau_H\circ V-\pi.$$

For every comma $k\in K$,

$$\varepsilon(k)=-\pi(k).$$

Regular tuning is translation-invariant in the structural lattice. It does not model register-dependent tuning curves or context-dependent choices by itself.

A regular tuning is a map of **intervals**, not by itself a map of pitch points. To realize a pitch-point torsor $P_2$ over a selected interval group $G_2$, choose a structural reference point $p_0\in P_2$, a realized reference point $q_0\in P_3$, and a regular interval tuning $\tau:G_2\to\mathbb{R}$. These data induce the unique affine/equivariant point realization $\widehat{\tau}:P_2\to P_3$ satisfying

$$\widehat{\tau}(p_0)=q_0$$

and

$$\widehat{\tau}(p+g)=\widehat{\tau}(p)+\tau(g).$$

Thus concert-pitch reference data are part of point realization even when the interval tuning itself is fully regular.

### 1.8.2 Non-regular realization

Let $G_2$ be the declared L2 interval group for the realization, normally $H$ or the ambient $\Gamma$, and let $P_2$ be a pitch-point torsor over $G_2$. Let $C$ be a declared context space. A **non-regular pitch realization** is a contextual map

$$\Phi:P_2\times C\to P_3,$$

or equivalently a family of maps $\Phi_c:P_2\to P_3$, where $P_3$ is a real log-frequency torsor.

The context $c\in C$ MAY encode:

- register- or pitch-class-dependent policy state not already determined by the pitch point;
- harmonic or melodic context;
- instrument state;
- measured inharmonicity;
- performance time.

No homomorphism law is imposed on this contextual realization: $P_2$ is a torsor rather than an interval group, and the context space $C$ need not carry any group structure. Register-dependent stretched piano tuning belongs here, not in the space of fixed linear maps $\tau$.

### 1.8.3 Optimization targets

A tuning optimizer MUST declare:

- its candidate map class;
- its target interval set or distribution;
- its cost function;
- its weights;
- any purity constraints;
- numerical tolerance and optimizer provenance.

Terms such as TOP, minimax, least-squares, or pure-octave tuning identify optimization policies, not new structural quotients.

## 1.9 Unit equivalence

A domain MAY introduce an equivalence by a designated unit element.

Let $G_2$ denote the L2 interval group whose equivalence classes are actually being formed. It may be the reachable image $H$ or an ambient group $\Gamma$. Unit equivalence is the optional quotient

$$G_2/\langle V(\hat u)\rangle.$$

The chosen $G_2$ MUST be recorded. In ordinary 12-EDO with $G_2=\Gamma=\mathbb{Z}$ and $V(\hat u)=12$, this gives $\mathbb{Z}/12\mathbb{Z}$.

Reachable and ambient pitch-class spaces can differ. For the 6-EDO fixture $H=2\mathbb{Z}\subset\Gamma=\mathbb{Z}$ with $V(\hat u)=6$,

$$H/6\mathbb{Z}\cong\mathbb{Z}/3\mathbb{Z},\qquad \Gamma/6\mathbb{Z}\cong\mathbb{Z}/6\mathbb{Z}.$$

Unit equivalence MUST NOT be conflated with temperament itself.

### 1.9.1 Metrics and quotienting

UMT-3.2 does not mandate that all voice-leading distances be computed before octave quotienting. Instead, every metric MUST declare its domain.

Two legitimate examples are:

- a pitch-space metric on registered pitches;
- a quotient metric on pitch classes, often defined by minimizing a pitch-space distance over octave representatives.

The conformance requirement is that an implementation MUST NOT silently substitute one for the other.

## 1.10 Torsors and reference

Intervals form groups; pitch points and time points do not.

A **torsor** $P$ over an abelian group $G$ is a set with a simply transitive action

$$P\times G\to P.$$

For $p,q\in P$, there is a unique interval

$$\mathrm{int}(p,q)\in G$$

such that

$$p+\mathrm{int}(p,q)=q.$$

There is no intrinsic addition $P\times P\to P$.

Reference pitch, concert pitch, transposition, and changes of origin are therefore represented explicitly rather than being baked into interval coordinates.

---

# Part II -- The Rate-Continuum Interface

## 2.1 What is shared

The common pitch/rhythm core is the action of positive multiplicative proportions. If a ratio $\rho$ acts on a positive rate $f$ by

$$f\mapsto \rho f,$$

then the same abstract proportion may be interpreted as a frequency ratio, tempo ratio, or subdivision ratio. The orientation of a ratio MUST be declared when translating between reciprocal quantities: if a duration $d$ is the reciprocal of a rate for a fixed cycle count, then $f\mapsto \rho f$ corresponds to $d\mapsto \rho^{-1}d$. A system MUST NOT silently reuse a rate ratio as a duration ratio without accounting for this inversion.

This shared algebra justifies reuse of:

- monzos;
- exact multiplicative composition;
- regular mappings;
- comma kernels;
- representative-selection machinery;
- ratio-complexity functions.

## 2.2 What is not shared automatically

Pitch and rhythm acquire different additional structure.

Pitch requires, among other things, acoustic signal realization, spectra, auditory pitch inference, register, and pitch trajectories.

Rhythm requires, among other things, an ordered timeline, causality, event incidence, duration concatenation, hierarchy, grouping, and temporal constraints.

No perceptual cutoff frequency is part of the formal core. Implementations MAY use psychoacoustic transition ranges for analysis or UI, but such thresholds are model parameters with provenance, not structural axioms.

## 2.3 Typed acoustic operations

UMT-3.2 distinguishes these operations:

1. **Frequency-ratio arithmetic:** $f_1/f_2$.
2. **Linear signal superposition:** $x_1(t)+x_2(t)$.
3. **Envelope or beating analysis:** a derived low-frequency modulation observable in suitable superpositions.
4. **Nonlinear intermodulation:** physical or auditory nonlinearities that can generate components related to $mf_1\pm nf_2$.
5. **Event concatenation:** addition of durations on a timeline.

These operations may interact physically or perceptually, but they MUST NOT be represented as one untyped binary operation.

## 2.4 Temperament versus grid quantization

Temperament is an exact quotient phenomenon induced by a group homomorphism:

$$\Lambda_B\xrightarrow{V}H.$$

Grid quantization is approximation of an additive coordinate by a discrete representable set.

Both can lose information, but their residuals have different types:

- temperament residue: an exact element of $K$;
- tuning error: a real metric deviation;
- grid/device residual: a real or rational coordinate error.

A conforming implementation MUST preserve these types separately.

---
# Part III -- Generated Structures

## 3.1 Modular generated sets

Let $p>0$ be a declared period and $g$ a declared generator in an ordered additive realization space. For $n\ge 1$, define

$$\mathcal{G}(g,p,n)=\{jg\bmod p\mid j=0,\ldots,n-1\}.$$

The data $(p,g)$ are **designated data**. A rank-2 temperament does not canonically determine which basis vector is "period" and which is "generator": changing a basis by an element of $GL(2,\mathbb{Z})$ changes those coordinates without changing the underlying free group.

Therefore a generated-scale object MUST store its designated period and generator explicitly.

## 3.2 Three-gap behavior

For consecutive points generated by rotation on a circle, the Three-Gap Theorem bounds the number of distinct adjacent gap sizes by three under its standard hypotheses.

UMT-3.2 uses this theorem as a property of generated circular sets, not as a definition of every musical scale.

An implementation that claims a three-gap conformance result MUST record:

- whether $g/p$ is rational or irrational;
- how duplicate generated points are handled when the orbit closes;
- the cardinality $n$;
- the sorted circular gaps used in the test.

## 3.3 MOS and well-formed scale profiles

UMT-3.2 uses **MOS** operationally for a designated period-generator construction at a cardinality where the projected generated set has two positive step sizes, with the equal-step case treated as degenerate if the chosen profile allows it.

The term **well-formed scale** has a broader and historically specific music-theoretic literature. A UMT implementation MAY expose a well-formed-scale predicate, but it MUST declare the exact definition used rather than treating "well-formed" and "two gap sizes" as interchangeable labels.

A generated family may contain scales at every cardinality up to orbit closure. Under the operational two-gap predicate above, a quarter-comma-meantone generator has early MOS cardinalities

$$2,3,5,7,12,19,31,\ldots$$

The inclusion of $3$ is intentional: the three generated pitch classes have exactly two positive circular gap sizes. Lists of MOS cardinalities do not imply the nonexistence of generated scales at intervening cardinalities, and other meantone generators or other MOS definitions MUST be identified rather than silently inheriting this example.

## 3.4 Modes and rotations

A **cyclic mode** of a finite circular scale is a rotation of its ordered step pattern. Whether rotated modes are considered identical, equivalent, or distinct is a declared equivalence relation of the application.

UMT-3.2 does not force mode identity at the core because modal function may depend on a designated reference degree.

## 3.5 Euclidean rhythms

For integers $0<k\le n$, a Euclidean rhythm $E(k,n)$ is a maximally even distribution of $k$ onsets among $n$ pulse positions under a declared rotation convention.

Generated scales and Euclidean rhythms share modular arithmetic, balance properties, and continued-fraction structure. UMT-3.2 therefore permits common algorithms for modular distribution.

It does **not** identify every MOS construction with every Euclidean-rhythm construction as the same theorem or the same object.

When $\gcd(k,n)=1$, balanced finite binary words are closely related to primitive Christoffel words. Infinite aperiodic mechanical words of irrational slope belong to the Sturmian setting. A finite Euclidean-rhythm word MUST NOT simply be called an infinite Sturmian word.

---

# Part IV -- Pitch

## 4.1 Pitch instantiation

A common rational pitch profile uses a prime basis such as

$$B=(2,3,5,\ldots).$$

The designated unit is usually the octave monzo $\hat u=(1,0,\ldots)$ and its metric size is normalized to one octave or 1200 cents when desired.

The exact interval structures are:

- L1: $\Lambda_B$;
- L2 reachable structure: $H=\mathrm{im}V$;
- L2 optional ambient structure: $\Gamma$;
- L3 interval metric: a real log-frequency group.

## 4.2 Pitch points

For each interval layer, pitch positions form a torsor over the corresponding interval group.

A pitch reference contains at least:

- a designated pitch point $p_0$;
- a physical frequency $f_0>0$;
- the interval coordinate system used to compare other points with $p_0$.

Changing concert pitch or transposing-instrument reference changes reference data; it does not mutate the abstract interval lattice.

A modulation model MAY change a designated reference region, tonic annotation, spelling policy, or style-specific functional state. UMT-3.2 does not assert that every musical modulation is merely a reference change.

## 4.3 Chords and voices

A **voice set** is a finite set $V_c$ of voice identities.

A registered chord is a function

$$c:V_c\to P,$$

where $P$ is a pitch-point space.

Forgetting voice labels produces a finite multiset of pitch points. Keeping the labels preserves unisons, doublings, and later voice continuity.

Parallel juxtaposition of independent voice collections is represented by disjoint union of voice sets. The empty voice set is the neutral object for this operation.

## 4.4 Voice leading

### 4.4.1 Structural relation

A voice-leading relation from $V_1$ to $V_2$ is represented by a span

$$V_1\xleftarrow{\alpha}E\xrightarrow{\beta}V_2.$$

An edge $e\in E$ relates a source voice $\alpha(e)$ to a destination voice $\beta(e)$.

This permits:

- one-to-one continuation;
- splits;
- merges;
- repeated relations;
- entries and exits represented by explicit unmatched voices or by a chosen null/birth-death extension.

Composition MAY be defined by pullback when the application needs categorical composition of relations.

Displacement along a related edge is derived from the two pitch points under the selected interval metric; it need not be redundantly stored unless a historical or intended displacement differs from the observed endpoints.

### 4.4.2 Declared-span cost

Given an explicit span $E$, an implementation MAY assign a cost

$$C(E)=C_{\mathrm{move}}+C_{\mathrm{split}}+C_{\mathrm{merge}}+C_{\mathrm{birth}}+C_{\mathrm{death}}.$$

Such a cost is a cost of a declared transformation. It is not automatically a metric on chords.

### 4.4.3 Equal-mass optimal transport

If chords are represented as positive measures of equal total mass, classical Wasserstein transport is available. For $1\le p<\infty$,

$$W_p(\mu,\nu)=\left(\inf_{\gamma\in\Pi(\mu,\nu)}\int d(x,y)^p\,d\gamma(x,y)\right)^{1/p},$$

with the corresponding essential-supremum formulation for $p=\infty$.

The ground metric $d$ MUST be declared, for example registered log-pitch distance or a pitch-class quotient metric.

### 4.4.4 Unequal voice counts

Counting one unit of mass per voice gives different total masses when voice counts differ. Classical balanced Wasserstein transport does not by itself solve that case.

A conforming unequal-voice-count profile MUST use one of:

- unbalanced optimal transport with declared creation/destruction penalties;
- partial transport;
- an assignment/edit metric with explicit birth/death costs;
- normalization to equal mass **only when the application explicitly accepts the resulting loss of absolute multiplicity information**.

A chord containing one C and a chord containing two identical Cs MUST NOT become indistinguishable merely because an undocumented normalization turned both into the same probability measure.

### 4.4.5 Relation-aware optimization

If a score already contains voice identities or a declared span, the metric MAY be constrained by those relations instead of re-optimizing over every possible coupling. The implementation MUST state whether the output is:

- cost of the declared voice leading; or
- minimum cost over an admissible family of voice leadings.

These answer different questions.

## 4.5 Pitch notation at L0

A notated pitch spelling may contain:

- letter or degree name;
- accidental or alteration object;
- octave/register designation;
- notation-system identifier;
- optional exact comma or monzo annotation;
- optional display-only information.

UMT-3.2 separates three maps.

### 4.5.1 Parsing

A notation parser is a context-dependent map or relation

$$\mathrm{parse}:S\dashrightarrow P_1$$

from spellings to exact pitch interpretations.

It may be partial or ambiguous. For example, the meaning of an accidental can depend on a declared notation system, key signature, historical practice, or microtonal convention.

### 4.5.2 Canonical writing

A notation system MAY provide a writer

$$\mathrm{write}:P_1\to S$$

or a canonical writer on a subset of representable pitches. This writer is an orthographic policy, not the temperament section $\sigma$ itself.

### 4.5.3 Spelling relative to a tempered class

If a notated exact pitch $m\in\Lambda_B$ maps to $x=V(m)$, then its difference from a canonical lift is

$$\rho_\sigma(m)=m-\sigma(x)\in K.$$

Thus two spellings can remain different exact L1 objects while mapping to the same L2 sounding class.

This is the formal basis of enharmonic preservation in UMT-3.2.

## 4.6 Register, inversion, and roots

Register and octave equivalence are explicit choices, not implicit erasures.

A chord may carry optional analytical annotations such as:

- designated root;
- inversion label;
- pitch-class set;
- otonal or utonal fit;
- virtual-pitch estimate;
- style-specific harmonic function.

These annotations are not primitive truths of the chord object. Each analytical method MUST identify its model and provenance.

## 4.7 Continuous pitch

A realized note may carry a pitch trajectory

$$\gamma:[t_0,t_1]\to P_3,$$

where $P_3$ is a real log-frequency pitch torsor.

A common representation is

$$\gamma(t)=\Phi(x,c(t))+v(t),$$

where:

- $x\in P_2$ is a nominal structural pitch point;
- $c(t)\in C$ is the realization context at time $t$;
- $\Phi$ is the selected contextual L2-to-L3 realization;
- $v(t)$ is a real-valued deviation acting on the log-frequency torsor.

This distinguishes:

- a structural interval such as an exact $7/6$ lift;
- a nominal tempered pitch with a continuous bend;
- vibrato around a nominal pitch;
- continuous portamento or glissando;
- stepped glissando represented as a sequence of nominal events.

A sampled device encoding at L4 MUST retain the L3 trajectory or a declared approximation record if round-trip reconstruction is required.

## 4.8 Adaptive just-intonation realization

Adaptive JI is modeled as **context-dependent lift selection**, optionally followed by an exact or metric realization.

After choosing a reference coordinate for the passage, let structural pitch coordinates $x_e\in H$ be indexed by events $e$. Equivalently, one may formulate the construction on the corresponding pitch-point torsors. An adaptive lift assignment chooses

$$m_e\in V^{-1}(x_e).$$

The assignment may minimize an objective such as

$$J=\lambda_v C_{\mathrm{vertical}}+\lambda_h C_{\mathrm{horizontal}}+\lambda_s C_{\mathrm{spelling}}+\lambda_d C_{\mathrm{drift}}.$$

Possible terms include:

- deviation of simultaneous intervals from selected just targets;
- melodic displacement from prior lifts;
- accumulated comma-pump drift;
- preference for notated spelling;
- bounded retuning speed.

The optimizer MUST report the selected lifts and the comma residues that distinguish them from the configured canonical policy.

Adaptive JI is not, in general, one fixed homomorphic section and not one fixed linear tuning $\tau$.

## 4.9 Inharmonic spectra and empirical tunings

Arithmetic ratio simplicity is not a universal sensory-consonance law. UMT-3.2 therefore offers three separate representation paths.

### 4.9.1 Direct empirical L3 scale

A measured or traditional tuning may be stored directly as real interval values with uncertainty and provenance. No JI lattice explanation is required.

This is the minimum adequate representation for tunings whose cultural or acoustic basis is not established by a small-integer model.

### 4.9.2 Spectrum-conditioned dissonance model

Given a spectrum model $S$, an implementation MAY define a sensory-dissonance or local-consonance function

$$d_S(\rho).$$

Local minima can be used as candidate interval regions. They are real-valued observations, not exact lattice generators by themselves.

### 4.9.3 Optional lattice inference

An implementation MAY infer a symbolic-real or rational lattice model from empirical candidate intervals. Such inference MUST declare:

- the candidate-selection procedure;
- tolerance or uncertainty regions;
- the number of generators requested or selected;
- the optimization criterion;
- the approximation residual for every fitted interval;
- whether independence is assumed or certified.

There is no canonical instruction to "take a maximally independent subset of local minima." Different maximal subsets can exist, numerical minima are unstable under measurement error, and approximate real independence is not an exact observable.

### 4.9.4 Separation from Tenney height

Tenney height and spectrum-derived dissonance may correlate in restricted contexts, but neither is defined as a special case of the other in UMT-3.2.

## 4.10 Tonal function

Cadence, tonic/dominant function, chord grammar, and style-specific harmonic syntax remain outside the core.

The core provides hooks:

- designated reference pitches or pitch classes;
- exact and metric interval relations;
- chord annotations;
- event context;
- style-model namespaces.

A functional-harmony extension MAY build on those hooks without changing the core proportion types.

---
# Part V -- Rhythm and Time

## 5.1 Structural and physical timelines

Rhythm adds ordered additive structure that is not contained in the multiplicative proportion lattice.

UMT-3.2 distinguishes:

- a **structural beat timeline** $T_b$, an affine torsor over an exact ordered duration group $D_b$;
- a **performance clock timeline** $T_c$, an affine torsor over a real duration group.

The default notated-duration profile uses

$$D_b=\mathbb{Q}$$

in declared beat units. This exactly represents ordinary rational subdivisions and nested tuplets.

An implementation MAY use a different exact ordered additive group if required, but it MUST declare it.

Time points subtract to durations. Time points do not intrinsically add.

Meter is structure **on** the timeline. It does not replace the timeline by a circle.

## 5.2 Events, noteheads, ties, rests, and sounding spans

UMT-3.2 separates notation events from realized sounding events.

### 5.2.1 Notated events

A notated event contains at least:

- event identity;
- a scope such as voice-local, staff-local, part-local, or global;
- temporal placement data, which may be a fixed exact span, references to temporal variables/constraints, or a grace-event placement rule;
- event kind: pitched notehead, unpitched note, rest, control event, or another declared type;
- membership in a rhythm tree or other temporal structure when applicable;
- optional tie endpoints and articulation marks.

A voice-local event MUST carry a voice identity. A genuinely global control or structural event MAY omit a voice identity. An unmeasured event is not required to have a fixed structural onset before its temporal constraints are solved.

### 5.2.2 Ties

A tie is a relation between distinct notated noteheads. The tied noteheads MUST remain distinct at L0 when notation-preserving round trip is required.

A later realization stage MAY combine tied noteheads into one sustained sounding gesture while retaining the source tie relation.

Therefore UMT-3.2 does **not** merge ties at L0.

### 5.2.3 Sounding events

A sounding event or gesture contains a realized onset and offset on $T_c$, plus any realized pitch trajectory and control data.

Articulation is represented by the relationship between notated structure and sounding realization, not by destructively rewriting the notation event.

### 5.2.4 Rests and silence

A rest is a notated event in a particular voice or staff context. It is not the global set-theoretic complement of sounding intervals.

A rest may coexist with sounding events in other voices. Conversely, a physical silent interval may arise without an explicit rest object.

Acoustic silence, voice-local inactivity, and notated rests are therefore distinct predicates.

## 5.3 Hierarchical rhythm and cyclic rhythm

UMT-3.2 has two primary structured rhythm families in addition to the flat timeline.

### 5.3.1 Weighted ordered rhythm tree

A rhythm tree is a rooted ordered tree. Each internal node represents a parent duration and has positive exact child weights

$$(w_1,\ldots,w_n),\qquad w_i\in\mathbb{Q}_{>0}$$

or an equivalent exact proportion representation.

Children divide the parent span proportionally to their weights.

This one structure can represent:

- equal divisive subdivision;
- tuplets;
- additive grouping such as $2+2+3$;
- nested mixed subdivision.

The tree MUST preserve child order and nesting.

### 5.3.2 Cyclic pulse pattern

A cyclic rhythm is represented by:

- a cycle length or period;
- a finite pulse lattice or exact cyclic coordinate set;
- onset positions or a binary/weighted necklace;
- an optional designated rotation/reference point.

Cyclic patterns need not imply hierarchy.

### 5.3.3 Flattening

A flattening operation from a tree or necklace to structural event times MUST preserve:

- event order;
- total parent duration;
- exact rational boundaries when the source data are exact.

Flattening is generally lossy because different trees can produce the same set of boundaries. A round-trip implementation MUST retain the source tree rather than expecting to reconstruct it uniquely from flattened times.

## 5.4 Meter and grouping

### 5.4.1 Metrical hierarchy

A meter profile may be represented by nested periodic point sets

$$\cdots\subseteq L_2\subseteq L_1\subseteq L_0\subset T_b,$$

where lower-indexed sets contain finer pulses and selected subsets mark stronger metrical positions.

The exact convention for level numbering MUST be declared.

For example, in eighth-note units within a six-eighth span:

- 6/8 may have primary beats at $\{0,3\}$;
- 3/4 may have primary beats at $\{0,2,4\}$.

The two meters can therefore share a total duration while differing structurally.

Levels need not be subgroups. Additive meters such as $2+2+3$ require periodic point-set patterns that need not be closed under addition modulo the period.

### 5.4.2 Grouping structure

Grouping is a separate ordered tree or segmentation over events/time spans. Phrase grouping, motive grouping, and additive grouping need not coincide with meter.

A syncopation analysis MAY compare grouping accents, event accents, and metric weights, but UMT-3.2 does not impose one universal scalar definition of syncopation.

### 5.4.3 Hypermeter and anacrusis

Hypermeter extends metrical organization above the ordinary bar level.

Anacrusis is represented by event/grouping structure whose beginning precedes a designated metrical reference point. No cyclic identification of the entire timeline is required.

## 5.5 Polyrhythm and polymeter

Terminology varies across musical traditions. UMT-3.2 therefore adopts the following **operational convention** for its default profile:

- **polyrhythm**: simultaneous rhythmic layers share a reference span or recurring downbeat while differing in internal pulse/subdivision organization;
- **polymeter**: simultaneous metric layers have distinct recurring bar or cycle periods, so their major reference points realign only according to the relation between those periods.

Applications MAY expose other terminological profiles, but they MUST preserve the underlying data: separate metric structures on one common ordered timeline.

## 5.6 Multiplicative tempo proportions

Tempo ratios form a multiplicative proportion domain and may instantiate Part I.

A metric modulation can therefore be represented as an exact proportion such as $3/2$ or $5/4$, with direction and reference tempo stated explicitly.

A sequence of exact tempo-ratio operations is a path in a proportion lattice. If a compositional system applies a regular mapping to that lattice, exact ratio cycles can carry nonzero kernel residues in the same algebraic sense as pitch temperaments.

UMT-3.2 calls such a mapping a **tempo-proportion temperament** when this is an intentional compositional model.

This is not the same operation as rounding event times to a PPQN grid.

## 5.7 Additive grids and quantization

Let $P\in\mathbb{Z}_{>0}$. A device or score grid is

$$G_P=\frac{1}{P}\mathbb{Z}\subset\mathbb{R}$$

in a declared unit.

Let

$$i:G_P\hookrightarrow\mathbb{R}$$

be inclusion.

### 5.7.1 Floor and ceiling adjunctions

Define grid floor $q_\downarrow:\mathbb{R}\to G_P$ and ceiling $q_\uparrow:\mathbb{R}\to G_P$.

The correct order adjunctions are

$$i\dashv q_\downarrow$$

because

$$i(g)\le x\iff g\le q_\downarrow(x),$$

and

$$q_\uparrow\dashv i$$

because

$$q_\uparrow(x)\le g\iff x\le i(g).$$

For floor quantization,

$$i(q_\downarrow(x))\le x.$$

This inequality is the counit of the adjunction $i\dashv q_\downarrow$ in the poset reading. The numeric quantization residual

$$e_\downarrow(x)=x-i(q_\downarrow(x))\ge0$$

is derived from that comparison; it is not literally the natural transformation itself.

### 5.7.2 Nearest quantization

Nearest-grid rounding is often preferable in devices, but it is not the same order adjunction as floor or ceiling. It has its own laws and a signed residual

$$e_N(x)=x-i(q_N(x)).$$

A quantizer MUST declare its tie-breaking policy.

### 5.7.3 Idempotence on represented values

Every deterministic grid quantizer in the core profile MUST satisfy

$$q(i(g))=g$$

for $g\in G_P$.

No universal law $q(x)\le x$ applies to nearest or ceiling rounding.

### 5.7.4 Local duration rounding can drift

At $P=96$, one fifth of a quarter note is

$$96/5=19.2$$

ticks.

Independently flooring each of five equal subdurations gives

$$5\cdot19=95$$

ticks and therefore misses the parent endpoint by one tick.

This demonstrates drift under **independent local floor rounding**. It does not prove that PPQN necessarily loses the parent endpoint.

### 5.7.5 Endpoint-preserving quantization

A constrained quantizer MAY quantize cumulative boundaries or apportion integer child durations subject to

$$\sum_i n_i=N_{\mathrm{parent}}.$$

For the same 96-tick parent, a boundary-rounding policy can yield boundaries

$$0,19,38,58,77,96$$

and child durations

$$19,19,20,19,19,$$

which sum exactly to 96.

The local residual MUST still be reported. When the source boundaries and grid are exact rationals, the residual SHOULD remain exact rational data; otherwise it is an L3 real quantity with the ordinary error/provenance contract. Endpoint drift is eliminated.

### 5.7.6 Hierarchical quantization

For nested tuplets, an implementation SHOULD support recursive constrained quantization:

1. quantize or fix the parent endpoints;
2. distribute integer child spans within that fixed parent span;
3. enforce any declared minimum-span or distinct-onset constraints;
4. recurse into each child;
5. record every local residual;
6. preserve the original exact tree for re-rendering at a different resolution.

A constrained allocation can be infeasible. For example, three children that are each required to occupy at least one device tick cannot fit inside a two-tick parent. The adapter MUST report infeasibility or an explicitly permitted collision/collapse; it MUST NOT silently violate the declared minimum-span constraints.

This avoids treating a flattened sequence of already-rounded durations as the source of truth.

## 5.8 Tempo maps and rubato

A tempo realization maps structural beat time to physical clock time.

For a bounded score span, let

$$\theta:I_b\to I_c$$

be an orientation-preserving homeomorphism between intervals: continuous, strictly increasing, and bijective onto its target interval.

For an unbounded domain, an implementation MUST state the corresponding endpoint or properness assumptions required for the intended homeomorphism.

### 5.8.1 Constant tempo

Constant tempo is affine:

$$\theta(t)=a+bt,\qquad b>0.$$

### 5.8.2 Variable tempo

Accelerando, ritardando, and rubato are represented by non-affine monotone maps. Differentiability is optional. Where $\theta'(t)>0$ exists, it has units of clock time per structural beat. If clock time is measured in seconds and the structural unit is one beat, the corresponding instantaneous beat rate is $1/\theta'(t)$ beats per second, or $60/\theta'(t)$ beats per minute.

A profile that exposes instantaneous tempo through this derivative MUST require enough regularity and positivity for the reported quantity to exist.

### 5.8.3 Not the same type as pitch tuning

A regular pitch tuning

$$\tau:G_2\to\mathbb{R}$$

on its declared L2 interval group $G_2$ is a group homomorphism on intervals. A tempo map

$$\theta:I_b\to I_c$$

is generally a nonlinear map between affine ordered timelines.

They are not the same mathematical construction.

They can, however, implement a shared higher-level interface:

> choose a realization from an admissible family by optimizing local target fidelity, continuity, and accumulated deviation subject to structural constraints.

This shared **realization optimization interface** is the meaning of adequacy target A4.

### 5.8.4 Pauses and zero-structural-duration delays

An orientation-preserving homeomorphism cannot insert a positive amount of clock time at one isolated structural beat point while assigning zero structural duration to that delay. A fermata, caesura, cue wait, or similar event that has those semantics MUST therefore be represented by one of:

- an explicit structural span that is subsequently stretched;
- a temporal-constraint variable or external predicate;
- a declared generalized monotone time relation outside the homeomorphism profile.

An implementation MUST NOT hide such a pause as a discontinuity while still claiming the homeomorphism profile.

## 5.9 Swing and local time reparameterization

Swing may be represented as a monotone within-beat reparameterization at a declared metric level.

A swing model MUST specify:

- the metrical level to which it applies;
- the reference beat interval;
- its subdivision ratio or curve;
- whether the parameter is constant or context-dependent;
- boundary continuity requirements;
- interaction with tuplets and written unequal notes.

UMT-3.2 permits tempo-dependent or context-dependent swing models but does not require an empirical law relating swing ratio to tempo.

## 5.10 Temporal constraint networks

Unmeasured and partially measured time is represented by relations among temporal variables rather than by compulsory projection onto a grid.

A **temporal constraint network** (TCN) contains:

- time variables, typically event onsets and offsets;
- typed constraints;
- a declared solver profile;
- optional external predicates;
- solution/provenance information.

UMT-3.2 distinguishes solver profiles because not all temporal constraints reduce to shortest paths.

### 5.10.1 STP profile: difference constraints

The Simple Temporal Problem profile permits constraints

$$\ell_{ij}\le t_j-t_i\le u_{ij}.$$

These are difference constraints. They can be represented by a weighted directed graph and solved with standard shortest-path/negative-cycle methods; all-pairs shortest paths provide tight implied bounds.

Only this profile receives the unconditional shortest-path consistency claim.

### 5.10.2 Linear-ratio profile

A ratio constraint such as

$$0\le\ell\le\frac{t_k-t_j}{t_j-t_i}\le u$$

requires the semantic condition

$$t_j-t_i>0.$$

Under that positive-denominator condition and constant rational bounds, the ratio bounds can be cross-multiplied into linear inequalities:

$$\ell(t_j-t_i)\le t_k-t_j\le u(t_j-t_i).$$

The strict positivity condition must itself be represented faithfully. A solver profile MAY:

- support strict linear inequalities directly; or
- replace the strict condition by $t_j-t_i\ge\delta$ only when a positive lower bound $\delta$ is justified by the model or source data.

An implementation MUST NOT invent an arbitrary $\delta>0$ merely to accommodate a solver, because doing so changes the feasible set.

These constraints are generally **not** difference-bound graph constraints because they involve three variables with non-unit coefficients. A conforming linear-ratio profile MUST use an appropriate linear feasibility method, or a more general solver whose contract subsumes it, and MUST NOT claim Floyd-Warshall/shortest-path completeness for these constraints.

### 5.10.3 Qualitative and external-predicate profile

Constraints such as

- "after the breath";
- "until the previous sound decays below a threshold";
- "enter on a conductor cue";
- style- or performer-dependent relational instructions

may refer to variables outside pure clock arithmetic.

Such constraints are represented as typed predicate references with declared semantics and an evaluation contract. Executable callbacks MAY be an implementation mechanism, but a native interchange format MUST NOT require recipients to deserialize or execute arbitrary code received from the file. General external predicates do not carry a universal decidability guarantee.

An implementation MUST report whether a network is:

- statically solved;
- partially solved with residual external conditions;
- validated only at performance time;
- unsupported by the selected solver profile.

### 5.10.4 Measured music as generated constraints

A metrical structure plus a tempo map can generate a highly constrained temporal network. This makes measured music one important source of constraints, not the definition of all time.

### 5.10.5 Unmeasured music

Unmeasured instructions are positive temporal data. They may specify precedence, broad interval bounds, proportional relations, cue dependencies, or other constraints without declaring any notional quantization grid.

A rest, fermata, breath, free-time span, or culturally specific timing instruction MUST NOT be represented merely as a rounding residual unless that is actually the intended semantics.

### 5.10.6 Performer agency

UMT-3.2 does not define performer agency as the dimension of a solution set.

Where mathematically meaningful, an implementation MAY report descriptors such as:

- dimension or degrees of freedom of a linear feasible set;
- interval widths;
- feasible-set volume under a declared measure;
- disconnected components;
- external unresolved predicates.

No one scalar is required to capture musical agency.

---
# Part VI -- The Score as an Event-Indexed Object

## 6.1 Why marginal pitch and time aggregates are insufficient

A bare pair

$$(A,R)$$

of a pitch aggregate $A$ and a time aggregate $R$ does not by itself encode which pitch belongs to which event, which voice produced it, or which duration and notation objects belong together.

This is a data-modeling problem, not a theorem that categorical products always destroy correlation. A product of sufficiently rich event-indexed structures can preserve correlation; a product of independent marginals cannot.

UMT-3.2 therefore makes **event identity** primary.

## 6.2 Event index

Let $E_s$ be a finite set of score-event identities.

A score is an event-indexed record containing at least:

- event scope, with a voice identity where the event is voice-local;
- notational pitch data where applicable;
- exact or structural pitch interpretation where applicable;
- structural temporal placement, either fixed or constraint-referenced;
- notation duration or grace rule where applicable, plus ties, articulations, and grouping membership;
- optional L3 pitch and time realization data;
- dynamics/control slots;
- provenance.

Not every event is pitched or voice-local. Rest events, breath marks, global tempo/control events, and structural markers may occupy the same event-indexed framework with typed optional fields. The event type determines which fields are required.

## 6.3 Context objects

A score also carries context that is shared by many events, for example:

- proportion basis and valuation profile;
- temperament mapping and ambient lattice;
- representative/spelling policy;
- reference pitch and tuning model;
- meter and grouping structures;
- tempo map or temporal-constraint network;
- notation-system identifiers;
- interchange provenance.

Shared context MUST be referenced rather than copied inconsistently into every event.

## 6.4 Projections

Pitch-only, time-only, notation-only, and realization-only views are projections of the event-indexed score.

These projections may intentionally forget information. A projection MUST declare its loss set when used for interchange or round trip.

## 6.5 Score transformations

A score transformation contains:

1. a relation or span between source and destination event identities;
2. transformations of the attached pitch structures;
3. transformations of the attached temporal structures;
4. preservation or transformation rules for notation and provenance.

This supports both independent transformations, such as pure transposition, and dependent transformations, such as pitch changes conditioned on metric position.

When event identity is one-to-one, a simpler function may be used. Splits, merges, insertions, and deletions require a more general relation.

## 6.6 Transformation composition

If a profile claims compositional score morphisms, it MUST define:

- identity transformations;
- composition of event relations;
- composition of pitch components;
- composition of temporal components;
- how residual/provenance records compose.

A label such as "functorial" MUST NOT be used without these operations and their laws.

---

# Part VII -- Realization

## 7.1 Pipeline overview

A common exact-temperament realization path is

```text
L0 notation
   | parse / interpret
   v
L1 exact semantic structure
   | exact mapping V, when applicable
   v
L2 structural classes / organization
   | realization policy
   v
L3 real metric realization
   | device encoding / quantization
   v
L4 device representation
```

This diagram is not a mandatory route for every object. An empirical tuning may enter at L3; notation whose semantics are natively tempered may target L2; temporal-constraint objects need not pass through a temperament map at all. A profile that enters or bypasses this path at another layer MUST record its entry layer and any interpretation, approximation, or structural information that was omitted.

Backward paths are type-specific. UMT-3.2 explicitly rejects the claim that every adjacent pair is the same kind of lens.

## 7.2 L0 <-> L1: notation codecs

The L0/L1 boundary is governed by a **notation codec**, not by the temperament section.

A codec may contain:

- `parse`: notation to one or more semantic interpretations;
- `write`: semantic object to notation;
- an orthographic residual carrying information that the semantic object does not retain;
- representability checks.

A notation codec MAY satisfy a lens-like round-trip law on a declared subset, but such a law is profile-specific and MUST be tested rather than assumed.

Examples of L0 information that may require an orthographic residual include:

- explicit accidental spelling when multiple spellings parse to the same exact interval;
- courtesy accidentals;
- tied notehead boundaries;
- tuplet bracket choices;
- enharmonic layout conventions;
- display-only articulations.

## 7.3 L1 <-> L2: structural quotient lens

For the reachable image $H=\mathrm{im}V$, the map

$$V:\Lambda_B\to H$$

with any representative policy $\sigma:H\to\Lambda_B$ satisfies the set-level lens laws of 1.7.

The exact residual is

$$\rho_\sigma(m)\in K.$$

This residual is not a floating error. It is exact structural information about which lift of the tempered class was present.

If an application uses ambient classes in $\Gamma\setminus H$, the backward detempering operation is partial unless an additional extension policy is supplied.

## 7.4 L2 -> L3: metric realization

L2-to-L3 realization may use:

- a regular interval tuning $\tau$ together with reference data, inducing the affine point realization $\widehat{\tau}$;
- a non-regular contextual realization $\Phi$;
- adaptive lift selection followed by exact/metric valuation;
- a tempo map $\theta$;
- a temporal-constraint solution;
- a continuous pitch or control trajectory.

The map need not be injective.

Therefore "lossless given the choice" is only valid under one of these conditions:

1. the realization map is injective on the represented domain; or
2. the L2 source object is retained alongside the L3 result.

UMT-3.2 uses condition 2 as the default round-trip design.

## 7.5 Common optimization interface

Adaptive pitch realization and expressive time realization share an engineering interface without sharing one mathematical type.

Given structural input $x$, context $c$, typed admissible family $\mathcal{A}(x,c)$, and real-valued objective $J$, define the extended-real infimum value

$$J_*(x,c)=\inf_{y\in\mathcal{A}(x,c)} J(y;x,c).$$

and the exact minimizer set

$$\mathcal{M}(x,c)=\{y\in\mathcal{A}(x,c)\mid J(y;x,c)=J_*(x,c)\}.$$

An exact optimizer selects $y^*\in\mathcal{M}(x,c)$ only when that set is nonempty. When $J_*(x,c)$ is finite, an approximate optimizer MAY instead return $y_\epsilon$ satisfying a declared guarantee such as

$$J(y_\epsilon;x,c)\le J_*(x,c)+\epsilon.$$

The optimizer MUST report infeasibility, an unattained infimum, non-uniqueness when relevant to semantics, and any approximation tolerance rather than fabricating a unique exact minimizer.

Examples:

- adaptive JI: $y$ is a set of exact lifts and/or realized pitches;
- rubato: $y$ is an orientation-preserving time map;
- endpoint-preserving grid realization: $y$ is an integer allocation satisfying parent constraints.

The shared abstraction is constrained optimization with provenance, not equality of the underlying mathematical objects.

## 7.6 L3 -> L4: device realization

A device adapter maps metric values to a representable device state. It MUST declare:

- representable domain;
- quantization or encoding policy;
- saturation/clipping behavior;
- resolution;
- error/residual model;
- any stateful dependencies.

Not every device map is a Galois connection. Floor and ceiling lattice quantizers have the order adjunctions described in 5.7; nearest rounding and stateful encodings generally do not.

## 7.7 Pitch-device quantization

Examples include:

- integer MIDI note plus pitch-bend encoding over a declared bend range;
- MIDI Tuning Standard representations;
- per-note pitch-control encodings;
- fixed-resolution oscillator-control words;
- lookup-table indices.

A DAC sample rate MUST NOT be described as if it quantized sinusoidal pitch to Fourier-bin frequencies. Sampling constrains the discrete-time signal representation and usable bandwidth; the available oscillator frequency resolution depends on the synthesis/control architecture.

## 7.8 Temporal device quantization

Examples include:

- PPQN tick positions;
- sample indices for event scheduling;
- fixed-time control frames;
- hardware timer ticks.

Endpoint-preserving and hierarchy-aware policies SHOULD be preferred when structural duration totals must survive realization.

## 7.9 Residual taxonomy

UMT-3.2 never stores one undifferentiated "error" field.

A realization record may contain:

| Residual type | Domain | Example |
|---|---|---|
| structural residue | exact $K$ element | spelling comma lost by temperament |
| tuning deviation | real interval | realized fifth vs selected just target |
| empirical model residual | real with uncertainty | measured scale step vs fitted lattice |
| temporal realization deviation | real duration | rubato deviation from affine tempo |
| grid residual | real/rational duration | exact onset minus represented tick |
| device-control residual | real control value | requested pitch vs bend code |
| notation residual | symbolic | courtesy accidental or tuplet-display choice |

Residuals MUST preserve units and provenance. They MUST NOT be added numerically unless they live in compatible spaces and the addition is mathematically meaningful.

## 7.10 Provenance

Every non-exact realization or inference that participates in a conformance decision MUST carry provenance sufficient to identify the semantic profile, algorithm/version, and parameters that affect the result. Implementations SHOULD make the result reproducible by recording, as applicable:

- source structural object identifier;
- algorithm/version;
- parameters;
- rounding mode;
- random seed;
- optimization tolerance;
- source measurement identifiers;
- uncertainty model;
- export/import format and version.

---

# Part VIII -- Interchange and Serialization

## 8.1 Principle

No interchange format is assumed to carry the entire UMT-3.2 model. An adapter MUST declare:

1. what it imports;
2. what it exports;
3. which UMT layers are represented;
4. which information is dropped, approximated, or reconstructed;
5. the exact format/version profile tested.

## 8.2 Scala `.scl`

A Scala scale file can contain scale-degree interval entries written either as exact ratios or as cents values.

Therefore an `.scl` file is not uniformly "L3 only":

- a ratio entry can preserve an exact rational interval value;
- a cents entry is a metric real-valued interval specification.

The file by itself does not encode a UMT regular-temperament mapping matrix, comma kernel, or full spelling system.

An importer MUST retain whether each entry was exact-rational or metric-decimal when round-trip fidelity matters.

## 8.3 Scala `.kbm`

A keyboard-mapping file associates keys with scale degrees and reference information. It does not by itself provide the UMT mapping matrix $V$ or comma basis.

A combined `.scl`/`.kbm` import SHOULD preserve both the scale definition and key-degree mapping as distinct objects.

## 8.4 MusicXML

MusicXML can represent substantial L0 notation, including spelling, voices, ties, articulations, time-modification data, and nested tuplet notation.

A MusicXML adapter MUST still be tested against the specific MusicXML version/profile it supports. UMT-specific structures such as an arbitrary regular-temperament mapping or exact comma-residue model require extension metadata or a companion serialization unless a standard representation is explicitly available.

## 8.5 MEI

MEI adapters SHOULD follow the same rule: import and export the structures that the selected MEI version can carry and report UMT-specific data that require extension or companion metadata.

UMT-3.2 does not make a universal claim about all current or future MEI modules.

## 8.6 MIDI and related performance protocols

MIDI-family adapters are primarily performance/realization adapters. Depending on the selected protocol and extension, they may carry:

- note numbers;
- event timing;
- pitch-bend data;
- per-note expressive control;
- tuning messages;
- controller data.

They generally do not, without additional metadata, carry all notation spelling, exact proportion lattices, mapping kernels, rhythm-tree nesting, or analysis annotations.

A MIDI adapter MUST state the exact MIDI version, extension set, pitch-bend range, timing representation, and tuning mechanism used.

## 8.7 MNX

MNX is an evolving specification. A UMT-3.2 adapter MUST pin the exact MNX revision or commit/profile tested and MUST NOT write timeless capability claims such as "MNX carries X" without a version qualifier.

The same version-pinning rule SHOULD be used for any evolving interchange standard.

## 8.8 Native UMT serialization

A native UMT serialization MUST be able to preserve all exact structural information required by the selected conformance profile.

The container itself requires a version and declared profile set. Domain sections are present only when required by the represented objects; this is necessary because UMT permits, for example, direct empirical L3 scales with no L1 basis and domains with no distinguished periodic unit.

```text
umt_version
profiles
basis?
  generator_ids
  exact_rational_values?        # rational profile
  real_values_with_provenance?  # symbolic-real profile
  independence_contract
unit?
  monzo
mapping?
  ambient_rank
  matrix
  image_metadata
  kernel_basis
representative_policy?
  kind
  policy_id?
  algorithm_version?
  parameters
pitch_reference?
tuning_or_realization?
meter_and_grouping?
rhythm_trees?
temporal_constraints?
events?
notation_objects?
realization_records?
residuals?
provenance
```

A custom or adaptive representative policy that cannot be reproduced from a stable policy identifier, version, and parameters MUST serialize the selected lifts/residues actually used by the score when those choices are required for round trip.

The exact wire syntax may be JSON, CBOR, another schema language, or a future standardized format. This document specifies semantic requirements, not one mandatory byte encoding.

## 8.9 Required serialization invariants

A native round-trip MUST preserve, where present:

- monzo integer coordinates exactly;
- mapping matrix entries exactly;
- distinction between ambient group $\Gamma$ and image $H$;
- comma/kernel basis or enough information to recompute it;
- whether a representative policy is homomorphic or merely set-theoretic;
- enough policy identity/parameters or resolved lift data to reproduce semantically relevant choices;
- L0 spelling and tie identity;
- rhythm-tree nesting and ordered child weights;
- exact notated durations;
- chosen tuning/tempo/constraint solver profile;
- continuous trajectory source data or declared approximation;
- residual types and provenance.

---

# Part IX -- Laws and Conformance

## 9.1 Core exact-algebra laws

For a declared basis and mapping, a conforming implementation MUST test the applicable laws below.

### Law P1 -- Free-lattice arithmetic

Monzo addition is associative, has zero, and has additive inverses.

### Law P2 -- Exact rational valuation

In the rational-basis profile,

$$r(m+n)=r(m)r(n)$$

using exact rational arithmetic.

### Law P3 -- Mapping homomorphism

$$V(m+n)=V(m)+V(n).$$

### Law P4 -- Kernel correctness

$$m\in K\iff V(m)=0.$$

### Law P5 -- Kernel saturation for map-derived kernels

For randomly generated $n\ne0$ and $m$,

$$nm\in K\Rightarrow m\in K.$$

This is a theorem for mappings into a free abelian ambient group and SHOULD be used as an implementation check.

### Law P6 -- Direct-comma validation

A directly supplied comma subgroup intended to define a torsion-free regular temperament MUST pass saturation validation or be explicitly saturated with a reported change.

### Law P7 -- Image distinction

The implementation MUST compute or represent $H=\mathrm{im}V$ separately from $\Gamma$ when the image is proper.

### Law P8 -- Right-inverse law

For every representative policy $\sigma$ and $x\in H$,

$$V(\sigma(x))=x.$$

### Law P9 -- Residue law

For all $m$,

$$\rho_\sigma(m)=m-\sigma(V(m))\in K.$$

### Law P10 -- Set-level lens laws

GetPut, PutGet, and PutPut of 1.7.3 MUST hold for the representative policy.

### Law P11 -- Linear-splitting declaration

If $\sigma$ is advertised as homomorphic, the implementation MUST additionally test

$$\sigma(x+y)=\sigma(x)+\sigma(y).$$

No such law is required of an adaptive or minimum-cost representative policy unless it explicitly claims linearity.

## 9.2 Complexity laws

Each complexity function MUST declare one of these profiles, or a profile with explicitly stated equivalent laws:

- `group_length`: $h(0)=0$, $h(-m)=h(m)$, and $h(m+n)\le h(m)+h(n)$. A `group_length` MAY have nonzero elements of zero length; if it also claims separation, it MUST satisfy $h(m)=0\Rightarrow m=0$.
- `lattice_seminorm`: all `group_length` laws plus integer homogeneity

  $$h(nm)=|n|h(m)\qquad(n\in\mathbb{Z}).$$

  Its null set is then a subgroup.
- `lattice_norm`: all `lattice_seminorm` laws plus identity of indiscernibles, $h(m)=0\Leftrightarrow m=0$. Equivalently for present purposes, this is the restriction to the lattice of a norm-compatible homogeneous length when such an extension is supplied.
- `cost`: no length/norm laws are implied beyond nonnegativity unless separately stated.

The unqualified profile name `norm` is deliberately not used, because a norm on a vector space includes scalar homogeneity while terminology for norms/lengths on abstract groups is not uniform.

The weighted function $h_{1,w}$ of 1.3.1 is a `lattice_norm`. Tenney height on prime coordinates is also a `lattice_norm` and MUST satisfy the reduced-rational identity in 1.3.2 within numerical tolerance of the logarithm evaluation.

## 9.3 Tuning laws

### Law T1 -- Regular tuning homomorphism

For a regular tuning,

$$\tau(x+y)=\tau(x)+\tau(y).$$

### Law T2 -- Comma error

For $k\in K$,

$$\varepsilon(k)=-\pi(k).$$

### Law T3 -- Non-regular realizations are not falsely linear

A realization that depends on register, context, or time MUST NOT be serialized or advertised as a regular homomorphism unless it actually satisfies the homomorphism law on its domain.

## 9.4 Point-space laws

For torsor points $p,q,r$ and intervals $g,h$:

1. $(p+g)+h=p+(g+h)$;
2. $p+0=p$;
3. $p+\mathrm{int}(p,q)=q$;
4. $\mathrm{int}(p,q)+\mathrm{int}(q,r)=\mathrm{int}(p,r)$.

No `point + point` operation belongs to the core torsor interface.

## 9.5 Voice-leading laws

A profile that claims a **metric** MUST test identity of indiscernibles, symmetry, and triangle inequality on the actual represented state space.

Classical $W_p$ metric claims require:

- $p\ge1$;
- a metric ground cost;
- equal total mass;
- the standard admissible coupling set.

Unequal-mass profiles MUST test the laws of their chosen unbalanced/edit metric rather than inheriting classical Wasserstein claims automatically.

A declared-span cost MUST NOT be called a chord metric unless the required metric laws have actually been proved or tested for the optimization over spans that defines it.

## 9.6 Notation laws

A notation codec MUST declare its round-trip domain.

Possible laws include:

- for a relation-valued parser, `x in parse(write(x))` on writable semantic objects;
- the stronger equality `parse(write(x)) = x` only in a profile whose parser is single-valued on writer output;
- for a canonical spelling `s` with unique semantic interpretation `x`, `write(x) = s`;
- a noncanonical `s` round-trips exactly only when the orthographic residual needed to reconstruct it is retained;
- tie identities survive L0 round trip;
- ambiguous parses remain ambiguous unless an explicit context or selection rule resolves them.

## 9.7 Rhythm-tree laws

For exact rhythm trees:

1. child weights are positive;
2. flattened child spans exactly partition the parent structural span;
3. child order is preserved;
4. recursive flattening preserves the root total duration;
5. serialization preserves tree topology and weights.

## 9.8 Quantization laws

### Floor profile

- monotone;
- identity on grid values;
- $i(q_\downarrow(x))\le x$;
- residual $e_\downarrow\ge0$.

### Ceiling profile

- monotone;
- identity on grid values;
- $x\le i(q_\uparrow(x))$;
- residual $e_\uparrow\le0$ under the convention $e=x-i(q(x))$.

### Nearest profile

- identity on grid values;
- bounded residual according to grid spacing and tie policy;
- no universal one-sided inequality.

### Endpoint-preserving allocation profile

For each parent span with integer target duration $N$,

$$\sum_i n_i=N.$$

Every child residual MUST be reported relative to the exact structural child duration. If the profile requires positive integer child spans or distinct device onsets, infeasible allocations MUST be reported rather than silently collapsed.

## 9.9 Tempo-map laws

A tempo map in the homeomorphism profile MUST be:

- continuous;
- strictly increasing;
- bijective between its declared source and target intervals;
- endpoint-consistent.

If derivative-based tempo is exposed, the profile MUST separately specify differentiability or absolute-continuity requirements and MUST distinguish clock-time-per-beat $\theta'$ from its reciprocal beat rate.

## 9.10 Temporal-constraint laws

### STP profile

- every constraint is a difference bound;
- consistency is determined using a correct difference-constraint algorithm;
- a negative cycle or equivalent contradiction is reported;
- tight implied pairwise bounds are computed when requested.

### Linear-ratio profile

- every ratio denominator has an enforced positive-duration condition;
- a non-strict lower bound $\delta>0$ is used only when justified by the model/source, never invented solely for solver convenience;
- transformed inequalities are sent to a solver that correctly handles the resulting linear constraints and any strict inequalities;
- the result is not labeled an STP shortest-path proof unless the instance actually reduces to difference constraints.

### External-predicate profile

- every predicate has a type and evaluation contract;
- unresolved predicates are surfaced;
- no universal decidability claim is made.

## 9.11 Generated-structure laws

A generated-set implementation MUST:

- preserve the designated $p$ and $g$;
- handle orbit closure and duplicates explicitly;
- compute circular gaps from sorted distinct points;
- distinguish ordinary generated cardinalities from cardinalities satisfying the selected MOS/well-formed predicate.

A Euclidean-rhythm implementation MUST declare its rotation convention and verify maximal evenness under the selected definition.

## 9.12 Realization/provenance laws

1. Exact source data required for round trip are never reconstructed from L3 floating point when the exact source was available.
2. Every lossy adapter emits a loss record.
3. Every metric or empirical value used in an exact-to-real decision carries provenance.
4. Residuals are type-tagged and unit-tagged.
5. A later re-realization may use the original structural source rather than compounding previous device rounding.

## 9.13 Mandatory adversarial fixtures

The conformance corpus MUST include at least these concrete cases.

### F1 -- Kernel saturation versus surjectivity

Mapping:

$$V=[2]:\mathbb{Z}\to\mathbb{Z}.$$

Expected:

- kernel $0$, saturated;
- image $2\mathbb{Z}$;
- mapping not surjective to ambient $\mathbb{Z}$.

### F2 -- Unsaturated direct comma subgroup

Input subgroup generated by twice a primitive comma.

Expected:

- direct-comma validator reports torsion/nonsaturation;
- map-derived-kernel validator is not misapplied.

### F3 -- 12-EDO quotient

5-limit mapping with syntonic and Pythagorean commas killed.

Expected structural reachable quotient: free rank 1, isomorphic to $\mathbb{Z}$ before octave equivalence.

After octave equivalence with octave mapped to 12 steps, pitch classes form $\mathbb{Z}/12\mathbb{Z}$.

### F4 -- 6-EDO image

Patent val $[6,10,14]$.

Expected:

- ambient $\Gamma=\mathbb{Z}$;
- image $H=2\mathbb{Z}$;
- an odd EDO step has no automatic L1 detempering under this mapping.

### F5 -- Height with a generator below 1

Use an arbitrary symbolic basis whose real valuation includes a generator below 1.

Expected:

- generic complexity uses explicit positive weights;
- implementation does not blindly use $\log_2 b_i$ as a positive norm weight.

### F6 -- Nonhomomorphic representative policy

Choose a minimum-cost or hand-coded right inverse that violates additivity.

Expected:

- right-inverse and lens laws pass;
- homomorphism law fails and is not claimed;
- direct-sum group decomposition is not attributed to this policy.

### F7 -- Enharmonic spelling

Two L1 spellings mapping to the same 12-EDO L2 class.

Expected:

- distinct L0/L1 objects preserved;
- equal L2 class;
- distinct exact comma residues relative to a canonical lift when applicable.

### F8 -- Unequal voice count

Compare a one-voice unison state with a two-voice doubled unison state.

Expected:

- equal-mass probability normalization is rejected for a multiplicity-sensitive metric unless explicitly selected;
- unbalanced/edit profile reports the configured birth/split cost.

### F9 -- Tie round trip

Two tied noteheads across a barline.

Expected:

- two L0 noteheads and tie relation survive;
- realization may contain one sustained sounding gesture;
- exporting back to a capable notation format reconstructs the two noteheads and tie.

### F10 -- 6/8 versus 3/4

Equal total span and pulse resolution.

Expected: metrical hierarchy distinguishes the primary-beat point sets.

### F11 -- Additive 2+2+3

Expected: weighted ordered tree/grouping preserves child weights and flattens to exact boundaries $0,2,4,7$ in the chosen unit.

### F12 -- Naive quintuplet floor

$P=96$, five equal children in one 96-tick parent.

Expected local floor durations: $19,19,19,19,19$ and endpoint total 95.

### F13 -- Endpoint-preserving quintuplet

Same source.

Expected: integer children sum to 96, for example $19,19,20,19,19$ under the declared policy, with per-child residuals recorded.

### F14 -- Nested tuplet re-realization

Realize a nested 5-inside-3 tree at two PPQN values.

Expected: both realizations derive from the same exact source tree, not from re-quantizing the first tick sequence.

### F15 -- Strictly increasing but discontinuous candidate

Provide a strictly increasing map that has a jump and therefore is not a homeomorphism onto the desired continuous interval.

Expected: rejected by the homeomorphism tempo profile.

### F16 -- STP contradiction

Constraints imply both $t_2-t_1\le1$ and $t_2-t_1\ge2$.

Expected: STP solver reports inconsistency.

### F17 -- Ratio constraint not representable as a difference edge

Use a three-event bounded ratio constraint.

Expected: routed to linear-ratio profile, not silently passed to Floyd-Warshall as one graph edge.

### F18 -- External temporal predicate

Use "enter after detected decay threshold".

Expected: predicate remains typed external data unless an acoustic detector is configured; no static-decidability claim.

### F19 -- Inharmonic empirical scale

Import measured real scale degrees with uncertainty.

Expected: valid L3 representation without forced rationalization; optional fitted lattice stored separately with residuals.

### F20 -- Continuous pitch

A vibrato and a continuous glissando.

Expected: trajectory survives native round trip and device export records any sampling/quantization approximation.

### F21 -- Scala mixed exact/metric entries

Import an `.scl` containing at least one rational ratio and at least one cents value.

Expected: importer preserves the distinction.

### F22 -- Reachable versus ambient octave classes

Use the 6-EDO mapping $[6,10,14]$ with $H=2\mathbb{Z}$, $\Gamma=\mathbb{Z}$, and octave image $6$.

Expected:

- reachable quotient $H/6\mathbb{Z}\cong\mathbb{Z}/3\mathbb{Z}$;
- ambient quotient $\Gamma/6\mathbb{Z}\cong\mathbb{Z}/6\mathbb{Z}$;
- the two are not silently identified.

### F23 -- Unmeasured event without fixed onset

Create a notated event whose onset is a temporal variable constrained only to occur after another event.

Expected: valid score object without fabricating a rational structural onset.

### F24 -- Global control event without voice

Create a global tempo/control marker.

Expected: valid event scope with no fake voice identity.

### F25 -- Strict ratio positivity

Use a ratio constraint whose denominator duration is required only to be strictly positive and for which no model-derived positive lower bound is known.

Expected: the solver preserves the strict inequality or reports unsupported strict constraints; it does not invent an arbitrary $\delta$.

### F26 -- Unattained optimization infimum

Use admissible set $(0,1)$ and objective $J(y)=y$.

Expected: infimum $0$ is reported but the exact minimizer set is empty; no exact $y^*$ is fabricated.

### F27 -- Infeasible positive-span device allocation

Quantize three children, each requiring at least one tick, into a two-tick parent.

Expected: infeasibility or an explicitly selected collapse policy is reported.

### F28 -- Context-dependent realization typing

Use one structural pitch point with different instrument/time contexts that produce different L3 realizations.

Expected: the realization is represented as $\Phi(x,c)$ or an equivalent family $\Phi_c(x)$, not falsely serialized as one unary context-free map.

### F29 -- Direct empirical object without a unit

Serialize an L3 empirical scale profile that has no exact basis and no distinguished periodic unit.

Expected: native serialization remains valid with optional `basis` and `unit` sections absent.

### F30 -- GitHub math-source and normative-vocabulary lint

Scan the normative Markdown source.

Expected:

- zero occurrences of the avoided named-operator macro;
- named functions use the source-compatible alternatives selected by this revision;
- uppercase normative keywords are drawn only from the vocabulary declared in 0.3.

### F31 -- Regular interval tuning requires a point reference

Provide a regular interval tuning $\tau:G_2\to\mathbb{R}$ but no mapping from a structural reference pitch point to a realized reference point.

Expected: interval sizes can be evaluated, but absolute pitch points are not considered fully realized until reference data determine the affine map $\widehat{\tau}:P_2\to P_3$.

### F32 -- Reciprocal rate/duration orientation

Apply a declared ratio $\rho=3/2$ to a positive rate for a fixed cycle count and compare the corresponding duration.

Expected: the rate is multiplied by $3/2$ while the reciprocal duration is multiplied by $2/3$; an adapter that labels both changes as the same directed ratio without an orientation declaration fails conformance.

### F33 -- Saturation excludes the zero multiplier

Take $K=\{0\}\le\mathbb{Z}$, $x=1$, and $n=0$.

Expected: the fact $nx=0\in K$ MUST NOT be used to infer $x\in K$. Saturation checks quantify only over nonzero integer multipliers.

### F34 -- Group length versus lattice norm

Let $h_1$ be a nonzero weighted $\ell_1$ lattice norm and define

$$g(m)=\sqrt{h_1(m)}.$$

Expected: $g$ satisfies the separating `group_length` laws but fails integer homogeneity in general, since $g(4m)=2g(m)\ne4g(m)$ for $m\ne0$. It MUST NOT be advertised as a `lattice_norm`.

### F35 -- Three-note quarter-comma-meantone MOS

Take period $p=1200$ cents and quarter-comma-meantone generator

$$g=300\log_2 5\approx696.578\text{ cents}.$$

Generate three points modulo $p$ and sort them.

Expected: the circular gaps consist of two positive sizes (approximately $193.157$ and $503.422$ cents), so cardinality $3$ satisfies the operational MOS predicate of 3.3.

---

# Part X -- Known Gaps and Open Problems

## 10.1 Perceptual grounding

UMT-3.2 intentionally separates exact ratio complexity from sensory dissonance. A complete empirical account linking arithmetic, spectrum, context, culture, and perception remains outside the specification.

## 10.2 Rate-continuum perception

The transition among individually perceived events, modulation, flutter, roughness, periodicity pitch, and fused tone depends on stimulus details. UMT-3.2 therefore has no single formal cutoff. A future perceptual extension could supply parameterized auditory models.

## 10.3 Adaptive JI optimization at scale

Global adaptive tuning can become a combinatorial or continuous optimization problem over long passages. UMT-3.2 specifies the representation and objective interface but not one universally optimal algorithm.

## 10.4 Historical and culture-specific pitch models

The core can carry exact, empirical, and spectrum-conditioned tunings, but it does not decide which model is historically or culturally appropriate for a repertoire.

## 10.5 Voice-leading semantics

Spans, assignment costs, and unbalanced transport cover many structural cases, but musical identity of voices can depend on instrumentation, register, notation, phrasing, and perceptual streaming. One universal metric is not mandated.

## 10.6 Temporal constraints beyond STP and linear ratios

Rich performance instructions may require nonlinear dynamics, stochastic constraints, cue graphs, acoustic-state predicates, or interactive systems. UMT-3.2 makes solver capability explicit rather than pretending all such networks have one polynomial shortest-path algorithm.

## 10.7 Tala and other highly structured temporal systems

The weighted-tree, meter, grouping, and constraint primitives are intended to be extensible, but conformance for Carnatic tala and other sophisticated systems requires dedicated repertoire-aware studies.

## 10.8 Timbre-basis coupling over time

A time-varying spectrum may imply time-varying consonance candidates. Whether a useful symbolic lattice should also vary over time is an open modeling question.

## 10.9 Native schema standardization

Part VIII specifies semantic fields but not one canonical wire format. A production implementation should define a versioned schema, canonical integer-matrix encoding, and stable identifiers.

## 10.10 Formal proof coverage

Several algebraic claims in UMT-3.2 are elementary theorems; others are API contracts. A future formalization in Lean, Coq, Agda, or another proof assistant would be valuable for separating theorem obligations from conformance tests.

---

# Appendix A -- Core Data-Type Sketch

This appendix is informative. It gives implementation-oriented pseudotypes without requiring a particular programming language.

```text
type IntVector = vector<int>
type IntMatrix = matrix<int>
type Rational = exact_fraction<int>

type BasisGenerator = {
    id: string,
    rational_value?: Rational,
    real_value?: RealWithProvenance,
}

type Basis = {
    generators: list<BasisGenerator>,
    independence: IndependenceContract,
}

type Monzo = IntVector  # length = rank(Basis)

type TemperamentMap = {
    ambient_rank: int,
    matrix: IntMatrix,
    image_snf: SmithNormalForm,
    kernel_basis: list<Monzo>,
}

type RepresentativePolicy = {
    domain: ImageLattice,
    kind: "linear_split" | "minimum_cost" | "adaptive" | "custom",
    policy_id?: string,
    algorithm_version?: string,
    parameters: object,
    resolved_lifts_if_needed?: object,
}

type StructuralResidue = {
    kernel_coordinates: IntVector,
    monzo: Monzo,
}

type PitchSpelling = {
    notation_system: string,
    symbol_data: object,
    semantic_pitch?: ExactPitch,
    orthographic_residual?: object,
}

type VoiceId = stable_id
type EventId = stable_id

type Chord = map<VoiceId, PitchPoint>

type VoiceLeadingSpan = {
    edges: list<{source?: VoiceId, target?: VoiceId}>,
    policy: string,
}

type RhythmTree = {
    exact_span: Rational,
    children: list<{weight: Rational, node: RhythmTreeOrEvent}>,
}

type EventScope = VoiceLocal | StaffLocal | PartLocal | Global

type TemporalPlacement =
      FixedSpan{onset: Rational, duration: Rational}
    | ConstraintPlacement{onset_var: TimeVarId, offset_var?: TimeVarId}
    | GracePlacement{anchor: EventId, rule: GraceRule}

type NotatedEvent = {
    id: EventId,
    scope: EventScope,
    voice?: VoiceId,  # required when scope = VoiceLocal
    placement: TemporalPlacement,
    kind: EventKind,
    pitch_spelling?: PitchSpelling,
    tie_links?: list<EventId>,
    rhythm_tree_path?: list<int>,
}

type TempoMap = OrientationPreservingHomeomorphism

type ExternalPredicateRef = {
    namespace: string,
    predicate_id: string,
    version: string,
    parameters: object,
}

type TemporalConstraint =
      DifferenceBound
    | LinearRatioBound
    | ExternalPredicateRef

type Residual =
      StructuralCommaResidue
    | TuningDeviation
    | EmpiricalFitResidual
    | TimeRealizationResidual
    | GridResidual
    | DeviceResidual
    | OrthographicResidual
```

---

# Appendix B -- Principal Corrections from UMT-2

This appendix records the substantive UMT-2 claims changed in UMT-3.2.

1. **Unified proportion algebra, not complete identity.** Pitch and rhythm share multiplicative proportion structure, but each has additional domain-specific structure.
2. **No unique 20 Hz axiom.** Perceptual transition ranges are empirical model parameters, not the sole formal divergence point.
3. **Signal addition is typed separately.** Linear superposition, beating/envelope analysis, nonlinear intermodulation, and duration concatenation are not one additive operation.
4. **Height fixed.** Tenney's reduced-rational formula is restricted to prime coordinates; generic basis complexity uses positive declared weights; octave-equivalent measures may be seminorms.
5. **Kernel saturation fixed.** A kernel of a map into a free abelian group is automatically saturated. Maximal-minor gcd conditions concern image primitivity/surjectivity, not kernel saturation.
6. **Direct comma subgroups separated.** Saturation validation remains necessary when commas are supplied independently.
7. **Mappings need not be surjective.** The image $H$ is distinguished from ambient $\Gamma$.
8. **6-EDO fixed.** Patent val $[6,10,14]$ reaches $2\mathbb{Z}$ inside the ambient step lattice $\mathbb{Z}$.
9. **Unit typing fixed.** The distinguished unit is a lattice element $\hat u$ whose physical valuation is a separate real quantity.
10. **Two section notions separated.** Homomorphic splitting and arbitrary representative policy are different types.
11. **Lens theorem narrowed correctly.** The set-level lens laws hold for any right inverse on the reachable image; direct-sum group decomposition requires a homomorphic splitting.
12. **Notation is not a section.** A section may define a canonical lift, while actual notation is an L0 spelling parsed to an exact L1 interpretation.
13. **L0/L1 no longer mislabeled as the temperament lens.** It is a notation codec with its own possible residuals and round-trip laws.
14. **Regular tuning narrowed.** A fixed homomorphism models translation-invariant tuning; register-dependent stretch and irregular systems use non-regular realization maps.
15. **Octave-distance rule weakened.** Pitch-space and pitch-class metrics are both valid if their domains are explicit.
16. **Generated-scale claims narrowed.** Rank-2 structure does not canonically pick period/generator; intervening generated cardinalities still exist; MOS and well-formed terminology are not blindly equated.
17. **Christoffel/Sturmian wording fixed.** Finite Christoffel relationships are distinguished from infinite Sturmian words.
18. **Voice-leading transport fixed.** Balanced Wasserstein requires equal mass; unequal voice counts require unbalanced/partial/edit treatment or explicit normalization semantics.
19. **Sethares/Tenney identity removed.** Arithmetic complexity and spectrum-conditioned dissonance are distinct models.
20. **Inharmonic basis inference made noncanonical.** Empirical minima can suggest candidates but do not automatically define an exact independent basis.
21. **Ties fixed.** L0 preserves tied noteheads and the tie relation; lower realization layers may combine them acoustically.
22. **Rests fixed.** A notated rest is voice-local notation, not the complement of global sounding intervals.
23. **Polyrhythm/polymeter labeled as an operational convention.** The underlying separate metric structures are primary.
24. **Grid adjunction direction fixed.** Inclusion is left adjoint to floor; ceiling is left adjoint to inclusion.
25. **Quantization error wording fixed.** The numeric residual is derived from the adjunction comparison; it is not literally the unit/counit natural transformation.
26. **Nearest rounding separated.** It does not inherit floor's one-sided Galois law.
27. **PPQN example fixed.** Five floored quintuplet durations drift to 95 ticks, while endpoint-preserving allocation can sum to 96 with local residuals.
28. **Rubato/adaptive JI identity removed.** They implement a common constrained-realization optimization interface but have different mathematical types.
29. **Tempo map strengthened.** The homeomorphism profile requires continuity and bijection in addition to strict increase.
30. **TCN solver claims split by language.** STP difference constraints use shortest paths; ratio constraints generally require linear/convex solving; arbitrary external predicates have no universal decidability guarantee.
31. **Performer agency no longer equals dimension.** Multiple feasible-set descriptors may be reported.
32. **Score/product criticism corrected.** A bare pair of pitch/time marginals loses incidence; categorical products are not intrinsically incapable of correlation when richer indexed factors are used.
33. **L2/L3 round-trip fixed.** Realization is not assumed injective; exact source structure is retained for lossless round trip.
34. **DAC sample rate removed as pitch-bin quantization.** Device pitch resolution depends on the actual synthesis/control representation.
35. **Scala classification fixed.** `.scl` entries may be exact ratios or cents values.
36. **Interchange claims version-pinned.** Evolving formats such as MNX are not assigned timeless capability claims.

---

# Appendix C -- Principal Corrections from UMT-3 to UMT-3.1

This revision resulted from a second full type/adequacy audit of UMT-3.

1. **GitHub math compatibility.** Removed every use of the avoided named-operator macro from the normative Markdown and replaced named functions with `\mathrm{...}` or ordinary set notation.
2. **Real valuation completed.** Explicitly extended $\nu_3$ from basis generators to arbitrary monzos before using $\nu_3(m)$.
3. **Logarithmic unit orientation fixed.** The standard logarithmic coordinate now requires $u>1$; inverse units handle naturally subunit valuations without reversing order silently.
4. **Patent-val typing fixed.** EDO rounding now applies to the real valuation $\nu_3(\beta_i)$, not to a formal generator symbol.
5. **Context-dependent realization typed correctly.** Non-regular pitch realization is $\Phi:P_2\times C\to P_3$ (or a family $\Phi_c$), so dependence on time/instrument/context is explicit.
6. **Reachable versus ambient unit quotients separated.** Octave equivalence records whether it is formed on $H$ or $\Gamma$; 6-EDO exposes the resulting 3-class versus 6-class distinction.
7. **Unmeasured event model fixed.** Notated events may reference temporal variables/constraints instead of requiring a fabricated fixed onset.
8. **Event scope fixed.** Global/staff/part events no longer require fake voice identities.
9. **Grid domain completed.** $P$ is explicitly a positive integer; exact rational residuals remain exact where possible.
10. **Hierarchical quantization feasibility added.** Positive-span/distinct-onset constraints can make integer allocation infeasible and must be reported.
11. **Tempo derivative units fixed.** $\theta'$ is clock-time per beat; beat rate is its reciprocal. Zero-structural-duration pauses are explicitly outside the homeomorphism profile unless modeled by an augmented span or constraints.
12. **Ratio-TCN strictness fixed.** A solver may not invent $\delta>0$ to replace a strict positive denominator condition; that substitution is allowed only when the lower bound is semantically justified.
13. **External predicates made portable/safe.** Serialization carries named predicate contracts rather than requiring arbitrary executable callbacks.
14. **Pipeline optionality fixed.** The L0-to-L4 diagram is now explicitly one common path rather than a universal route.
15. **Optimization existence fixed.** The spec defines an infimum and minimizer set, handles empty/nonunique minimizers, and gives an explicit approximate-solution contract.
16. **Native schema optionality fixed.** Basis, unit, mapping, and other domain sections are optional according to profile; adaptive/custom representative choices must be reproducible or materialized as resolved lift data.
17. **Relation-valued notation laws fixed.** Round-trip laws no longer apply function equality to an ambiguous parser without qualification.
18. **Regular point realization completed.** A regular interval tuning now induces a pitch-point realization only after a structural and physical reference pair is supplied; the resulting map is affine/equivariant rather than the interval homomorphism itself.
19. **Rate/duration orientation fixed.** A proportion multiplying a rate by $\rho$ multiplies its reciprocal duration by $\rho^{-1}$ for a fixed cycle count; the interpreted quantity and direction must be declared.
20. **Conformance corpus expanded.** Added fixtures F22--F32 for the newly corrected cases.

---


# Appendix D -- Principal Corrections from UMT-3.1 to UMT-3.2

This revision incorporates an independent audit of UMT-3.1 after checking the audit claims against the specification and, where terminology depended on external literature, against the relevant sources.

1. **A4 cross-reference fixed.** The common optimization interface is in 7.5, not 7.3.
2. **Saturation quantifier fixed.** The kernel-saturation implication now explicitly quantifies over nonzero integer multipliers; the zero multiplier is excluded.
3. **Normative vocabulary normalized.** The sole undeclared uppercase normative synonym was replaced by the declared `MUST`/`MAY` vocabulary.
4. **Three-note meantone MOS clarified.** Under UMT's operational two-gap predicate, the quarter-comma-meantone example explicitly includes cardinality 3. The audit's numerical gap values are understood specifically for quarter-comma meantone, not for every member of the meantone family.
5. **Complexity taxonomy made unambiguous.** Instead of assuming one universal meaning of the word `norm`, conformance now distinguishes `group_length`, `lattice_seminorm`, `lattice_norm`, and unconstrained `cost`. Integer homogeneity is mandatory for the lattice seminorm/norm profiles, while a nonhomogeneous group length remains representable under its correct type.
6. **Octave-equivalent seminorm wording tightened.** Vanishing on the octave subgroup no longer, by itself, implies the seminorm laws; quotient norm claims now depend on the full null subgroup.
7. **Source provenance improved.** The reference appendix records the SHA-256 of the immediate UMT-3.1 predecessor and describes the pinned UMT-2 gist as historical ancestry rather than as the document directly rewritten by this revision.
8. **Conformance fixtures expanded.** F30 now checks normative-keyword vocabulary; F33 tests the zero-multiplier saturation trap; F34 separates group length from lattice norm; F35 verifies the three-note quarter-comma-meantone MOS example.

The audit's note about Sethares's `consance.html` URL was correct and required no change. The MIDI Tuning URL was independently confirmed and likewise remains unchanged.

---

# Appendix E -- Reference Notes

These references are informative and identify the source revision and external concepts used or corrected by UMT-3.2. They are not a claim that each source endorses UMT-3.2 as a whole.

0a. Immediate predecessor artifact for this revision: `UMT-3.1.md`, SHA-256 `8449afae46882da37291732240864c6563270143eee730da091dcf5c78d6434e`.

0b. Historical UMT-2 source revision from which the UMT-3 series descends: https://gist.githubusercontent.com/metastable-void/d83c6140af47a274d10bc5e32d934443/raw/7cf85753da06560dc78ff6c0d32317f6abb211a1/umt.md

1. Rina Dechter, Itay Meiri, Judea Pearl, "Temporal Constraint Networks," *Artificial Intelligence* 49 (1991). The paper distinguishes simple temporal problems from more general temporal constraint problems and gives the graph/shortest-path treatment for the simple difference-constraint case. UCLA reprint: https://ftp.cs.ucla.edu/pub/stat_ser/r113-L-reprint.pdf

2. William A. Sethares, "Local Consonance and the Relationship Between Timbre and Scale," *Journal of the Acoustical Society of America* 94 (1993). Author page: https://sethares.engr.wisc.edu/papers/consance.html

3. Scala scale-file format, Huygens-Fokker Foundation. The format permits scale entries written as ratios or cents values: https://www.huygens-fokker.org/scala/scl_format.html

4. MusicXML 4.0 reference and schema, including `time-modification`, `actual-notes`, `normal-notes`, and nested tuplet examples: https://www.w3.org/2021/06/musicxml40/

5. Lenaic Chizat, Gabriel Peyre, Bernhard Schmitzer, Francois-Xavier Vialard, "Unbalanced Optimal Transport: Dynamic and Kantorovich Formulation," arXiv:1508.05216. This provides transport models that allow creation/destruction of mass rather than requiring equal total mass: https://arxiv.org/abs/1508.05216

6. Norman Carey and David Clampitt, "Aspects of Well-Formed Scales," *Music Theory Spectrum* 11(2), 1989, pp. 187--206. DOI landing page: https://academic.oup.com/mts/article-abstract/11/2/187/1088094

7. Three-Gap Theorem literature: e.g. the survey/result page for "The Three Gap Theorem (Steinhaus Conjecture)" in *Journal of the Australian Mathematical Society*: https://www.cambridge.org/core/journals/journal-of-the-australian-mathematical-society/article/three-gap-theorem-steinhaus-conjecture/EA75E140919DEA9A55FEFD01EB2F677F

8. Genevieve Paquin, "On a generalization of Christoffel words: epichristoffel words," arXiv:0805.4174, for the finite-Christoffel/infinite-Sturmian distinction: https://arxiv.org/abs/0805.4174

9. W3C Music Notation Community Group, for current MusicXML/MNX development and version status: https://www.w3.org/community/music-notation/

10. MIDI Association, MIDI Tuning specification information: https://midi.org/midi-tuning-updated-specification

11. GitHub Docs, "Writing mathematical expressions," for the Markdown math-rendering environment used by the publication target: https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/writing-mathematical-expressions

---

# Appendix F -- Design Summary

UMT-3.2 can be summarized in one sentence:

> Keep exact symbolic proportion, structural quotient, metric realization, and device approximation separate; unify pitch and rhythm only where they genuinely share a typed construction, and preserve every other difference explicitly.

The resulting architecture is intentionally less rhetorically dramatic than UMT-2. Its central claims are stated with explicit type contracts, adversarial fixtures, and implementation-oriented conformance obligations so that remaining failures can be localized and corrected rather than hidden by analogy.
