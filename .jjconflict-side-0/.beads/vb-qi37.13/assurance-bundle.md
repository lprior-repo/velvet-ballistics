bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 13
updated_at: 2026-05-18T21:48:33Z
attempt: 1-of-7

# Assurance Bundle

STATUS: APPROVED

## Requirements mapped to evidence

- Structured envelopes for operator commands: covered by `contract.md`, `traceability-matrix.jsonl`, `proof-obligations.jsonl`, and `vb_qi37_13_structured_reconciliation` focused test evidence: 14 passed.
- Stable diagnostics with `code`, `message`, `exit_code`, and `kind=DiagnosticReport`: covered by `envelope_schema_tests` (12 passed) and structured reconciliation invalid UTF-8 / invalid run-id tests.
- Public exit codes 0-8: covered by existing `formal-verification-report.md` and `verification-ledger.jsonl` with 9 PASS, 0 failures/waivers.
- Text/YAML/postcard structured emitters and postcard validation: covered by existing formal report plus `vb_ui_model` postcard evidence: 14 passed, 152 filtered out.
- No local regression in touched CLI surface: covered by `cargo clippy -p vb_cli --all-features -- -D warnings` (`No issues found`) and `cargo fmt --check -p vb_cli` (exit 0/no output).

## Review evidence

- `proof-review.md`: `STATUS: APPROVED`.
- `contract-verification-review.md`: `STATUS: APPROVED`.
- `test-plan-review.md`: `STATUS: APPROVED`.
- `test-suite-review.md`: `STATUS: APPROVED`.
- `formal-verification-report.md`: `STATUS: APPROVED`.
- `black-hat-review.md`: `STATUS: APPROVED`.

## Raw command evidence from active context

- `TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_cli --test vb_qi37_13_structured_reconciliation --all-features` -> `cargo test: 14 passed (1 suite, 0.00s)`.
- `TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_cli --test envelope_schema_tests --all-features` -> `cargo test: 12 passed (1 suite, 0.00s)`.
- `TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_ui_model --all-features postcard` -> `cargo test: 14 passed, 152 filtered out (2 suites, 0.00s)`.
- `TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo clippy -p vb_cli --all-features -- -D warnings` -> `cargo clippy: No issues found`.
- `TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo fmt --check -p vb_cli` -> no output, exit 0.

## Blockers / waivers

- Blocking findings: none.
- Required waivers: none for this closure.
