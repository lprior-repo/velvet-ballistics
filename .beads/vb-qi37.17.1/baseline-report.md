# Baseline Report - vb-qi37.17.1

## Metadata
- **Bead**: vb-qi37.17.1 (cli: Add incident command)
- **Date**: 2026-05-17
- **Source**: origin/main at 19a8663f (fix: resolve recovery BDD test and kani workflow conflicts)
- **Workspace**: /home/lewis/src/go-skill-vb-qi37.17.1 (jj workspace)

## Incident Command Status (Current Implementation)

### What Exists
- `commands_incident.rs` - `IncidentReport` struct, `build_incident_report()`, `build_repair_hints()`
- `args.rs` - `Command::Incident { run_id, db, output }`, `parse_incident()`
- `app_impl.rs` - `cmd_incident()` wiring (lines 3108-3210)
- CLI help string includes `incident` in the command list
- `main.rs` declares `mod commands_incident`

### Functionality
- Opens FjallJournal, reads events for a given run_id
- Calls `build_incident_report` which parses StepStarted, ActionCompletedEvent, ActionFailedEvent, RunFailedEvent, RunCancelled events
- Outputs JSON/JSONL/Text with: run_id, failure_code, failed_at_step, side_effects, repair_hints
- Handles missing run (empty events) with structured error output

### Acceptance Gap Analysis
Acceptance criteria: "incident returns structured failure evidence without stack traces; tests cover failed, missing, and non-failed runs."

- Structured failure evidence: DONE (JSON output with failure_code, failed_at_step, side_effects, repair_hints)
- No stack traces: NEEDS VERIFICATION (text output doesn't explicitly include stack traces, but need to check all paths)
- Tests for failed runs: MISSING
- Tests for missing runs: MISSING
- Tests for non-failed runs: MISSING

### Code Quality Concerns (pre-existing, not in incident scope)
- `unwrap_or_default()` in json formatting (app_impl.rs lines 3181, 3185) - zero-unwrap violation
- `unwrap_or("unknown")` in text formatting (line 3202) - zero-unwrap violation

## Build Status (Baseline)

### moon ci --force Result: 6 completed, 4 failed, 11 skipped (35s)

#### Failed Tasks
1. **velvet-ballistics:fuzz-smoke** - `replay_events()` takes 3 args but 2 supplied (fuzz target)
2. **velvet-ballistics:lint-src** - `recover_full_journal()` takes 5 args but 3 supplied (vb_cli); 10 xtask lint errors
3. **velvet-ballistics:fmt** - diff marker encountered
4. **velvet-ballistics:check** - `vb_storage` recovery_bdd_tests + replay_resume tests; 10+ recover_full_journal call-site errors

#### Root Cause of Build Blockers
- `recovery/replay/core.rs` changed function signatures:
  - `replay_events(journal, run, tracker)` -> `replay_events(events, tracker, expected_action_abi_digests)` (3 args)
  - `recover_full_journal(journal, run, tracker)` -> `recover_full_journal(journal, run, tracker, expected_action_abi_digests, expected_policy_digests)` (5 args)
- 10+ call sites across vb_cli, vb_storage tests, and fuzz targets not updated

### Build Impact on Bead
- Incident command code itself compiles fine (uses `journal.events_for_run()` directly, not recover_full_journal)
- Full moon ci cannot pass until cmd_diff and other callers are fixed
- This is a **BLOCK_LOCAL** for the incident bead's test execution

## Scope Determination
The incident command implementation is complete but:
1. Has zero-unwrap violations in formatting paths (minor, fixable)
2. Has no test coverage
3. Cannot be tested until repo-level compile errors are resolved

**Decision**: Fix the cmd_diff compile error as part of this bead (it's the same code path pattern - recover_full_journal call site). Other build issues (xtask, fuzz, fmt) are out of scope.

## Risk Tags
- BLOCK_LOCAL: compile error in cmd_diff (recover_full_journal signature change)
- ZERO_UNWRAP_VIOLATION: unwrap_or_default/unwrap_or in cmd_incident text formatting
- NO_TESTS: incident command has zero dedicated test coverage
