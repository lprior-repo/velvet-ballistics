STATUS: APPROVED
reviewer_skill: test-reviewer
reviewer_invocation_id: tier-a-0-002-s10-test-reviewer-blackhat-rereview-gpt55
writer_invocation_id: tier-a-0-002-s9-blackhat-repair-gpt55
previous_reviewer_invocation_id: tier-a-0-002-s10-test-reviewer-rereview-sif-7bb1d2c4
review_state: 10
bead_id: tier-a-0-002
workspace: /home/lewis/src/femdation-tier-a-0-002
schema_version: test-suite-review/v1
reviewed_at: 2026-06-18T08:40:00.000000+00:00

# Test Suite Re-Review — tier-a-0-002

## Findings

None. The State 9 black-hat repair added valid failing-first coverage for the
State 11 implementation repair. The suite uses the public gate entrypoints,
asserts exact exit codes and diagnostics, keeps expensive checks bounded, and
does not relabel the known global Moon failure as local bead evidence.

## Black-Hat Repair Coverage Matrix

| Required red coverage | Reviewed test evidence | Current outcome | Disposition |
|---|---|---:|---|
| Grouped/spaced unbounded imports | `test_quarantine_gate_blocks_unbounded_channel` stages `negative_unbounded_grouped_import.rs` and `negative_unbounded_spaced_path.rs`, then requires exit `1`, exact `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` file/line snippets, `active=1`, and no cross-pattern false positives. | RED_EXPECTED at grouped fixture (`active=0`) | approved |
| RQ-002 master binding | `test_static_evidence_binds_master_rejection_triggers` derives the actual master `Automatic rejection triggers` fenced block and requires every `ForbiddenImportName` master ref to bind §43 lines `2056..2060`; it also rejects report-field trigger laundering in RRO/proof-map text. | RED_EXPECTED | approved |
| RQ-005 real symbol binding | `test_static_evidence_binds_real_formatter_symbols` requires real formatter symbols in `scripts/forbid-runtime-fmt.rs` (`ResidueMatch::active_line`, `ResidueMatch::allowlisted_line`, `ScanReport::summary_line`, `emit_pass`, `emit_fail`), requires RQ-005 RRO refs to point at those symbols, and rejects stale `ResidueMatch::fmt` references in proof artifacts. | RED_EXPECTED | approved |
| Production rustc compile timeout | `test_moon_ci_quarantine_dependency_correctly_ordered` first proves Moon wiring, then requires either an outer Moon timeout around `scripts/forbid-runtime-fmt.sh` or a wrapper timeout on the `rustc` compile line. | RED_EXPECTED | approved |
| BLOCK_GLOBAL classification | Local `bash scripts/forbid-runtime-fmt.sh` and `moon run :forbid-runtime-fmt` pass; bounded `timeout 120s moon run :check` fails only in `check-removed-crate-residue` for active `vb_codegen` residue outside this bead. | BLOCK_GLOBAL | approved |

## Suite Review Gates

| Gate | Result | Evidence |
|---|---:|---|
| Public API only | pass | Tests invoke `bash scripts/forbid-runtime-fmt.sh`, `moon run :forbid-runtime-fmt`, or parse committed public artifacts; they do not call private Rust functions. |
| Exact assertions | pass | Exit codes, exact diagnostic prefixes/snippets, summary counts, missing/nonexistent-symbol messages, and timeout wiring diagnostics are asserted. |
| Determinism | pass | Fixtures are committed under `fixtures/forbid-runtime-fmt/`; temp repos are isolated with `mktemp -d` and cleaned by trap. |
| Mutation resistance | pass | Raw substring detection, wrong master refs, stale `ResidueMatch::fmt`, and unbounded compile are each killed by a named test. |
| Resource governance | pass | Red tests are targeted by name; the only full Moon check is bounded by `timeout 120s` and classified as global, not as local passing evidence. |

## Raw Re-Run Evidence

