# Test Plan: vb-kkvb — Expand xtask Command Shell

## Summary

- Behaviors identified: 30
- Trophy allocation target: **40 named unit tests** / 17 integration scenarios / 2 e2e scenarios / 1 static-governance group
- Proptest invariants: 11
- Fuzz targets: 3
- Kani/proof harnesses: 7
- Error variants covered: all 7 `XtaskCommandError` variants
- Mutation threshold: **>=90% killed mutants** with no surviving mutant in command routing, diagnostics, structured status schema, placeholder/deferred classification, or dependency-boundary checks.
- Red phase expectation: these tests must fail against the current bead workspace until the typed command families, structured status, explicit errors, fuzz/property harnesses, and dependency-boundary checks are implemented.

Required public command families: `ai-context`, `ai-plan`, `ai-check`, `ai-evidence`, `invariants`, `scans`, `cert-check`, `perf`, `replay`, `crash`, `diff`, `mutants`, `loom`, `kani`, `fuzz`, `prop`, `repro`, `test-plan`, `review`, `why-failed`.

Structured output format decision for this plan: **JSON Lines** (`OutputFormat::JsonLines`). Every successful/deferred status renders as one compact JSON object plus trailing newline. Required field order for exact renderer tests is `command`, `status`, `message`, `next_steps`.

Default deferred placeholder text for intentionally non-operational required families:

```json
{"command":"perf","status":"deferred","message":"perf automation deferred: implementation is outside bead vb-kkvb","next_steps":["open follow-up bead for perf engine integration"]}
```

Default unavailable error for commands that are known but intentionally not runnable through `route_command`:

```rust
Err(XtaskCommandError::Unavailable {
    command: "perf",
    reason: "perf automation is not implemented in bead vb-kkvb",
})
```

Contracted Rust API surface to test through public functions and CLI behavior:

- `parse_xtask_command(args: impl IntoIterator<Item = OsString>) -> Result<XtaskCommand, XtaskCommandError>`
- `required_command_families() -> &'static [CommandFamilySpec]`
- `validate_command_registry(specs: &[CommandFamilySpec]) -> Result<ValidatedCommandRegistry, XtaskCommandError>`
- `route_command(command: XtaskCommand, env: &XtaskEnvironment) -> Result<StructuredStatus, XtaskCommandError>`
- `placeholder_status(command: CommandFamily, reason: DeferredReason) -> Result<StructuredStatus, XtaskCommandError>`
- `render_structured_status(status: &StructuredStatus, format: OutputFormat) -> Result<String, XtaskCommandError>`
- `assert_runtime_dependency_boundary(manifest: &WorkspaceManifest) -> Result<(), XtaskCommandError>`

No planned assertion may be only `is_ok()` or `is_err()`. Every assertion below names exact field values, exact route variants, exact process exit state, or exact error variants.

## 1. Behavior Inventory

1. Xtask accepts non-interactive argv and never prompts when any contracted command is invoked.
2. Parser classifies known required command families as valid commands when command spelling exactly matches the required kebab-case atom.
3. Parser classifies existing legacy commands as valid legacy commands when their documented spelling is supplied.
4. Parser classifies help requests as help when `--help`, `-h`, or command-specific help is supplied.
5. Parser classifies version requests as version when `--version` or `-V` is supplied.
6. Parser rejects unknown command families with `XtaskCommandError::UnknownCommand { command }` when the first command token is unrecognized.
7. Parser rejects wrong-case command names with `UnknownCommand` when normalization would otherwise hide spelling drift.
8. Parser rejects extra-dash and delimiter-like command names with `UnknownCommand` when the token is not an exact public spelling.
9. Parser rejects missing option or positional values with `XtaskCommandError::MissingRequiredInput { command, input }` when a selected variant requires input.
10. Parser rejects syntactically present invalid values with `XtaskCommandError::InvalidInput { command, input, reason }` when bead IDs, paths, modes, or formats fail validation.
11. Required command registry returns exactly the 20 required public command families once when inspected.
12. Required command registry exposes stable kebab-case names when inspected.
13. Registry validation accepts the required unique registry when all command names are unique.
14. Registry validation rejects duplicate normalized command names with `XtaskCommandError::InternalInvariantViolation { invariant }` when duplicates are present.
15. Registry validation rejects schema drift with `InternalInvariantViolation` when a spec omits a required field or has inconsistent status schema metadata.
16. Known required command parsing routes each command to a distinct typed enum variant when each required command is supplied.
17. Route construction is deterministic when the same argv sequence is parsed repeatedly.
18. Route construction uses only immutable/owned input and no global mutable state when invoked repeatedly and under Miri.
19. Route dispatch selects handlers by closed typed variants when a validated command is routed.
20. Placeholder command families return explicit deferred/unavailable status when deeper automation is outside this bead.
21. Placeholder status is never rendered as successful execution when no real tool ran.
22. Representative command status contains `command`, `status`, `message`, and `next_steps` when rendered.
23. All success and deferred structured statuses expose the same required field names across command families.
24. Structured status renderer returns exact machine-readable output when rendering succeeds.
25. Structured status renderer returns `XtaskCommandError::OutputRenderFailed { command, reason }` when rendering or writing fails.
26. CLI unknown command exits with code `2` and emits an actionable diagnostic when invoked from the outside.
27. CLI missing or invalid input exits with code `2`, emits an actionable typed diagnostic, and does not prompt when invoked from the outside.
28. Legacy xtask commands preserve documented routing behavior when invoked with valid and invalid legacy inputs.
29. Runtime dependency boundary rejects forbidden tooling dependencies with `XtaskCommandError::DependencyBoundaryViolation { crate_name, dependency }` when `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc` gains `xtask`, Clap, JSON/YAML/HTTP, or tooling-only edges.
30. Route command returns `XtaskCommandError::Unavailable { command, reason }` when a known required command is explicitly unavailable rather than represented as a placeholder status.

## 2. Trophy Allocation

