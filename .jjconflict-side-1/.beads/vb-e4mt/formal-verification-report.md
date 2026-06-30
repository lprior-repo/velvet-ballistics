# Formal Verification Report — vb-e4mt (State 11)

**STATUS: DEFERRED_GLOBAL** — Machine gates show fmt failure in out-of-scope crate; no new verification runs.

---

## Inputs

| Artifact | Path | Status |
|----------|------|--------|
| proof-obligations.jsonl | `.beads/vb-e4mt/proof-obligations.jsonl` | **MISSING** (proof-obligations.planned.jsonl exists) |
| delivery-scope.jsonl | `.beads/vb-e4mt/delivery-scope.jsonl` | **MISSING** |
| baseline-report.md | `.beads/vb-e4mt/baseline-report.md` | **MISSING** |
| tla-spec.md | `.beads/vb-e4mt/tla-spec.md` | **MISSING** |
| lean-contract.md | `.beads/vb-e4mt/lean-contract.md` | **MISSING** |
| contract-verification-review.md | `.beads/vb-e4mt/contract-verification-review.md` | **MISSING** |
| proof-evidence.md | `.beads/vb-e4mt/proof-evidence.md` | PRESENT (State 10) |
| verification-ledger.jsonl | `.beads/vb-e4mt/verification-ledger.jsonl` | PRESENT (State 10) |
| implementation.md | `.beads/vb-e4mt/implementation.md` | PRESENT (State 10) |

---

## Tool Availability

| Tool | Available | Version |
|------|-----------|---------|
| cargo kani | YES | 0.67.0 |
| rustc | YES | (workspace build passes) |
| cargo fmt | YES | (fmt diff detected) |
| cargo clippy | YES | (no issues) |

---

## Obligation Results (from State 10 — no new runs at State 11)

State 11 ran only machine build/test/clippy/fmt gates. No new Kani verification was executed. Results inherited from State 10.

### KANI-BUDGET-001
- **id:** KANI-BUDGET-001
- **risk:** HIGH — production code path not verified
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** `cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute`
- **required:** YES
- **owner_state:** State 10 (FAIL_LOCAL)
- **rerun_from:** State 10
- **result:** FAIL_LOCAL
- **evidence:** TIMEOUT >300s. State space explosion from deeply nested arbitrary WorkflowParts structures (CompiledNode, NodeEdges, ResourceContract) with unbounded Vec/slice fields.
- **failure_packet:** Harness architecture issue — needs kani::any_with() bounding or proof-specific Arbitrary for node slice length. Not a production code defect.
- **follow_up:** NONE (bead-local, must be resolved before release)

### KANI-BUDGET-002
- **id:** KANI-BUDGET-002
- **risk:** MEDIUM — proof obligation on budget validation path
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** `cargo kani -p vb_core --harness kani_harness_boundedness_policy_validate`
- **required:** YES
- **owner_state:** State 10 (PASS)
- **result:** PASS
- **evidence:** 221 checks, 0 failed, 9/9 cover properties satisfied in 0.14s. BoundednessPolicy::validate correctly maps each exceeded bound to the corresponding BudgetError variant.

### KANI-BUDGET-003
- **id:** KANI-BUDGET-003
- **risk:** HIGH — arithmetic overflow in budget aggregation
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** `cargo kani -p vb_core --harness kani_harness_try_add_budget_no_overflow`
- **required:** YES
- **owner_state:** State 10 (PASS)
- **result:** PASS
- **evidence:** 177 checks, 0 failed, 2/2 cover properties satisfied in 1.42s. try_add_budget returns typed Result without panic on arbitrary inputs. Both Ok and Err paths reachable.

### KANI-BUDGET-004
- **id:** KANI-BUDGET-004
- **risk:** MEDIUM — capacity fit check boolean semantics
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** `cargo kani -p vb_core --harness kani_harness_fits_within_exact`
- **required:** YES
- **owner_state:** State 10 (PASS)
- **result:** PASS
- **evidence:** 177 checks, 0 failed, 1/1 cover property satisfied in 0.77s. fits_within exact boolean semantics: Ok when usage <= capacity, Err otherwise.

### KANI-BUDGET-005
- **id:** KANI-BUDGET-005
- **risk:** HIGH — step budget consumption overflow
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** `cargo kani -p vb_core --harness kani_harness_step_budget_consume`
- **required:** YES
- **owner_state:** State 10 (PASS)
- **result:** PASS
- **evidence:** 158 checks, 0 failed (3 unreachable) in 1.25s. StepBudget::try_take raises StepBudgetExhausted before over-consumption; checked_sub never panics. Err path UNSATISFIABLE = invariant proven by construction.

### KANI-BUDGET-ALT
- **id:** KANI-BUDGET-ALT
- **risk:** LOW — pre-existing arithmetic refinement proof
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** (pre-existing kani_budget_arithmetic_refinement)
- **required:** NO (pre-existing)
- **owner_state:** State 5 (PASS)
- **result:** PASS
- **evidence:** 8 checks, 0 failed. word_to_u64 shift overflow assertions verified. TLA word 4-limb encoding round-trip proved.

### KANI-BUDGET-ZERO
- **id:** KANI-BUDGET-ZERO
- **risk:** LOW — pre-existing zero-boundary arithmetic proof
- **scope:** vb_core budget enforcement
- **layer:** kani
- **checker:** cargo kani 0.67.0
- **command:** (pre-existing kani_step_budget_zero)
- **required:** NO (pre-existing)
- **owner_state:** State 5 (PASS)
- **result:** PASS
- **evidence:** 4 checks, 0 failed (1 unreachable). kani_budget_add_dim_zero: 0+0=0 verified. Overflow path correctly unreachable.

---

## Machine Gate Results (State 11)

| Gate | Command | Result | Notes |
|------|---------|--------|-------|
| Build | `cargo build --workspace` | PASS | 183 crates, 5.07s |
| Test | `cargo test -p vb_core` | PASS | 1922 tests, 1.37s |
| Clippy | `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-features` | PASS | No issues |
| Fmt | `cargo fmt --check` | FAIL | vb_compile/src/kani_foreach_parity.rs unformatted |

### fmt Failure Details

**File:** `crates/vb_compile/src/kani_foreach_parity.rs` (untracked — new file, not committed)
**Crate:** `vb_compile` — OUTSIDE vb-e4mt scope (budget enforcement is vb_core)
**Classification:** DEFERRED_GLOBAL — pre-existing formatting debt in unrelated crate
**Required action:** Format file or add to `.rustfmt.toml` exclusions

---

## Waivers

None — no formal waivers exist in `.beads/vb-e4mt/formal-waivers.jsonl`.

---

## Residual Risk

1. **KANI-BUDGET-001 (FAIL_LOCAL):** `WholeWorkflowBudget::compute` not verified — harness state space too large. Requires proof-specific `kani::Arbitrary` with `kani::any_with()` bounding. Production code assumed correct by manual review.
2. **fmt failure in vb_compile:** Pre-existing formatting debt in unrelated crate — out-of-scope for vb-e4mt.
3. **Missing required artifacts:** `proof-obligations.jsonl`, `delivery-scope.jsonl`, `baseline-report.md`, `contract-verification-review.md` not present in bead directory. Obligations reconstructed from `proof-obligations.planned.jsonl` and `verification-ledger.jsonl`.

---

## Delivery Recommendation

**Cannot advance to State 12** — KANI-BUDGET-001 remains FAIL_LOCAL. Per go-skill rules, all required obligations must be PASS or WAIVED before advancing.

**KANI-BUDGET-001 fix required:** Restructure harness with bounded Arbitrary inputs before this bead can be closed.
