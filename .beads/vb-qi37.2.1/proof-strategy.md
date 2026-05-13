# Proof Strategy: vb-qi37.2.1 — Aggregate Resource Budget Model

## Risk Classification

| Risk | Category | Primary lane |
|---|---|---|
| Checked arithmetic overflow/underflow | Rust-local invariant + bounded arithmetic | Kani + Lean |
| Capacity comparison inclusivity | Pure function contract | Kani + Lean |
| Policy validation exactness | Type-level enforcement | Lean |
| Conversion losslessness | Numeric refinement | Lean |
| Roundtrip add-sub | Paired operation invariant | Lean |
| Admission rejection cleanliness | Integration correctness | Kani + Integration |
| Reservation release correctness | Lifecycle invariant | Integration |
| Static governance | Holzman Rust rules | Clippy + moon ci |
| No forbidden parsers in runtime core | Performance/perf-only | grep scan + moon ci |
| Mutation robustness | Fault injection | cargo-mutants |

## Verifier Lane Decisions

### Lane 1: Lean (6 theorems — KERNEL)

**VbCore.Budget.AddSafe** — `try_add_budget` overflow safety
- Command: `lake build` or `moon run :verify-proof`
- Artifact: `lean-report.md`
- Abstraction: Usage + Budget as `Dimension → ℕ`, `NoOverflow` = ∀dim, sum < 2^64
- Refinement: Rust returns `Ok(new)` iff Lean `AddUsage u b = new`; `Err(Overflow {resource})` for first overflow dim
- Shell exclusions: All I/O, storage, wall-clock, async, FFI, mutable state

**VbCore.Budget.SubSafe** — `try_subtract_budget` underflow safety
- Command: `lake build` or `moon run :verify-proof`
- Artifact: `lean-report.md`
- Abstraction: `NoUnderflow` = ∀dim, u(dim) ≥ b(dim)
- Refinement: Rust returns `Ok(new)` iff Lean `SubUsage u b = new`; `Err(Underflow {resource})` for first underflow dim

**VbCore.Budget.FitsWithin** — capacity comparison inclusivity
- Command: `lake build` or `moon run :verify-proof`
- Abstraction: `Fits u c ↔ ∀dim, u(dim) ≤ c(dim)`; equality admits
- Refinement: Rust returns `Ok(())` iff Lean `Fits u c`; `Err(CapacityExceeded {resource, requested, available})` for first failing dim

**VbCore.Budget.PolicyExact** — policy validation exactness
- Command: `lake build` or `moon run :verify-proof`
- Abstraction: `Validate b p` succeeds iff ∀dim, b(dim) ≤ p(dim)
- Refinement: Rust returns `Ok(())` exactly when policy accepts; `Err(PolicyExceeded {resource, actual, limit})` for first exceeding dim

**VbCore.Budget.AddSubRoundtrip** — add-then-subtract roundtrip
- Command: `lake build` or `moon run :verify-proof`
- Abstraction: After `NoOverflow`, `SubUsage (AddUsage u b) b = u`
- Refinement: Rust `usage.try_add_budget(budget)?.try_subtract_budget(budget)? == usage`

**VbCore.Budget.ConvLossless** — workflow-to-aggregate conversion
- Command: `lake build` or `moon run :verify-proof`
- Abstraction: WholeBudget + ResourceContract → AggregateResourceBudget; succeeds when all narrowed values fit
- Refinement: Rust returns `Ok(arb)` with exact field values; `Err(Overflow {resource})` for first narrowing overflow

### Lane 2: Kani (4 harnesses — CRITICAL SAFETY)

| Harness | Target | Claim |
|---|---|---|
| KANI-ADD-SAFETY | `AggregateResourceUsage::try_add_budget` | Symbolic usage + budget: overflow returns Overflow before mutation |
| KANI-SUB-SAFETY | `AggregateResourceUsage::try_subtract_budget` | Symbolic usage + budget: underflow returns Underflow before mutation |
| KANI-FITS-INCLUSIVITY | `AggregateResourceUsage::fits_within` | Symbolic: result Ok iff all dims satisfy usage <= capacity |
| KANI-ADMISSION | `admit_run_with_budget` | Never returns Ok when resulting usage would exceed capacity |

Commands: `cargo kani` per harness; evidence in `formal-verification-report.md`

### Lane 3: Proptest (6 properties — SCOPE COMPLEXITY)

| Property | Target | Claim |
|---|---|---|
| PROPTEST-ADD | `try_add_budget` | Non-overflow add = component-wise checked_add; overflow returns Overflow |
| PROPTEST-SUB | `try_subtract_budget` | Non-underflow sub = component-wise checked_sub; underflow returns Underflow |
| PROPTEST-FITS | `fits_within` | Ok iff every usage dim <= capacity dim |
| PROPTEST-POLICY | `validate_aggregate_budget` | Ok iff every budget dim <= policy limit |
| PROPTEST-ROUNDTRIP | `AggregateResourceUsage` | usage.add(budget)?.sub(budget)? == usage for non-overflow add |
| PROPTEST-CONV | `from_whole_workflow_budget` | Successful conversion preserves every dim value |

Command: `cargo test -p vb_core --test aggregate_resource_budget_properties`

### Lane 4: Integration (12 tests — LIFECYCLE)

