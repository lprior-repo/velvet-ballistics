STATUS: PASS

# State 9 Black-Hat Repair Test Writer Report — tier-a-0-002

state: 9 repair after black-hat rejection
skill: test-writer
repair_invocation_id: tier-a-0-002-s9-blackhat-repair-gpt55
parent_invocation_id: tier-a-0-002-s13-black-hat-reviewer-gpt55
workspace: /home/lewis/src/femdation-tier-a-0-002
artifact_root: .beads/tier-a-0-002
completed_at: 2026-06-18T07:12:14Z

## Scope

This State 9 repair adds failing-first tests and fixtures only. No production
implementation file was modified. The new tests expose the State 13 black-hat
blockers so the next implementation repair cannot relaunder them as passing
evidence.

## Repairs Made

1. **BH-001 unbounded-channel syntax bypass:** Added hot-crate fixtures for
   `use tokio::sync::mpsc::{unbounded_channel};` and
   `tokio :: sync :: mpsc :: unbounded_channel::<u8>()`, then extended
   `test_quarantine_gate_blocks_unbounded_channel` to require exit `1`, exact
   `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` diagnostics, and exact summary
   counts for those forms.
2. **BH-002 RQ-002 static evidence laundering:** Added
   `test_static_evidence_binds_master_rejection_triggers`, which derives the
   actual master §43 `Automatic rejection triggers` line numbers and requires
   each `ForbiddenImportName` source master reference to bind section `43` plus
   the discovered line. It also rejects proof/RRO evidence that still counts
   report-field triggers `7..10`.
3. **BH-003 RQ-005 nonexistent formatter symbol:** Added
   `test_static_evidence_binds_real_formatter_symbols`, which fails if RQ-005
   cites nonexistent `ResidueMatch::fmt` and requires real source refs:
   `ResidueMatch::active_line`, `ResidueMatch::allowlisted_line`,
   `ScanReport::summary_line`, `emit_pass`, and `emit_fail`.
4. **BH-004 production compile timeout gap:** Strengthened
   `test_moon_ci_quarantine_dependency_correctly_ordered` with a production
   compile-bound check. The test accepts either an outer Moon `timeout` around
   `scripts/forbid-runtime-fmt.sh` or a wrapper `timeout` on the `rustc` compile
   line; the current implementation fails because only the already-compiled
   scanner execution is bounded.
5. **Global Moon check classification:** The repaired report and ledger keep
   `moon run :check` classified as `BLOCK_GLOBAL` because the observed failure
   remains the pre-existing `check-removed-crate-residue` `vb_codegen` residue,
   after the local `forbid-runtime-fmt` gate passes.

## Files Written or Updated

- `scripts/test-forbid-runtime-fmt.sh`
- `fixtures/forbid-runtime-fmt/negative_unbounded_grouped_import.rs`
- `fixtures/forbid-runtime-fmt/negative_unbounded_spaced_path.rs`
- `.beads/tier-a-0-002/test-writer-report.md`
- `.beads/tier-a-0-002/transcripts/state-9-blackhat-repair.txt`
- `.beads/tier-a-0-002/agent-invocation-ledger.jsonl`

## Exact Test Names

1. `test_quarantine_gate_blocks_json_import`
2. `test_quarantine_gate_blocks_unbounded_channel`
3. `test_moon_ci_quarantine_dependency_correctly_ordered`
4. `test_static_evidence_binds_master_rejection_triggers`
5. `test_static_evidence_binds_real_formatter_symbols`

## Behavior and Static Evidence Matrix

| Black-hat finding | New/strengthened test | Current result | Blocker exposed |
|---|---|---:|---|
| BH-001 grouped/spaced unbounded channel bypass | `test_quarantine_gate_blocks_unbounded_channel` | RED_EXPECTED | grouped import exits `0` with `active=0` |
| BH-002 RQ-002 wrong master evidence | `test_static_evidence_binds_master_rejection_triggers` | RED_EXPECTED | source refs bind `(2,99)`/`(12,*)`, not §43 lines `2056..2060`; proof map still counts report fields `7..10` |
| BH-003 RQ-005 nonexistent formatter ref | `test_static_evidence_binds_real_formatter_symbols` | RED_EXPECTED | RRO/proof artifacts still cite `ResidueMatch::fmt` and miss real formatter symbols |
| BH-004 compile step unbounded | `test_moon_ci_quarantine_dependency_correctly_ordered` | RED_EXPECTED | Moon command is direct bash and wrapper `rustc` line lacks `timeout` |
| Global moon residue | `timeout 120s moon run :check` | BLOCK_GLOBAL | `check-removed-crate-residue` active `vb_codegen` residue outside this bead |

## Commands Run and Outcomes