| # | Behavior | Layer | Tool/Technique | Why |
|---|----------|-------|----------------|-----|
| 1 | Non-interactive argv never prompts | Integration + manual QA + static | `assert_cmd`, timeout, source scan | Public behavior is process-level and prompt-free; static scan catches prompt APIs. |
| 2 | Required commands parse | Integration + proptest | Public `parse_xtask_command`; CLI smoke | Main compatibility surface is CLI spelling. |
| 3 | Legacy commands parse | Integration regression | `assert_cmd` against legacy commands | Prevents regressions at external boundary. |
| 4 | Help requests classify | E2E/integration | `cargo xtask --help`; public parser | Help rendering is external user behavior. |
| 5 | Version requests classify | Integration | public parser / CLI | Parser compatibility surface. |
| 6 | Unknown commands fail closed | Integration + fuzz + mutation | CLI, Bolero/cargo-fuzz, cargo-mutants | Security/fail-closed behavior lives at parser shell boundary. |
| 7 | Wrong-case rejected | Unit + integration | parser exact error | Exact spelling stability is pure enough for unit and observable via CLI. |
| 8 | Extra-dash/delimiters rejected | Unit + fuzz | parser + fuzz corpus | Hostile token class. |
| 9 | Missing input rejected | Integration + mutation | CLI validation path | Public diagnostics and exit code matter. |
| 10 | Invalid input rejected | Unit + proptest + integration | validators + CLI | Validation is pure but must be observed externally. |
| 11 | Required registry exact set | Unit + integration help golden + Lean | golden exact vector + help output | Pure registry plus public help contract. |
| 12 | Kebab-case names stable | Unit + static golden + Lean | exact name assertions | Compatibility surface is stable string set. |
| 13 | Unique registry accepted | Unit + proptest + Kani | registry validator | Pure deterministic validation. |
| 14 | Duplicate names rejected | Unit + proptest + Kani + mutation | synthetic registry | Pure invariant with critical error variant. |
| 15 | Schema drift rejected | Unit + proptest + mutation | synthetic drift registry | Protects output schema consistency. |
| 16 | Distinct typed variants | Unit + proptest + Kani + Lean | public parser/model | Closed enum mapping is pure invariant. |
| 17 | Deterministic parse | Proptest + Kani + Lean | repeated generated argv | Determinism must hold for broad input space. |
| 18 | No global mutable route state | Static + Miri | Miri repeated parse/route | Ownership/global-state issues are low-level. |
| 19 | Closed variant dispatch | Static/review + unit | source scan plus representative routes | Avoids testing private internals but verifies no post-parse stringly dispatch. |
| 20 | Placeholder explicit deferred | Integration + unit + mutation | route + CLI | Contracted current implementation behavior for unavailable families. |
| 21 | Placeholder not fake success | Integration + Lean + mutation | status kind exact value | Prevents silent quality false-pass. |
| 22 | Representative required fields | Integration + schema snapshot + Lean | renderer output parsed by field names | Machine-readable output is public behavior. |
| 23 | Schema stable all families | Unit + proptest + schema golden | generated command family statuses | Pure schema invariant over finite family set. |
| 24 | Renderer success exact output | Unit | `render_structured_status` | Pure renderer contract where format is explicit. |
| 25 | Renderer failure typed | Unit + failure injection + Miri | writer/renderer test seam | Error variant completeness. |
| 26 | Unknown CLI code-2 diagnostic | E2E + integration + manual | `assert_cmd`, transcript | User/agent boundary is black-box CLI. |
| 27 | Missing/invalid CLI diagnostic | Integration + fuzz | CLI + parser wrapper | Shell boundary and typed diagnostic. |
| 28 | Legacy behavior preserved | Integration regression + coverage | existing fixtures/tests | Prevents breaking old documented commands. |
| 29 | Runtime dependency boundary | Static + integration | manifest model, cargo tree, cargo deny | Repository dependency graph is system-level state. |
| 30 | Known unavailable command errors exactly | Unit + integration + mutation | `route_command`; CLI diagnostic | Covers `XtaskCommandError::Unavailable` as an exact error variant rather than conflating it with deferred status. |

Expected ratio after implementation: integration is the largest layer because the external CLI compatibility surface matters most; unit tests cover pure registry/schema/renderer/validator logic; E2E is limited to help/unknown-command/manual gauntlet smoke; static gates cover dependency and forbidden construct guarantees.

### Deletion-Resistant Unit Test Catalog (Minimum 35 Required; 40 Planned)

These are named unit tests for pure/public Rust API behavior. Each test must have the concrete assertion listed; deleting any one removes unique behavioral coverage.

