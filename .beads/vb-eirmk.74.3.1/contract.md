# vb-eirmk.74.3.1 Closure Evidence Validator Migration Contract

## Goal
Migrate the Python closure evidence checker (scripts/check-vb-jpq7-closure-evidence.py) to native Rust,
preserving all validation behavior, exit codes, and output markers.

## Behavior Requirements
1. Query all closed vb-jpq7 children via `bd list --parent vb-jpq7 --status all --json -n 0`
2. Validate JSONL manifest rows for 12 required fields
3. Validate commit_sha format: 7-64 lowercase hex chars
4. Validate timestamp: RFC3339 UTC ending with Z
5. Validate exit_code is integer (not string/bool)
6. Validate raw_log_path exists and is non-empty with provenance bindings
7. Reject evidence markers: summary-only, cached-only, skipped-only, subagent-only, delegated-only
8. Accept closure rows: exit_code=0, status in {PASS, CLOSURE_PASS, EXECUTED_PASS}, evidence_kind in {raw-command, command, raw-log, live-command}
9. Accept resolution rows: exit_code!=0 with split_followup (split_bead_id, rationale) or approved_waiver (waiver_id, rationale)
10. Detect duplicate split_followup with same rationale
11. Validate closure prose for rejected markers

## Exit Codes
- 0: VB_JPQ7_CLOSURE_EVIDENCE_PASS
- 1: VB_JPQ7_CLOSURE_EVIDENCE_FAIL with failures listed
- 2: Operational error (bd query failure, invalid JSON, etc.)

## CLI Interface
--parent (default: vb-jpq7), --manifest (default: .beads/vb-jpq7/closure-evidence-manifest.jsonl),
--bd-workdir (default: .), --self-test

## Architecture
Pure JSON validation separated from filesystem/CLI actions. CLI handles I/O, module handles validation logic.

## Design Decisions
- serde_json for JSON parsing (existing dependency)
- No unwrap/panic/todo/dbg in production code
- No /tmp paths; use persistent directories only
- Comprehensive unit tests for all validation cases plus CLI integration tests
- Self-test uses local fixture directory, not /tmp