| Command | Outcome |
|---|---|
| `bash -n scripts/test-forbid-runtime-fmt.sh` | PASS |
| `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | PASS |
| `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | RED_EXPECTED: grouped import bypass missed |
| `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers` | RED_EXPECTED: RQ-002 master/source evidence is non-binding |
| `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols` | RED_EXPECTED: RQ-005 still maps nonexistent `ResidueMatch::fmt` |
| `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | RED_EXPECTED: production compile step unbounded |
| `bash scripts/test-forbid-runtime-fmt.sh` | RED_EXPECTED at unbounded grouped-import fixture after JSON test passes |
| `bash scripts/forbid-runtime-fmt.sh` | PASS: `summary: active=0 allowlisted=0 files_scanned=828 hot_paths=291 cold_paths=537` |
| `moon run :forbid-runtime-fmt` | PASS |
| `timeout 120s moon run :check` | BLOCK_GLOBAL: `check-removed-crate-residue` active `vb_codegen` residue |

## Raw Evidence Excerpts

### BH-001 grouped import bypass

```text
[2/5] test_quarantine_gate_blocks_unbounded_channel
AssertionFailed: grouped-import unbounded-channel fixture expected exit 1, got 0
Output:
summary: active=0 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0
```

### BH-002 RQ-002 master binding failure

```text
MASTER-REF-MISMATCH: TokioSyncMpscUnbounded source ref (12, 427) does not bind §43 line 2056: unbounded queue/loop/retry/fanout
MASTER-REF-MISMATCH: SerdeYaml source ref (12, 421) does not bind §43 line 2057: YAML interpreted at runtime
MASTER-REF-MISMATCH: SerdeJson source ref (2, 99) does not bind §43 line 2058: JSON inserted into runtime core
MASTER-REF-MISMATCH: Hyper source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: Reqwest source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: Axum source ref (12, 423) does not bind §43 line 2059: HTTP inserted into runtime core
MASTER-REF-MISMATCH: HashMapStringGeneric source ref (12, 411) does not bind §43 line 2060: HashMap<String, Value> runtime state
RRO-NONBINDING: RQ-002 still cites report-field triggers 7-10
PROOF-MAP-NONBINDING: RQ-002 evidence still counts report fields 7-10
```

### BH-003 RQ-005 formatter binding failure

```text
RRO-NONEXISTENT-SYMBOL: RQ-005 source_refs include ResidueMatch::fmt
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ResidueMatch::active_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ResidueMatch::allowlisted_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::ScanReport::summary_line
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::emit_pass
RRO-MISSING-SOURCE-REF: RQ-005 missing scripts/forbid-runtime-fmt.rs::emit_fail
ARTIFACT-NONEXISTENT-SYMBOL: proof-to-rust-map.md still cites ResidueMatch::fmt
ARTIFACT-NONEXISTENT-SYMBOL: proof-test-source-alignment.md still cites ResidueMatch::fmt
```

### BH-004 production compile bound failure

```text
UNBOUNDED-COMPILE: production gate has no timeout around rustc compile step
moon command lines: ["command: 'bash scripts/forbid-runtime-fmt.sh'"]
wrapper rustc lines: ['compile_output="$(rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o "$tmp_bin" 2>&1)"']
```

### BLOCK_GLOBAL moon check residue

```text
velvet-ballistics:check-removed-crate-residue | crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract/discovery.rs:223: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen':             "crates/vb_codegen/src/generated/interface.rs".to_string(),
velvet-ballistics:check-removed-crate-residue | summary: active=1 allowlisted=26 files_scanned=2475
Error: task_runner::run_failed
Task velvet-ballistics:check-removed-crate-residue failed to run.
```

## Validator Evidence

```text
COMMAND: python3 /home/lewis/.agents/skills/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/src/femdation-tier-a-0-002 --bead tier-a-0-002 --state 9 --source-checkout /home/lewis/src/velvet-ballistics --format text
STATUS: FAIL
E_STATUS_NOT_APPROVED black-hat-review.md - status tokens=['REJECTED', 'REJECTED', 'REJECTED']
```

The remaining validator blocker is the expected State 13 black-hat rejection
being repaired by these red tests. Invocation ledger integrity findings are not
present after reseal.

## Proof/Refinement Coverage Matrix

| Proof ID | Refinement ID | Requirement | Contract Clause | Source Refs | Behavior Test Refs | Verifier | Evidence Command | State 9 Repair Status |
|---|---|---|---|---|---|---|---|---|
| PO-RQ-001 | RRO-RQ-001 | RQ-001 | `3.2_pass_iff_no_active_residue` | `ResidueQuarantine::run`; `ResidueQuarantine::decide` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | PASS |
| PO-RQ-002 | RRO-RQ-002 | RQ-002 | `3.4_closed_set_invariant` | `ForbiddenImportName`; `ForbiddenImport::from_name`; master §43 trigger lines | `scripts/test-forbid-runtime-fmt.sh::test_static_evidence_binds_master_rejection_triggers` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers` | RED_EXPECTED; exposes non-binding evidence |
| PO-RQ-003 | RRO-RQ-003 | RQ-003 | `3.2_pass_iff_no_active_residue` | `ResidueQuarantine::decide`; `GateError::exit_code`; token/path detector | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | RED_EXPECTED; exposes grouped import bypass |
| PO-RQ-004 | RRO-RQ-004 | RQ-004 | `3.4_closed_set_invariant` | `diff_against_allowlist`; `.moon/tasks/all.yml::forbid-runtime-fmt`; `.moon/tasks/all.yml::check` | `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | RED_EXPECTED; compile step unbounded |
| PO-RQ-005 | RRO-RQ-005 | RQ-005 | `3.3_stderr_format` | `ResidueMatch::active_line`; `ResidueMatch::allowlisted_line`; `ScanReport::summary_line`; `emit_pass`; `emit_fail` | `scripts/test-forbid-runtime-fmt.sh::test_static_evidence_binds_real_formatter_symbols` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols` | RED_EXPECTED; exposes nonexistent `ResidueMatch::fmt` mapping |
