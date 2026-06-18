STATUS: APPROVED

# Black Hat Re-Review — tier-a-0-002

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
state: 13 black-hat-reviewer re-review after State 11/12 repairs
reviewer_skill: black-hat-reviewer
reviewer_invocation_id: tier-a-0-002-s13-black-hat-rereview-gpt55
parent_invocation_id: tier-a-0-002-s12-formal-verifier-repair-gpt55
source_checkout: /home/lewis/src/velvet-ballistics
workspace: /home/lewis/src/femdation-tier-a-0-002
artifact_root: .beads/tier-a-0-002
reviewed_at: 2026-06-18T09:45:00Z
attempt: 1-of-7
model: openai/gpt-5.5

## Gate Result

**STATUS: APPROVED**

The repaired residue quarantine gate now closes the State 13 defects locally: grouped and spaced unbounded-channel syntax is blocked, RQ-002 binds the scanner to the actual master §43 rejection lines, RQ-005 names real formatter symbols and deterministic channel replay passes, the production `rustc` compile step is bounded, and the previously overlong scanner functions were split. The broader `moon run :check` failure remains a pre-existing global removed-crate residue outside this bead.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---|---:|---|---|
| BH-001 unbounded channel detector bypass via grouped/spaced valid Rust syntax | CRITICAL | `scripts/forbid-runtime-fmt.rs:120-124`, `scripts/forbid-runtime-fmt.rs:979-990`, `scripts/test-forbid-runtime-fmt.sh:683-723` | closed |
| BH-002 RQ-002 static evidence checked wrong master content | CRITICAL | `scripts/forbid-runtime-fmt.rs:90-108`, `scripts/forbid-runtime-fmt.rs:527-574`, `scripts/test-forbid-runtime-fmt.sh:388-494` | closed |
| BH-003 RQ-005 proof/source parity cited nonexistent formatter symbol | HIGH | `scripts/forbid-runtime-fmt.rs:271-289`, `scripts/forbid-runtime-fmt.rs:634-690`, `scripts/test-forbid-runtime-fmt.sh:502-556` | closed |
| BH-004 CI runtime not bounded around compilation | HIGH | `scripts/forbid-runtime-fmt.sh:31-35`, `scripts/forbid-runtime-fmt.sh:62-67`, `scripts/test-forbid-runtime-fmt.sh:317-385` | closed |
| BH-005 scanner function-size discipline violation | MEDIUM | `scripts/forbid-runtime-fmt.rs:396-404`, `scripts/forbid-runtime-fmt.rs:846-858` | closed |

### Closure evidence

- **BH-001:** `matches_tokio_mpsc_unbounded` now consumes whitespace-normalized `compact` input and checks grouped `tokio::sync::mpsc::{...}` imports. `test_quarantine_gate_blocks_unbounded_channel` asserts the direct call, grouped import, and spaced path fixtures all fail closed with exact diagnostics.
- **BH-002:** `ForbiddenImport::from_name` now binds the seven `ForbiddenImportName` variants to master §43 trigger lines 2056-2060, and `master_line_matches`/`expected_master_trigger` validate the exact line text before scanning.
- **BH-003:** RRO-RQ-005, `proof-to-rust-map.md`, and `proof-test-source-alignment.md` now point at real symbols: `ResidueMatch::active_line`, `ResidueMatch::allowlisted_line`, `ScanReport::summary_line`, `emit_pass`, and `emit_fail`. Separate stdout/stderr replay was run for pass, active grouped import, allowlisted pass, and missing-master error; all repeated outputs were byte-identical.
- **BH-004:** The wrapper applies `timeout 30s` to the `flock ... rustc` compile step and separately to scanner execution. The Moon task invokes that wrapper, and the structural test rejects an unbounded compile path.
- **BH-005:** `parse_allowlist_line` is now 9 lines and `walk_directory` is 13 lines; the previous monoliths are split into focused helper functions.

---

## Proof/Test/Source Parity Matrix

