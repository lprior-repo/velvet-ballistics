# Martin Fowler Test Plan: vb-kkvb

These are downstream executable-specification scenarios. This State 1 artifact does not implement tests.

## Happy Path Tests
- `given_xtask_help_when_rendered_then_all_required_command_families_appear_once`
- `given_each_required_command_when_parsed_then_it_routes_to_its_distinct_enum_variant`
- `given_representative_command_when_invoked_then_structured_status_contains_required_fields`
- `given_deferred_command_family_when_invoked_then_status_is_deferred_not_successful_execution`
- `given_existing_legacy_xtask_command_when_invoked_then_existing_behavior_is_preserved`

## Error Path Tests
- `given_unknown_subcommand_when_invoked_then_exit_is_nonzero_and_diagnostic_is_actionable`
- `given_command_requires_input_when_input_missing_then_missing_required_input_error_is_returned`
- `given_invalid_required_input_when_invoked_then_typed_validation_error_is_reported`
- `given_renderer_write_failure_when_status_rendered_then_output_render_failed_error_is_returned`
- `given_forbidden_runtime_dependency_edge_when_boundary_checked_then_dependency_boundary_violation_is_returned`
- `given_registry_schema_drift_when_validated_then_internal_invariant_violation_is_returned`

## Edge Case Tests
- `given_empty_argv_after_binary_name_when_parsed_then_help_or_missing_command_error_is_returned_without_panic`
- `given_command_name_with_wrong_case_when_parsed_then_unknown_command_error_is_returned`
- `given_command_name_with_extra_dash_when_parsed_then_unknown_command_error_is_returned`
- `given_hostile_argv_tokens_when_fuzzed_then_parser_never_panics_and_fails_closed`
- `given_malformed_option_values_when_fuzzed_then_missing_or_invalid_input_error_is_returned`
- `given_all_required_command_names_when_registry_validated_then_no_duplicates_exist`
- `given_duplicate_command_name_in_registry_when_validated_then_internal_invariant_violation_is_returned`
- `given_all_command_families_when_placeholder_status_rendered_then_required_fields_are_present_for_each_family`

## Contract Verification Tests
- `test_precondition_noninteractive_argv_stream`
- `test_precondition_known_or_unknown_command_classification`
- `test_precondition_required_inputs_are_validated`
- `test_precondition_runtime_dependency_boundary_is_preserved`
- `test_postcondition_help_lists_required_command_families_once`
- `test_postcondition_known_commands_route_to_distinct_typed_variants`
- `test_postcondition_structured_status_has_required_fields`
- `test_postcondition_unknown_command_fails_closed`
- `test_invariant_routing_is_deterministic`
- `test_invariant_command_names_are_unique_and_stable_kebab_case`
- `test_invariant_no_forbidden_constructs_or_unsafe_are_introduced`
- `test_invariant_runtime_core_has_no_tooling_dependencies`

## Given/When/Then Scenarios

### Scenario 1: Help lists required command families
Given: the workspace contains the expanded xtask command registry
When: a user invokes xtask help non-interactively
Then:
- the help output includes `ai-context`, `ai-plan`, `ai-check`, `ai-evidence`, `invariants`, `scans`, `cert-check`, `perf`, `replay`, `crash`, `diff`, `mutants`, `loom`, `kani`, `fuzz`, `prop`, `repro`, `test-plan`, `review`, and `why-failed`
- each required family appears exactly once
- the command exits successfully

### Scenario 2: Known command routes to typed placeholder status
Given: `ai-context` is a known required command family
When: a user invokes the command with valid minimal inputs
Then:
- the parser returns the typed `ai-context` command variant
- the handler returns a structured status with `command`, `status`, `message`, and `next_steps`
- if the underlying operation is not implemented by this bead, `status` is deferred/unavailable rather than fake success

### Scenario 3: Unknown command fails closed
Given: `not-a-real-command` is not in the required or legacy command set
When: a user invokes `xtask not-a-real-command`
Then:
- the process exits non-zero
- the diagnostic identifies the unknown command
- no placeholder success output is emitted

### Scenario 4: Missing required input is actionable
Given: a command variant requires a bead ID, path, mode, or format
When: the user omits that required input
Then:
- the command returns `MissingRequiredInput`
- the diagnostic names the missing input and command
- the process exits non-zero without prompting for the missing value

### Scenario 5: Runtime dependency boundary is preserved
Given: the xtask command shell has been expanded
When: dependency-boundary verification inspects `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`
Then:
- none of those crates depends on `xtask`, Clap, JSON/YAML/HTTP tooling, or other xtask-only dependencies because of this bead
- dependency-boundary evidence is recorded for review

### Scenario 6: Existing legacy commands are not regressed
Given: legacy commands such as UI snapshot/token/overlap commands existed before this bead
When: those commands are invoked with their documented valid inputs
Then:
- they route to their previous handlers
- their success and error behavior remains compatible unless a separate bead contracts a change

### Scenario 7: Registry validation rejects duplicate command names
Given: a synthetic registry contains duplicate normalized command names
When: registry validation runs
Then:
- validation returns `InternalInvariantViolation`
- no `ValidatedCommandRegistry` is constructed

### Scenario 8: Structured status schema is stable across families
Given: every required command family can produce a placeholder or success status
When: each status is rendered
Then:
- each rendered status exposes the same required field names
- `command` equals the stable public command spelling
- `next_steps` is present even for deferred commands

### Scenario 9: Hostile CLI input is fuzzed at the xtask wrapper boundary
Given: a Bolero or cargo-fuzz target generates arbitrary command names, malformed option tokens, missing values, duplicate flags, and delimiter-like argv sequences
When: the generated argv sequence is passed to the xtask parse wrapper
Then:
- parsing never panics
- unknown names fail closed as `UnknownCommand`
- missing values fail as `MissingRequiredInput`
- invalid present values fail as `InvalidInput`
- no generated input is treated as silent success unless it maps to a valid known command and valid inputs

### Scenario 10: Non-interactive behavior has static evidence
Given: the expanded xtask source is scanned for stdin, TTY confirmation, editor launch, network wait, and interactive subprocess prompts
When: the static non-interactive scan runs
Then:
- no forbidden prompt source is present in the contracted command shell
- any future exception requires an explicit contract amendment and reviewer approval

## Mutation Targets
- Remove a required command family from help and ensure tests fail.
- Duplicate a command name and ensure registry validation tests fail.
- Change a placeholder `deferred` status to `ok` and ensure tests fail.
- Swallow unknown commands as success and ensure error-path tests fail.
- Add a forbidden runtime dependency edge and ensure dependency-boundary tests fail.

## Manual QA Scenarios
- Run help in a non-interactive shell and capture output.
- Run at least one representative required command and capture structured output.
- Run an unknown command and capture non-zero diagnostic.
- Confirm no command waits for stdin or opens an editor.