```text
COMMAND: bash -n scripts/test-forbid-runtime-fmt.sh
exit_status=0

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import
[1/5] test_quarantine_gate_blocks_json_import
  ok: exit 1 with serde_json RUNTIME-FMT line
  ok: summary reports active=1 allowlisted=0
  ok: exact GateError checks cover PatternFileMissing and AllowlistParseFailure
exit_status=0

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel
[2/5] test_quarantine_gate_blocks_unbounded_channel
AssertionFailed: grouped-import unbounded-channel fixture expected exit 1, got 0
Output:
summary: active=0 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers
[4/5] test_static_evidence_binds_master_rejection_triggers
AssertionFailed: RQ-002 master/source binding expected exit 0, got 1
Output:
MASTER-REF-MISMATCH: TokioSyncMpscUnbounded source ref (12, 427) does not bind §43 line 2056: unbounded queue/loop/retry/fanout
MASTER-REF-MISMATCH: SerdeYaml source ref (12, 421) does not bind §43 line 2057: YAML interpreted at runtime
MASTER-REF-MISMATCH: SerdeJson source ref (2, 99) does not bind §43 line 2058: JSON inserted into runtime core
MASTER-REF-MISMATCH: Hyper source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: Reqwest source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: Axum source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: HashMapStringGeneric source ref (12, 411) does not bind §43 line 2060: HashMap<String, Value> runtime state
RRO-NONBINDING: RQ-002 still cites report-field triggers 7-10
PROOF-MAP-NONBINDING: RQ-002 evidence still counts report fields 7-10
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols
[5/5] test_static_evidence_binds_real_formatter_symbols
AssertionFailed: RQ-005 formatter source binding expected exit 0, got 1
Output:
RRO-NONEXISTENT-SYMBOL: RQ-005 source_refs include ResidueMatch::fmt
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ResidueMatch::active_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ResidueMatch::allowlisted_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ScanReport::summary_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::emit_pass
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::emit_fail
ARTIFACT-NONEXISTENT-SYMBOL: proof-to-rust-map.md still cites ResidueMatch::fmt
ARTIFACT-NONEXISTENT-SYMBOL: proof-test-source-alignment.md still cites ResidueMatch::fmt
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered
[3/5] test_moon_ci_quarantine_dependency_correctly_ordered
AssertionFailed: production rustc compile bound expected exit 0, got 1
Output:
UNBOUNDED-COMPILE: production gate has no timeout around rustc compile step
moon command lines: ["command: 'bash scripts/forbid-runtime-fmt.sh'"]
wrapper rustc lines: ['compile_output="$(rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o "$tmp_bin" 2>&1)"']
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh
[1/5] test_quarantine_gate_blocks_json_import
  ok: exit 1 with serde_json RUNTIME-FMT line
  ok: summary reports active=1 allowlisted=0
  ok: exact GateError checks cover PatternFileMissing and AllowlistParseFailure
[2/5] test_quarantine_gate_blocks_unbounded_channel
AssertionFailed: grouped-import unbounded-channel fixture expected exit 1, got 0
Output:
summary: active=0 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0
exit_status=1

COMMAND: bash scripts/forbid-runtime-fmt.sh
summary: active=0 allowlisted=0 files_scanned=828 hot_paths=291 cold_paths=537
exit_status=0

COMMAND: moon run :forbid-runtime-fmt
summary: active=0 allowlisted=0 files_scanned=828 hot_paths=291 cold_paths=537
Tasks: 1 completed
exit_status=0

COMMAND: timeout 120s moon run :check
velvet-ballistics:check-removed-crate-residue | crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract/discovery.rs:223: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen':             "crates/vb_codegen/src/generated/interface.rs".to_string(),
velvet-ballistics:check-removed-crate-residue | summary: active=1 allowlisted=26 files_scanned=2475
Task velvet-ballistics:check-removed-crate-residue failed to run.
exit_status=1
classification=BLOCK_GLOBAL

COMMAND: python3 /home/lewis/.agents/skills/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/src/femdation-tier-a-0-002 --bead tier-a-0-002 --state 10 --source-checkout /home/lewis/src/velvet-ballistics --format text
validator_status=FAIL
E_STATUS_NOT_APPROVED black-hat-review.md - status tokens=['REJECTED', 'REJECTED', 'REJECTED']
```

## Disposition

Approved for State 11 repair. The review artifacts may advance with the known
State 13 black-hat rejection still open; that rejection is exactly what these
red tests are designed to drive closed.
