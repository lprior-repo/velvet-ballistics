# Proof Repair Guide: vb-qi37.12 State 6 Attempt 3

## Required Repairs

1. Repair `TLA-DEADLOCK-011`.
Remove explicit `Stutter` from `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla` `Next`. Keep `Spec == Init /\ [][Next]_vars /\ ...`. Rerun `tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla` and record raw output. Approval requires no `CHECK_DEADLOCK FALSE` and no explicit unconditional self-loop action in `Next`.

2. Discharge `SCAN-DISCARD-006`.
Create or refresh `silent-discard-scan-report.md` from the scoped raw candidates. The report must classify every candidate from `crates/vb_storage/src`, `crates/vb_runtime/src`, `crates/vb_compile/src`, and `crates/workspace_tests/src`; identify typed best-effort exceptions; and state zero unclassified release-critical silent discards with evidence. Raw grep output alone is not proof.

3. Discharge or validly waive `FUZZ-DECODE-009`.
Wire `vb_qi37_12_persisted_payload_decode` and run `cargo fuzz run vb_qi37_12_persisted_payload_decode -- -runs=1000`, or provide a waiver with owner, reason, expiry, limitation, and compensating evidence accepted by the verification reviewer. The oracle must reject corrupt/truncated recovery-critical payloads as typed errors and never hydrate them as empty success.

4. Resolve unexecuted required gate rows.
Either run `moon ci` and focused failure-injection evidence for `TEST-JOURNAL-007`, `TEST-RUNTIME-008`, and `GATE-RELEASE-010`, or update the obligation matrix so these are not required for State 6 approval and have explicit downstream ownership/expiry. Do not claim approval while required rows are `NOT_RUN`.

## Rerun Targets

- `jq -c . .beads/vb-qi37.12/proof-obligations.jsonl >/dev/null`
- `jq -c . .beads/vb-qi37.12/proof-execution-ledger.jsonl >/dev/null`
- `jq -c . .beads/vb-qi37.12/proof-findings.jsonl >/dev/null`
- `tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`
- `verus .beads/vb-qi37.12/proof/verus/discard_classification.rs`
- `verus .beads/vb-qi37.12/proof/verus/diagnostic_envelope.rs`
- `verus .beads/vb-qi37.12/proof/verus/recovery_decode_class.rs`
- `cargo fuzz run vb_qi37_12_persisted_payload_decode -- -runs=1000`
