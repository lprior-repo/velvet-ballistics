bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 11
updated_at: 2026-05-18T00:00:00Z
attempt: 1

## Machine Gate Report

### verify-standard Lane

| Gate | Command | Exit | Status |
|------|---------|------|--------|
| test | `cargo test -p vb_cli --all-features` | 0 | PASS |
| clippy | `cargo clippy -p vb_cli --lib --bins --all-features -- -D warnings -D unsafe_code` | 0 | PASS |
| fmt | `cargo fmt --check -p vb_cli` | 0 | PASS |

### Verification Ledger

| Obligation | Result | Evidence |
|-----------|--------|---------|
| parse_run_id rejects zero | PASS | `cargo test -p vb_cli parse_run_id_rejects_zero` → 1 passed |
| read_journal_events StorageError | PASS | `cargo test -p vb_cli read_journal_events_returns_storage_error` → 1 passed |

### Classification

- BLOCK_LOCAL: 0 (fixed)
- BLOCK_REGRESSION: 0
- DEFERRED_GLOBAL: 0 (xtask pre-existing clippy errors not in delivery scope)
