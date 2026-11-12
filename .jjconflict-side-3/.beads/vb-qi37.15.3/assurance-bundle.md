bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 13
updated_at: 2026-05-18T00:00:00Z

# Assurance Bundle — vb-qi37.15.3

## Requirement-to-Evidence Map

### PRE-001: run_id argument is valid (non-zero, valid characters)

| Evidence Type | Artifact | Status |
|---|---|---|
| Contract clause | contract.md: PRE-001 | ✓ |
| Proof obligation | proof-obligations.jsonl: TRACE-ERR-001 | ✓ |
| Test | main_tests.rs: `parse_run_id_rejects_zero`, `parse_run_id_accepts_valid_decimal` | ✓ |
| Verification command | `cargo test -p vb_cli parse_run_id_rejects_zero` → 1 passed | ✓ |
| Review | proof-review.md (APPROVED), test-suite-review.md (APPROVED) | ✓ |

### PRE-002: --db names accessible Fjall journal directory

| Evidence Type | Artifact | Status |
|---|---|---|
| Contract clause | contract.md: PRE-002 | ✓ |
| Test | cli_trace_integration.rs: `read_journal_events_returns_storage_error_when_dir_not_found` | ✓ |
| Verification command | `cargo test -p vb_cli read_journal_events_returns_storage_error` → 1 passed | ✓ |
| Review | test-suite-review.md (APPROVED) | ✓ |

### POST-001: trace outputs all journal events

| Evidence Type | Artifact | Status |
|---|---|---|
| Contract clause | contract.md: POST-001 | ✓ |
| Proof | proof-evidence.md: TRACE-CLI-001, TRACE-VERUS-001 | ✓ |
| Test | commands_journal.rs: `trace_outputs_all_journal_events` | ✓ |
| Review | proof-review.md (APPROVED) | ✓ |

### ERR-002: Journal directory not found → StorageError

| Evidence Type | Artifact | Status |
|---|---|---|
| Contract clause | contract.md: ERR-002 | ✓ |
| Implementation | app_impl.rs: `read_journal_events` checks `db.exists()` | ✓ |
| Test | cli_trace_integration.rs: exit code 5 | ✓ |
| Verification command | `cargo test -p vb_cli read_journal_events_returns_storage_error` → 1 passed | ✓ |

---

## Unresolved Waiver / Debt Table

| Item | Type | Evidence |
|---|---|---|
| None | — | — |

---

## Gate Evidence Summary

| Gate | Result | Command |
|---|---|---|
| test (vb_cli) | PASS (564 passed) | `cargo test -p vb_cli --all-features` |
| clippy (vb_cli) | PASS (No issues found) | `cargo clippy -p vb_cli --lib --bins --all-features -- -D warnings -D unsafe_code` |
| fmt (vb_cli) | PASS (No diff) | `cargo fmt --check -p vb_cli` |

---

## Approval Chain

| Review | STATUS | Date |
|---|---|---|
| proof-review.md | APPROVED | 2026-05-18 |
| contract-verification-review.md | APPROVED | 2026-05-18 |
| test-plan-review.md | APPROVED | 2026-05-18 |
| test-suite-review.md | APPROVED | 2026-05-18 |
| black-hat-review.md | APPROVED | 2026-05-18 |