| # | Unit test name | Public API | Concrete assertion |
|---|----------------|------------|--------------------|
| U01 | `required_registry_contains_ai_context_once()` | `required_command_families` | count of `ai-context` equals `1`. |
| U02 | `required_registry_contains_ai_plan_once()` | `required_command_families` | count of `ai-plan` equals `1`. |
| U03 | `required_registry_contains_ai_check_once()` | `required_command_families` | count of `ai-check` equals `1`. |
| U04 | `required_registry_contains_ai_evidence_once()` | `required_command_families` | count of `ai-evidence` equals `1`. |
| U05 | `required_registry_contains_invariants_once()` | `required_command_families` | count of `invariants` equals `1`. |
| U06 | `required_registry_contains_scans_once()` | `required_command_families` | count of `scans` equals `1`. |
| U07 | `required_registry_contains_cert_check_once()` | `required_command_families` | count of `cert-check` equals `1`. |
| U08 | `required_registry_contains_perf_once()` | `required_command_families` | count of `perf` equals `1`. |
| U09 | `required_registry_contains_replay_once()` | `required_command_families` | count of `replay` equals `1`. |
| U10 | `required_registry_contains_crash_once()` | `required_command_families` | count of `crash` equals `1`. |
| U11 | `required_registry_contains_diff_once()` | `required_command_families` | count of `diff` equals `1`. |
| U12 | `required_registry_contains_mutants_once()` | `required_command_families` | count of `mutants` equals `1`. |
| U13 | `required_registry_contains_loom_once()` | `required_command_families` | count of `loom` equals `1`. |
| U14 | `required_registry_contains_kani_once()` | `required_command_families` | count of `kani` equals `1`. |
| U15 | `required_registry_contains_fuzz_once()` | `required_command_families` | count of `fuzz` equals `1`. |
| U16 | `required_registry_contains_prop_once()` | `required_command_families` | count of `prop` equals `1`. |
| U17 | `required_registry_contains_repro_once()` | `required_command_families` | count of `repro` equals `1`. |
| U18 | `required_registry_contains_test_plan_once()` | `required_command_families` | count of `test-plan` equals `1`. |
| U19 | `required_registry_contains_review_once()` | `required_command_families` | count of `review` equals `1`. |
| U20 | `required_registry_contains_why_failed_once()` | `required_command_families` | count of `why-failed` equals `1`. |
| U21 | `required_registry_len_is_twenty()` | `required_command_families` | registry length equals `20`. |
| U22 | `required_registry_sorted_names_equal_contract_set()` | `required_command_families` | sorted names equal exactly `["ai-check","ai-context","ai-evidence","ai-plan","cert-check","crash","diff","fuzz","invariants","kani","loom","mutants","perf","prop","replay","repro","review","scans","test-plan","why-failed"]`. |
| U23 | `validate_registry_accepts_required_specs()` | `validate_command_registry` | validated registry length equals `20` and first sorted name equals `ai-check`. |
| U24 | `validate_registry_rejects_duplicate_ai_context()` | `validate_command_registry` | returns `Err(InternalInvariantViolation { invariant: "duplicate command family: ai-context" })`. |
| U25 | `validate_registry_rejects_missing_next_steps_schema()` | `validate_command_registry` | returns `Err(InternalInvariantViolation { invariant: "structured status schema drift: missing next_steps" })`. |
| U26 | `parse_unknown_command_returns_unknown_command()` | `parse_xtask_command` | `xtask not-a-real-command` returns `Err(UnknownCommand { command: "not-a-real-command" })`. |
| U27 | `parse_wrong_case_ai_context_returns_unknown_command()` | `parse_xtask_command` | `xtask AI-CONTEXT` returns `Err(UnknownCommand { command: "AI-CONTEXT" })`. |
| U28 | `parse_extra_dash_ai_context_returns_unknown_command()` | `parse_xtask_command` | `xtask ai--context` returns `Err(UnknownCommand { command: "ai--context" })`. |
| U29 | `parse_ai_context_missing_bead_returns_missing_required_input()` | `parse_xtask_command` | `xtask ai-context --bead` returns `Err(MissingRequiredInput { command: "ai-context", input: "bead" })`. |
| U30 | `parse_ai_context_empty_bead_returns_invalid_input()` | `parse_xtask_command` | `xtask ai-context --bead ""` returns `Err(InvalidInput { command: "ai-context", input: "bead", reason: "bead id must not be empty" })`. |
| U31 | `parse_test_plan_xml_format_returns_invalid_input()` | `parse_xtask_command` | `xtask test-plan --format xml` returns `Err(InvalidInput { command: "test-plan", input: "format", reason: "unsupported output format: xml" })`. |
| U32 | `placeholder_perf_returns_deferred_status()` | `placeholder_status` | returns `StructuredStatus { command: "perf", status: "deferred", message: "perf automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for perf engine integration"] }`. |
| U33 | `placeholder_fuzz_returns_deferred_status()` | `placeholder_status` | returns `StructuredStatus { command: "fuzz", status: "deferred", message: "fuzz automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for fuzz engine integration"] }`. |
| U34 | `route_perf_returns_unavailable_when_unavailable_mode_enabled()` | `route_command` | returns `Err(Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })`. |
| U35 | `render_deferred_fuzz_status_returns_exact_json_line()` | `render_structured_status` | returns exactly `{"command":"fuzz","status":"deferred","message":"fuzz automation deferred: implementation is outside bead vb-kkvb","next_steps":["open follow-up bead for fuzz engine integration"]}\n`. |
| U36 | `render_status_failure_returns_output_render_failed()` | `render_structured_status` | injected broken pipe returns `Err(OutputRenderFailed { command: "fuzz", reason: "broken pipe" })`. |
| U37 | `dependency_boundary_rejects_vb_core_clap()` | `assert_runtime_dependency_boundary` | synthetic manifest returns `Err(DependencyBoundaryViolation { crate_name: "vb_core", dependency: "clap" })`. |
| U38 | `dependency_boundary_rejects_vb_runtime_xtask()` | `assert_runtime_dependency_boundary` | synthetic manifest returns `Err(DependencyBoundaryViolation { crate_name: "vb_runtime", dependency: "xtask" })`. |
| U39 | `route_ai_context_deferred_status_is_deterministic()` | `route_command` | two calls with same command/env return identical `StructuredStatus { command: "ai-context", status: "deferred", message: "ai-context automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for ai-context engine integration"] }`. |
| U40 | `parse_empty_argv_returns_missing_required_input_command()` | `parse_xtask_command` | argv `xtask` returns `Err(MissingRequiredInput { command: "xtask", input: "command" })`. |

## 3. BDD Scenarios

### Behavior 1: Xtask accepts non-interactive argv and never prompts

Test function name: `fn xtask_never_prompts_when_invoked_noninteractively()`

Given: the expanded xtask binary is run in a non-interactive process with stdin closed and a 2-second timeout.
When: each required command family is invoked with valid minimal inputs or as a known deferred placeholder.
Then: each invocation terminates before timeout.
And: stdout/stderr contain structured output or an actionable typed diagnostic.
And: no output contains `Press`, `confirm`, `continue?`, `stdin`, `editor`, or `TTY required`.

Red phase expectation: current implementation lacks most required families, so invocations should fail before implementation.

### Behavior 2: Parser classifies required command families

Test function name: `fn parser_returns_typed_variant_when_required_command_spelling_matches()`

Given: argv sequences `xtask <family>` for each of the 20 required families.
When: `parse_xtask_command` parses each argv sequence.
Then: the result equals the exact `XtaskCommand::Required(CommandFamily::<Variant>)` for that family.
And: `command.public_name()` equals the original kebab-case token.

### Behavior 3: Parser classifies legacy commands

Test function name: `fn parser_preserves_legacy_route_when_legacy_command_supplied()`

Given: documented legacy argv sequences for `ui-snapshot`, `ui-tokens`, `ui-overlap-check`, `ai-fast`, `ai-deep`, and `ai-release`.
When: `parse_xtask_command` parses each sequence.
Then: the result equals the corresponding legacy typed route variant and preserves its supplied options exactly.

### Behavior 4: Parser classifies help requests

Test function name: `fn parser_returns_help_route_when_help_flag_supplied()`

Given: argv sequences `xtask --help`, `xtask -h`, and `xtask ai-context --help`.
When: `parse_xtask_command` parses each sequence.
Then: each result equals the exact help route for global or command help.
And: CLI help output includes each required family exactly once.

### Behavior 5: Parser classifies version requests

Test function name: `fn parser_returns_version_route_when_version_flag_supplied()`

Given: argv sequences `xtask --version` and `xtask -V`.
When: `parse_xtask_command` parses each sequence.
Then: each result equals `XtaskCommand::Version`.
And: CLI version output contains package name `xtask` and a semantic version string from the package metadata.

### Behavior 6: Parser rejects unknown command families

Test function name: `fn parser_returns_unknown_command_when_command_family_is_unrecognized()`

Given: argv `xtask not-a-real-command`.
When: `parse_xtask_command` parses the sequence.
Then: the result equals `Err(XtaskCommandError::UnknownCommand { command: "not-a-real-command" })`.
And: no `StructuredStatus` is produced.

