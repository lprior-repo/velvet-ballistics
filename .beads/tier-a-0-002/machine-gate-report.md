STATUS: PASS

# Machine Gate Report — tier-a-0-002 State 12 Repair

| Command | Exit | Classification | Evidence |
|---|---:|---|---|
| `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | 0 | PASS | `evidence/state12-repair-po-rq-001.log` |
| `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | 0 | PASS | `evidence/state12-repair-po-rq-003.log` |
| `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | 0 | PASS | `evidence/state12-repair-po-rq-004.log` |
| `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers` | 0 | PASS | `evidence/state12-repair-rro-rq-002.log` |
| `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols` | 0 | PASS | `evidence/state12-repair-rro-rq-005.log` |
| `bash scripts/test-forbid-runtime-fmt.sh` | 0 | PASS | `evidence/state12-repair-test-forbid-runtime-fmt-all.log` |
| `bash scripts/forbid-runtime-fmt.sh` | 0 | PASS | `evidence/state12-repair-forbid-runtime-fmt-direct.log` |
| `moon run :forbid-runtime-fmt` | 0 | PASS | `evidence/state12-repair-moon-forbid-runtime-fmt.log` |
| `rustup run nightly-2026-04-28 rustfmt --edition 2024 --check scripts/forbid-runtime-fmt.rs` | 0 | PASS | `evidence/state12-repair-rustfmt-nightly-edition2024-forbid-runtime-fmt-rs.log` |
| `rustup run nightly-2026-04-28 rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o target/gate-tools/forbid-runtime-fmt-rustc-check` | 0 | PASS | `evidence/state12-repair-rustc-forbid-runtime-fmt-rs.log` |
| `timeout 120s moon run :check` | 1 | FAIL_GLOBAL | `evidence/state12-repair-moon-check.log` |
| original PO-RQ-002 planned command | 1 | FAIL_LOCAL stale planned command | `evidence/state12-repair-po-rq-002-planned.log` |
| original PO-RQ-005 planned command | 0 | FAIL_LOCAL non-binding planned command | `evidence/state12-repair-po-rq-005-planned.log` |

Notes:
- Direct and Moon residue quarantine gates report `summary: active=0 allowlisted=0 files_scanned=828 hot_paths=291 cold_paths=537`.
- `moon run :check` fails after local residue quarantine passes; the blocker is existing `check-removed-crate-residue` `vb_codegen` residue outside this bead's scope.
- Existing `black-hat-review.md` remains rejected until re-review even though the repair evidence above closes the listed findings locally.
