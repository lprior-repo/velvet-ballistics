# Contract Specification: vb-ahfl

## Context

- Bead: `vb-ahfl`.
- Accepted State 3 scope artifact: UI model artifact schema bounds and CLI parity for `vb_ui_model`, `vb_ui_makepad`, and `velvet-ballastics` CLI JSON/JSONL surfaces, as recorded in `.beads/vb-ahfl/delivery-scope.jsonl`.
- `BLOCKER-SCOPE-001` repair decision: State 3 explicitly scopes this artifact stack to the State 2 delivery-scope UI artifact contract. The stale bead DB title `engine: End-to-end YAML to IR semantic evidence` is not a hidden requirement for this contract stack. If an owner/orchestrator rejects the State 2 delivery scope and chooses engine YAML-to-IR, the required action is regeneration of State 2/3/4/5 for engine lifecycle semantics, not approving or extending this UI-scope contract with implicit TLA+ debt.
- Authoritative repository rule: `/velvet-ballistics-MASTER.md` is the master contract; State 2 cites master lines 5316-5344 and 5765-5839 for UI artifacts.

## Domain Terms

- UI artifact: cold-path plain data emitted by CLI/runtime-facing surfaces and consumed by UI model/Makepad screens.
- Universal artifact metadata: `schema_version`, `kind`, `generated_at`, `source`, and `redaction_status`.
- Bounded collection: collection with statically enforced maximum length or explicit limit/cursor/truncation metadata before crossing UI/Makepad boundary.
- Canonicalization: deterministic projection used to compare CLI JSON/JSONL output with `vb_ui_model` artifacts while ignoring representation-only ordering or formatting differences.
- Redacted value: secret-sensitive value represented only by redaction status, taint, digest, and bounded summary; raw secret bytes/text never serialize.
- Cold-path UI model: UI data structures that must not depend on hot runtime core internals, Makepad rendering code, async runtimes, HTTP, or workflow execution mutation.

## Assumptions

- State 2 `delivery-scope.jsonl` is the accepted scope source for this State 3 artifact set; bead JSON title mismatch is explicitly scoped out unless the owner/orchestrator regenerates the delivery artifacts.
- Package and module targets must be confirmed by implementation/test/proof states before exact proof or test command closure; unknown formal targets are marked BLOCKED rather than invented.
- No production code, tests, or proof/model code is authored in State 3.

## Open Questions

- OQ-001 / BLOCKER-SCOPE-001: Resolved for this State 3 repair by explicit State 2 delivery-scope acceptance. Regeneration is required only if the owner/orchestrator chooses engine YAML-to-IR scope instead of UI artifact schema parity.
- OQ-002: What exact universal metadata type name should implementation expose in `vb_ui_model`?
- OQ-003: Which CLI commands are authoritative artifact emitters for each UI artifact kind?

## Preconditions

- PRE-001: State 4 and later states must consume this contract only as the UI artifact schema parity contract described in `.beads/vb-ahfl/delivery-scope.jsonl`; engine YAML-to-IR compile/admit/run/journal/replay semantics require regenerated State 2/3/4/5 artifacts and are not covered by this contract.
- PRE-002: Every UI artifact constructor or conversion entry point must receive or derive universal metadata: `schema_version`, `kind`, `generated_at`, `source`, and `redaction_status`.
- PRE-003: Every collection crossing into Makepad or CLI/UI parity comparison must have a known maximum, or explicit `limit`, `cursor`, and `truncated` semantics.
- PRE-004: CLI/UI parity comparison must operate on canonicalized typed artifacts, not ad hoc string formatting.
- PRE-005: Secret-sensitive fields must be classified before serialization.
- PRE-006: `vb_ui_model` must remain cold-path plain data and cannot require Makepad, runtime execution internals, async runtime, HTTP, YAML parsing, or workflow mutation to construct artifacts.

## Postconditions

- POST-001: All exported UI artifacts serialize with universal metadata present and semantically valid.
- POST-002: Workflow graph nodes expose `step_idx`, `step_id`, `kind`, `status`, `output_slot`, `taint`, `badges`, and `position`.
- POST-003: Workflow graph edges expose `from_step_idx`, `to_step_idx`, `edge_kind`, `condition_summary`, `is_failure_path`, `is_taint_path`, and `packet_state`.
- POST-004: Event rows expose `seq`, `timestamp`, `run_id`, `step_idx`, `event_kind`, `status`, `evidence_digest`, and `attempt`.
- POST-005: Oversized artifact collections are rejected or clipped with explicit truncation metadata; they never silently exceed UI bounds.
- POST-006: Secret-sensitive values serialize only redacted status, taint, digest, and bounded summary.
- POST-007: CLI-emitted artifacts and `vb_ui_model` artifacts compare equal after canonicalization for every artifact kind in scope.
- POST-008: Makepad-facing code consumes bounded typed UI model data without importing runtime core internals or parsing runtime source formats.

## Invariants

- INV-001: Schema version and artifact kind agree across CLI output and UI model representation.
- INV-002: Artifact canonicalization is deterministic for equal semantic inputs.
- INV-003: Collection bounds are enforced before render-facing model use.
- INV-004: Redaction is explicit and fail-closed: unknown secret sensitivity cannot serialize raw value.
- INV-005: Workflow graph identity is stable: `step_idx` and `step_id` remain associated across nodes, edges, events, and parity reports.
- INV-006: Event ordering is stable by `seq`; equal event streams canonicalize identically.
- INV-007: UI model remains cold-path and does not create Makepad/runtime/async/HTTP dependency cycles.