### Behavior 7: Parser rejects wrong-case command names

Test function name: `fn parser_returns_unknown_command_when_command_case_is_wrong()`

Given: argv `xtask AI-CONTEXT` and `xtask Ai-Context`.
When: `parse_xtask_command` parses each sequence.
Then: each result equals `Err(XtaskCommandError::UnknownCommand { command: supplied_token })`.

### Behavior 8: Parser rejects extra-dash and delimiter-like command names

Test function name: `fn parser_returns_unknown_command_when_command_token_is_not_exact_spelling()`

Given: argv examples `xtask ai--context`, `xtask --ai-context`, `xtask ai-context/../why-failed`, `xtask ai_context`, and `xtask ai.context`.
When: `parse_xtask_command` parses each sequence.
Then: each result equals `Err(XtaskCommandError::UnknownCommand { command: supplied_token })`.

### Behavior 9: Parser rejects missing required inputs

Test function name: `fn parser_returns_missing_required_input_when_required_option_value_is_absent()`

Given: argv `xtask ai-context --bead` with the required `bead` value omitted.
When: `parse_xtask_command` parses the sequence.
Then: the result equals `Err(XtaskCommandError::MissingRequiredInput { command: "ai-context", input: "bead" })`.
And: CLI invocation exits with code `2` without prompting for the missing value.

### Behavior 10: Parser rejects invalid present inputs

Test function name: `fn parser_returns_invalid_input_when_present_value_fails_validation()`

Given: argv sequence `xtask ai-context --bead ""`.
When: `parse_xtask_command` parses the sequence.
Then: the result equals `Err(XtaskCommandError::InvalidInput { command: "ai-context", input: "bead", reason: "bead id must not be empty" })`.
And: argv sequence `xtask test-plan --format xml` returns `Err(XtaskCommandError::InvalidInput { command: "test-plan", input: "format", reason: "unsupported output format: xml" })`.

### Behavior 11: Required command registry returns exact family list

Test function name: `fn required_command_families_returns_exact_required_family_set_once()`

Given: the public `required_command_families()` API.
When: the returned specs are mapped to public command names.
Then: the ordered or sorted set equals exactly `ai-context`, `ai-plan`, `ai-check`, `ai-evidence`, `invariants`, `scans`, `cert-check`, `perf`, `replay`, `crash`, `diff`, `mutants`, `loom`, `kani`, `fuzz`, `prop`, `repro`, `test-plan`, `review`, `why-failed`.
And: each name count equals 1.

### Behavior 12: Required command names are stable kebab-case

Test function name: `fn required_command_families_use_stable_kebab_case_names()`

Given: the public registry specs.
When: every public name is inspected.
Then: each name matches the regex `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`.
And: no name contains `_`, `.`, `/`, uppercase letters, leading dash, trailing dash, or repeated dash.

### Behavior 13: Registry validation accepts unique registry

Test function name: `fn registry_validation_returns_validated_registry_when_names_are_unique()`

Given: the exact specs returned by `required_command_families()`.
When: `validate_command_registry(specs)` runs.
Then: it returns `ValidatedCommandRegistry` whose names equal the required set and whose length is 20.

### Behavior 14: Registry validation rejects duplicate command names

Test function name: `fn registry_validation_returns_internal_invariant_violation_when_names_duplicate()`

Given: a synthetic registry containing two specs with normalized public name `ai-context`.
When: `validate_command_registry(specs)` runs.
Then: it returns `Err(XtaskCommandError::InternalInvariantViolation { invariant: "duplicate command family: ai-context" })`.
And: no `ValidatedCommandRegistry` value is constructed.

### Behavior 15: Registry validation rejects schema drift

Test function name: `fn registry_validation_returns_internal_invariant_violation_when_schema_metadata_drifts()`

Given: a synthetic registry whose status metadata omits `next_steps` or declares a non-standard field set.
When: `validate_command_registry(specs)` runs.
Then: it returns `Err(XtaskCommandError::InternalInvariantViolation { invariant: "structured status schema drift: missing next_steps" })`.

### Behavior 16: Known commands route to distinct enum variants

Test function name: `fn parser_routes_each_required_family_to_distinct_enum_variant()`

Given: one argv sequence for every required family.
When: `parse_xtask_command` parses all sequences.
Then: the resulting variants are pairwise distinct.
And: `ai-context` is not equal to `ai-plan`, `mutants` is not equal to `fuzz`, and `why-failed` is not equal to `review`.

### Behavior 17: Route construction is deterministic

Test function name: `fn parser_returns_identical_result_when_same_argv_is_parsed_repeatedly()`

Given: any generated argv sequence from valid, unknown, missing-input, and invalid-input classes.
When: `parse_xtask_command(argv.clone())` is called twice.
Then: the two results are exactly equal, including command variant or exact error variant and fields.

### Behavior 18: Route construction uses no global mutable state

Test function name: `fn route_construction_uses_no_global_mutable_state_when_repeated()`

Given: repeated parse and route invocations in a Miri-enabled test process.
When: the same known command and then a different known command are parsed and routed 100 times.
Then: every result matches the command-specific expected route/status.
And: no earlier invocation changes later route output.

### Behavior 19: Route dispatch selects handlers by closed typed variants

Test function name: `fn route_command_returns_status_for_closed_variant_without_stringly_dispatch()`

Given: a validated `XtaskCommand` value for `CommandFamily::Kani` and an `XtaskEnvironment` with deterministic paths.
When: `route_command(command, &env)` runs.
Then: it returns `StructuredStatus { command: "kani", status: "deferred", message: "kani automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for kani engine integration"] }`.
And: static review evidence shows no unchecked post-parse free-form string handler lookup.

### Behavior 20: Placeholder commands return explicit deferred/unavailable status

Test function name: `fn placeholder_status_returns_deferred_when_command_family_is_not_implemented()`

Given: `CommandFamily::Perf` and `DeferredReason::NotImplementedInThisBead`.
When: `placeholder_status(command, reason)` runs.
Then: it returns `StructuredStatus { command: "perf", status: "deferred", message: "perf automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for perf engine integration"] }`.
And: the status is not `success`, `ok`, `passed`, or `executed`.

### Behavior 21: Placeholder status is never fake success

Test function name: `fn placeholder_status_never_reports_successful_execution_when_deferred()`

Given: every required family that is intentionally deferred in this bead.
When: `placeholder_status` creates a status for each family.
Then: each status `status` field equals `deferred` or `unavailable` exactly.
And: no status field equals `ok`, `success`, `passed`, or `executed` unless a real separately contracted implementation ran and produced evidence.

