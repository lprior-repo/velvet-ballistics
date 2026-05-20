# Proof Strategy: vb-a001 — runtime: repair for_each compiled parity

## Bead Summary

- **Type:** Bug fix (P0)
- **Status:** Fix applied, verified by explore agent. Remaining work: test evidence.
- **Change:** `lower_canonical_for_each` (part_02.rs line ~178) now passes `Some(ForEachNext)` as the `next_step` to `emit_single_body_set`, so the body SetConst node gets `next = Some(ForEachNext)` pointing to index 2.
- **Discovery scan:** Zero matches for `unsafe|unwrap|expect|panic|todo|spawn|tokio|Mutex|Atomic` in `part_02.rs`. Zero verification annotations present. This is a pure edge-fix.

## Risk Classification

| Risk | Trigger | Severity |
|------|---------|----------|
| Rust-local invariant (edge ordering) | PRE-002: body SetConst.next must be ForEachNext > body_step | **HIGH** — wrong edge = validation reject or infinite loop |
| Temporal/state-machine (termination) | INV-004: for_each loop must terminate | **HIGH** — infinite loop = hung engine |
| Runtime parity (observable behavior) | INV-005: run == run-compiled | **HIGH** — broken parity = incorrect output |
| Unsafe / UB | Discovery scan: **NONE** | NONE |
| Concurrency | Discovery scan: **NONE** | NONE |
| Supply-chain / dependency | No new dependencies | NONE |

## Verifier Lane Selection

### Lane 1: Static Scan (fast, gate)
**Obligations:** SCAN-001
- **Tool:** `cargo clippy -p vb_compile --lib -D warnings`
- **Purpose:** Zero-warning gate. No new code patterns introduced.
- **Priority:** P0 — runs first, blocks everything if fails.
- **Mode:** verify-fast

