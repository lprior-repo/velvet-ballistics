# Test Plan: cli Add submit command and job ledger

## Summary
- Behaviors identified: 5
- Trophy allocation: 0 unit / 4 integration / 1 e2e
- Proptest invariants: 0
- Fuzz targets: workflow/input decode parser boundaries covered by existing parser/codec tests and invalid scenarios
- Kani harnesses: 0

## 1. Behavior Inventory
- Submit persists job/run metadata before success.
- Submit returns structured identifiers for successful submission.
- Submit supports later inspect/events lookup.
- Submit rejects missing input/workflow paths.
- Submit rejects invalid durability arguments non-interactively.

## 2. Trophy Allocation
- Integration tests use real temp Fjall journal and CLI subprocess.
- E2E scenario is submit followed by inspect/events.

## 3. BDD Scenarios
- `cli_submit_persists_ledger_before_success`: Given valid workflow/input/temp DB; When submit runs; Then exit 0 and a run header/events are inspectable.
- `cli_submit_json_returns_structured_identifiers`: Given valid workflow/input/temp DB; When submit runs with `--json`; Then stdout parses with numeric `run_id`, hex `digest`, `status=submitted`, and `step_count=2`; stderr empty.
- `cli_submit_supports_later_inspection`: Given submit success; When `inspect <run_id> --db`; Then stdout references the submitted run or status data.
- `cli_submit_rejects_missing_input_bin`: Given missing input path; When submit runs; Then non-zero exit, stderr names read error, stdout empty.
- `cli_submit_rejects_unknown_durability`: Given unknown durability; When submit runs; Then non-zero exit and stderr names expected durability values.

## 4. Proptest Invariants
- None for CLI shell.

## 5. Fuzz Targets
- Workflow YAML and postcard input fuzzing remain existing/follow-up parser obligations.

## 6. Kani Harnesses
- None.

## 7. Mutation Checkpoints
- Moving success print before ledger writes must be caught by submit/inspect test.
- Omitting run header write must fail later inspection.
- Returning malformed JSON must fail structured identifier test.
- Threshold: 90%.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| valid submit text | valid all inputs | submitted + inspectable | integration |
| valid submit json | valid all inputs + --json | exact identifiers | integration |
| missing input | absent file | non-zero stderr read error | integration |
| invalid durability | bad enum | non-zero stderr enum error | integration |
| submit inspect | persisted run | later lookup works | e2e |