### Behavior 22: Representative status contains required fields

Test function name: `fn rendered_status_contains_command_status_message_and_next_steps_when_command_invoked()`

Given: representative invocation `xtask ai-context --bead vb-kkvb --format jsonl`.
When: the command routes and the status is rendered.
Then: the rendered JSON Line contains exactly required fields `command`, `status`, `message`, and `next_steps` in that order.
And: `command` equals `ai-context`.
And: `status` equals `deferred`.
And: `message` equals `ai-context automation deferred: implementation is outside bead vb-kkvb`.
And: `next_steps` equals `["open follow-up bead for ai-context engine integration"]`.

### Behavior 23: Structured status schema is stable across families

Test function name: `fn rendered_status_uses_same_required_field_names_for_all_success_and_deferred_families()`

Given: a success or deferred `StructuredStatus` for each required command family.
When: each status is rendered as `OutputFormat::JsonLines`.
Then: every rendered status contains the same required field names `command`, `status`, `message`, `next_steps`.
And: no family omits `next_steps`; for `fuzz`, `next_steps` equals `["open follow-up bead for fuzz engine integration"]`.

### Behavior 24: Renderer emits exact machine-readable output

Test function name: `fn renderer_returns_exact_structured_text_when_status_is_valid()`

Given: `StructuredStatus { command: "fuzz", status: "deferred", message: "fuzz automation deferred", next_steps: ["run verify-deep when implemented"] }`.
When: `render_structured_status(&status, OutputFormat::JsonLines)` runs.
Then: the returned string equals exactly `{"command":"fuzz","status":"deferred","message":"fuzz automation deferred","next_steps":["run verify-deep when implemented"]}\n`.
And: parsing that line as JSON yields `command == "fuzz"`, `status == "deferred"`, `message == "fuzz automation deferred"`, and `next_steps == ["run verify-deep when implemented"]`.

### Behavior 25: Renderer failures map to OutputRenderFailed

Test function name: `fn renderer_returns_output_render_failed_when_render_or_write_fails()`

Given: `StructuredStatus { command: "fuzz", status: "deferred", message: "fuzz automation deferred", next_steps: ["run verify-deep when implemented"] }` and a renderer/writer test seam that deterministically refuses output with reason `broken pipe`.
When: rendering or writing is attempted.
Then: the result equals `Err(XtaskCommandError::OutputRenderFailed { command: "fuzz", reason: "broken pipe" })`.

### Behavior 26: CLI unknown command exits code 2 with actionable diagnostic

Test function name: `fn cli_exits_nonzero_and_names_unknown_command_when_subcommand_is_unknown()`

Given: the built xtask binary.
When: a process invokes `cargo xtask not-a-real-command`.
Then: process exit code is exactly `2`.
And: stderr or structured error output includes `UnknownCommand` and `not-a-real-command`.
And: output includes remediation hint `run xtask --help`.
And: output does not include a success/deferred structured status.

### Behavior 27: CLI missing/invalid input exits code 2 with actionable typed diagnostic

Test function name: `fn cli_exits_nonzero_and_names_input_when_required_input_is_missing_or_invalid()`

Given: the built xtask binary.
When: the process invokes `cargo xtask ai-context --bead`.
Then: process exit code is exactly `2`.
And: diagnostic includes exact error variant `MissingRequiredInput`.
And: diagnostic names command `ai-context` and input `bead`.
And: when the process invokes `cargo xtask test-plan --format xml`, process exit code is exactly `2` and diagnostic includes `InvalidInput`, command `test-plan`, input `format`, and reason `unsupported output format: xml`.
And: command terminates without prompting.

### Behavior 28: Legacy xtask commands preserve routing behavior

Test function name: `fn legacy_xtask_commands_preserve_documented_behavior_when_invoked()`

Given: fixture-backed legacy commands `ui-snapshot`, `ui-tokens`, `ui-overlap-check`, `ai-fast`, `ai-deep`, and `ai-release`.
When: each is invoked with its documented valid minimal arguments and existing fixtures.
Then: each routes to the same legacy handler and emits the same documented output shape or evidence path as before this bead.
And: known legacy errors still report their existing actionable diagnostics unless explicitly superseded by the new typed error taxonomy.

### Behavior 29: Runtime dependency boundary rejects forbidden tooling dependencies

Test function name: `fn dependency_boundary_returns_violation_when_runtime_crate_depends_on_tooling()`

Given: a synthetic `WorkspaceManifest` where `vb_core` depends on `clap`.
When: `assert_runtime_dependency_boundary(&manifest)` runs.
Then: it returns `Err(XtaskCommandError::DependencyBoundaryViolation { crate_name: "vb_core", dependency: "clap" })`.
And: a real workspace dependency scan proves `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc` have no `xtask`, Clap, JSON/YAML/HTTP, or tooling-only dependencies.

### Behavior 30: Known unavailable command returns exact Unavailable error

Test function name: `fn route_command_returns_unavailable_when_known_command_is_not_runnable()`

Given: `XtaskCommand::Required(CommandFamily::Perf)` and `XtaskEnvironment { unavailable_families: [CommandFamily::Perf], output_format: OutputFormat::JsonLines }`.
When: `route_command(command, &env)` runs.
Then: the result equals `Err(XtaskCommandError::Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })`.
And: no `StructuredStatus` is rendered for that route.

## 4. Proptest Invariants

### Proptest: `parse_xtask_command` determinism

- Invariant: identical argv vectors produce identical `Result<XtaskCommand, XtaskCommandError>` values, including all error fields.
- Strategy: generate `Vec<OsString>` with a binary name, one command-like token from valid required names, legacy names, unknown ASCII, wrong-case, delimiter-like, and option/value tails.
- Anti-invariant: no generated unknown token may produce a success route or placeholder status.

### Proptest: `route_command` determinism over accepted command domain

- Invariant: for any accepted `XtaskCommand::Required(family)` and deterministic `XtaskEnvironment`, two calls to `route_command(command.clone(), &env)` return exactly the same `StructuredStatus` or exactly the same `Unavailable` error.
- Strategy: generate `CommandFamily` from the exact 20-family enum and `XtaskEnvironment` with `output_format = OutputFormat::JsonLines`, `workspace_root = "/workspace"`, `bead_id = Some("vb-kkvb")`, and `unavailable_families` as a generated subset of required families.
- Concrete accepted example: `CommandFamily::AiContext` not in `unavailable_families` returns `StructuredStatus { command: "ai-context", status: "deferred", message: "ai-context automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for ai-context engine integration"] }` on both calls.
- Concrete rejected example: `CommandFamily::Perf` in `unavailable_families` returns `Err(XtaskCommandError::Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })` on both calls.
- Anti-invariant: routing must never alternate between `StructuredStatus` and `Unavailable` for identical command/environment inputs.

