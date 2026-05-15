# vb-qi37.6 Proof Repair Guide — FINAL ATTEMPT (7-of-7)

## Exact Nearest Route

State 6 retry 7 (final) confirms the same 3 blockers as all prior retries. No new repair evidence was produced by State 5 after retry 4. No State 10/11 repair artifacts exist. The repair route is unchanged but this is the final attempt — per retry_policy_7, attempt 7 failure blocks landing:

1. Route `INTEG-011` to **State 10 implementation**: repair the storage/artifact validation path so `cargo test -p vb_storage submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability --lib` passes. The failure is `journal open failed: artifact structure validation failed` — this is a production storage behavior defect, not a proof-writer issue.
2. Route `INTEG-012` to **State 10 implementation**: align storage `ADMISSION_GATE_COUNT` to canonical `15` (runtime is canonical `15`, storage currently emits `2`). After State 10 repair, rerun the exact planned runtime/storage command and capture output proving storage emits `15`.
3. Route `GATE-016` to **State 11 formal-verifier** after State 10 repairs: run `moon ci` successfully from a Git-aware, quota-sufficient isolated workspace, OR produce raw-log-backed classification that every remaining failure is `DEFERRED_GLOBAL` with no bead-local proof/lint/unsafe/panic/unwrap/index/arithmetic regression.

## Do Not Route Back to State 5

State 5 may only refresh proof evidence if verifier/proof artifacts change or if State 10/11 produce new raw evidence consumed by State 6. No such new evidence exists at this time.

## Already Acceptable For Next Retry (No Change Needed)

- Verus/TLC evidence: `VERUS-CAP-001`, `VERUS-CARD-003`, `VERUS-CERT-007`, `TLA-LIFE-004`, `TLA-DENY-005`, `TLA-DRIVE-006` are proven and stable.
- Kani split harnesses: `KANI-CAP-002` and `RUNTIME-KANI-010` split harness mapping was accepted in retry 4.
- Fuzz: `SCHEMA-FUZZ-008` and `SCHEMA-FUZZ-009` passed 1000-run GNU target execution.
- `INTEG-013`, `INTEG-014`: exact command pass.
- `contract-verification-review.md`: APPROVED.

## Rerun Requirements

After State 10 and State 11 repairs complete:
1. Validate all JSONL files: `jq -c . .beads/vb-qi37.6/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.6/traceability-matrix.jsonl >/dev/null`.
2. Capture raw command evidence for `INTEG-011`, `INTEG-012`, and `GATE-016`.
3. Update `proof-writer-report.md` and `proof-evidence.md` with new raw evidence.
4. Retry State 6 only after fresh evidence is on disk.
