# Test Plan: cli Add simulate command

## Summary
- Behaviors identified: 4
- Trophy allocation: 0 unit / 3 integration / 1 e2e
- Proptest invariants: 0
- Fuzz targets: workflow YAML parser boundary covered by invalid workflow scenario and existing parser coverage
- Kani harnesses: 0

## 1. Behavior Inventory
- Simulate reports a deterministic dry-run summary for a valid workflow.
- Simulate structured JSON includes total steps/actions/branches and trace entries.
- Simulate rejects invalid workflow artifacts with diagnostics.
- Simulate does not require or create a durable DB side effect.

## 2. Trophy Allocation
- Integration subprocess tests dominate because the public API is CLI.
- E2E scenario runs a real temporary workflow through `vb simulate`.

## 3. BDD Scenarios
- `cli_simulate_valid_workflow_reports_dry_run_summary`: Given valid temp workflow; When `vb simulate workflow.yaml`; Then exit 0, stdout contains `simulation summary` and `dry-run complete`, stderr empty.
- `cli_simulate_json_emits_deterministic_trace`: Given valid temp workflow; When `vb simulate workflow.yaml --json`; Then exit 0 and stdout parses with `success=true`, `total_steps=2`, and nonempty `trace`.
- `cli_simulate_invalid_workflow_reports_diagnostic`: Given malformed workflow; When simulate runs; Then non-zero exit and stderr contains compile/parse diagnostic.
- `cli_simulate_does_not_create_db_side_effects`: Given a temp directory with no DB path; When simulate runs without `--db`; Then no DB directory is created and command succeeds.

## 4. Proptest Invariants
- None for CLI shell.

## 5. Fuzz Targets
- Workflow YAML parser fuzz target remains outside this bead.

## 6. Kani Harnesses
- None.

## 7. Mutation Checkpoints
- Deleting trace entries must fail JSON test.
- Executing action instead of describing would-execute must fail output/no-side-effect checks.
- Threshold: 90%.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| valid text | valid workflow | dry-run summary | integration |
| valid json | valid workflow + --json | exact structured totals | integration |
| invalid workflow | malformed file | non-zero diagnostic | integration |
| no side effects | no db | no db created | e2e |
