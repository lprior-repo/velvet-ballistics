# Final Evidence Decision: vb-oul6u

bead_id: vb-oul6u
state: 14
agent: evidence-packaging + truth-serum
completed_at: 2026-07-02T00:50:00Z
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u

STATUS: APPROVED

---

## Decision

State 14 evidence-packaging gate is **PASSED**. The bead `vb-oul6u`
(*Lint: remove runtime metric `as_conversions` suppression*) is
**approved for landing** in the next femdation dispatch (or in
the user's session-completion pipeline).

## Evidence Anchor

| Required Artifact | Path | Status |
|---|---|---|
| State 1 dispatch + STATE.md | `.beads/vb-oul6u/STATE.md` | (provided) |
| State 4 proof-strategy + proof-plan-review | `.beads/vb-oul6u/proof-strategy.md` + `.beads/vb-oul6u/proof-plan-review.md` | (approved) |
| State 5 proof-writer (NO_FORMAL_PROOF_WORK_REQUIRED) | `.beads/vb-oul6u/proof-evidence.md` + `.beads/vb-oul6u/proof-writer-report.md` | (accepted) |
| State 6 proof-review (NO_PROOF_WORK, 0 findings) | `.beads/vb-oul6u/proof-review.md` | **STATUS: APPROVED** |
| State 7 bridge (proof-to-implementation + bridge review) | `.beads/vb-oul6u/proof-to-rust-map.md` + `.beads/vb-oul6u/proof-to-rust-review.md` | **STATUS: APPROVED** |
| State 11 holzman-rust implementation | `.beads/vb-oul6u/implementation.md` | COMPLETED_WITH_RESIDUAL_BLOCKER (parent-resolved, option (a) accepted) |
| State 12 formal-verification | `.beads/vb-oul6u/formal-verification-report.md` + `.beads/vb-oul6u/verification-ledger.jsonl` (3 rows: PO-OUL6U-LINT-001 / PO-OUL6U-RA003-002 / PO-OUL6U-CALLSITE-003) | **STATUS: APPROVED** |
| State 13 black-hat-review | `.beads/vb-oul6u/black-hat-review.md` | **STATUS: APPROVED** |
| State 14 evidence-packaging | `.beads/vb-oul6u/assurance-bundle.md` + `.beads/vb-oul6u/truth-serum-report.md` + this file | **STATUS: APPROVED** |
| Raw command logs (active-execution-context) | `.beads/vb-oul6u/evidence/*.log` | present, non-empty, exit-code recorded |

## Raw Command Evidence Summary

| Command | Exit | Result | Source |
|---|---|---|---|
| `cargo clippy -p vb_runtime --lib --bins --all-features -- -D clippy::as_conversions` | 0 | 0 diagnostics | `.beads/vb-oul6u/evidence/clippy-as-conversions-verifier-rerun.log` |
| `cargo test -p vb_runtime --lib trace_ring_fill_pct` | 0 | 3 passed (0 failed) | `.beads/vb-oul6u/evidence/cargo-test-trace-ring-verifier-rerun.log` |
| `cargo check -p vb_runtime --all-targets --all-features` | 0 | 1 crate compiled | `.beads/vb-oul6u/evidence/cargo-check-verifier-rerun.log` |
| `cargo test -p vb_runtime --lib --all-features` | 0 | 1807 passed (workspace enrollment) | captured live in truth-serum-report.md §Witness 3 |
| `cargo clippy -p vb_runtime --lib --bins --all-features -- -D warnings -D clippy::as_conversions -D clippy::unwrap_used -D clippy::arithmetic_side_effects -D clippy::indexing_slicing` | 0 | strengthened gate | captured live in truth-serum-report.md §Witness 4 |
| `rg -n 'unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!|unreachable!' crates/vb_runtime/src/runtime.rs` | 0 | 0 matches (empty production panic surface in modified file) | truth-serum-report.md §Witness 5 |

## Parent-Approved Deviation Acknowledgement

The State-11 implementation substituted a `u32_to_f32_exact` helper
(in `crates/vb_runtime/src/runtime.rs:32-46`) for the contract
INV-004-mandated `f32::from(u32)` form because the Rust standard
library does **not** implement `From<u32> for f32`. The femdation
parent reviewed the residual blocker in STATE.md §Residual Blocker
and selected option (a): **accept the helper as the canonical
form**, with documented equivalence proof.

This final-evidence decision recognizes that the parent-approved
deviation is:

1. **Not a finding** (it is an owner-approved pre-resolution of the
   residual blocker, not a defect).
2. **Not a waiver** (no behavior-affecting waiver was issued; the
   deviation is over the contract SPIRIT, which is preserved by
   the helper).
3. **Not blocking landing** (the helper's bit-equivalence to
   `(n as f32)` is proven by `evidence/ieee-754-bit-equivalence.log`
   and pinned by the 3/3 RA-003 test corpus re-run in the active
   execution context).

The following artifacts acknowledge and document the deviation:

- `.beads/vb-oul6u/formal-verification-report.md` §"Parent-Approved
  Deviation" with 5-step equivalence proof.
- `.beads/vb-oul6u/black-hat-review.md` PHASE 1 row "INV-004" and
  §"Pre-existing OUT-OF-SCOPE blocks" mention.
- `crates/vb_runtime/src/runtime.rs:614-619` in-file annotation
  pointing back to the equivalence log and the 3/3 test corpus.

## Out-of-Scope Items (NOT blockers for this bead)

These are pre-existing issues documented in `STATE.md §Pre-existing
BLOCK_GLOBAL` and are explicitly OUT OF SCOPE for `vb-oul6u`. They
must be filed as separate prerequisite-repair beads:

1. **264 pre-existing clippy errors** in `lib.rs` cfg-block
   `#[allow(...)]` vs workspace `[lints]` `forbid` conflicts.
2. **2 pre-existing `as_conversions`** in
   `crates/vb_runtime/tests/recovery_hydration_tests.rs:1145,1151`.

These are recorded in `assurance-bundle.md` §"Waivers And Deferred
Work" row "Pre-existing BLOCK_GLOBAL" with reason "owner_approved_debt".

## Residual Non-Blocking Notes

- The `moon ci` rollup cannot resolve Git `main` in this jj
  workspace (environment issue, not a bead issue). Exact `cargo`
  gates all pass.
- The contract text amendment (INV-004 reference to `f32::from(u32)`)
  is filed as future maintenance debt in `assurance-bundle.md`
  §"Waivers And Deferred Work". Not blocking landing because the
  in-file annotation at `runtime.rs:614-619` documents the
  deviation with pointer to the equivalence proof.

## Final Verdict

**STATUS: APPROVED.** Bead `vb-oul6u` is approved for landing.

Required artifacts produced in this combined p12-14 dispatch:

| Artifact | Status |
|---|---|
| `.beads/vb-oul6u/formal-verification-report.md` | STATUS: APPROVED |
| `.beads/vb-oul6u/verification-ledger.jsonl` | 3 PASS rows, JSONL-valid |
| `.beads/vb-oul6u/black-hat-review.md` | STATUS: APPROVED |
| `.beads/vb-oul6u/assurance-bundle.md` | STATUS: APPROVED |
| `.beads/vb-oul6u/truth-serum-report.md` | STATUS: APPROVED |
| `.beads/vb-oul6u/final-evidence-decision.md` (this file) | STATUS: APPROVED |

Bead is ready for the femdation's next-step handoff.
