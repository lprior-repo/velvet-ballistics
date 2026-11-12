# Test Plan: cli Structured Output Contract Tests

## Summary
- Behaviors identified: 5
- Trophy allocation: 0 unit / 4 integration / 1 e2e
- Proptest invariants: 0
- Fuzz targets: CLI argv parser boundary deferred to existing parser tests
- Kani harnesses: 0

## 1. Behavior Inventory
- CLI help remains bounded and non-interactive when invoked without state.
- CLI structured status writes machine payload to stdout only when `--json` is used.
- CLI invalid command writes diagnostics to stderr only when command is unknown.
- CLI bad workflow input returns non-zero without stack traces.
- CLI unsupported master emit contract is locked as red evidence until feature beads implement it.

## 2. Trophy Allocation
- Integration: subprocess tests through `env!("CARGO_BIN_EXE_vb")`.
- E2E: representative `validate`/`status`/`help` command paths.

## 3. BDD Scenarios
- `cli_help_is_bounded_and_non_interactive`: Given no workflow state; When `vb --help`; Then exit 0, stdout contains command list, stderr empty, output length <= 8192, and no panic/stack trace text.
- `cli_status_json_writes_payload_to_stdout_only`: Given no runtime state; When `vb status --json`; Then exit 0, stdout parses as JSON with status `running`, stderr empty.
- `cli_unknown_command_returns_stderr_diagnostic_without_stack_trace`: Given invalid command; When `vb definitely-not-a-command`; Then non-zero exit, stdout empty, stderr names unknown command, no panic/stack trace.
- `cli_invalid_workflow_keeps_error_on_stderr`: Given malformed workflow file; When `vb validate broken.yaml`; Then non-zero exit and stderr contains parse/compile diagnostic.
- `cli_emit_text_yaml_postcard_contract_is_not_silent`: Given master requires emit modes; When unsupported emit-mode flag is attempted on status/help; Then test captures deterministic rejection or red gap.

## 4. Proptest Invariants
- None: process-level CLI tests only.

## 5. Fuzz Targets
- Argv parsing fuzz target recommended as follow-up if parser surface expands.

## 6. Kani Harnesses
- None.

## 7. Mutation Checkpoints
- Removing stderr assertions must fail.
- Returning success for unknown command must fail.
- Printing panic text must fail.
- Threshold: 90% mutation kill rate minimum for touched CLI tests.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| help | no args/--help | exit 0 bounded stdout | integration |
| status json | valid command | JSON status running | integration |
| unknown command | invalid command | non-zero stderr diagnostic | integration |
| invalid workflow | malformed file | non-zero parse/compile diagnostic | integration |

## Open Questions
- Whether parent emitter beads will replace `--json|--jsonl` with master `--emit text|yaml|postcard` during this wave.
