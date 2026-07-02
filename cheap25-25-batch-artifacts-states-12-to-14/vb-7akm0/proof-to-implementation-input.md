# Proof-to-Implementation Bridge Input: vb-7akm0

## Purpose

This document provides the input needed by `proof-to-implementation` (State 7) to map the
6 planned proof obligations to Rust source refs, the exact implementation changes
(visibility narrowings), the independent verification gates, and the dependency order.
This is a **Rust visibility refactor bead** — the "implementation" targets are existing
production source files where `#[allow(unreachable_pub)]` is removed and visibility
metadata is changed. No new production code is introduced; no production symbol changes
its semantics.

## Bead Identifier

- **bead_id:** vb-7akm0
- **title:** Lint: remove `#[allow(unreachable_pub)]` suppressions by narrowing visibility (P1 bug)
- **state:** State 4 (proof-planner) → State 4b (proof-plan-reviewer) → State 5 (holzman-rust)
- **behavior_affecting:** false (all 30 proof seeds; verified via delivery-scope.jsonl)

## Pre-Condition: Decision-Ack Artifact

**MUST EXIST before ApplyTreatment for category G:**
- **Artifact:** `.beads/vb-7akm0/decision-ack.md`
- **Format:** Plain markdown with exactly one line of the form `Decision: RetireOrphanTest` or `Decision: RegisterOrphanTest`, plus a rationale block.
- **Default recommendation:** RetireOrphanTest (per codebase-map.md Open Questions recommendation 1 and contract.md §2.7 LS-ORPHAN.1 default).
- **Owner:** Implementation owner (State 5). Proof-writer does not create it.
- **Pre-condition gate:** PO-DECISION-001.

## Proof Obligation → Implementation Mapping

### PO-LINT-001 (verifier: `moon-lint-src`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-LINT-001 | All 25 files (categories A, B, C, D, E, F, G.touch, G.1, G.2) | `xtask/src/main.rs:2`, `crates/vb_validate/src/diag/diag_tests.rs:6`, `crates/vb_validate/src/schema_support/schema_tests.rs:4`, `crates/vb_validate/src/fact_table.rs:4`, `crates/vb_validate/src/gate_07_stack.rs:4`, `crates/vb_validate/src/gate_08_accessor.rs:4`, `crates/vb_validate/src/gate_09_slots.rs:4`, `crates/vb_validate/src/gate_10_node.rs:4`, `crates/vb_validate/src/gate_11_loop.rs:4`, `crates/vb_validate/src/gate_12_14_15.rs:4`, `crates/vb_validate/src/gate_13_cycles.rs:4`, `crates/vb_validate/src/taint_prop.rs:15`, `crates/vb_validate/src/type_check.rs:15`, `crates/vb_validate/src/secret_leak.rs:14`, `crates/vb_validate/src/type_sigs.rs:4`, `crates/vb_validate/src/schema_support/schema_doc.rs:4`, `crates/vb_validate/src/schema_support/schema_id.rs:4`, `crates/vb_validate/src/schema_support/schema_fields.rs:4`, `crates/vb_validate/src/diag/diag_codes.rs:4`, `crates/vb_validate/src/diag/diag_convert.rs:6`, `crates/vb_validate/src/diag/diag_render.rs:4`, `crates/vb_validate/src/diagnostic.rs:7`, `crates/vb_cli/src/commands_diff.rs:2`, `crates/vb_cli/src/commands_incident.rs:2`, `crates/vb_cli/src/lifecycle.rs:471` | Attribute deletion + visibility metadata change |

**Evidence command:** `moon run :lint-src 2>&1 | tee .evidence/lint-src/run-001/exit-code.txt`
**Post-condition:** exit 0; zero surviving `#[allow(unreachable_pub)]`; zero `unreachable_pub` warnings.

### PO-COMPILE-001 (verifier: `cargo-check`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-COMPILE-001 | All pub fn → fn and pub → pub(crate) narrowings | Same 18 source files as PO-LINT-001 minus xtask/main.rs and 3 file-only attribute deletions | Visibility metadata change (no semantic change) |