### Lane 2: Kani Bounded Model Check
**Obligations:** KANI-001 through KANI-004
- **Tool:** `cargo kani`
- **Scope:** `vb_compile::mod_compile_lowering` (lowering correctness) and `vb_core::workflow` (validation correctness)
- **Purpose:** Prove PRE-002 (body SetConst.next = ForEachNext), PRE-005 (no false-positive backward edge), PRE-006 (reachability), POST-003 (rejection of malformed IR).
- **Priority:** P0 — proves the mathematical correctness of the edge fix.
- **Mode:** verify-proof
- **Note:** Kani proofs require `kani::Arbitrary` impls for WorkflowParts/RunFrame — **NOT hardcoded test data** (GOD RULE #1).

### Lane 3: TLA+ Temporal Model
**Obligations:** TLA-PARITY-001, TLA-TERM-001
- **Tool:** `tlc`
- **Scope:** `specs/ForEachParity.tla` with `ForEachParity.cfg`
- **Purpose:** Prove INV-005 (parity) and INV-004 (termination) as temporal properties with bounded state (nodes ≤ 20, limit ≤ 5, iterList ≤ 5).
- **Priority:** P1 — proves the high-level behavioral claim, but only needed after Kani confirms the lowering is correct.
- **Mode:** verify-proof
- **Note:** Must model bounded hardware (no unbounded Nat) per GOD RULE #3.

### Lane 4: Fowler Tests (integration / CLI-level)
**Obligations:** FOWLER-001 through FOWLER-006
- **Tool:** `cargo test`
- **Scope:** `crates/vb_cli/tests/ir_artifact_admission.rs`
- **Purpose:** End-to-end validation: run vs run-compiled parity, empty list, rejection tests, round-trip, runtime primitives.
- **Priority:** P1 — required for delivery acceptance.
- **Mode:** verify-standard
- **Execution:** Tests already exist per delivery-scope.jsonl. Plan verifies they pass.

### Lane 5: Property Tests
**Obligations:** PROPTTEST-001
- **Tool:** `cargo test -p vb_compile lower`
- **Purpose:** Proptest-generated lowering tests verify INV-002 (forward-edge invariant) across random inputs.
- **Priority:** P2 — broadens coverage beyond hand-crafted tests.
- **Mode:** verify-standard

### Lane 6: Mutation Testing
**Obligations:** MUTANT-001, MUTANT-002
- **Tool:** `cargo mutants`
- **Scope:** `vb_compile` (lowering) and `vb_core` (validation)
- **Purpose:** Verify test suite kills all mutations in the touched code.
- **Priority:** P2 — strongest signal of test sufficiency.
- **Mode:** verify-standard

### Lane 7: Coverage Gate
**Obligations:** COVERAGE-001
- **Tool:** `cargo llvm-cov`
- **Scope:** `part_02.rs` in `lower_canonical_for_each` and `emit_single_body_set`
- **Purpose:** ≥95% branch coverage on the touched function.
- **Priority:** P2 — quality gate.
- **Mode:** verify-standard

### Lane 8: Workspace Gauntlet
**Obligations:** GATE-001, GATE-002
- **Tool:** `moon ci` and `moon run :verify-proof`
- **Purpose:** All 11,118 workspace tests pass + all formal verification lanes pass.
- **Priority:** P0 — final delivery gate.
- **Mode:** verify-fast / verify-proof

## Execution Order (Dependency Graph)

```
Lane 1 (Static Scan) ──┐
                         │
Lane 2 (Kani) ←──────────┘ (can run in parallel with Lane 1)
                         │
Lane 3 (TLA+) ───────────┘ (requires Lane 2 proofs conceptually)
                         │
Lane 4 (Fowler) ────────┐
Lane 5 (Proptest) ──────┤
                         │
Lane 6 (Mutation) ←─────┘ (requires Lane 4 tests to exist)
Lane 7 (Coverage) ──────┘
                         │
Lane 8 (Gauntlet) ─────── (requires all above)
```

### Recommended sequential execution for efficiency:

1. **SCAN-001** (clippy) — instant, blocks early
2. **KANI-001..004** (kani) — parallel within lane
3. **TLA-PARITY-001, TLA-TERM-001** (tlc) — parallel within lane
4. **FOWLER-001..006** (cargo test) — parallel within lane
5. **FOWLER-005, FOWLER-006** (runtime primitives) — can run earlier if needed for isolation
6. **PROPTEST-001** (proptest) — after tests compile
7. **MUTANT-001, MUTANT-002** (cargo mutants) — requires Lane 4 tests
8. **COVERAGE-001** (llvm-cov) — requires Lane 4 tests
9. **GATE-001, GATE-002** (moon ci / verify-proof) — final gate

## Waivers

| Lane | Waiver Reason |
|------|--------------|
| Flux RS | Not applicable — no refinement types in scope. This is an edge-value fix, not a type-state change. |
| Loom | Not applicable — no concurrency primitives in for_each lowering. |
| Miri | Not applicable — no unsafe code, no raw pointer manipulation in touched path. |
| Verus | Planned but deferred — Verus proofs for `lower_canonical_for_each` edge emission and `drive_deterministic_full` termination are in the proof-obligations.jsonl but require proof-writing effort beyond this bead's scope. Kani provides equivalent bounded-model-checking coverage for PRE-002/PRE-005/PRE-006. Verus proof obligations are tracked as `planned` with `owner_state: 2` (proof-writing). |
| cargo-fuzz | Not applicable — POST-004 (round-trip) covered by FOWLER-004 (Fowler integration test). Fuzz is a nice-to-have, not a required gate for this bead. |

## GOD Rules Compliance Checklist

1. **No hardcoded Kani shapes** — Kani harnesses must use `kani::Arbitrary` for WorkflowParts/RunFrame.
2. **No vacuum Verus proofs** — Verus spec/proof functions must bind to actual `lower_canonical_for_each` and `drive_deterministic_full`.
3. **No unbounded TLA+ math** — TLA+ model uses bounded state: nodes ≤ 20, limit ≤ 5, iterList ≤ 5.
4. **No loop oscillations** — If Kani/Verus exposes a flaw, fix the implementation, not the proof.
5. **No blind mutation sweeps** — `cargo mutants` scoped to touched crates only (vb_compile, vb_core).
