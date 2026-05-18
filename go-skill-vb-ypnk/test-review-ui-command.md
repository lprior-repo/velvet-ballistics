# Test Plan Review: LETHAL-6 — `ui` Command Missing

## VERDICT: REJECTED

---

## Preamble

**Mode 1 — Plan Inquisition** (no implementation exists)

The `ui` command described in this plan **does not exist** in `crates/vb_cli`. The `Command` enum has no `Ui` variant. `args.rs` has no `parse_ui` function. `app_impl.rs` has no `cmd_ui` function. `CliExitCode` has no `InvalidWorkspace` or `MissingWorkspaceFlag` variants. `ParseError` has no `UnknownUiFlag` or `InvalidUiArgument` variants.

This review is conducted against the existing codebase as reality. The plan assumes a contract that has not been defined.

---

## Axis 1 — Contract Parity

**[FAIL] LETHAL**

### Finding 1: `CliError::InvalidWorkspace` does not exist

The plan references `CliError::InvalidWorkspace` in:
- Behavior 3 ("returns `Err(CliError::InvalidWorkspace)`")
- Behavior 5 ("rejected with `Err(CliError::InvalidWorkspace)`")
- BDD Scenario 3
- Error Variant Map
- Combinatorial Coverage Matrix (lines 206, 208, 209)
- Fuzz Target (line 171)
- Mutation Checkpoints (line 189)

`CliError` does not appear anywhere in `vb_cli/src/`. The only error enum in scope is `CliExitCode` (`exit_code.rs`), which has variants `Success`, `ValidationFailed`, `VerificationFailed`, `CompileFailed`, `RuntimeFailed`, `StorageError`, `IpcError`, `ActionPolicyError`, `ReplayDivergence`. **No `InvalidWorkspace` variant exists.**

**Impact**: Every test that asserts `Err(CliError::InvalidWorkspace)` is unimplementable until `CliError` is defined with that variant.

### Finding 2: `CliError::MissingWorkspaceFlag` does not exist

Referenced in:
- Behavior 4
- BDD Scenario 4
- Error Variant Map

**Impact**: Same as Finding 1. `CliError::MissingWorkspaceFlag` does not exist in the codebase.

### Finding 3: `ParseError::UnknownUiFlag` does not exist

Referenced in:
- BDD Scenario 5 (line 130): `Err(ParseError::UnknownUiFlag("--bogus-flag"))`
- Error Variant Map

`ParseError` (`args/error.rs`) has no `UnknownUiFlag` variant. Existing variants are: `MissingArgument`, `UnknownEmitTarget`, `UnknownDurability`, `UnknownProfile`, `UnknownCommand`, `InvalidStatusArgument`, `UnknownActionCommand`, `UnknownActionRegistry`, `MissingActionRegistryValue`, `UnknownActionListFlag`, `UnexpectedActionListArgument`, `UnknownActionInspectFlag`, `UnexpectedActionInspectArgument`, `InvalidActionId`, `NoCommand`, `InvalidStep`, `ReasonTooLong`.

**Impact**: The test for unknown UI flags cannot be written until `ParseError::UnknownUiFlag` is added.

### Finding 4: `ParseError::InvalidUiArgument` does not exist

Referenced in:
- BDD Scenario 5 (line 130): `Err(ParseError::InvalidUiArgument(..))`
- Error Variant Map

No such variant in `ParseError`.

### Finding 5: `parse_ui` function does not exist

The plan assumes a `parse_ui(args: &[OsString]) -> Result<Command, ParseError>` function. No such function exists in `args.rs`. The `parse_args` function (line 259 of `args.rs`) dispatches via `match subcommand`, and `"ui"` is not a case — it falls through to `Err(ParseError::UnknownCommand("ui".into()))`.

**Impact**: The entire unit test layer for parsing assumes `parse_ui` exists as a separate function. This is inconsistent with the existing pattern in `args.rs` where each command's parse function is called directly from `parse_args`.

### Finding 6: `cmd_ui` function does not exist

The plan assumes a `cmd_ui` function in `app_impl.rs`. No such function exists. The `run_from_env` function (line 89 of `app_impl.rs`) dispatches on `Command` enum variants, and `Command::Ui` does not exist.