**Evidence command:** `cargo check --workspace --all-features 2>&1 | tee .evidence/cargo-check/run-001/exit-code.txt`
**Post-condition:** exit 0; `cargo check` reports `Finished `dev` profile`.

### PO-TEST-001 (verifier: `cargo-test`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-TEST-001 | All 18 files in B/C/D/E.2/E.3/F/G.touch that have test consumers | `crates/vb_validate/src/{gate_tests.rs, type_taint_tests.rs, secret_leak/tests.rs, schema_support/schema_tests.rs, schema_support/schema_fields/*.rs, diag/diag_tests.rs, diag/diag_render/render_tests.rs}`, `crates/vb_cli/tests/lifecycle_integration.rs`, `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs:29` | Sibling-module direct-path imports must continue to compile after `pub fn` → `fn` and `pub` → `pub(crate)` |

**Evidence command:** `cargo test --workspace 2>&1 | tee .evidence/cargo-test/run-001/exit-code.txt`
**Post-condition:** exit 0; same test count as pre-change baseline (recorded in `baseline-report.md`).

### PO-EXTERN-001 (verifier: `grep-externality` + `check-verus-production-binding` + `check-production-inner-drift`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-EXTERN-001 (grep) | `vb_validate::diag::diag_codes::CODE_*`, `vb_validate::diagnostic::*`, `vb_cli::lifecycle::test_helpers::create_run_header` | All crates except `.git` and `.evidence` | Pre-ApplyTreatment grep |
| PO-EXTERN-001 (check-verus) | Verus production-binding health | `scripts/check-verus-production-binding.sh` | Pre-existing gate |
| PO-EXTERN-001 (check-drift) | Verus production_inner drift | `scripts/check-production-inner-drift.sh` | Pre-existing gate |

**Evidence commands:** see `proof-obligations.planned.jsonl` PO-EXTERN-001 row.
**Post-condition:** all four grep evidence files captured; both Verus gates exit 0.

### PO-DECISION-001 (verifier: `decision-ack`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-DECISION-001 | `.beads/vb-7akm0/decision-ack.md` | New file | Decision record artifact |

**Pre-condition:** MUST be created before ApplyTreatment for `commands_diff.rs` and `commands_incident.rs`.

### PO-DECISION-GREP-001 (verifier: `grep`)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-DECISION-GREP-001 | `verification/verus/production_inner/` | Verus mirror directory | Pre-ApplyTreatment grep |

**Post-condition:** empty output → production_inner mirror independent of local IncidentReport.

## Dependency Order (apply in this order to minimize bisect noise)

