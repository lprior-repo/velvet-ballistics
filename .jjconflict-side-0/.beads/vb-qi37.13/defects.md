# State 12 Rerun Defects — vb-qi37.13

STATUS: REJECTED
ROUTE: State 10 implementation repair -> State 11 formal-verifier rerun -> State 12 black-hat rerun.

## DEFECT-001 — Supported `--json` routes still emit raw text diagnostics

- severity: LETHAL
- owner_state: State 10
- rerun_from: State 10 implementation
- contract_refs: `.beads/vb-qi37.13/contract.md` lines 15 and 23-25.
- source_refs: `crates/velvet_ballastics/src/main.rs` lines 715-720 (`verify` invalid UTF-8 raw `errln!`); lines 218-224 (`parse_run_id` raw `errln!` with no `OutputFormat`).
- evidence: `verify <invalid-utf8-file> --json` emitted raw stderr `file is not valid UTF-8: ...`; `inspect not-a-run --db <tmp>/db --json` emitted raw stderr `invalid run_id ...`.
- required_fix: every public structured failure path must emit `DiagnosticReport` with `schema_version`, `kind`, stable `code`, `exit_code`, and `message` on stderr, with stdout empty.
- required_tests: black-box JSON and JSONL tests for invalid UTF-8 verify and invalid run-id inspect/events/replay/trace/retry/resume representative routes.

## DEFECT-002 — State 11 structured-diagnostic coverage is overclaimed

- severity: MAJOR
- owner_state: State 11 after State 10 repair
- rerun_from: State 11 formal-verifier
- evidence_refs: `.beads/vb-qi37.13/formal-verification-report.md` lines 61-66; `.beads/vb-qi37.13/machine-gate-report.md` lines 28-31; `crates/velvet_ballastics/tests/vb_qi37_13_structured_reconciliation.rs` lines 239-479 only cover the sampled matrix.
- required_fix: rerun State 11 after implementation repair and record raw command evidence for the expanded structured diagnostic matrix. Do not claim class-wide non-parse coverage from sampled routes.
