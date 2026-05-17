# vb-wg64 State 3 Contract: Clean-Clone CI Repair

## Scope

- Bead: `vb-wg64`
- State: 3 contract
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`
- Baseline: clean-clone forced gate `moon ci --base HEAD --head HEAD --force` failed with exit 1.
- Known failing lanes: `velvet-ballastics:fmt`, `velvet-ballastics:lint-src`, `velvet-ballastics:check`.

## Requirements

### REQ-CI-001: Forced Clean-Clone CI Passes

The repaired workspace must pass the canonical forced CI gate:

```bash
moon ci --base HEAD --head HEAD --force
```

Acceptance: command exits 0 in a clean workspace after repair is applied.

### REQ-CI-002: Formatting Drift Removed

Workspace formatting must be rustfmt-clean.

Acceptance: `rtk cargo fmt --all -- --check` exits 0, and any formatting-only changes do not alter behavior.

### REQ-CI-003: Source Lint Failures Repaired Minimally

Known source lint failures must be repaired in the mapped files without broad suppression.

Acceptance: the relevant clippy lanes no longer fail for the mapped source issues in `xtask/src/forbidden_scan.rs`, `crates/vb_cli/src/app_impl.rs`, `crates/vb_cli/src/commands_ai_context.rs`, and `crates/vb_cli/src/mode_activation_tests.rs`.

### REQ-CI-004: Test Warning Cleanup Preserves Assertions

Unused imports and unused variables in `crates/vb_storage/tests/recovery_bdd_tests.rs` may be removed or renamed only when assertions, setup effects, and test intent are preserved.

Acceptance: `rtk cargo check -p vb_storage --tests` exits 0 without the mapped unused warnings, and no assertion is deleted unless proven unreachable or duplicate by direct evidence.

### REQ-CI-005: Missing Test-Support Module Resolved

The `vb_cli` test module resolution failure around `mode_error` must be resolved in the smallest behavior-preserving way.

Acceptance: `crate::mode_error::{CommandMode, ModeError, command_mode}` resolves for the existing tests, or the stale module declaration/import is removed only if the test contract is proven obsolete.

### REQ-CI-006: Output Helper Behavior Preserved

The `commands_ai_context::json_out` clippy repair must preserve JSON, JSONL, and text output semantics.

Acceptance: the repair is syntactic or equivalently structured, with the same success/error write behavior and no output format changes.

## Invariants

### INV-CI-001: No Production Behavior Change

No production behavior may change except lint-safe output helper restructuring, import cleanup, and module exposure required to satisfy existing tests.

### INV-CI-002: Test-Only Cleanup Preserves Proof Value

Test-only unused cleanup must preserve all meaningful assertions, setup effects, and scenario coverage.

### INV-CI-003: No Broad Allowlist Without Evidence

No broad `allow`, `expect`, or lint allowlist may be added unless the exact lint is documented as false-positive or out-of-scope with local justification.

### INV-CI-004: Checked Access and Arithmetic in Repairs

Repairs for `xtask/src/forbidden_scan.rs` must avoid unchecked indexing and unchecked arithmetic side effects when replacing the mapped clippy violations.

### INV-CI-005: Canonical Forced CI Is the Release Gate

Targeted gates are preflight only. The bead is not accepted until `moon ci --base HEAD --head HEAD --force` exits 0 in the isolated clean workspace.

### INV-CI-006: State 3 Is Non-Implementation

This state must not modify production or test code. It may only create contract/planning artifacts and update state records.

## Allowed Future Change Types

- Rustfmt-only formatting changes.
- Local import cleanup.
- Local unused variable cleanup or `_` prefixing when side effects remain intact.
- Local helper restructuring for lint compliance with identical observable output.
- Narrow module exposure/addition to satisfy already-declared test contracts.
- Narrow, locally justified lint attributes only when no code-equivalent repair is better.

## Disallowed Future Change Types

- Runtime behavior changes unrelated to CI repair.
- Deleting assertions to silence warnings.
- Disabling test files or CI lanes.
- Broad crate-level or workspace-level lint allowlists.
- Changing Moon, Cargo, or CI definitions to hide failures.
- Replacing clean-clone CI with targeted partial gates as final acceptance.

## Acceptance Contract

The repair is acceptable only when all are true:

- `moon ci --base HEAD --head HEAD --force` exits 0.
- `rtk cargo fmt --all -- --check` exits 0, or its coverage is subsumed by the forced Moon CI evidence.
- Mapped clippy/check failures from `baseline-report.md` and `codebase-map.md` are absent.
- Any new module or helper exists only to preserve existing test contracts or lint-safe output behavior.
- Diff review confirms no production behavior changes outside the allowed categories.
- `bd close vb-wg64 --force` succeeds after implementation and verification states.
- `bd dolt push` succeeds after bead closure.
