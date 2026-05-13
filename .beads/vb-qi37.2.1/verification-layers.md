# Verification Layers: vb-qi37.2.1 — Aggregate Resource Budget Model

## Boundary

- **Verified kernel:** `crates/vb_core/src/budget.rs` — pure checked arithmetic, budget types, policy validation, dimension operations. No I/O, storage, wall-clock, async, or FFI in this module.
- **Lean contract projection:** `VbCore.Budget.AddSafe`, `VbCore.Budget.SubSafe`, `VbCore.Budget.FitsWithin`, `VbCore.Budget.PolicyExact`, `VbCore.Budget.AddSubRoundtrip`, `VbCore.Budget.ConvLossless` (see `lean-contract.md`).
- **Runtime shell:** `crates/vb_runtime/src/admission.rs`, `crates/vb_runtime/src/shard/lifecycle.rs`, `crates/vb_runtime/src/shard/types.rs`.
- **External systems excluded from formal proof:** Fjall journal, Mio IPC, external action ABI, timer wheel, trace ring, frame pools, value store, symbol/list/object/blob arenas.

## Layer Assignment

| Contract Clause | Layer | Tool | Rationale |
|---|---|---|---|
| PRE-001: CompiledWorkflow via try_from_parts | integration | `cargo nextest -p vb_core` | Real `try_from_parts` path with fixture workflows |
| PRE-002: WorkflowParts.resource_contract covers all fields | integration + static | `cargo nextest -p vb_core` + clippy | Validated at construction; no dynamic resource contract entry |
| PRE-003: Entry/target StepIdx valid, no cycles | integration + fuzz | `cargo nextest -p vb_core` + `cargo fuzz` | Cycle detection tested unit-level; fuzz covers malformed IR |
| PRE-004: All dimensions finite | unit + proptest | `cargo nextest -p vb_core` | Every constructor rejects NaN, Inf, negative, or unbounded inputs |
| PRE-005: Capacity snapshot fully initialized | unit | `cargo nextest -p vb_core` | Zero-capacity tests for all production-required dimensions |
| PRE-006: Reservation before frame insertion | integration | `cargo nextest -p vb_runtime` | Shard lifecycle integration tests verify ordering |
| PRE-007: All fallible ops return Result | static | `cargo check` + clippy + `moon ci` | Lint gates reject panic/unwrap/expect paths |
| POST-001: Successful construction returns finite exact dimensions | unit + proptest + lean | `cargo nextest -p vb_core` + `lake build` | Exact field assertions; Lean THM-CONV-LOSSLESS |
| POST-002: requested <= available admits | unit + kani + lean | `cargo nextest -p vb_core` + Kani + `lake build` | Equality admit tests; Kani fits_within harness; Lean THM-FITS-INCLUSIVITY |
| POST-003: checked_add never wraps/saturates | unit + kani + proptest + lean | `cargo nextest -p vb_core` + Kani + proptest + `lake build` | Per-dimension overflow tests; Kani THM-ADD-SAFETY; Lean THM-ADD-SAFETY |
| POST-004: checked_sub never wraps/saturates | unit + kani + proptest + lean | `cargo nextest -p vb_core` + Kani + proptest + `lake build` | Per-dimension underflow tests; Kani THM-SUB-SAFETY; Lean THM-SUB-SAFETY |
| POST-005: requested > available rejects exactly | unit + kani | `cargo nextest -p vb_core` + Kani harness | One-over-per-dimension tests; Kani capacity harness |
| POST-006: Add then subtract recovers original | unit + lean | `cargo nextest -p vb_core` + `lake build` | Roundtrip tests; Lean THM-ADD-SUB-ROUNDTRIP |
| POST-007: Rejection leaves state unchanged | integration | `cargo nextest -p vb_runtime` | Shard snapshot comparison tests |
| POST-008: RunAdmission immutable after creation | unit | `cargo nextest -p vb_core` | Field accessor returns copies/references only |
| INV-001: No accepted workflow has unknown bounds | integration + proptest | `cargo nextest -p vb_core` + proptest | Validated at try_from_parts + budget computation |
| INV-002: ResourceContract vs BoundednessPolicy scope separation | unit | `cargo nextest -p vb_core` | Separate validation paths tested |
| INV-003: Validation order: structural → budget → policy → capacity → reservation | integration | `cargo nextest -p vb_runtime` | Admission integration tests verify ordering |
| INV-004: Capacity comparison inclusive: equality admits | unit + kani + lean | `cargo nextest -p vb_core` + Kani + `lake build` | Equality tests; Kani THM-FITS-INCLUSIVITY; Lean THM-FITS-INCLUSIVITY |
| INV-005: Every arithmetic operation is checked | unit + kani + lean + static | `cargo nextest -p vb_core` + Kani + `lake build` + `cargo check` | All `add_dim`/`sub_dim`/`check_capacity`/`check_policy` functions |
| INV-006: Release idempotent only with existing reservation | integration | `cargo nextest -p vb_runtime` | Double-release tests |
| INV-007: Active usage never exceeds shard-local capacity | integration + kani | `cargo nextest -p vb_runtime` + Kani harness | Shard lifecycle tests; Kani `admission_cannot_return_ok_with_usage_above_capacity` |
| INV-008: 16 dimensions independent; overflow in one does not affect another | unit + kani | `cargo nextest -p vb_core` + Kani | Per-dimension isolation tests |
| ERR-001: WorkflowBudget(WorkflowError) for invalid entry/target/cycle | unit + fuzz | `cargo nextest -p vb_core` + `cargo fuzz` | Named error variant tests + fuzz corpus |
| ERR-002: PolicyExceeded for budget > policy limits | unit | `cargo nextest -p vb_core` | Per-dimension PolicyExceeded tests |
| ERR-003: CapacityExceeded for requested > available | unit + kani | `cargo nextest -p vb_core` + Kani | Per-dimension CapacityExceeded tests |
| ERR-004: Overflow for checked_add failure | unit + kani + proptest + lean | `cargo nextest -p vb_core` + Kani + proptest + `lake build` | Per-dimension overflow tests; Kani THM-ADD-SAFETY; Lean THM-ADD-SAFETY |
| ERR-005: Underflow for checked_sub failure | unit + kani + proptest + lean | `cargo nextest -p vb_core` + Kani + proptest + `lake build` | Per-dimension underflow tests; Kani THM-SUB-SAFETY; Lean THM-SUB-SAFETY |
| ERR-006: InvalidCapacity for zero production-required dimensions | unit | `cargo nextest -p vb_core` | Zero-capacity tests |
| ERR-007: ReservationNotFound for unknown RunId release | integration | `cargo nextest -p vb_runtime` | Unknown run release tests |
| ERR-008: StepCeilingExceeded / PerTickCeilingExceeded | unit | `cargo nextest -p vb_core` | Step ceiling zero and overflow tests |
| PERF-001: No JSON/YAML/HTTP/string-command parsing in runtime core | static | `cargo check` + `moon ci` + grep scan | Forbidden-parser scan over changed files |
| PERF-002: No runtime allocations in hot budget paths | unit + static | `cargo check` + `moon ci` | All budget operations are value-based; no heap allocation |
| GOV-001: No unsafe/unwrap/expect/panic/todo/dbg in production | static | `cargo clippy` + `moon ci` | Hard deny lint gates |
| GOV-002: No unchecked indexing/slicing/casts/arithmetic | static | `cargo clippy` + `moon ci` | Hard deny lint gates |
| BH-BUD-01: u32 saturation | unit + kani | `cargo nextest -p vb_core` + Kani | Step ceiling hard limit tests |
| BH-BUD-02: max_run_time_seconds hardcoded to 0 | unit | `cargo nextest -p vb_core` | Verify sourced from WholeWorkflowBudget |
| BH-BUD-03: information loss | lean | `lake build` | THM-CONV-LOSSLESS proves no lossy narrowing |
| BH-BUD-06: saturating_add inconsistency | unit + lean | `cargo nextest -p vb_core` + `lake build` | No saturating arithmetic in budget module |
| BH-BUD-07: gather_items saturating | unit + kani + lean | `cargo nextest -p vb_core` + Kani + `lake build` | Per-dimension checked arithmetic verified |

