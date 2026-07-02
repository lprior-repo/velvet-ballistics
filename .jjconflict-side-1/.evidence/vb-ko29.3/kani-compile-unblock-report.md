## vb-ko29.3 Kani cfg compile unblock report

Scope: `crates/vb_core` and `crates/vb_validate` cfg(kani) compilation/listing only.

Repairs:
- Updated stale cfg(kani) budget harness initializers for the current budget structs:
  `AggregateResourceUsage`, `AggregateResourceBudget`, `AggregateResourceCapacity`, and
  `WholeWorkflowBudget` now include timer/trace/journal/queue/ipc/blob/input fields.
- Kept budget harnesses active; no harness was suppressed or hidden.
- Replaced repeated zero-valued stale budget literals in boundary harnesses with local zero
  constructors/defaults so future field additions are less likely to break cfg(kani) listing.

Raw evidence:
- `cargo-kani-version.log` — `cargo-kani 0.67.0`, exit 0.
- `vb_core-kani-list-before-cratedir.log` — reproduced stale initializer errors, exit 1.
- `vb_validate-kani-list-before-cratedir.log` — reproduced dependent vb_core cfg(kani) errors, exit 1.
- `vb_core-kani-list-final-r3.log` — final vb_core `cargo kani list` after split harness repair, exit 0.
- `vb_validate-kani-list-final-r2.log` — final vb_validate `cargo kani list` after split harness repair, exit 0.

Result: cfg(kani) harness inventory is unblocked for both crates. Remaining warnings are pre-existing
unused/mut and unsupported-construct warnings from broader crate compilation, not list blockers.
