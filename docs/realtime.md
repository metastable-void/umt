# Realtime boundary

The semantic core of this crate is **not** realtime-safe, does not try to be,
and should not be used on an audio thread. It allocates, it uses
arbitrary-precision arithmetic, and it runs normal-form, graph, and
optimization algorithms whose running time depends on the data.

That is the correct design for a theory library. Exactness above L3 is
UMT-3.2's central requirement (section 0.6.1), and exactness costs allocation.

The boundary between that world and a realtime consumer is
`realization::PerformancePlan`. This document states what compiling one buys,
and - just as importantly - what it does not.

## What the core does that a realtime thread must not

| Operation | Why it is not realtime-safe |
|---|---|
| `Monzo`, `BeatTime`, `Q`, `Z` arithmetic | arbitrary precision, allocates |
| `TemperamentMap::new` | Smith and Hermite normal forms |
| `MinimumComplexityPolicy::choose` | a bounded but data-dependent search |
| `SpanCostModel::minimum_over_assignments` | exhaustive enumeration |
| `LinearTemporalProblem::solve` | Fourier-Motzkin elimination, exponential in the worst case |
| `StpProblem::solve` | all-pairs shortest paths over rationals |
| `TickGrid::quantize_tree` | exact rational arithmetic throughout |
| serialization | allocates, and formats text |

None of these is a defect. They are all authoring-time work, and a plan is how
that work stops being repeated.

## What a compiled plan guarantees

`PerformancePlan::realtime_contract()` returns a `RealtimeContract` whose five
fields are each a property the build step established:

- **`no_allocation_on_read`** - `events()` and `events_in()` return borrowed
  slices of the plan's own storage. `events_in` is two binary searches; it
  copies nothing and its result shares the plan's allocation, which a unit test
  asserts by comparing pointers.
- **`no_arbitrary_precision`** - every stored value is a `u32`, `i32`, or
  `u16`. No `BigInt` or `BigRational` is reachable from a `PlannedEvent`.
- **`bounded_ranges_validated`** - `MAX_TICK` and `MAX_MILLICENTS` are checked
  when each event is added, so a reader needs no range checks of its own.
- **`device_mapping_resolved`** - voices are `u16` indices and pitches are
  `i32` millicents from a stored reference. No lookup by name, and no string
  comparison, happens on the performance thread.
- **`events_sorted`** - sorted once at build time, by a total derived
  ordering, so the order is deterministic and a reader can seek.

`PerformancePlan` and `PlannedEvent` are `Send + Sync`, so a plan can be built
on one thread and read on another. A test asserts that too, because it is part
of the contract rather than an accident of the fields.

## What it does not guarantee

- **Building a plan is not realtime-safe.** `PerformancePlanBuilder::build`
  allocates, sorts, and validates. That is the whole point of it being a
  separate step.
- **There is no `RealtimeSafe` marker trait.** Prompt section 38 forbids
  claiming one without a contract the type actually satisfies, so the claim is
  a value with named fields rather than a trait that cannot be checked.
- **Nothing here bounds the consumer.** A plan makes no promise about what a
  caller does with the events it reads.
- **No lock-free structure is provided.** Handing a plan to an audio thread -
  by `Arc` swap, by a triple buffer, by whatever the host uses - is the host's
  problem, and a good one for it to own.
- **No audio, no scheduling, no device I/O.** A plan is data.

## Re-realization

Because a plan is compiled *from* a score rather than by mutating one (prompt
section 37), re-realizing at a different resolution, tuning, or tempo policy
means compiling again from the exact source. It never means unpicking a
previous result, which is what UMT-3.2 section 9.12 asks for: "a later
re-realization may use the original structural source rather than compounding
previous device rounding".

The residuals a compilation cost are recorded on the plan, so the previous
result can be *reported* without being reused.
