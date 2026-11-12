bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 11
updated_at: 2026-05-18T00:00:00Z
attempt: 1

## Formal Verification Report

### Machine Gate Results

**Test Gate (cargo test -p vb_cli --all-features):**
- 564 passed, 1 ignored (16 suites)
- Exit code: 0
- STATUS: PASS

**Clippy Gate (cargo clippy -p vb_cli --lib --bins --all-features -- -D warnings -D unsafe_code):**
- No issues found
- Exit code: 0
- STATUS: PASS

**Fmt Gate (cargo fmt --check -p vb_cli):**
- No diff
- Exit code: 0
- STATUS: PASS

### Previous FAIL_FIRST Tests — Now PASS

| Test | Before | After | Delta |
|------|--------|-------|-------|
| `parse_run_id_rejects_zero` | FAIL (Ok(RunId(0))) | PASS | FIXED |
| `read_journal_events_returns_storage_error_when_dir_not_found` | FAIL (exit 0) | PASS | FIXED |

### Regression Check

No regressions introduced by State 10 fixes:
- 564 vb_cli tests: all pass
- Clippy: clean
- Fmt: clean
- Both FAIL_FIRST tests now pass

### Classification

- Both failures: **BLOCK_LOCAL** (in delivery scope, fixed in this state)
- No **BLOCK_REGRESSION**, no **DEFERRED_GLOBAL**
- No new warnings, no banned patterns