---

## Axis 2 — Assertion Sharpness

**[FAIL] LETHAL**

### Finding 7: Line 89 uses partial match `Ok(Command::Ui { .. })`

> **Then**: `parse_args` returns `Ok(Command::Ui { .. })` (parsing succeeds)

This is a vague assertion. It does not verify the workspace field contains the expected path. A test passing this assertion could silently drop the workspace value or set it to `PathBuf::new()`. The plan itself states (line 59):
> **And** the variant returned must be exactly `Command::Ui { workspace: expected_path }` — not a surrounding wrapper, not a partial match.

The scenario on line 89 directly contradicts this requirement.

### Finding 8: Signal-induced exit code is platform-dependent

> When the UI is killed by signal, `velvet-ballastics` exits with code 1 (signal-induced failure)

On Unix, a process killed by signal exits with 128 + signal number (e.g., SIGTERM=15 → exit 143). On Windows, the exit code behavior differs. The plan does not acknowledge this platform dependency.

---

## Axis 3 — Trophy Allocation

**[FAIL] LETHAL**

### Finding 9: Trophy ratio is undefined (no `pub fn` count to compare against)

The plan says "Trophy allocation: 1 unit / 1 integration / 1 e2e" but:
- The plan describes **5 distinct behaviors** but only plans **1 unit test** (`parse_ui_command_accepts_valid_workspace_flag`)
- For a CLI parser with flag parsing, the minimum expected unit tests is proportional to the number of parsing paths (happy path, missing flag, empty value, unknown flag, non-UTF-8, traversal, etc.)
- The existing `args.rs` has **~300 lines of unit tests** for a single command (`cancel`), and the unit test count for cancel covers far fewer variants than what the `ui` plan describes

**The 1 unit / 1 integration / 1 e2e allocation is inadequate** for a CLI with this many error variants.

### Finding 10: Path normalization is a pure function that needs proptest

The plan explicitly identifies (Section 4) that if workspace path normalization is extracted into a pure function (`fn normalize_workspace_path(path: &Path) -> Result<CanonicalPath, CliError>`), it would need proptest. But the plan dismisses this with "not applicable for this bead" because no pure function exists **yet**. The plan's own Section 3 (Behavior 5) describes path normalization/validation logic as part of `cmd_ui`. If this logic is extracted for testability (as it should be), it becomes a pure function requiring proptest.

**This is a deferred obligation with no concrete trigger condition.** The plan should specify the extraction point and proptest requirement as a precondition.

---

## Axis 4 — Boundary Completeness

**[PARTIAL PASS] MINOR (per missing boundary)**

### Finding 11: Platform-specific permission-denied handling acknowledged but not resolved

> Path without read permission: `CliExitCode::ValidationFailed` or `ExitCode::SUCCESS` (depends on platform)

The plan acknowledges platform divergence but doesn't specify which platform is authoritative or how platform differences are handled in assertions.

### Finding 12: Signal exit code not specified for non-Unix platforms

> When the UI is killed by signal, `velvet-ballastics` exits with code 1

This is Unix-specific (exit 1). On Windows, signal handling differs. The test would pass on Linux but fail on Windows.

---

## Axis 5 — Mutation Survivability

**[PARTIAL PASS] MAJOR**

### Finding 13: Tests for non-existent `parse_ui` and `cmd_ui` cannot catch mutations

The Mutation Checkpoints table (Section 7) lists:
- Removing `--workspace` flag check → caught by `parse_ui_command_accepts_valid_workspace_flag`
- Swapping `MissingArgument("--workspace")` to `MissingArgument("--ws")` → caught by `ui_command_returns_missing_workspace_flag_error_when_flag_is_absent`

But since `parse_ui` and `cmd_ui` don't exist, these tests cannot be written, and the mutations they catch cannot be verified. The checkpoint table is aspirational, not executable.

---

## Axis 6 — Evidence Plan Audit

**[PARTIAL PASS] MINOR**

### Finding 14: "Valid existing directory" test has insufficient preconditions

> Given: A clean environment and a valid workspace path