## Lean Scope

- **Theorem modules:**
  - `VbCore.Budget.AddSafe` — `try_add_budget` checked addition safety
  - `VbCore.Budget.SubSafe` — `try_subtract_budget` checked subtraction safety
  - `VbCore.Budget.FitsWithin` — `fits_within` inclusive capacity comparison
  - `VbCore.Budget.PolicyExact` — `validate_aggregate_budget` exact policy enforcement
  - `VbCore.Budget.AddSubRoundtrip` — add-then-subtract roundtrip preservation
  - `VbCore.Budget.ConvLossless` — `from_whole_workflow_budget` lossless conversion
- **Rust target:** `crates/vb_core/src/budget.rs`
- **Abstraction relation:** Rust `AggregateResourceBudget`, `AggregateResourceUsage`, `AggregateResourceCapacity` abstract to Lean total maps `Dimension → ℕ`. Error variants abstract to error reason strings.
- **Shell exclusions:** All I/O, storage, wall-clock time, async scheduling, FFI, mutable state, external trait objects.
- **Non-goals:** Runtime admission integration, artifact store trait dispatch, capability set operations, shard state mutation, reservation lifecycle, finish/fail/cancel/shutdown paths, Fjall/Mio integration.

## Waivers

- **WAIVER-001:** Runtime admission and reservation lifecycle — covered by integration tests, Kani harnesses, proptest, and manual QA.
- **WAIVER-002:** `WholeWorkflowBudget::compute` IR traversal — covered by integration/unit tests, proptest invariants, and fuzz.
- No other waivers.
