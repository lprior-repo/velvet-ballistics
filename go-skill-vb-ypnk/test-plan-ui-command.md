# Test Plan: LETHAL-6 — `ui` Command Missing

## Summary
- Behaviors identified: 3
- Trophy allocation: 1 unit / 1 integration / 1 e2e
- Proptest invariants: 0
- Fuzz targets: 1 (workspace path string parsing)
- Kani harnesses: 0

---

## 1. Behavior Inventory

1. **UI command parses and accepts a valid workspace path**
   — `vb ui --workspace /path/to/valid/workspace` produces a `Command::Ui { workspace }` variant and is dispatched to `cmd_ui`

2. **UI command launches the interactive UI and blocks until the UI process exits**
   — When `cmd_ui` is called with a valid workspace path, it spawns the interactive UI process and `run_from_env` blocks synchronously on that handle; exit code is propagated to the process

3. **UI command returns `Err(CliError::InvalidWorkspace)` when the workspace path does not exist or is not a valid Velvet workspace directory**
   — `parse_args` or `cmd_ui` validates the workspace path and returns `Err(CliError::InvalidWorkspace)` for non-existent paths, permission-denied paths, and paths that exist but lack required workspace markers

4. **UI command returns `Err(CliError::MissingWorkspaceFlag)` when the `--workspace` flag is omitted entirely**
   — `parse_args` for the `ui` subcommand returns `Err(ParseError::MissingArgument("--workspace"))` which maps to `Err(CliError::MissingWorkspaceFlag)` in the dispatch layer

5. **UI command validates workspace path format before attempting to launch**
   — Empty string path, relative path traversal attempts (`../`), and non-UTF-8 paths are all rejected with `Err(CliError::InvalidWorkspace)`

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|---|---|---|
| `ui` command accepted by `parse_args` with `--workspace` flag | Unit | Pure parsing — all flag parsing lives in `args.rs` unit tests |
| `ui` command rejected by `parse_args` when `--workspace` is absent | Unit | Parse error mapping is deterministic |
| `ui` command validates workspace directory and returns `InvalidWorkspace` for invalid paths | Integration | Requires filesystem state — real `PathBuf`, not mocks |
| `ui` command spawns interactive UI process and blocks until exit | E2E | Full process spawn + blocking semantics — black-box |
| `ui` command propagates UI process exit code correctly | E2E | Full process lifecycle — black-box |

**Ratio justification**: ~40% unit (parse path validation), ~40% integration (filesystem validation), ~20% e2e (process spawn/block). This skews toward e2e because the primary contract is blocking UI process management, which cannot be unit tested without spawning real processes.

---

## 3. BDD Scenarios

### Behavior: `ui` command is recognized and parsed with `--workspace` flag

**Given**: A clean environment with no prior CLI state
**When**: The user runs `velvet-ballastics ui --workspace /valid/workspace/path`
**Then**: `parse_args` returns `Ok(Command::Ui { workspace: PathBuf::from("/valid/workspace/path") })`
**And**: `run_from_env` dispatches to `cmd_ui`

```rust,ignore
fn parse_ui_command_accepts_valid_workspace_flag()
// Given-When-Then: above
```

**And** the variant returned must be exactly `Command::Ui { workspace: expected_path }` — not a surrounding wrapper, not a partial match.

---

### Behavior: `ui` command blocks until UI process exits and returns its exit code

**Given**: A clean environment and a valid workspace path
**When**: The user runs `velvet-ballastics ui --workspace /valid/workspace/path`
**Then**: The CLI process blocks synchronously while the interactive UI is running
**And**: When the UI exits with code 0, `velvet-ballastics` exits with code 0
**And**: When the UI exits with code 42, `velvet-ballastics` exits with code 42
**And**: When the UI is killed by signal, `velvet-ballastics` exits with code 1 (signal-induced failure)

```rust,ignore
fn ui_command_blocks_until_ui_exits_with_success()
fn ui_command_propagates_ui_nonzero_exit_code()
fn ui_command_exits_with_failure_when_ui_is_killed_by_signal()
```

**Error variant — UI fails to start**:
**Given**: The workspace path is valid but the UI binary cannot be launched (e.g., missing shared library)
**When**: `cmd_ui` attempts to spawn the UI process
**Then**: `velvet-ballastics` exits with `CliExitCode::ValidationFailed` and an error message describing the launch failure

---

### Behavior: `ui` command returns `Err(CliError::InvalidWorkspace)` for an invalid workspace path

**Given**: A clean environment
**When**: The user runs `velvet-ballastics ui --workspace /nonexistent/path`
**Then**: `parse_args` returns `Ok(Command::Ui { .. })` (parsing succeeds)
**And**: `cmd_ui` returns `CliExitCode::ValidationFailed`
**And**: stderr contains a message indicating the workspace is invalid

**Error variants**:

| Input | Expected Error |
|---|---|
| Non-existent absolute path | `CliError::InvalidWorkspace` |
| Path is a file, not a directory | `CliError::InvalidWorkspace` |
| Path with permission denied | `CliError::InvalidWorkspace` |
| Empty string path (`--workspace ""`) | `CliError::InvalidWorkspace` |
| Relative path traversal (`--workspace ../escape`) | `CliError::InvalidWorkspace` |
| Non-UTF-8 bytes in path | `CliError::InvalidWorkspace` |

```rust,ignore
fn ui_command_returns_invalid_workspace_error_when_path_does_not_exist()
fn ui_command_returns_invalid_workspace_error_when_path_is_a_file()
fn ui_command_returns_invalid_workspace_error_when_path_permission_denied()
fn ui_command_returns_invalid_workspace_error_when_path_is_empty_string()
fn ui_command_returns_invalid_workspace_error_when_path_traverses_parent()
fn ui_command_returns_invalid_workspace_error_when_path_contains_non_utf8()
```

---

### Behavior: `ui` command returns `Err(CliError::MissingWorkspaceFlag)` when `--workspace` is absent

**Given**: A clean environment
**When**: The user runs `velvet-ballastics ui`
**Then**: `parse_args` returns `Err(ParseError::MissingArgument("--workspace"))`
**And**: `run_from_env` prints a user-visible error to stderr containing "missing argument: --workspace"
**And**: The process exit code is `CliExitCode::ValidationFailed` (value 1)

```rust,ignore
fn ui_command_returns_missing_workspace_flag_error_when_flag_is_absent()
```

**Error variant — unknown flag alongside `ui`**:
**Given**: A clean environment
**When**: The user runs `velvet-ballastics ui --bogus-flag`
**Then**: `parse_args` returns `Err(ParseError::UnknownUiFlag("--bogus-flag"))` or `Err(ParseError::InvalidUiArgument(..))`
**And**: Process exit code is `CliExitCode::ValidationFailed`

```rust,ignore
fn ui_command_returns_error_for_unknown_ui_flag()
```

---

## 4. Proptest Invariants

No pure functions with multiple inputs exist in the `ui` command path at this time. The `parse_args` function for `ui` is a simple `named_flag` lookup with no combinatorial explosion. Proptest invariants are not applicable for this bead.

If workspace path normalization logic is extracted into a pure function in the future (e.g., `fn normalize_workspace_path(path: &Path) -> Result<CanonicalPath, CliError>`), the following invariant applies:

```
Invariant: normalize_workspace_path returns Err(InvalidWorkspace) for all non-existent paths
Strategy: any non-existent PathBuf
Anti-invariant: any existing directory PathBuf must return Ok
```

---

## 5. Fuzz Targets

### Fuzz Target: `parse_ui_args` workspace path injection

**Input type**: Arbitrary byte sequences fed as the `--workspace` flag value
**Risk**: Panic due to `unwrap()` on invalid UTF-8, logic error where relative path traversal escapes sandbox, panic due to unchecked indexing on empty path string
**Corpus seeds**:
- Empty string: `""`
- `/nonexistent/path`
- `../parent_escape`
- Non-UTF-8 bytes: `b"/path/with/\xff/invalid/utf8"`
- Very long path (>4096 bytes)
- Path with null byte: `/valid\0/path`
- `.` (current directory)
- `..` (parent directory)
- `/tmp/velvet-test-{random}` (temp directory that exists)
- Symbolic link targets

**Mitigation in implementation**: Use `PathBuf::from` which accepts OsString, validate with `path.exists()` and `path.is_dir()`, reject empty paths, canonicalize and verify prefix.

---

## 6. Kani Harnesses

Not applicable. The `ui` command involves process spawning and blocking I/O which are outside the scope of Kani's bounded model checking. The critical invariants (workspace path validation, exit code propagation) are covered by integration and E2E tests respectively.

---

## 7. Mutation Checkpoints

Critical mutations that must be caught:

| Location | Mutation | Must be caught by test |
|---|---|---|
| `args.rs` — `parse_ui` | Removing the `--workspace` flag check entirely | `parse_ui_command_accepts_valid_workspace_flag` |
| `args.rs` — `parse_ui` | Swapping `MissingArgument("--workspace")` to `MissingArgument("--ws")` | `ui_command_returns_missing_workspace_flag_error_when_flag_is_absent` |
| `app_impl.rs` — `cmd_ui` | Changing `workspace.exists()` to `!workspace.exists()` (inverted check) | `ui_command_returns_invalid_workspace_error_when_path_does_not_exist` |
| `app_impl.rs` — `cmd_ui` | Removing the `is_dir()` check (file would pass) | `ui_command_returns_invalid_workspace_error_when_path_is_a_file` |
| `app_impl.rs` — `cmd_ui` | Replacing `CliExitCode::ValidationFailed` with `CliExitCode::Success` after invalid workspace | `ui_command_returns_invalid_workspace_error_when_path_does_not_exist` |
| `app_impl.rs` — `cmd_ui` | Changing `wait()` to `try_wait()` (non-blocking) | `ui_command_blocks_until_ui_exits_with_success` |

