# Proof Plan Review Input: vb-qi37.12

## Review Request

Review State 4 proof planning repair after State 3 schema repair. Reject if any required row is unmapped, lacks repaired State 3 TLA metadata, uses generic commands where focused commands are required, is non-executable without an explicit blocker/waiver, or claims evidence that belongs in execution/review artifacts.

## Source Artifacts Read

- `.beads/vb-qi37.12/STATE.md`
- `.beads/vb-qi37.12/codebase-map.md`
- `.beads/vb-qi37.12/delivery-scope.jsonl`
- `.beads/vb-qi37.12/contract.md`
- `.beads/vb-qi37.12/tla-spec.md`
- `.beads/vb-qi37.12/lean-contract.md`
- `.beads/vb-qi37.12/verification-layers.md`
- `.beads/vb-qi37.12/proof-obligations.jsonl`
- `.beads/vb-qi37.12/traceability-matrix.jsonl`
- `.beads/vb-qi37.12/proof-review.md`
- `.beads/vb-qi37.12/proof-findings.jsonl`
- `.beads/vb-qi37.12/proof-repair-guide.md`
- `.beads/vb-qi37.12/contract-verification-review.md`
- `.beads/vb-qi37.12/proof-evidence.md`

## Discovery Commands

- `pwd -P`
- `case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; *) exit 0;; esac`
- `test -s ".beads/vb-qi37.12/contract.md"`
- `test -s ".beads/vb-qi37.12/traceability-matrix.jsonl"`
- `test -s ".beads/vb-qi37.12/delivery-scope.jsonl"`
- `jq -c . ".beads/vb-qi37.12/proof-obligations.jsonl" >/dev/null`
- `jq -c . ".beads/vb-qi37.12/traceability-matrix.jsonl" >/dev/null`
- `jq -c . ".beads/vb-qi37.12/delivery-scope.jsonl" >/dev/null`
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <delivery-scope files>`
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <delivery-scope files>`

## Planned Obligation Summary

- `TLA-ACK-001`: TLA+ persistence-before-ack safety/liveness.
- `TLA-REC-002`: TLA+ corrupt recovery fail-closed safety/liveness.
- `VERUS-CLS-003`: Verus abstract discard-classification kernel.
- `VERUS-DIAG-004`: Verus diagnostic envelope preservation kernel.
- `VERUS-DEC-005`: Verus corrupt decode classification kernel.
- `SCAN-DISCARD-006`: focused static scan plus classification report, required as a planned obligation; prior execution evidence is contextual only.
- `TEST-JOURNAL-007`: focused storage commands `rtk cargo test -p vb_storage decode_rejects` and `rtk cargo test -p vb_storage process_lock`.
- `TEST-RUNTIME-008`: focused runtime command `rtk cargo test -p vb_runtime diagnostic`.
- `FUZZ-DECODE-009`: wired fuzz target evidence with absolute TMPDIR/CARGO_TARGET_DIR command, required as a planned obligation; prior execution evidence is contextual only.
- `GATE-RELEASE-010`: final release-critical `moon ci` gate.
- `TLA-DEADLOCK-011`: required no-deadlock evidence with repaired TLA metadata and no deadlock-mask waiver.
- `NA-KANI-012`: Kani not applicable to repaired active obligations unless later reopened.
- `NA-PROPTEST-013`: proptest not applicable to repaired active obligations unless later reopened.
- `WAIVE-LEAN-014`: theorem-prover waiver for absent theorem-only kernel.
- `NA-LOOM-015`: Loom not applicable unless implementation adds concurrency interleavings.
- `NA-MIRI-016`: Miri not applicable unless implementation adds unsafe/UB risk.
- `NA-FLUX-017`: Flux not applicable because Verus owns the Rust-local kernels.
- `NA-DEPS-018`: dependency audit not applicable because dependencies did not change.

## Reviewer Focus

- Check that repaired canonical IDs replace stale `PO-*` IDs.
- Check that active rows, including `SCAN-DISCARD-006`, `FUZZ-DECODE-009`, and `TLA-DEADLOCK-011`, stay `status:"planned"` and do not launder PASS evidence from `proof-execution-ledger.jsonl` or `proof-evidence.md`.
- Check that all TLA+ planned rows include module/model/config/variables/actions/invariants/temporal properties/fairness/state constraints/refinement metadata matching repaired State 3.
- Check that focused critical test obligations use exact focused commands and that only `GATE-RELEASE-010` uses `moon ci`.
- Check whether Kani/proptest should stay non-applicable after repaired State 3 or must be reopened with explicit required rows.
- Check waiver rows for owner, reason, expiry/follow-up trigger, limitation, and compensating evidence.

## Non-Claims

- State 4 repair did not edit production code, tests, proof/model/harness/spec files, dependencies, or config.
- State 4 repair did not run TLA+, Verus, fuzz, tests, or CI.
- Planned rows do not claim PASS evidence.
