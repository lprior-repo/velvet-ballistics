bead_id: vb-8iwj
phase: Wave 3 integration evidence
updated_at: 2026-05-11T00:00:00Z

# Integration Evidence

STATUS: LANDING_BLOCKED

## Conflict resolution

Created separate JJ merge workspace:

```text
jj workspace add --name Velvet-ballistics-vb-8iwj-wave3-integration -r luomokpo -r mumknuuy -r wzrtnuot -m "vb-8iwj: integrate wave 3 CLI workspaces" /home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-integration
```

Initial conflict:

```text
crates/velvet_ballastics/tests/cli_integration.rs 3-sided conflict
```

Resolution: manually combined all three EOF-appended test modules and preserved:

- `vb-qi37.13.4` structured output tests, including real YAML parse/non-JSON assertion.
- `vb-qi37.15.1` simulate text/JSON/error/no-DB tests.
- `vb-qi37.15.2` submit ledger/JSON/error tests.

Conflict marker check after resolution:

```text
grep for <<<<<<<, >>>>>>>, %%%%%%%, +++++++ markers in cli_integration.rs -> no matches reported by tool.
jj status -> no unresolved conflict warning; working copy has M crates/velvet_ballastics/tests/cli_integration.rs.
```

## Scoped machine evidence

```text
cargo +nightly fmt -p velvet_ballastics --check -> exit 0
rtk cargo check -p velvet_ballastics --all-targets -> cargo build: 0 errors, 1 duplicate-package warning
```

Integrated CLI tests:

```text
cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested -> 1 passed, 85 filtered out
cli_help_is_bounded_and_non_interactive -> 1 passed, 85 filtered out
cli_status_json_writes_payload_to_stdout_only -> 1 passed, 85 filtered out
cli_unknown_command_returns_stderr_diagnostic_without_stack_trace -> 1 passed, 85 filtered out
cli_simulate_valid_workflow_reports_dry_run_summary -> 1 passed, 85 filtered out
cli_simulate_json_emits_deterministic_trace -> 1 passed, 85 filtered out
cli_simulate_invalid_workflow_reports_diagnostic -> 1 passed, 85 filtered out
cli_simulate_does_not_create_db_side_effects -> 1 passed, 85 filtered out
cli_submit_persists_ledger_before_success -> 1 passed, 85 filtered out
cli_submit_json_returns_structured_identifiers -> 1 passed, 85 filtered out
cli_submit_rejects_missing_input_bin -> 1 passed, 85 filtered out
cli_submit_rejects_unknown_durability -> 1 passed, 85 filtered out
```

## Manual QA evidence

Status YAML:

```text
rtk cargo run -p velvet_ballastics --bin vb -- status --emit yaml
schema_version: velvet-ballastics/cli-output/v1
kind: status
status: running
running: true
shutting_down: false
command_queue:
  depth: 0
  capacity: 1024
active_runs:
  active: 0
  max_active_runs: 1024
trace_ring:
  capacity: 4096
  dropped: 0
step_budget_per_tick: 1000
runtime_policy: Strict
```

Simulate JSON using a temp workflow matching `CLI_WORKFLOW`:

```text
rtk cargo run -p velvet_ballastics --bin vb -- simulate /tmp/.../cli-workflow.yaml --json
{
  "kind": "simulate",
  "schema_version": "velvet-ballastics/v1",
  "success": true,
  "total_actions": 0,
  "total_branches": 0,
  "total_steps": 2,
  "trace": [
    {"description": "Set constant value", "kind": "set_const", "step": 0},
    {"description": "Finish -- would complete run", "kind": "finish", "step": 1}
  ]
}
```

Submit JSON using a temp workflow and temp DB:

```text
rtk cargo run -p velvet_ballastics --bin vb -- submit /tmp/.../cli-workflow.yaml --input-bin /dev/null --db /tmp/.../submit-db --durability strict --json
{
  "digest": "67e9d102e4b112a6177310c84eec69abe8c356a7dd1e65dfc60208385bb4a6a0",
  "run_id": 1778512346823153217,
  "status": "submitted",
  "step_count": 2
}
```

Note: an earlier manual QA attempt using `tests/fixtures/valid/minimal.yaml` failed because that fixture includes `description`, which this CLI compiler rejects as `unknown top-level workflow field: description`. Retried with the exact valid `CLI_WORKFLOW` shape.

## Not run

`moon run :quick`, `moon run :test`, and `moon ci` were not run in this integration workspace. Prior known repo-wide Moon debt is tracked in `vb-w823`; this pass ran scoped CLI gates only.

## Landing blocker

Classification: LANDING_BLOCKED.

The merge workspace proves the three sibling changes can be combined locally, but original beads are not closed because State 15 requires actual landing/sync/cleanup. Source was not pushed and no bookmark was forced. Original workspaces remain present by user instruction.