| Requirement | Proof/RRO Claim | Source Binding | Executable Evidence | Reviewer Disposition |
|---|---|---|---|---|
| RQ-001 pass iff no active `serde_json` residue | PO/RRO-RQ-001 verified | `ResidueQuarantine::run`, `ResidueQuarantine::decide`, `classify_line`, wrapper compile/run path | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` PASS via full self-test | PASS |
| RQ-002 closed forbidden-set/master parity | PO/RRO-RQ-002 verified | `ForbiddenImport::from_name`, `expected_master_trigger`, `master_line_matches`, master §43 lines 2056-2060 | `test_static_evidence_binds_master_rejection_triggers` PASS; stale planned command quarantined as non-closing audit | PASS |
| RQ-003 unbounded residue exit behavior | PO/RRO-RQ-003 verified | `ForbiddenImport::matches_line`, `matches_tokio_mpsc_unbounded`, `grouped_tokio_mpsc_import_contains_unbounded`, `GateError::exit_code` | `test_quarantine_gate_blocks_unbounded_channel` PASS, including direct, grouped import, and spaced-path forms | PASS |
| RQ-004 allowlist and Moon wiring | PO/RRO-RQ-004 verified | `AllowlistRef::load`, `ResidueQuarantine::diff_against_allowlist`, `.moon/tasks/all.yml::forbid-runtime-fmt`, `.moon/tasks/all.yml::check` | `test_moon_ci_quarantine_dependency_correctly_ordered` PASS; `moon run :forbid-runtime-fmt` PASS | PASS |
| RQ-005 deterministic output and formatter parity | PO/RRO-RQ-005 verified | `ResidueMatch::active_line`, `ResidueMatch::allowlisted_line`, `ScanReport::summary_line`, `emit_pass`, `emit_fail`, wrapper `sort -u` | `test_static_evidence_binds_real_formatter_symbols` PASS plus reviewer replay: pass/active/allowlisted/error outputs identical across repeated runs | PASS |

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---:|---|
| Block runtime JSON/YAML/HTTP/HashMap/unbounded residue in hot crates | PASS | Seven `ForbiddenImportName` variants in `scripts/forbid-runtime-fmt.rs:19-67`; policy match in `scripts/forbid-runtime-fmt.rs:120-126`. |
| Grouped and spaced unbounded-channel syntax | PASS | Fixtures `negative_unbounded_grouped_import.rs` and `negative_unbounded_spaced_path.rs`; self-test PASS. |
| Allowlist precedence | PASS | `AllowlistRef::lookup` and `diff_against_allowlist` move matched keys out of active residue; self-test PASS. |
| Exact diagnostics and exit codes | PASS | `GateError::exit_code`, formatter methods, and replay probes confirm channel split and deterministic lines. |
| Moon CI wiring and bounded runtime | PASS | `.moon/tasks/all.yml:105-120`; wrapper `timeout 30s` around compile and run. |
| Global CI residue classification | PASS_WITH_EXTERNAL_BLOCKER | `timeout 120s moon run :check` fails only in `check-removed-crate-residue` on `vb_codegen`, after local residue gate passes. |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|---|---:|---:|---:|
| `parse_allowlist_line` | 9 | 25 | PASS |
| `allowlist_line_parts` | 22 | 25 | PASS |
| `parse_allowlist_key` | 25 | 25 | PASS |
| `walk_directory` | 13 | 25 | PASS |
| `directory_child` | 25 | 25 | PASS |
| `collect_directory_child` | 13 | 25 | PASS |

The scanner remains boring functional core with the bash wrapper as the imperative shell. The policy is static, explicit, and small enough to review.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---:|
| Zero `unsafe` in new Rust | PASS |
| Zero `.unwrap()` / `.expect()` | PASS |
| Zero `panic!` / `todo!` / `dbg!` | PASS |
| Checked counters and line-number conversion | PASS |
| Illegal states represented by enums | PASS |
| Bounded production wall clock | PASS |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---:|
| No Option-based state machine | PASS |
| Closed-set policy modeled as enums | PASS |
| No clever abstraction hiding IO | PASS |
| Contract language matches implementation | PASS |

---

## PHASE 5: The Bitter Truth

This is now a narrow, explicit CI quarantine gate instead of an assurance story padded around substring fixtures. It is still a line scanner, not a Rust parser, but the bead contract is a residue quarantine over a closed pattern set, and the repaired tests now cover the valid Rust syntax that previously bypassed the gate.

---

## Quality Gates / Raw Evidence

| Gate | Result | Evidence |
|---|---:|---|
| `bash scripts/test-forbid-runtime-fmt.sh` | PASS | self-test PASSED; grouped/spaced forms, RQ-002, RQ-005, compile bound all exercised. |
| `bash scripts/forbid-runtime-fmt.sh` | PASS | `summary: active=0 allowlisted=0 files_scanned=828 hot_paths=291 cold_paths=537`. |
| `moon run :forbid-runtime-fmt` | PASS | Moon task completes with active=0. |
| `rustup run nightly-2026-04-28 rustfmt --edition 2024 --check scripts/forbid-runtime-fmt.rs` | PASS | no output. |
| `rustup run nightly-2026-04-28 rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o target/gate-tools/forbid-runtime-fmt-rereview` | PASS | no output. |
| Separate stdout/stderr deterministic replay | PASS | pass, active grouped import, allowlisted pass, and missing-master error all repeated byte-identically. |
| `timeout 120s moon run :check` | FAIL_GLOBAL | unrelated `check-removed-crate-residue` active `vb_codegen` residue at `crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract/discovery.rs:223`. |

---

## Verdict

**STATUS: APPROVED**

### Summary

Local residue quarantine scope is approved. All State 13 black-hat findings are closed by source changes plus executable evidence, and the only remaining red gate is a documented global residue outside this bead.

### Residual Risks

1. The scanner remains a conservative line scanner rather than a Rust parser; future syntax forms beyond the contracted closed set may need new fixtures.
2. The broader `moon run :check` remains blocked by pre-existing `vb_codegen` removed-crate residue outside tier-a-0-002.
3. Master-line drift is fail-closed via exact line checks; legitimate master edits require updating the scanner refs and evidence together.
