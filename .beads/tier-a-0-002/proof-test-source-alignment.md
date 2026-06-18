STATUS: PASS

# Proof/Test/Source Alignment — tier-a-0-002

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| RQ-001 | PO-RQ-001 | RRO-RQ-001 | `scripts/forbid-runtime-fmt.sh::*`, `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::*` | `test_quarantine_gate_blocks_json_import` | [] | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | PASS | closed |
| RQ-002 | PO-RQ-002 | RRO-RQ-002 | `velvet-ballistics-MASTER.md::section_43_automatic_rejection_triggers_2056_2060`, `ResiduePolicy::from_master`, `ForbiddenImport::from_name`, `expected_master_trigger`, `master_line_matches` | `test_static_evidence_binds_master_rejection_triggers` | [] | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers` | PASS | closed |
| RQ-003 | PO-RQ-003 | RRO-RQ-003 | `scripts/forbid-runtime-fmt.sh::*`, `GateError::exit_code`, `ResidueQuarantine::decide` | `test_quarantine_gate_blocks_unbounded_channel` | [] | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | PASS | closed |
| RQ-004 | PO-RQ-004 | RRO-RQ-004 | `ResidueQuarantine::diff_against_allowlist`, `AllowlistRef::load`, `.moon/tasks/all.yml::*` | `test_moon_ci_quarantine_dependency_correctly_ordered` | [] | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | PASS | closed |
| RQ-005 | PO-RQ-005 | RRO-RQ-005 | `scripts/forbid-runtime-fmt.sh::sort_unique`, `ResidueMatch::active_line`, `ResidueMatch::allowlisted_line`, `ScanReport::summary_line`, `emit_pass`, `emit_fail` | `test_static_evidence_binds_real_formatter_symbols` | [] | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols` | PASS | closed |

Evidence logs: RQ-001 `evidence/state12-repair-po-rq-001.log`; RQ-002 `evidence/state12-repair-rro-rq-002.log`; RQ-003 `evidence/state12-repair-po-rq-003.log`; RQ-004 `evidence/state12-repair-po-rq-004.log`; RQ-005 `evidence/state12-repair-rro-rq-005.log`.

All RRO rows are `mapping_status=verified` and `status=verified`.
