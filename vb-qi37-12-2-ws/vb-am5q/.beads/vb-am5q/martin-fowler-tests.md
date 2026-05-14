# Martin Fowler Test Plan: cli/runtime — Converged Binary Mode Activation Boundaries

## Overview

This test plan uses Given-When-Then style scenarios to specify the behavior of command mode activation boundaries in the `velvet-ballastics` converged binary.

## Happy Path Tests

### Pure Mode: validate

- `test_validate_mode_runs_without_opening_fjall_storage`
  - Given: a valid workflow YAML file exists at a known path
  - When: `velvet-ballastics validate <workflow.yaml>` is invoked WITHOUT `--db` argument
  - Then: command succeeds with exit code 0, no Fjall database is created or opened

- `test_validate_mode_succeeds_without_any_storage_path_present`
  - Given: no `--db` argument is provided and no database path exists
  - When: `velvet-ballastics validate <workflow.yaml>` is invoked
  - Then: command succeeds with exit code 0

- `test_validate_mode_exit_code_stable_without_storage`
  - Given: a valid workflow YAML file exists at a known path, no `--db` argument is provided
  - When: `velvet-ballastics validate <workflow.yaml>` is invoked
  - Then: exit code is 0 and depends ONLY on workflow validity, NOT on storage availability

### Pure Mode: verify

- `test_verify_mode_runs_without_fjall`
  - Given: a valid workflow YAML file exists at a known path
  - When: `velvet-ballastics verify <workflow.yaml>` is invoked WITHOUT `--db` argument
  - Then: command succeeds with exit code 0, no Fjall database is created or opened

- `test_verify_mode_exit_code_stable_without_storage`
  - Given: a valid workflow YAML file exists, no `--db` argument is provided
  - When: `velvet-ballastics verify <workflow.yaml>` is invoked
  - Then: exit code is 0 and depends ONLY on workflow validity and verification profile

### Pure Mode: compile

- `test_compile_mode_runs_without_fjall`
  - Given: a valid workflow YAML file exists at a known path
  - When: `velvet-ballastics compile <workflow.yaml> --emit ir --out output.vbir` is invoked
  - Then: command succeeds with exit code 0, no Fjall database is accessed

### Pure Mode: graph

- `test_graph_mode_runs_without_fjall`
  - Given: a valid workflow YAML file exists at a known path
  - When: `velvet-ballastics graph <workflow.yaml>` is invoked
  - Then: command succeeds with exit code 0, no Fjall database is accessed

### Pure Mode: simulate

- `test_simulate_mode_runs_without_fjall`
  - Given: a valid workflow YAML file exists at a known path
  - When: `velvet-ballastics simulate <workflow.yaml>` is invoked
  - Then: command succeeds with exit code 0, no Fjall database is accessed

### Storage Mode: run

- `test_run_mode_opens_storage_when_durability_set`
  - Given: a valid workflow YAML file, input binary, and valid database path exist
  - When: `velvet-ballastics run <workflow.yaml> --input-bin input.vbin --durability journaled --db /tmp/test.db` is invoked
  - Then: FjallJournal::open is called and succeeds, command completes with exit code reflecting run status

- `test_run_mode_skips_storage_when_durability_none`
  - Given: a valid workflow YAML file and input binary exist
  - When: `velvet-ballastics run <workflow.yaml> --input-bin input.vbin --durability none` is invoked WITHOUT `--db`
  - Then: FjallJournal::open is NOT called, command completes successfully

### Storage Mode: submit

- `test_submit_mode_opens_storage`
  - Given: a valid workflow YAML file, input binary, and valid database path exist
  - When: `velvet-ballastics submit <workflow.yaml> --input-bin input.vbin --db /tmp/test.db --durability journaled` is invoked
  - Then: FjallJournal::open is called, workflow source and run header are stored

### Runtime Mode: ipc-serve

- `test_ipc_serve_mode_opens_storage_and_runtime`
  - Given: a valid socket path and database path exist
  - When: `velvet-ballastics ipc-serve --socket /tmp/sock --db /tmp/test.db` is invoked
  - Then: FjallJournal::open is called, Runtime::new_with_journal is called, IPC server binds to socket

## Error Path Tests

### Storage Errors

- `test_storage_init_failure_returns_structured_error`
  - Given: an invalid or inaccessible database path is provided
  - When: any storage-dependent command (run, submit, inspect, etc.) is invoked with that path
  - Then: command fails with `CliExitCode::StorageError` and structured error message

