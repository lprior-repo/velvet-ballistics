# Contract Specification: vb-kkvb

## Context
- Feature: Expand the existing first-party `xtask` command center shell with stable typed subcommand routing and structured non-interactive output.
- Existing surface: workspace member `xtask` with Clap-based routing, UI snapshot/token commands, and initial gate commands.
- Required command families: `ai-context`, `ai-plan`, `ai-check`, `ai-evidence`, `invariants`, `scans`, `cert-check`, `perf`, `replay`, `crash`, `diff`, `mutants`, `loom`, `kani`, `fuzz`, `prop`, `repro`, `test-plan`, `review`, `why-failed`.
- Domain terms:
  - Command family: stable top-level xtask subcommand name exposed to agents and humans.
  - Typed route: enum-backed command variant, not stringly dynamic dispatch.
  - Structured status: machine-readable, non-interactive command result with stable fields.
  - Placeholder status: intentionally non-operational status for command families whose deeper implementation is not part of this bead.
  - Runtime core crates: `crates/vb_core`, `crates/vb_runtime`, `crates/vb_storage`, `crates/vb_ipc`.

## Assumptions
- This state writes planning artifacts only; production code, tests, proof code, and harness code are downstream work.
- `xtask` may use CLI-only dependencies already suitable for tooling, but must not add dependencies to runtime core/storage/ipc crates.
- Structured output may be emitted by `xtask`; the repository ban on YAML/JSON/HTTP applies to runtime core paths, not first-party tooling shells.
- All fallible operations in the design must be modeled as `Result<T, XtaskCommandError>`.

## Open Questions
- OQ-001: Should structured output be line-delimited JSON, pretty JSON, or a stable key-value format? Contract requires stable fields independent of the chosen renderer.
- OQ-002: Which subcommands eventually execute real tools versus placeholder status only? Contract requires explicit placeholder status until real execution is separately contracted.

## Preconditions
- PRE-001: The caller provides arguments through a non-interactive argv stream; the command must never prompt for stdin confirmation.
- PRE-002: The first non-binary argument is either a known top-level command family, an existing legacy command, or a help/version request.
- PRE-003: Option values that denote paths, bead IDs, modes, or formats are present when required by the selected command variant.
- PRE-004: The workspace manifest keeps `xtask` as a tooling workspace member and keeps runtime core crates dependency-isolated.
- PRE-005: Route construction receives only owned or borrowed immutable input; it must not require global mutable state.

## Postconditions
- POST-001: Help output lists every required command family exactly once with the stable public spelling.
- POST-002: Known command families route to distinct typed enum variants; no required family aliases to the wrong handler.
- POST-003: A representative command invocation returns structured non-interactive status containing at least `command`, `status`, `message`, and `next_steps` fields.
- POST-004: Placeholder command families report a deliberate placeholder/deferred status, not silent success pretending that deeper automation ran.
- POST-005: Unknown command families fail closed with a non-zero exit status and actionable diagnostic.
- POST-006: Missing or invalid required inputs fail with a typed validation error and actionable diagnostic.
- POST-007: Existing legacy xtask commands keep their documented routing behavior unless explicitly changed by a separate bead.
- POST-008: The implementation adds no dependencies from `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc` to `xtask` or other tooling-only crates.

## Invariants
- INV-001: The top-level route table is deterministic: the same argv sequence always yields the same route or the same error.
- INV-002: Command family names are unique after normalization; no duplicate or shadowed public spelling exists.
- INV-003: Public command spelling is stable kebab-case and exactly matches the required command-family list.
- INV-004: Routing is type-directed through enums or equivalent closed variants; handlers must not be selected through unchecked free-form string matching after validation.
- INV-005: Every fallible parse, validation, output, and execution boundary returns `Result<T, XtaskCommandError>` or an equivalent error type; no `panic`, `unwrap`, `expect`, `todo`, `unimplemented`, or `dbg` is allowed.
- INV-006: Structured output schema is stable across command families: all success/deferred statuses expose the same required field names.
- INV-007: Non-interactive behavior is preserved: no command waits for stdin, TTY confirmation, network input, or editor interaction.
- INV-008: Runtime core crates do not gain YAML, JSON, HTTP, `xtask`, Clap, or tooling-only dependencies because of this change.
- INV-009: No unsafe code is introduced.

## Error Taxonomy
- ERR-001: `XtaskCommandError::UnknownCommand { command }` when the requested top-level command family is not recognized.
- ERR-002: `XtaskCommandError::MissingRequiredInput { command, input }` when a selected command lacks a required option or positional input.
- ERR-003: `XtaskCommandError::InvalidInput { command, input, reason }` when supplied input is syntactically present but invalid.
- ERR-004: `XtaskCommandError::OutputRenderFailed { command, reason }` when structured status cannot be rendered or written.
- ERR-005: `XtaskCommandError::DependencyBoundaryViolation { crate_name, dependency }` when the change introduces forbidden runtime dependency coupling.
- ERR-006: `XtaskCommandError::Unavailable { command, reason }` when a known command family is intentionally deferred or unavailable and must report that fact explicitly.
- ERR-007: `XtaskCommandError::InternalInvariantViolation { invariant }` when validated construction detects duplicate commands, schema drift, or impossible routing state.

## Contract Signatures
- `pub fn parse_xtask_command(args: impl IntoIterator<Item = OsString>) -> Result<XtaskCommand, XtaskCommandError>`
- `pub fn required_command_families() -> &'static [CommandFamilySpec]`
- `pub fn validate_command_registry(specs: &[CommandFamilySpec]) -> Result<ValidatedCommandRegistry, XtaskCommandError>`
- `pub fn route_command(command: XtaskCommand, env: &XtaskEnvironment) -> Result<StructuredStatus, XtaskCommandError>`
- `pub fn placeholder_status(command: CommandFamily, reason: DeferredReason) -> Result<StructuredStatus, XtaskCommandError>`
- `pub fn render_structured_status(status: &StructuredStatus, format: OutputFormat) -> Result<String, XtaskCommandError>`
- `pub fn assert_runtime_dependency_boundary(manifest: &WorkspaceManifest) -> Result<(), XtaskCommandError>`

## Lean-Owned Clauses
- INV-001, INV-002, INV-003, INV-004, INV-006, POST-001, POST-002, POST-003, POST-004 are pure deterministic routing/schema clauses projected in `lean-contract.md`.
- CLI parser hostile-input behavior is not Lean-owned because it crosses the Clap/argv shell boundary; it requires executable Bolero or cargo-fuzz evidence in `verification-layers.md` and `proof-obligations.jsonl`.

## Non-goals
- Implementing the underlying `loom`, `kani`, `fuzz`, `perf`, `mutants`, or replay engines.
- Proving Clap internals, filesystem I/O, stdout/stderr writes, process exit codes, or Moon task behavior in Lean.
- Changing runtime core behavior.
- Claiming performance improvement.

## Independent Review Requirement
- These artifacts are not self-approved. Downstream work must wait for `contract-verification-review.md` with `STATUS: APPROVED` from an independent reviewer.
