# Proof Plan Review Input: vb-a001

## Bead: vb-a001 — runtime: repair for_each compiled parity

### What changed

`lower_canonical_for_each` in `part_02.rs` (line ~178) now passes `Some(ForEachNext)` as the `next_step` argument to `emit_single_body_set`. The body SetConst node (index 1) now gets `next = Some(ForEachNext)` pointing to node index 2, instead of the broken `None` or misrouted edge that caused `CompiledWorkflow::try_from_parts` to reject the IR with "backward edge" or "unreachable node".

### Why the fix is correct (explorer-verified)

- The 4-node emission pattern (ForEachStart→SetConst→ForEachNext→SetConst) was always correct structurally.
- The only defect was the `next` field on the body SetConst node: `next=None` → `next=Some(ForEachNext_step)` where `ForEachNext_step = checked_step_offset(id, 2)`.
- The forward-edge validator rejects `target ≤ ci` on `next` fields, but body edges bypass forward-edge validation (validated via reachability BFS instead).
- With `next = Some(ForEachNext)` pointing to index 2, the body SetConst's `next` edge is forward (2 > 1), AND the body edge from ForEachStart/ForEachNext also points to index 1, making all 4 nodes reachable via BFS.

### Proof strategy summary

**19 obligations across 8 verifier lanes**, ordered by dependency:

1. **Static scan (SCAN-001):** Clippy zero-warning gate on vb_compile.
2. **Kani (KANI-001..004):** Bounded model check of lowering correctness and validation behavior. Proves PRE-002, PRE-005, PRE-006, POST-003.
3. **TLA+ (TLA-PARITY-001, TLA-TERM-001):** Temporal model proves parity invariant (INV-005) and termination (INV-004). Bounded: nodes ≤ 20, limit ≤ 5.
4. **Fowler tests (FOWLER-001..006):** CLI-level integration tests covering happy path, empty list, rejection, round-trip, and runtime primitives.
5. **Proptest (PROPTEST-001):** Property tests verify forward-edge invariant across random inputs.
6. **Mutation (MUTANT-001, MUTANT-002):** `cargo mutants` on vb_compile and vb_core.
7. **Coverage (COVERAGE-001):** llvm-cov branch coverage on part_02.rs.
8. **Gauntlet (GATE-001, GATE-002):** Full workspace CI + formal verification gate.

### Waivers (explicit)

- **Flux RS, Loom, Miri:** Not applicable — no refinement types, no concurrency, no unsafe in touched path.
- **Verus:** Deferred beyond this bead's scope. Kani provides equivalent bounded-model-checking coverage. Verus proofs tracked in proof-obligations.jsonl with status `planned`.
- **cargo-fuzz:** POST-004 covered by FOWLER-004 integration test. Fuzz is optional nice-to-have.

### Reviewer focus areas

1. **Kani harness design:** Ensure `kani::Arbitrary` is used for WorkflowParts — NOT hardcoded test data. Per GOD RULE #1.
2. **TLA+ bounds:** Verify state constraints (nodes ≤ 20, limit ≤ 5, iterList ≤ 5) are tight enough to be meaningful but loose enough to be feasible. Per GOD RULE #3.
3. **Fowler test coverage:** Confirm the corpus artifact includes non-empty list, empty list, and multi-item cases.
4. **Mutation scope:** Ensure `cargo mutants` does NOT sweep the full workspace — only vb_compile and vb_core. Per GOD RULE #5.