What is a "valid workspace path"? The plan has an Open Question (line 250) asking exactly this. Without knowing the required workspace markers (`.velvet/workspace`, `Cargo.toml`, etc.), the integration test cannot be implemented deterministically.

### Finding 15: UI binary name is unspecified

The E2E tests assume a UI binary can be spawned, but Open Question 2 asks "What is the UI binary/command that gets spawned?" Without this, the E2E tests cannot be implemented.

---

## Open Questions (from the plan itself — unresolved)

The plan itself identifies 6 open questions that block implementation. Any of these being unanswered blocks the test suite from being written:

1. **What is the exact `CliError` enum definition?** (BLOCKING)
2. **What is the UI binary/command that gets spawned?** (BLOCKING for E2E)
3. **What are the required workspace markers?** (BLOCKING for integration)
4. Does the `ui` command support additional flags beyond `--workspace`?
5. Where does `parse_ui` belong?
6. Is the `ui` command testable in CI?

---

## Summary of LETHAL Findings

| # | Finding | Location | Severity |
|---|---|---|---|
| 1 | `CliError::InvalidWorkspace` does not exist | Plan: lines 20, 27, 89, 95-102, 206, 208, 209, 237 | LETHAL |
| 2 | `CliError::MissingWorkspaceFlag` does not exist | Plan: lines 24, 115, 238 | LETHAL |
| 3 | `ParseError::UnknownUiFlag` does not exist | Plan: lines 130, 239 | LETHAL |
| 4 | `ParseError::InvalidUiArgument` does not exist | Plan: lines 130, 240 | LETHAL |
| 5 | `parse_ui` function does not exist | Plan: sections 3, 7 | LETHAL |
| 6 | `cmd_ui` function does not exist | Plan: sections 3, 7 | LETHAL |
| 7 | Partial match `Ok(Command::Ui { .. })` on line 89 contradicts exact assertion requirement | Plan: line 59 vs line 89 | LETHAL |
| 9 | 1 unit / 1 integration / 1 e2e is inadequate for 5 behaviors with multiple error variants | Plan: lines 31-41 | LETHAL |

---

## MAJOR Findings (must fix before resubmission)

1. **Signal exit code is Unix-specific** (line 70) — will fail on Windows
2. **Platform divergence on permission-denied paths** (line 218) — assertion is ambiguous
3. **Proptest obligation deferred without trigger condition** (line 141) — if path normalization is extracted, when does the proptest get written?
4. **Mutation checkpoints are aspirational** — `parse_ui` and `cmd_ui` don't exist, so no test can catch the listed mutations

---

## MINOR Findings (< 5, listed for completeness)

None below the 5-threshold.

---

## MANDATE

The following must exist before resubmission:

1. **`CliError` enum** with at minimum `InvalidWorkspace` and `MissingWorkspaceFlag` variants, defined in `vb_cli/src/cli_error.rs` or similar, with `From<ParseError>` mapping
2. **`ParseError::UnknownUiFlag`** variant added to `args/error.rs`
3. **`ParseError::InvalidUiArgument`** variant added to `args/error.rs`
4. **`Command::Ui { workspace: PathBuf }`** variant added to the `Command` enum in `args.rs`
5. **`parse_ui`** function implemented in `args.rs` following the existing pattern of `parse_<command>` functions
6. **`cmd_ui`** function implemented in `app_impl.rs` following the existing `cmd_*` pattern
7. **`run_from_env`** updated to dispatch `Command::Ui` to `cmd_ui`
8. Answer to Open Question 3 (required workspace markers) — needed for integration tests
9. Answer to Open Question 2 (UI binary name) — needed for E2E tests
10. **Unit test count must increase**: minimum 6 unit tests (happy path, missing flag, empty value, unknown flag, non-UTF-8, traversal) before this plan can be approved
11. **Exact assertions** on line 89: replace `Ok(Command::Ui { .. })` with `Ok(Command::Ui { workspace })` with the expected path
12. **Platform-agnostic signal handling** specification or `#[cfg(unix)]` gate on the signal test

Resubmit for full re-review from Axis 1 after all 12 items are resolved.