## Error Taxonomy

- UiArtifactError::ScopeConflict - State artifacts and bead JSON describe incompatible feature scopes.
- UiArtifactError::MissingUniversalMetadata - required metadata field is absent.
- UiArtifactError::SchemaVersionMismatch - CLI and UI artifacts use incompatible schema versions.
- UiArtifactError::KindMismatch - CLI and UI artifacts describe different artifact kinds.
- UiArtifactError::CollectionLimitExceeded - input exceeds a non-clipping bound.
- UiArtifactError::TruncationMetadataMissing - clipped data lacks explicit truncation metadata.
- UiArtifactError::RawSecretExposure - secret-sensitive value would serialize raw content.
- UiArtifactError::CanonicalizationMismatch - CLI and UI canonical forms differ.
- UiArtifactError::InvalidGraphReference - edge or event references an absent `step_idx` or inconsistent `step_id`.
- UiArtifactError::ColdPathBoundaryViolation - UI model introduces disallowed runtime, Makepad, async, HTTP, YAML parsing, or execution dependency.

## Contract Signatures

These signatures are contractual shapes only; State 3 does not implement them.

- `fn validate_scope_alignment(state2_scope: DeliveryScope, bead: BeadJson) -> Result<AcceptedScope, UiArtifactError>`
- `fn attach_universal_metadata<T>(artifact: T, metadata: UniversalArtifactMetadata) -> Result<UiArtifact<T>, UiArtifactError>`
- `fn bound_collection<T>(items: Vec<T>, policy: BoundsPolicy) -> Result<BoundedCollection<T>, UiArtifactError>`
- `fn canonicalize_cli_artifact(input: CliArtifactJson) -> Result<CanonicalUiArtifact, UiArtifactError>`
- `fn canonicalize_ui_artifact(input: UiArtifact) -> Result<CanonicalUiArtifact, UiArtifactError>`
- `fn compare_cli_ui_artifacts(cli: CliArtifactJson, ui: UiArtifact) -> Result<ParityMatch, UiArtifactError>`
- `fn redact_secret_value(input: SecretSensitiveValue) -> Result<RedactedValueView, UiArtifactError>`
- `fn validate_workflow_graph(graph: WorkflowGraphView) -> Result<ValidatedWorkflowGraphView, UiArtifactError>`

## Verus-Owned Clauses

- PRE-002, POST-001, INV-001: universal metadata completeness and schema/kind agreement.
- PRE-003, POST-005, INV-003: bounded collection length and truncation semantics.
- PRE-005, POST-006, INV-004: redaction projection excludes raw secret value.
- POST-002, POST-003, POST-004, INV-005, INV-006: graph/event structural references and ordering.
- POST-007, INV-002: canonicalization determinism and parity equality relation.

## TLA+-Owned Clauses

- None for the accepted UI artifact schema parity scope. INV-007 and POST-008 are static dependency/import boundary clauses, not a lifecycle protocol. If the owner/orchestrator changes scope to engine YAML-to-IR or introduces asynchronous CLI-to-UI lifecycle behavior, regenerate TLA+ obligations before implementation consumption.

## Theorem-Owned Clauses

- None required at contract time. Verus plus proptest/Kani is sufficient for bounded collection, redaction projection, graph reference, and deterministic canonicalization properties.

## Non-goals

- Implementing production code, tests, proof code, TLA+ modules, Lean modules, or generated artifacts.
- UI rendering behavior, layout, animation, screenshots, or Makepad widget implementation.
- Engine YAML-to-IR semantic evidence, compile/admit/run/journal/replay lifecycle behavior, and scheduler/protocol liveness. Those require regenerated State 2/3/4/5 artifacts if selected by the owner/orchestrator.
- Performance, vectorization, or zero-cost abstraction claims for this scope.

## Contract-Time Waiver Policy

- Required production-bound obligations are no longer contract-time waivers. They name concrete production modules/types where already present and exact downstream verifier/test commands that the owning state must make real with production-bound harnesses or report as blocking target-discovery failures.
- `VERUS-META-001`, `VERUS-BOUNDS-001`, `VERUS-REDACT-001`, `VERUS-GRAPH-001`, `KANI-CANON-001`, `PROP-PARITY-001`, `API-COMPAT-001`, `MUT-ERR-001`, and `FUZZ-REDACT-001` cannot be closed by abstract-only local models, missing-target waivers, or not-run evidence.
- Static boundary evidence must inspect dependency/import declarations and ignore comments; the repaired `STATIC-BOUNDARY-001` command replaces the State 6 overbroad text scan.

## State 3 Repair Completion Evidence

- Startup skill files read: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md`; both expose rust-contract v2.6.0 principles including TLA+ default for temporal behavior, Verus-first Rust core obligations, scope-aware high assurance, no invented formal targets, executable proof-obligation JSONL, and no implementation/proof/test code in this skill. No conflict observed; `.agents` would win on conflict.
- Isolation verified with `pwd -P` in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-ahfl`.
- Static boundary repair command was run in the isolated workspace and produced `PASS`/no matches for dependency/import violations while ignoring the comment that caused State 6 rejection.
