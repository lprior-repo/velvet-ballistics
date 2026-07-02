# Regression Diff: vb-te1i — Binary IPC BDD Acceptance

**Bead**: bdd: Binary IPC acceptance scenarios
**Date**: 2026-05-19

---

## Baseline vs Current State

### Compilation Baseline
- Previous build: Unknown (no baseline report in bead directory)
- Current: `cargo build --workspace` → 0 errors, 2 warnings

### Test Baseline
- Previous tests: Unknown (no baseline in bead directory)
- Current: `cargo test -p vb_ipc` → 686 passed

### Clippy Baseline
- Previous clippy: Unknown
- Current: 2 errors (dead_code in vb_cli/lifecycle.rs)

### Format Baseline
- Previous fmt: Unknown
- Current: Multiple files fail `cargo fmt --check`

---

## New Issues Introduced by This Bead

### 1. Formatting Issues in vb_te1i_binary_ipc_acceptance.rs
**File**: workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs
**Scope**: BEAD-LOCAL (this bead owns this file)

The following lines need formatting fixes:
- Line 23: Import statement formatting
- Line 162, 256, 398, 555: response_header assignment chaining
- Line 170, 323, 701: assert_eq! multi-line formatting
- Lines 480-517: IpcCommand match arm reformatting
- Line 567: response: IpcResponse assignment
- Line 634: header_array and IpcFrameHeader::decode split
- Line 679: IpcFrameHeader::new call reformatting
- Line 691: client.write_all reformatting

**Fix**: `cargo fmt -- workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs`

---

## Pre-Existing Workspace Debt (NOT introduced by this bead)

### 1. Clippy dead_code in vb_cli/lifecycle.rs
**File**: crates/vb_cli/src/lifecycle.rs
**Lines**: 47 (get_state), 66 (with_tracker)
**Scope**: Pre-existing — lifecycle.rs is NOT in vb-te1i delivery scope
**Action**: Fix or add `#[allow(dead_code)]` with justification

### 2. Formatting Issues in Non-Scoped Files
The following files have formatting issues but are NOT in the vb-te1i touched scope:
- vb_cli/src/app_impl.rs (app-level, not IPC)
- vb_cli/src/commands_ai_context.rs (AI context, not IPC)
- vb_cli/tests/lifecycle_integration.rs (CLI lifecycle tests)
- vb_compile/src/mod_compile_lowering/part_04.rs (compiler lowering)
- vb_compile/src/mod_compile_lowering/part_10.rs (compiler lowering)
- vb_compile/src/mod_compile_validation/part_04.rs (compiler validation)
- vb_runtime/src/durability_matrix.rs (runtime durability)
- vb_validate/src/schema.rs (validation schema)
- vb_validate/src/schema_fields.rs (validation schema fields)

---

## Regression Classification

| Issue | Introduced by vb-te1i? | Classification |
|-------|------------------------|---------------|
| vb_te1i_binary_ipc_acceptance.rs formatting | YES | FAIL_LOCAL |
| vb_cli/lifecycle.rs dead_code | NO | FAIL_REGRESSION (workspace-wide gate) |
| Other files formatting | NO | DEFERRED_GLOBAL |

---

## Recommendations

1. **Bead-local fix required**: Run `cargo fmt -- workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs`
2. **Workspace fix required**: Either fix vb_cli/lifecycle.rs dead_code or suppress with `#[allow(dead_code)]` with a comment explaining why it's unused
3. **DEFERRED_GLOBAL tracking**: File separate beads for workspace-wide formatting and dead_code cleanup
