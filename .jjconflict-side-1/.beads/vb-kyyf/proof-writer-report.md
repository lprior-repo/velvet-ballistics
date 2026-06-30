# vb-kyyf Proof-Writer Report — attempt 4

**Bead**: vb-kyyf
**State**: 5
**Sublane**: cfg-branch-binding-and-ledger-reconciliation-after-state6-rejection
**Delegate**: proof-writer
**Attempt**: 4

## Work Done

### 1. Ledger Reconciliation (contract-verification requirement)

Updated `.beads/vb-kyyf/proof-obligations.planned.jsonl`:

| Obligation | Old Status | New Status | Change |
|---|---|---|---|
| PO-008 / TLA-KYYF-001 | `blocked_tooling` | `planned` | Removed stale DISCOVERY assumption; updated command with TLC temp isolation; added TLC pass evidence summary |
| PO-009 / VERUS-KYYF-001 | `blocked_tooling` | `planned` | Removed stale DISCOVERY assumption; updated artifact to actual kernel path; updated command to direct Verus invocation |

**Rationale**: Both TLA spec file and normalization kernel exist. Contract-verification review correctly identified that the planned ledger was stale and contradicting the actual proof state.

### 2. Legacy Mirror Demotion (proof-review requirement)

Demoted `Spec*` and `Exec*` mirror layers in `verification/verus/vb_kyyf_normalization.rs` to non-gating context:

- `Spec*` types and proofs (lines 201-838) are now explicitly non-production-binding
- `Exec*` types and proofs (lines 289-469) are now explicitly non-production-binding
- Only `production_probe::*` obligations (lines 28-72) and `proof_prod_*` obligations (lines 74-195) count as production-binding evidence for PO-009

**Documented in proof-evidence.md** with line references.

### 3. cfg-branch Binding Blocker Identified

**Finding**: `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs` has two hand-duplicated implementations:
- `#[cfg(verus_keep_ghost)]` lines 8-326: Verus-annotated with `ensures` contracts; uses early `return` statements
- `#[cfg(not(verus_keep_ghost))] mod cargo_kernel` lines 328-518: safe Rust; uses if-else without early returns

**Structural non-equivalence**: Same observable behavior, but control flow is hand-duplicated. Proof-review correctly identified this as insufficient under GOD RULE 2.

**Blocker**: Cannot close cfg-branch binding gap without production code changes. Routing to implementation repair.

## Raw Evidence Captured

```
verus verification/verus/vb_kyyf_normalization.rs
→ verification results:: 42 verified, 0 errors

rtk cargo test -p vb_proof_kernels vb_kyyf_normalization --all-features
→ cargo test: 3 passed, 34 filtered out (1 suite, 0.00s)

rtk cargo clippy -p vb_proof_kernels --lib --all-features -- [strict flags]
→ cargo clippy: No issues found

rtk grep trust shortcuts
→ 0 matches for assume/external/axiom/sorry/unimplemented/todo

TLA files: verification/tla/VbKyyfReplayDeterminism.tla (26.5K), .cfg (471B) — both exist
```

## Unresolved Item

**cfg-branch binding**: Requires production code change to eliminate hand-duplicated implementations. This is a **blocker for PO-009 approval** that only implementation can resolve.

## Next Routing

1. **To contract-verification-reviewer**: Ledger is now reconciled. PO-008 and PO-009 no longer marked `blocked_tooling`. Rerun contract verification with updated ledger.

2. **To proof-reviewer**: Ledger reconciled; legacy mirrors demoted. However, cfg-branch binding gap remains unresolved. Proof-review will continue to reject PO-009 until production code is restructured.

3. **To implementation repair**: Blocker packet issued. Production code needs cfg-branch consolidation.

4. **To femdation**: Proof-writer attempt 4 complete. Ledger reconciled; legacy mirrors demoted; cfg-branch binding remains blocked. No further proof-writer action available without production changes.