**Threshold**: 90% mutation kill rate minimum.

---

## 8. Combinatorial Coverage Matrix

### Unit: `parse_args` for `ui` command

| Scenario | Input | Expected Output | Test Layer |
|---|---|---|---|
| Happy path — valid workspace | `["vb", "ui", "--workspace", "/tmp/valid"]` | `Ok(Command::Ui { workspace: "/tmp/valid" })` | unit |
| Missing `--workspace` flag | `["vb", "ui"]` | `Err(ParseError::MissingArgument("--workspace"))` | unit |
| Empty workspace value | `["vb", "ui", "--workspace", ""]` | `Err(CliError::InvalidWorkspace)` (via cmd_ui) | unit |
| Unknown flag | `["vb", "ui", "--bogus"]` | `Err(ParseError::UnknownUiFlag("--bogus"))` | unit |
| `--workspace` with value starting with `--` | `["vb", "ui", "--workspace", "--also-flag"]` | `Err(CliError::InvalidWorkspace)` | unit |
| Non-UTF-8 path bytes | OS bytes for `--workspace` with invalid UTF-8 | `Err(CliError::InvalidWorkspace)` | unit |

### Integration: `cmd_ui` workspace validation

| Scenario | Input | Expected Output | Test Layer |
|---|---|---|---|
| Valid existing directory | `/tmp/velvet-valid-workspace` (pre-created) | UI spawns, process blocks | integration |
| Non-existent path | `/nonexistent/workspace` | `CliExitCode::ValidationFailed` + stderr message | integration |
| Path is a file | (temp file path) | `CliExitCode::ValidationFailed` + stderr message | integration |
| Path without read permission | (chmod 000 temp dir) | `CliExitCode::ValidationFailed` or `ExitCode::SUCCESS` (depends on platform) | integration (platform-specific) |

### E2E: Full `ui` command invocation

| Scenario | Input | Expected Output | Test Layer |
|---|---|---|---|
| UI binary exists and exits 0 | Valid workspace + real UI binary | Exit code 0 | e2e |
| UI binary exits 42 | Valid workspace + fake UI returning 42 | Exit code 42 | e2e |
| `--workspace` absent | No args | Exit code 1 + "missing argument: --workspace" on stderr | e2e |
| UI binary not found | Valid workspace + `$PATH` without UI binary | Exit code 1 + descriptive error on stderr | e2e |

---

## 9. Error Variant Map

The following error variants must be explicitly named and tested:

| Error Type | Source | Causing scenario |
|---|---|---|
| `CliError::InvalidWorkspace` | `app_impl.rs` `cmd_ui` — workspace validation | Non-existent, file, empty, permission-denied, non-UTF-8, traversal |
| `CliError::MissingWorkspaceFlag` | `args.rs` `parse_ui` — missing `--workspace` | Flag entirely absent from CLI invocation |
| `ParseError::UnknownUiFlag(String)` | `args.rs` `parse_ui` — unknown flag | Any `--flag` not in the UI command's allowlist |
| `ParseError::InvalidUiArgument(String)` | `args.rs` `parse_ui` — malformed argument | Value that cannot be parsed as a valid path |

---

## Open Questions

1. **What is the exact `CliError` enum definition?** — The task refers to `CliError::InvalidWorkspace` and `CliError::MissingWorkspaceFlag` but the existing `CliExitCode` enum in `exit_code.rs` does not contain these variants. Does `CliError` live in a new module (e.g., `vb_cli/src/cli_error.rs`) or is it a new variant added to `CliExitCode`? The test-writer needs to know the exact type to assert on.

2. **What is the UI binary/command that gets spawned?** — Is there a known UI binary name (`velvet-ui`, `vb-ui`)? Is it spawned via `std::process::Command` with a hardcoded binary name, or is the binary path derived from the workspace or environment?

3. **What are the required workspace markers?** — The behavior spec says "Invalid path returns `Err(CliError::InvalidWorkspace)`" for paths that "lack required workspace markers." What files/directories constitute a valid Velvet workspace? Is it `Cargo.toml`, a `.velvet/workspace` marker, something else?

4. **Does the `ui` command support any additional flags beyond `--workspace`?** — E.g., `--port`, `--host`, `--headless`? The current behavior spec only covers `--workspace`.

5. **Where does `parse_ui` belong?** — In `args.rs` alongside `parse_status`, `parse_verify`, etc., or in a new `args/ui.rs` module following the existing sibling pattern?

6. **Is the `ui` command testable in the current CI environment?** — E2E tests that spawn a real UI process require the UI binary to exist. Should E2E tests be gated on a `#[cfg(feature = "ui")]` or similar, or should they use a fake/stub UI binary?