### Proptest: `route_command` accepted/rejected command domains

- Invariant: required families not listed in `env.unavailable_families` return deferred `StructuredStatus`; required families listed in `env.unavailable_families` return exact `Unavailable`; unknown commands cannot be constructed as `XtaskCommand::Required` and must be rejected earlier by `parse_xtask_command` as `UnknownCommand`.
- Strategy: generate all required `CommandFamily` values and a generated unavailable subset; compose with generated parse results from known and unknown command tokens.
- Concrete accepted assertion: `CommandFamily::Fuzz` absent from unavailable set returns `StructuredStatus { command: "fuzz", status: "deferred", message: "fuzz automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for fuzz engine integration"] }`.
- Concrete rejected assertion: `CommandFamily::Fuzz` present in unavailable set returns `Err(XtaskCommandError::Unavailable { command: "fuzz", reason: "fuzz automation is not implemented in bead vb-kkvb" })`.
- Anti-invariant: an unknown command token such as `not-a-real-command` must never reach `route_command`; parse must produce `Err(UnknownCommand { command: "not-a-real-command" })`.

### Proptest: known required command names route distinctly

- Invariant: every generated required family name maps to its corresponding unique typed variant and never aliases another family.
- Strategy: `prop_oneof!` over the exact 20 required family atoms plus generated valid option sets.
- Anti-invariant: pairwise distinct family names must never produce equal required-family variants.

### Proptest: registry uniqueness validation

- Invariant: registries with unique normalized public names validate; registries with duplicates fail with `InternalInvariantViolation` naming the duplicate.
- Strategy: generate vectors of `CommandFamilySpec` from valid kebab-case atoms, with a boolean controlling duplicate injection.
- Anti-invariant: any duplicated normalized name must always fail validation.

### Proptest: required command names stay kebab-case

- Invariant: all accepted generated public command names match the stable kebab-case regex and contain no forbidden characters.
- Strategy: generate valid kebab-case atoms for synthetic accepted specs and invalid atoms with uppercase, underscore, slash, dot, repeated dash, leading dash, and trailing dash.
- Anti-invariant: invalid name classes must fail registry validation with `InternalInvariantViolation` or `InvalidInput` as appropriate.

### Proptest: structured status required field stability

- Invariant: rendering any success/deferred `StructuredStatus` for any command family preserves required fields `command`, `status`, `message`, `next_steps`.
- Strategy: generate command family from required family enum, status kind from `success|deferred|unavailable` allowed statuses, non-empty message, and non-empty next-steps vector.
- Anti-invariant: an empty `next_steps`, missing message, or unsupported status kind must fail construction/validation with exact typed error.

### Proptest: placeholder status never reports fake success

- Invariant: `placeholder_status(family, reason).status` is exactly `deferred` or `unavailable` for every deferred reason.
- Strategy: generate all required command families and all `DeferredReason` values.
- Anti-invariant: generated placeholder statuses must never equal `ok`, `success`, `passed`, or `executed`.

### Proptest: invalid present inputs classify as `InvalidInput`

- Invariant: syntactically present but semantically invalid bead IDs, paths, modes, and output formats return `InvalidInput` with command/input/reason fields.
- Strategy: generate invalid bead IDs (empty, whitespace, slash, NUL-adjacent where possible, Unicode control), invalid formats other than exact `jsonl`, and invalid modes.
- Anti-invariant: invalid present values must not be converted to `UnknownCommand` or silent success.

### Proptest: missing required inputs classify as `MissingRequiredInput`

- Invariant: selected commands with required options but absent values return `MissingRequiredInput` with the exact missing input name.
- Strategy: generate command variants that require input and delete one required option or value from an otherwise valid argv.
- Anti-invariant: missing values must never prompt or default to placeholder success.

### Proptest: route_command determinism and accepted/rejected domain classification

- Invariant: `route_command` is deterministic (same `XtaskCommand` x `XtaskEnvironment` always yields identical `Result<StructuredStatus, XtaskCommandError>`) and classifies every generated command into exactly one of: `Ok(StructuredStatus { status: "deferred" })` for deferred placeholder families, or `Err(XtaskCommandError::Unavailable { command, reason })` for known-but-intentionally-unavailable routes that must return the exact error variant rather than a placeholder status.
- Strategy: generate `XtaskCommand` values covering all 20 required family variants plus legacy variants, paired with `XtaskEnvironment` models with deterministic paths (no filesystem I/O). For each combination, assert the result is equal across two invocations and that the status/error classification matches the contracted domain for that command.
- Anti-invariant: no generated command may route to success unless a real tool executed; no two distinct `XtaskCommand` variants may produce equal `Ok(StructuredStatus)` values for different command names; `route_command` must never return `Ok` for a command that the contract requires to return `Err(Unavailable { ... })`.
- Concrete assertions: for `XtaskCommand::Required(CommandFamily::Perf)` with `CommandFamily::Perf` in `env.unavailable_families`, `route_command` returns exactly `Err(XtaskCommandError::Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })`. For `XtaskCommand::Required(CommandFamily::AiContext)` with `CommandFamily::AiContext` absent from `env.unavailable_families`, two invocations return identical `Ok(StructuredStatus { command: "ai-context", status: "deferred", message: "ai-context automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for ai-context engine integration"] })`.

### Proptest: dependency boundary rejects forbidden edges

- Invariant: any runtime core manifest containing a forbidden dependency edge fails with `DependencyBoundaryViolation` naming the exact crate and dependency.
- Strategy: generate `WorkspaceManifest` models for `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc` with allowed dependency sets plus optional forbidden edge from `{xtask, clap, serde_json, serde_yaml, reqwest, hyper, toml, serde-saphyr}`.
- Anti-invariant: any forbidden edge in runtime crates must fail; the same edge in `xtask` must not fail this runtime-boundary check.

## 5. Fuzz Targets

### Fuzz Target: `xtask_parse_argv_hostile`

- Input type: arbitrary bytes converted to a bounded argv vector through lossy and platform-valid `OsString` strategies.
- Risk: panic, OOM, unknown token treated as success, wrong-case accepted, delimiter/path traversal token accepted, non-termination.
- Corpus seeds: `not-a-real-command`, `AI-CONTEXT`, `ai--context`, `--ai-context`, `ai_context`, `ai.context`, `../ai-context`, `ai-context\0tail` where platform permits, empty string, whitespace, `--`, `---`, very long 4 KiB token.
- Expected classification: valid exact known names route correctly; unknown hostile names return `UnknownCommand`; no panic; no prompt.