| Order | Category | Files | Rationale |
|-------|----------|-------|-----------|
| 1 | A (vestigial) | 4 files | Zero pub items; trivial deletion. First because no test code touches these files, so no risk of breaking tests in subsequent commits. |
| 2 | B (gate internal) | 7 files | `pub fn` → `fn` on duplicates. Test code (gate_tests.rs, gate_XX/tests.rs) is the only consumer; if tests fail, the change is local and bisectable. |
| 3 | C (taint/type/secret-leak) | 3 files | Same as B. Test code in-file; if tests fail, change is local. |
| 4 | D (schema support) | 4 files | `pub` → `pub(crate)`. Test code (schema_tests.rs, schema_fields/*.rs) is cross-module; if compile fails, the Rust 2021+ visibility rule was misapplied. |
| 5 | E.2 (diag_convert) + E.3 (diag_render) + F (diagnostic.rs) | 3 files | Externally-reachable items confirmed via PO-EXTERN-001 grep. |
| 6 | E.1 (diag_codes) | 1 file | Decision (option a or b); single commit. |
| 7 | G.touch (lifecycle.rs) | 1 file | Externally-reachable confirmed via PO-EXTERN-001 grep. |
| 8 | G.1 (commands_diff) + G.2 (commands_incident) | 2 files | **REQUIRES decision-ack.md.** After decision recorded, retire or register orphan test, then narrow items. |

Run `cargo test --workspace` after each category's commit to localize failures.
Run `moon run :lint-src` after each category to confirm the lint policy fires correctly.
Run `bash scripts/check-verus-production-binding.sh` and `bash scripts/check-production-inner-drift.sh` after category G to defend H7.

## Forbidden Actions (per bead spec)

1. **MUST NOT remove `#[allow(unreachable_pub)]` from items that are externally reachable** (decisions G). PO-EXTERN-001 is the structural guard.
2. **MUST retire or wire orphan test `vb_test_cli_diff_incident_behavior.rs` BEFORE applying category G changes.** PO-DECISION-001 is the structural guard.
3. **MUST NOT modify** `verification/verus/extern_*.rs`, `verification/verus/production_inner/*.rs`, `kani/`, `xtask/src/main.rs` (except the inner-attribute deletion in step 1), or the moon `lint-src` task. These are trusted infrastructure (TBP-001..TBP-012).
4. **MUST NOT introduce** new `#[allow(unreachable_pub)]` overrides as a workaround. If the lint fires post-change, fix the visibility, don't suppress the lint.
5. **MUST NOT change** production-code semantics. Only visibility metadata or attribute lines change. Test count and test pass/fail outcomes MUST match the pre-change baseline.

## Open Decisions for Implementation Owner

| ID | Decision | Default | Impact |
|----|----------|---------|--------|
| OQ-001 | Retire or register orphan test `vb_test_cli_diff_incident_behavior.rs`? | Retire (deletes 646 lines; cleans up `source-length-exceptions.txt:221` vb-jpq7.47 split-or-retire-before-release) | Affects categories G.1 and G.2 |
| OQ-002 | For `diag_codes.rs`: option (a) DeleteAllow or option (b) PubToPubCrate? | (a) DeleteAllow (preserves external API stability; if any unforeseen consumer exists, the lint would have fired before the allow-removal was applied, so it's safe) | Affects category E.1 |
| OQ-003 | One commit per category or one commit per file? | One commit per category (8 commits total) — easier to bisect if a test regresses | Affects git history granularity |

## Evidence Capture Schema

For each of the 6 obligations, the implementation owner MUST create the evidence directory
and capture the raw exit code + raw log. Schema:

```
.evidence/
├── lint-src/
│   └── run-001/
│       ├── exit-code.txt      # raw $? from moon run :lint-src
│       └── clippy-output.log  # raw stdout/stderr
├── cargo-check/
│   └── run-001/
│       ├── exit-code.txt
│       └── cargo-output.log
├── cargo-test/
│   └── run-001/
│       ├── exit-code.txt
│       ├── cargo-test-output.log
│       └── test-count.txt     # number of tests run (must equal pre-change baseline)
├── grep-externality/
│   └── run-001/
│       ├── diag-codes-CODE_.txt
│       ├── diagnostic-render.txt
│       ├── diagnostic-reexport.txt
│       └── lifecycle-create-run-header.txt
├── production-binding/
│   └── run-001/
│       ├── check-verus-prod-binding.txt
│       ├── check-verus-prod-binding-exit.txt
│       ├── check-prod-inner-drift.txt
│       └── check-prod-inner-drift-exit.txt
├── decision-ack/
│   └── run-001/
│       ├── decision-exit.txt
│       └── decision-ack-content-hash.txt
└── grep-precondition/
    └── run-001/
        ├── incident-report-production-inner.txt
        └── incident-report-precondition-exit.txt
```

## Bridge Handoff to State 7

This file is the input to `proof-to-implementation` (State 7). The next state consumes:
- The 6 obligations from `proof-obligations.planned.jsonl`
- The mapping table above
- The dependency order (apply in 8 commits)
- The forbidden actions list
- The open decisions list (OQ-001..OQ-003)
- The evidence capture schema

After State 7, `proof-to-rust-review` validates the implementation against this bridge.
Then State 8 (test-planner), State 9 (test-writer), State 10 (formal-verifier), State 11
(black-hat-reviewer), State 12 (evidence-packaging).