- `test_invalid_db_path_produces_storage_error`
  - Given: a path that does not exist or is not a valid Fjall database
  - When: `velvet-ballastics inspect <run_id> --db /nonexistent/path` is invoked
  - Then: command fails with exit code != 0 and JSON error includes path and cause

- `test_storage_error_exit_code`
  - Given: an invalid database path
  - When: `velvet-ballastics events <run_id> --db /invalid/path` is invoked
  - Then: exit code is `CliExitCode::StorageError` as integer

- `test_storage_error_json_output`
  - Given: an invalid database path
  - When: `velvet-ballastics events <run_id> --db /invalid/path --json` is invoked
  - Then: JSON output contains `{"success": false, "error": "...error opening journal..."}`

### Runtime Errors

- `test_runtime_init_failure_returns_structured_error`
  - Given: runtime initialization fails (e.g., invalid shard config)
  - When: `velvet-ballastics ipc-serve --socket /tmp/sock --db /tmp/test.db` is invoked and runtime creation fails
  - Then: command fails with `CliExitCode::RuntimeFailed` and structured error message

### UI Mode Unavailable

- `test_ui_mode_unavailable_returns_structured_diagnostic_without_affecting_runtime_storage`
  - Given: UI mode is not yet implemented
  - When: `velvet-ballastics ui` is invoked
  - Then: command returns structured diagnostic indicating UI mode is not available, storage subsystem is NOT initialized

### Invalid Mode

- `test_invalid_mode_fails_before_subsystem_initialization`
  - Given: an unknown or invalid command is provided to the CLI
  - When: `velvet-ballastics <unknown-command>` is invoked
  - Then: command fails with exit code != 0 BEFORE any subsystem (storage, runtime, UI) is initialized

## Edge Case Tests

### Pure Commands Without Storage Path

- `test_validate_succeeds_without_storage_path_present`
  - Given: NO `--db` argument is provided and no database exists anywhere
  - When: `velvet-ballastics validate <workflow.yaml>` is invoked
  - Then: command succeeds with exit code 0

- `test_verify_succeeds_without_db_argument`
  - Given: `--db` argument is NOT provided
  - When: `velvet-ballastics verify <workflow.yaml>` is invoked
  - Then: command succeeds with exit code 0

### Boundary: Durability Modes

- `test_run_durability_strict_opens_storage`
  - Given: durability is set to `strict`
  - When: `velvet-ballastics run` is invoked
  - Then: FjallJournal::open is called with strict durability

- `test_run_durability_journaled_opens_storage`
  - Given: durability is set to `journaled`
  - When: `velvet-ballastics run` is invoked
  - Then: FjallJournal::open is called with journaled durability

- `test_run_durability_none_skips_storage`
  - Given: durability is set to `none`
  - When: `velvet-ballastics run` is invoked WITHOUT `--db`
  - Then: FjallJournal::open is NOT called

### Empty/Zero Inputs

- `test_validate_with_empty_workflow_fails_with_parse_error`
  - Given: an empty file or invalid YAML
  - When: `velvet-ballastics validate /empty.yaml` is invoked
  - Then: command fails with exit code != 0 and parse error message

- `test_validate_with_very_long_workflow_succeeds`
  - Given: a workflow YAML approaching the 1 MiB limit exists
  - When: `velvet-ballastics validate <large_workflow.yaml>` is invoked
  - Then: command succeeds or fails with LIMIT_EXCEEDED error

## Contract Verification Tests

### Precondition Tests

- `test_main_dispatches_without_global_side_effects`
  - Verify: `main()` function returns before any subsystem init if command is Help or Version

- `test_subsystem_init_functions_are_distinct`
  - Verify: vb_storage, vb_runtime, vb_ipc, vb_ui each have separate initialization entry points

- `test_pure_commands_have_no_storage_dependency`
  - Verify: pure command handler modules do not import vb_storage

### Postcondition Tests

- `test_validate_mode_runs_without_opening_fjall_storage` (from Happy Path)
- `test_runtime_mode_opens_required_storage_and_ipc_components` (from Happy Path)
- `test_each_command_activates_correct_subsystems`
  - Given: each command mode classification is defined
  - When: a command is invoked
  - Then: only the expected subsystems are activated

### Invariant Tests