### Fuzz Target: `xtask_parse_options_hostile`

- Input type: arbitrary argv token sequences after a valid required family.
- Risk: missing value swallowed, duplicate flags accepted ambiguously, invalid format/path/bead accepted, Clap parser panic, silent success.
- Corpus seeds: `ai-context --bead`, `ai-context --bead ""`, `test-plan --format xml`, `diff --baseline`, `diff --baseline --target`, `repro --path ../../../etc/passwd`, `review --mode \n`, duplicate `--format json --format kv`, delimiter `-- --bead vb-kkvb`.
- Expected classification: missing values return `MissingRequiredInput`; invalid present values return `InvalidInput`; unknown flags return typed parser diagnostic; no success unless all required inputs are valid.

### Fuzz Target: `structured_status_render_hostile`

- Input type: arbitrary `StructuredStatus` model values and text fields with bounded lengths.
- Risk: invalid schema emitted, field injection, malformed structured output, panic on control characters, empty next steps accepted.
- Corpus seeds: command `fuzz`, status `deferred`, empty message, empty next steps, message containing newline, quotes, braces, YAML/JSON-like snippets, 64 KiB text.
- Expected classification: valid statuses render parseable structured output; invalid model values fail with `OutputRenderFailed` or construction validation error; required fields remain intact.

## 6. Kani Harnesses

### Kani Harness: bounded known/unknown command classification

- Property: for bounded command-token enum `{20 required, legacy, help, version, unknown}`, parser model returns known typed route for required/legacy/help/version and `UnknownCommand` for unknown.
- Bound: finite enum of all required families plus at least 8 unknown representative tokens.
- Rationale: proves fail-closed route lookup completeness independent of random generation.

### Kani Harness: pairwise distinct route variants

- Property: any two different required family enum values map to different `XtaskCommand` variants.
- Bound: all 20 x 20 pairs.
- Rationale: command aliasing would route agents to the wrong quality tool.

### Kani Harness: route determinism

- Property: same bounded argv model input yields identical route result on repeated lookup.
- Bound: command token plus up to 4 option/value tokens from bounded token enum.
- Rationale: deterministic automation shell is a core invariant.

### Kani Harness: registry uniqueness

- Property: validation accepts all unique bounded registries and rejects any registry with duplicate normalized names.
- Bound: registry length 0..=20 with bounded command-name IDs.
- Rationale: finite set proof is stronger than examples for duplicate/shadow bugs.

### Kani Harness: structured status schema completeness

- Property: all constructed success/deferred statuses contain required fields and non-empty `next_steps`.
- Bound: all command families x bounded status kinds x bounded message/next-step presence flags.
- Rationale: schema is a stable automation contract.

### Kani Harness: placeholder status excludes success

- Property: `placeholder_status_model(family, reason).status != success` for all families/reasons.
- Bound: all 20 command families x all deferred reasons.
- Rationale: prevents false-pass quality automation.

### Kani Harness: dependency boundary finite model

- Property: any forbidden edge in `{vb_core,vb_runtime,vb_storage,vb_ipc}` produces `DependencyBoundaryViolation`; no forbidden edge produces success.
- Bound: 4 runtime crates x bounded dependency enum containing allowed and forbidden dependencies.
- Rationale: dependency isolation is a release-critical invariant.

## 7. Mutation Testing Checkpoints

Mutation command target: `moon run :verify-deep` must run `cargo mutants` or an equivalent mutation profile for the `xtask` package and any supporting route/schema crate. Minimum kill rate: **>=90%** overall, **100% kill** for mutants in command-family registry, parser classification, error taxonomy, structured status required fields, placeholder/deferred status, and runtime dependency boundary.

Critical mutants and killing scenarios:

- Remove `ai-context` from required registry -> killed by `required_command_families_returns_exact_required_family_set_once` and help-output scenario.
- Remove any other required family -> killed by exact family set golden and CLI help exact-once test.
- Duplicate `ai-context` registry row -> killed by duplicate registry validation and exact count assertions.
- Change `ai-context` route to `ai-plan` variant -> killed by distinct typed variant tests and Kani pairwise proof.
- Accept uppercase command names -> killed by wrong-case rejection scenario and hostile argv fuzz corpus.
- Accept `ai--context` or `ai_context` -> killed by exact-spelling scenario and fuzz corpus.
- Convert unknown command to placeholder success -> killed by unknown CLI/parser scenarios and fuzz fail-closed target.
- Remove missing-input branch -> killed by missing required input integration, proptest, fuzz corpus.
- Convert invalid present input to missing input -> killed by `InvalidInput` exact variant scenarios.
- Drop `command` from rendered status -> killed by representative status schema and all-families schema proptest.
- Drop `status` from rendered status -> killed by schema tests and renderer exact-output test.
- Drop `message` from rendered status -> killed by schema tests.
- Drop `next_steps` from rendered status -> killed by schema tests and status proptest.
- Change placeholder `deferred` to `success` -> killed by placeholder-not-fake-success scenarios.
- Swallow renderer failure -> killed by `OutputRenderFailed` failure-injection scenario.
- Ignore forbidden `clap` edge in `vb_core` -> killed by dependency boundary synthetic manifest test.
- Ignore forbidden `xtask` edge in `vb_runtime` -> killed by dependency boundary synthetic manifest test.
- Allow `unsafe` or `unwrap` in production xtask source -> killed by static forbidden scan gate.
- Introduce stdin prompt source -> killed by non-interactive static scan and timeout integration tests.
- Remove legacy command branch -> killed by legacy regression scenarios.

Mutation report acceptance:

- `mutation-report.md` records total mutants, killed mutants, timeout mutants, unviable mutants, and survivors.
- Any survivor in registry/parser/status/dependency-boundary code blocks release even if global kill rate is >=90%.
- Surviving equivalent mutants must be documented with reviewer-approved rationale.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| required family happy path | exact `ai-context` | `XtaskCommand::Required(CommandFamily::AiContext)` | unit/integration |
| all required families | exact 20 names | pairwise distinct required variants | unit/proptest/Kani |
| legacy happy path | `ui-snapshot --all` etc. | corresponding legacy route and documented output | integration |
| help global | `xtask --help` | exit 0 and all 20 families exactly once | e2e/integration |
| help command | `xtask ai-context --help` | exit 0 and command help for `ai-context` | integration |
| version | `xtask --version` | exact package/version metadata | integration |
| unknown command | `not-a-real-command` | `Err(UnknownCommand { command: "not-a-real-command" })`; CLI exit code `2` | integration/fuzz |
| wrong case | `AI-CONTEXT` | `Err(UnknownCommand { command: "AI-CONTEXT" })` | unit/fuzz |
| extra dash | `ai--context` | `Err(UnknownCommand { command: "ai--context" })` | unit/fuzz |
| delimiter token | `--`, `---`, `../ai-context` | `Err(UnknownCommand { command: supplied_token })` for each supplied token | fuzz |
| empty argv after binary | `xtask` with no subcommand | `Err(MissingRequiredInput { command: "xtask", input: "command" })` | integration/fuzz |
| missing option value | `ai-context --bead` | `Err(MissingRequiredInput { command: "ai-context", input: "bead" })` | integration/fuzz/mutation |
| invalid bead id | `ai-context --bead ""` | `Err(InvalidInput { command: "ai-context", input: "bead", reason: "bead id must not be empty" })` | unit/proptest |
| invalid output format | `test-plan --format xml` | `Err(InvalidInput { command: "test-plan", input: "format", reason: "unsupported output format: xml" })` | unit/proptest |
| duplicate flags | `test-plan --format jsonl --format json` | `Err(InvalidInput { command: "test-plan", input: "format", reason: "duplicate option: format" })` | integration/fuzz |
| unique registry | exact required specs | `ValidatedCommandRegistry` length 20 | unit/Kani |
| duplicate registry | duplicate `ai-context` | `Err(InternalInvariantViolation { invariant: "duplicate command family: ai-context" })` | unit/proptest/Kani |
| schema drift registry | missing `next_steps` metadata | `Err(InternalInvariantViolation { invariant: "structured status schema drift: missing next_steps" })` | unit/proptest |
| deterministic parse | any generated argv | first result equals second result exactly | proptest/Kani |
| placeholder | `perf` deferred | `StructuredStatus { command: "perf", status: "deferred", message: "perf automation deferred: implementation is outside bead vb-kkvb", next_steps: ["open follow-up bead for perf engine integration"] }` | unit/integration |
| unavailable error | `route_command(perf)` with `perf` in unavailable set | `Err(Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })` | unit/integration/proptest |
| placeholder false success | all deferred families | status not in `{ok, success, passed, executed}` | unit/proptest/mutation |
| render status success | valid status model | parseable structured text with exact field values | unit |
| render write failure | injected broken pipe | `Err(OutputRenderFailed { command, reason: "broken pipe" })` | unit/Miri |
| all status schemas | all families | required fields present for each family | proptest/schema |
| runtime clean deps | real workspace manifests | no dependency-boundary errors; report names checked crates | static/integration |
| runtime forbidden dep | synthetic `vb_core -> clap` | `Err(DependencyBoundaryViolation { crate_name: "vb_core", dependency: "clap" })` | unit/proptest |
| no forbidden constructs | xtask source | no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in production paths | static |
| no prompt sources | xtask source + process timeout | no stdin/TTY/editor/network prompt; process terminates | static/integration/manual |
| full gauntlet | final workspace | `moon run :verify-fast`, `:verify-standard`, `:verify-deep`, `:verify-proof`, `:verify-all` evidence captured | e2e/release |

## Commands and Evidence Expectations

Downstream test-writer/implementer must add executable tests and verification evidence for these commands. This plan does not execute them.

- Fast lane: `moon run :verify-fast`
- Standard lane: `moon run :verify-standard`
- Deep lane: `moon run :verify-deep`
- Proof lane: `moon run :verify-proof`
- Full lane: `moon run :verify-all`
- Fuzz lane: `cargo bolero test xtask_parse_argv_hostile` or `cargo fuzz run xtask_parse_argv`
- Option fuzz lane: `cargo bolero test xtask_parse_options_hostile` or same fuzz target with option corpus
- Mutation lane: `cargo mutants -p xtask` or project-approved `moon run :verify-deep` wrapper
- Dependency lane: `cargo deny check` and `cargo tree -p vb_core -p vb_runtime -p vb_storage -p vb_ipc`
- Unsafe lane: `cargo geiger` or repository-approved unsafe source scan
- Manual QA transcript: help output, representative structured placeholder output, unknown command exit-code-2 diagnostic, non-interactive timeout proof

Required evidence artifacts:

- `formal-verification-report.md`
- `fuzz-report-vb-kkvb.md`
- `mutation-report.md`
- `dependency-boundary-report.md`
- `unsafe-scan-report.md`
- `noninteractive-report.md`
- `manual-qa-vb-kkvb.md`
- CLI transcript proving help includes all 20 required families exactly once
- CLI transcript proving representative placeholder command emits structured status with required fields
- CLI transcript proving unknown command exits with code `2` and actionable diagnostic

## Red Phase Expectations

The first downstream implementation state must write tests before production changes. Against the current workspace, the following must fail for the right reason:

- Help exact-family test fails because the current `xtask` help does not list all 20 required command families.
- Required command parser tests fail because `parse_xtask_command`, `XtaskCommand`, and `XtaskCommandError` are not yet exposed as contracted public API.
- Representative structured status test fails because required fields `command`, `status`, `message`, and `next_steps` are not yet emitted for the new families.
- Unknown command typed-error test fails because current Clap/anyhow behavior is not the contracted `UnknownCommand` diagnostic.
- Missing/invalid input typed-error tests fail until `MissingRequiredInput` and `InvalidInput` are implemented.
- Placeholder-not-fake-success tests fail until explicit `deferred`/`unavailable` statuses exist.
- Dependency-boundary tests fail until `WorkspaceManifest` model/checker or equivalent public boundary checker exists.
- Fuzz/property/Kani/mutation targets fail to build until harnesses are added.

Red phase is accepted only when failures demonstrate missing contracted behavior, not flaky environment setup or unbuildable unrelated code.

## Review-Ready Decisions

- Renderer format for executable tests: `OutputFormat::JsonLines`, compact JSON object, one trailing newline, field order `command`, `status`, `message`, `next_steps`.
- Deferred placeholder status for command families without deeper implementation: exact status value `deferred`, exact message pattern `"<command> automation deferred: implementation is outside bead vb-kkvb"`, exact next step `"open follow-up bead for <command> engine integration"`.
- Unavailable command error path: exact variant `XtaskCommandError::Unavailable { command, reason }`; `perf` canonical assertion is `Err(Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })`.
- CLI validation/unknown/unavailable failure exit semantics: process exit code exactly `2`; success/help exit code exactly `0`.
