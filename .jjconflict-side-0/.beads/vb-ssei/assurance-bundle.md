bead_id: vb-ssei
phase: 13
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Assurance bundle

Requirement coverage:
- REQ-SSEI-001 -> `vb_ssei_verification_admission_acceptance.rs` -> targeted cargo test PASS.
- REQ-SSEI-002 -> `test_admission_accepts_when_all_verification_gates_pass` -> PASS.
- REQ-SSEI-003 -> `test_strict_verify_emits_certificate_when_workflow_is_safe` -> PASS.
- REQ-SSEI-004 -> `test_admission_rejects_when_capability_missing` -> PASS.
- REQ-SSEI-005 -> `test_admission_rejects_when_ir_digest_mismatches_artifact` -> PASS.
- REQ-SSEI-006 -> `vb_hxm0_acceptance_catalog` -> PASS.

Raw command evidence is recorded in `machine-gate-report.md` and `verification-ledger.jsonl`.

Deferred global debt:
- `moon ci` unrelated fmt/check failures in `vb_codegen`/`vb_storage`, outside touched files, classified in `regression-diff.md`.