- `test_fjall_journal_open_only_from_storage_or_runtime_mode`
  - Given: FjallJournal::open is a symbol in the binary
  - When: pure mode commands are invoked
  - Then: FjallJournal::open is NOT called

- `test_pure_handler_never_calls_fjall_open`
  - Given: cmd_validate, cmd_verify, cmd_compile, cmd_explain, cmd_graph, cmd_simulate handlers
  - When: each handler executes
  - Then: no call to vb_storage module occurs

- `test_ui_dependencies_not_linked_in_pure_path`
  - Given: vb_ui_makepad crate is in the workspace
  - When: pure commands are compiled and linked
  - Then: makepad symbols are NOT present in the pure command binary path

- `test_pure_command_exit_stable_regardless_of_inactive_subsystems`
  - Given: pure command is invoked
  - When: storage subsystem is completely absent/unavailable
  - Then: exit code is the same as when storage is available

- `test_runtime_new_only_from_runtime_mode`
  - Given: Runtime::new_with_journal is a symbol in the binary
  - When: non-runtime commands (pure, storage) are invoked
  - Then: Runtime::new_with_journal is NOT called

- `test_no_hidden_subsystem_init`
  - Given: each command handler function
  - When: the handler executes
  - Then: every subsystem initialization is explicit in the handler's call graph

## Given-When-Then Scenarios

### Scenario 1: Validate a workflow without any storage path

**Given**: a file `test.yaml` containing a valid velvet-ballastics workflow with `version: velvet-ballastics/v1`, `name: test`, `when: { manual: {} }`, and `steps: [{ id: step1, do: { action: test }}]`

**When**: the user runs `velvet-ballastics validate test.yaml` with NO `--db` argument and NO environment variables pointing to storage

**Then**:
- exit code is 0
- output contains "valid"
- NO file named `*.db` or `fjall-*` exists in the current directory
- NO socket file is created
- NO Runtime instance is created
- FjallJournal::open is NOT called

### Scenario 2: Run a workflow with journaled durability

**Given**: a compiled workflow at `test.vbir`, input binary at `input.vbin`, and a temporary directory at `/tmp/vb-test`

**When**: the user runs `velvet-ballastics run-compiled test.vbir --input-bin input.vbin --durability journaled --db /tmp/vb-test`

**Then**:
- FjallJournal::open is called with path `/tmp/vb-test`
- Runtime instance is created
- run completes with exit code 0 or error reflecting run status
- Fjall keyspaces contain workflow source and run header

### Scenario 3: Inspect a run with an invalid database path

**Given**: run ID `12345` and path `/nonexistent/invalid/path`

**When**: the user runs `velvet-ballastics inspect 12345 --db /nonexistent/invalid/path`

**Then**:
- exit code is `CliExitCode::StorageError` (value 3 or similar)
- error message mentions "error opening journal" and the path
- NO runtime is created
- NO UI subsystem is initialized

### Scenario 4: UI mode unavailable returns diagnostic

**Given**: UI mode is not implemented

**When**: the user runs `velvet-ballastics ui`

**Then**:
- exit code is != 0
- error message indicates UI mode is not available
- storage subsystem is NOT initialized
- runtime subsystem is NOT initialized

### Scenario 5: Invalid command fails before any subsystem init

**Given**: no specific preconditions

**When**: the user runs `velvet-ballastics totally-invalid-command-name`

**Then**:
- exit code is != 0
- error is returned from argument parsing, BEFORE any of the following:
  - FjallJournal::open is called
  - Runtime::new is called
  - Makepad is initialized
  - Any subsystem module is touched

## End-to-End Pipeline Test

### test_full_pipeline_validate_without_storage

**Given**: a valid workflow file at `tests/fixtures/valid_workflow.yaml`

**When**: `velvet-ballastics validate tests/fixtures/valid_workflow.yaml` is run from a directory with no `--db` argument

**Then**:
- exit code is 0
- output contains "valid"
- Fjall is NOT initialized
- Runtime is NOT initialized
- IPC server is NOT running
- UI is NOT initialized

### test_full_pipeline_run_with_storage

**Given**: compiled workflow, input binary, temporary database directory

**When**: `velvet-ballastics run workflow.yaml --input-bin input.vbin --durability journaled --db /tmp/verify.db` runs to completion

**Then**:
- exit code reflects run outcome (0 for success, != 0 for failure)
- Fjall database contains workflow source, compiled IR, run header, and events
- Runtime created and ticked successfully
