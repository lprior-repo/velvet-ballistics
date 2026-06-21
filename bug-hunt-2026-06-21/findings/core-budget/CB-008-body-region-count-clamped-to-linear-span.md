# CB-008: `count_body_region_nodes` clamps count to a linear `body_span` heuristic

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_loop.rs:88`
- **Confidence**: confirmed

## Description

After a graph walk produces a step count for a loop body, the function
returns `count.max(u64::from(done.get().saturating_sub(body.get()).saturating_sub(1)))`.
This clamps the graph-walk count up to a "linear span" computed from the
raw index difference between `body` and `done`. If the IR is not laid out
contiguously between `body` and `done` (e.g. dead code embedded between
them, or the body jumps forward past `done`), the budget silently inflates
to the linear span — masking the graph walk's actual answer.

## Evidence

```rust
let body_span = done.get().saturating_sub(body.get()).saturating_sub(1);
Ok(count.max(u64::from(body_span)))
```

(`crates/vb_core/src/budget/traversal_loop.rs:88-89`)

If `body.get() == 5`, `done.get() == 50`, the floor becomes 44, even if the
graph walk only reached 10 nodes. If the body actually lives in
`[5, 50)` and the graph walk *should* have found 44 nodes but missed some,
the max hides the bug; if the body is a small sub-graph that happens to be
embedded in a wide index range, the budget is overstated.

## Adversarial Check

One might argue the floor is a defensive lower bound to compensate for
graph-walk under-counting. But the graph walk uses `region_visited`, which
should be authoritative. If the walk under-counts, the fix is to repair the
walk — not to mask the result with a heuristic whose own correctness
depends on the same IR layout assumption flagged in CB-007. Two wrong
heuristics do not make a right answer, and the heuristic also fails
silently when `done < body` (saturating to 0) — giving inconsistent
behavior depending on the index ordering.

## Suggested Fix

Drop the `body_span` clamp and trust the visited-bounded graph walk. If
under-counting is a real concern, add an explicit assertion that
`count == expected_linear_span` when the IR is canonical, instead of
silently taking the max.