| Test | Target | Claim |
|---|---|---|
| INTEG-ADMISSION-EQ | `admit_run_with_budget` | Equality with capacity admits |
| INTEG-ADMISSION-REJECT | `admit_run_with_budget` | Over capacity rejects + state unchanged |
| INTEG-ARTIFACT-REJECT | `admit_run_with_budget` | Missing artifact → ArtifactNotFound, no reservation mutation |
| INTEG-CAPABILITY-REJECT | `admit_run_with_budget` | Missing capability → CapabilityDenied, no reservation mutation |
| INTEG-REJECT-UNChanged | `vb_runtime::shard` | Budget rejection leaves runs/usage/pools/journals/trace unchanged |
| INTEG-RELEASE-FINISH | `shard/lifecycle.rs` | Finish releases reservation, usage returns to pre-admission |
| INTEG-RELEASE-FAIL | `shard/lifecycle.rs` | Failure releases reservation, usage returns to pre-admission |
| INTEG-RELEASE-CANCEL | `shard/lifecycle.rs` | Cancellation releases reservation |
| INTEG-RELEASE-SHUTDOWN | `shard/lifecycle.rs` | Shutdown drains all runs, releases all reservations |
| INTEG-RESERVATION-NOT-FOUND | `vb_runtime::shard` | Release unknown RunId → ReservationNotFound, usage unchanged |
| INTEG-DOUBLE-RELEASE | `vb_runtime::shard` | Double release → ReservationNotFound on second call |

Command: `cargo nextest run -p vb_runtime admission shard`

### Lane 5: Unit (6 test groups — PER-DIMENSION)

| Test | Target | Claim |
|---|---|---|
| UNIT-FROM-WORKFLOW | `from_workflow` | Valid bounded workflow produces exact finite budget dims |
| UNIT-FROM-WHOLE | `from_whole_workflow_budget` | Lossless conversion preserves all valid dim values |
| UNIT-VALIDATE-POLICY | `validate_aggregate_budget` | All 14 dims: equality accepts, one-over rejects with PolicyExceeded |
| UNIT-ADD-OVERFLOW-PER-DIM | `try_add_budget` | Per-dim overflow returns Overflow, leaves usage unchanged |
| UNIT-SUB-UNDERFLOW-PER-DIM | `try_subtract_budget` | Per-dim underflow returns Underflow, leaves usage unchanged |
| UNIT-FITS-PER-DIM | `fits_within` | Per-dim one-over returns CapacityExceeded with exact dim/values |

Command: `cargo nextest run -p vb_core aggregate`

### Lane 6: Fuzz (2 targets — MALFORMED INPUT)

| Target | Claim |
|---|---|
| FUZZ-IR-BUDGET | Malformed IR (invalid indices, cycles, overflow) → exact WorkflowError, never panics |
| FUZZ-DECODE | Deserialized artifact with malformed resource metadata → typed error, never panics |

Command: `cargo fuzz run workflow_aggregate_target` / `cargo fuzz run artifact_aggregate_target`

### Lane 7: Static (3 gates — GOV-001/GOV-002/PERF-001)

| Gate | Target | Claim |
|---|---|---|
| STATIC-GOV | `budget.rs` + `admission.rs` | No unsafe/unwrap/expect/panic/todo/dbg in production |
| STATIC-UNCHECKED | `budget.rs` + `admission.rs` | No unchecked indexing/slicing/casts/arithmetic in aggregate paths |
| STATIC-PARSER | `admission.rs` + `shard` | No JSON/YAML/HTTP/string-command parsing in runtime core |

Command: `cargo clippy` + `moon ci` + grep scan

### Lane 8: Mutation + Coverage (COMPLEMENTARY)

- MUTATION: `cargo mutants -p vb_core -p vb_runtime` — ≥90% kill rate on changed files
- COVERAGE: `cargo llvm-cov -p vb_core -p vb_runtime` — branch/line coverage for all aggregate branches

### Lane 9: Gauntlet

- GAUNTLET-PROOF: `moon run :verify-proof` — Lean kernel proofs pass
- GAUNTLET-ALL: `moon run :verify-all` — Full verification gauntlet passes

## Waiver Rationale

**WAIVER-001: Runtime admission + reservation lifecycle — integration/Kani/proptest/manual-QA**
- Reason: trait objects, mutable shard state, orthogonal check ordering
- Owner: vb-qi37.2.1 contract synthesizer

**WAIVER-002: `WholeWorkflowBudget::compute` IR traversal — unit/fuzz/proptest**
- Reason: mutable HashSet/HashMap internals, pointer-based node indexing; full IR modeling in Lean out of scope
- Owner: vb-qi37.2.1 contract synthesizer

## Execution Order

1. Lean theorems first (kernel correctness)
2. Kani harnesses (critical safety)
3. Unit tests (per-dimension correctness)
4. Proptest (scope complexity)
5. Integration (lifecycle correctness)
6. Fuzz (malformed input)
7. Static gates (governance)
8. Mutation + Coverage (complementary)
9. Gauntlet (final gate)

## Key Proof Strategy Decisions

1. **Lean is the source of truth for pure arithmetic**: The 6 Lean theorems define the mathematically precise semantics of checked arithmetic, capacity comparison, and policy validation in the kernel. All other lanes are refinement evidence.

2. **Kani is the critical bridge**: Kani verifies the Rust implementation refines the Lean theorems for concrete struct representations with symbolic inputs. This is the main proof of absence of overflow/underflow bugs in practice.

3. **Integration tests are not replaceable**: No formal method covers the ordering of artifact check → capability check → budget admission → reservation insertion in the runtime shell. 11 integration tests provide evidence that these interactions are correct.

4. **Static governance is non-negotiable**: Holzman Rust rules (`no unsafe/unwrap/expect/panic/todo/dbg`) are enforced by clippy + moon ci. This is a categorical gate, not a probabilistic one.

5. **No TLA+ required**: The system is not temporal in the relevant way. Admission lifecycle is captured by integration tests + Kani harness over the `admit_run_with_budget` function, not state-machine TLA+ specs.

6. **No Loom required**: Concurrency in the shard is lock-free and single-threaded per-shard with explicit tick ordering. No concurrent interleaving risk in the aggregate budget model itself.
