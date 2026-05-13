# Lean Contract Projection: vb-qi37.2.1 — Aggregate Resource Budget Model

## Boundary

- **Lean-owned kernel:** Pure checked-arithmetic budget types, dimension operations, capacity comparison, and policy validation in `crates/vb_core/src/budget.rs`. No I/O, no wall-clock time, no storage, no async scheduling.
- **Rust/runtime shell:** `vb_runtime::admission` (artifact store, capability checks, admission policy, reservation lifecycle, shard state mutation), `vb_runtime::shard` lifecycle (finish/fail/cancel/shutdown paths).
- **External systems excluded from Lean proof:** Fjall persistence, Mio IPC, external action ABI, timer wheel, trace ring, frame pools.

## Lean-Owned Clauses

The pure deterministic core of `AggregateResourceBudget`, `AggregateResourceUsage`, and `AggregateResourceCapacity` consists entirely of:

1. **Dimension value representation** — each field is a non-negative integer with an explicit upper bound.
2. **Checked addition** — `usage.try_add_budget(budget)` is equivalent to component-wise `checked_add` returning `Overflow { resource }` on any dimension overflow, never wrapping, saturating, or panicking.
3. **Checked subtraction** — `usage.try_subtract_budget(budget)` is equivalent to component-wise `checked_sub` returning `Underflow { resource }` on any dimension underflow, never wrapping, saturating, or panicking.
4. **Capacity comparison** — `fits_within(capacity)` is equivalent to the conjunction of `<=` for every comparable dimension; equality is included (admit).
5. **Policy validation** — `validate_aggregate_budget(budget, policy)` rejects exactly those dimensions exceeding the corresponding `BoundednessPolicy` field.
6. **Budget conversion** — `from_whole_workflow_budget(budget, contract)` is lossless for all dimensions that fit target widths.

## Theorem Obligations

### THM-ADD-SAFETY
- **Contract clause:** POST-003
- **Rust/spec target:** `AggregateResourceUsage::try_add_budget` in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.AddSafe`
- **Theorem shape:** `∀ (u: Usage) (b: Budget), NoOverflow u b → Result (AddUsage u b) (Overflow _)`
- **Model:** Abstract `Usage` and `Budget` as total maps from dimension name to ℕ; `NoOverflow` means every component-wise sum fits in 64 bits.
- **Refinement:** Rust `try_add_budget` returns `Ok(new_usage)` exactly when Lean `AddUsage u b = new_usage`; returns `Err(Overflow { resource })` exactly when dimension `resource` overflows.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI.
- **Evidence command:** `lake build` or `moon run :verify-proof`

### THM-SUB-SAFETY
- **Contract clause:** POST-004
- **Rust/spec target:** `AggregateResourceUsage::try_subtract_budget` in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.SubSafe`
- **Theorem shape:** `∀ (u: Usage) (b: Budget), NoUnderflow u b → Result (SubUsage u b) (Underflow _)`
- **Model:** Abstract `Usage` and `Budget` as total maps; `NoUnderflow` means every component-wise difference is non-negative.
- **Refinement:** Rust `try_subtract_budget` returns `Ok(new_usage)` exactly when Lean `SubUsage u b = new_usage`; returns `Err(Underflow { resource })` exactly when `resource` would go negative.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI.
- **Evidence command:** `lake build` or `moon run :verify-proof`

### THM-FITS-INCLUSIVITY
- **Contract clause:** INV-004
- **Rust/spec target:** `AggregateResourceUsage::fits_within` in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.FitsWithin`
- **Theorem shape:** `∀ (u: Usage) (c: Capacity), Fits u c ↔ ∀ dim, u dim ≤ c dim`
- **Model:** Usage and Capacity as dimension-keyed maps; `Fits` is true iff every dimension satisfies `<=`.
- **Refinement:** Rust `fits_within` returns `Ok(())` iff Lean `Fits u c` is true; returns `Err(CapacityExceeded { resource, ... })` naming the first failing dimension.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI.
- **Evidence command:** `lake build` or `moon run :verify-proof`

### THM-POLICY-EXACT
- **Contract clause:** INV-003
- **Rust/spec target:** `validate_aggregate_budget` in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.PolicyExact`
- **Theorem shape:** `∀ (b: Budget) (p: Policy), Validate b p = Ok () ↔ ∀ dim, b dim ≤ p dim`
- **Model:** Budget and Policy as dimension-keyed maps; `Validate` succeeds exactly when every budget dimension is `<=` the corresponding policy limit.
- **Refinement:** Rust `validate_aggregate_budget` returns `Ok(())` exactly when Lean `Validate b p` succeeds; returns `Err(PolicyExceeded { resource, actual, limit })` naming the first dimension where `actual > limit`.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI.
- **Evidence command:** `lake build` or `moon run :verify-proof`

### THM-ADD-SUB-ROUNDTRIP
- **Contract clause:** POST-006
- **Rust/spec target:** `AggregateResourceUsage::try_add_budget` + `try_subtract_budget` (paired) in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.AddSubRoundtrip`
- **Theorem shape:** `∀ (u: Usage) (b: Budget), NoOverflow u b → SubUsage (AddUsage u b) b = u`
- **Model:** After proving `NoOverflow`, `AddUsage u b` followed by `SubUsage (AddUsage u b) b` equals original `u`.
- **Refinement:** Rust roundtrip `usage.try_add_budget(budget)?.try_subtract_budget(budget)?` equals original `usage`.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI.
- **Evidence command:** `lake build` or `moon run :verify-proof`

### THM-CONV-LOSSLESS
- **Contract clause:** POST-001
- **Rust/spec target:** `AggregateResourceBudget::from_whole_workflow_budget` in `crates/vb_core/src/budget.rs`
- **Lean module:** `VbCore.Budget.ConvLossless`
- **Theorem shape:** `∀ (wb: WholeBudget) (rc: ResourceContract), NoOverflowConv wb rc → ∃ arb: AggregateResourceBudget, FromWhole wb rc = Ok arb`
- **Model:** Conversion maps WholeBudget fields to AggregateResourceBudget fields; succeeds exactly when all narrowed values fit target widths.
- **Refinement:** Rust `from_whole_workflow_budget` returns `Ok(arb)` with exact field values when no overflow; returns `Err(Overflow { resource })` for first overflowing dimension.
- **Shell exclusions:** All I/O, storage, wall-clock, async scheduling, FFI, mutable state, external trait objects.
- **Evidence command:** `lake build` or `moon run :verify-proof`

## Waivers

- **WAIVER-001:** Runtime admission and reservation lifecycle — covered by integration tests, Kani harnesses, proptest, and manual QA.
- **WAIVER-002:** `WholeWorkflowBudget::compute` IR traversal — covered by integration/unit tests, proptest invariants, and fuzz.
- No other waivers.
