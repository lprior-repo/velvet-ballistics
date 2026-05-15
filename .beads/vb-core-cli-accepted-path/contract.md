# Contract Specification: vb-core-cli-accepted-path

## Context

- Bead: `vb-core-cli-accepted-path` / `cli/runtime: Route YAML run and submit through accepted artifacts`.
- Source of truth read: State 2 artifacts `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, `STATE.md`; bead JSON from `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-cli-accepted-path --json`; master contract `velvet-ballistics-MASTER.md`.
- Feature: strict CLI and runtime paths must route YAML-origin runs and submits through persisted accepted artifacts before admission or acknowledgement.
- Release critical: yes.

## Domain Terms

- YAML source: cold authoring input only; never interpreted by runtime.
- `CompiledWorkflow` / `WorkflowParts`: compiled IR values; not sufficient proof for strict runtime admission by themselves.
- `AcceptedArtifact`: storage envelope containing compiled IR, digest, verification proof, accepted sequence, and capability evidence.
- Accepted run boundary: durable boundary containing workflow source, accepted artifact, run header, `RunAccepted`, and indexes required by storage/runtime.
- Strict/journaled policy: runtime policy requiring accepted-artifact admission.
- Relaxed policy: development/test path that may use non-storage artifact stores only when explicitly outside strict/journaled production admission.

## Assumptions

- Dependency beads `vb-core-accepted-artifact-format`, `vb-core-atomic-admission`, and `vb-core-storage-artifact-store` are in progress; this contract treats their unresolved schema/atomicity details as blocking dependencies, not optional behavior.
- The accepted artifact format must resolve the current gate-count mismatch noted in State 2: storage creates `gate_count=2`, runtime requires `REQUIRED_GATE_COUNT=15`.
- `cmd_submit` is a durable acceptance operation: it must not emit or persist `RunAccepted` unless the accepted artifact and run header are already durably bound.
- Legacy or helper paths such as `crates/velvet_ballastics/src/run.rs` are in scope if reachable from strict CLI modes.

## Open Questions

- OQ-001: Final accepted artifact v1 verification proof schema and required gate count are owned by `vb-core-accepted-artifact-format`.
- OQ-002: Exact atomic Fjall batch API shape for source/artifact/header/event/index persistence is owned by `vb-core-atomic-admission`.
- OQ-003: Exact runtime constructor name for storage-backed artifact-store injection is owned by implementation; it must refine the contract, not bypass it.

## Preconditions

- PRE-001: CLI strict `run` and `submit` inputs MUST be parsed by the strict YAML parser and compiled to valid `CompiledWorkflow` before any storage acceptance attempt.
- PRE-002: Durable strict/journaled modes MUST have an opened storage-backed journal and artifact store before runtime construction or durable acknowledgement.
- PRE-003: An accepted artifact MUST be produced from the compiled workflow using the repository's single accepted artifact format before strict runtime admission.
- PRE-004: A run header MUST name the compiled artifact digest that will be used by runtime admission.
- PRE-005: Direct compiled input (`WorkflowParts`, postcard IR, or `CompiledWorkflow`) MUST declare whether it is relaxed-only or strict; strict direct input requires a persisted accepted artifact envelope before admission.

## Postconditions

- POST-001: Strict YAML `run` MUST persist workflow source and accepted artifact envelope before runtime admission begins.
- POST-002: Strict `submit` MUST persist workflow source, accepted artifact envelope, run header, `RunAccepted`, and required indexes as one accepted-run durability boundary, or persist no acknowledgement.
- POST-003: Runtime strict/journaled admission MUST load the accepted artifact by digest from storage-backed `AcceptedArtifactStore` behavior, not from `AlwaysPresentArtifactStore`.
- POST-004: Missing, malformed, digest-mismatched, proof-invalid, gate-count-invalid, or capability-invalid artifacts MUST reject before run state insertion and before `RunAccepted` acknowledgement.
- POST-005: Relaxed/non-durable paths MAY keep raw compiled execution only if the policy is not strict/journaled and diagnostics identify it as non-production relaxed admission.
- POST-006: The CLI operator evidence for strict run/submit MUST expose enough durable identifiers to correlate source digest, artifact digest, run id/header, and journal events.

## Invariants

- INV-001: In strict/journaled policy, no run is admitted unless there exists exactly one persisted accepted artifact envelope for the compiled digest.
- INV-002: Digest binding is total: source record binding, accepted artifact digest, run header compiled digest, `RunAccepted.workflow`, and runtime admission digest all refer to the same compiled artifact identity or the operation rejects.
- INV-003: Raw `WorkflowParts` and raw `CompiledWorkflow` are never sufficient strict admission witnesses.
- INV-004: `AlwaysPresentArtifactStore` is test-only or relaxed-only and cannot satisfy production strict/journaled CLI runtime construction.
- INV-005: Accepted-run persistence is fail-closed: a partial source/artifact/header/event/index write cannot be observed as an accepted run.
- INV-006: Runtime execution never reparses YAML and never uses JSON/HTTP/text command routing in core runtime paths.
- INV-007: Every fallible operation in this path returns typed `Result<T, Error>` and never panics, unwraps, expects, or ignores `Result`.

## Temporal Clauses

- TLA-001: Strict `run` ordering is `ParseYaml -> Compile -> PersistSource -> PersistAcceptedArtifact -> RuntimeAdmissionByDigest -> InsertRunState -> ExecuteOrSuspend`.
- TLA-002: Strict `submit` ordering is `ParseYaml -> Compile -> PersistAcceptedRunBoundary -> AcknowledgeRunAccepted`; acknowledgement cannot precede the atomic boundary.
- TLA-003: Failure before the accepted-run boundary eventually reaches terminal rejection without durable acknowledgement.

## Error Taxonomy

- ERR-001 `StrictAdmissionMissingArtifact`: strict/journaled runtime cannot load the accepted artifact for the requested digest.
- ERR-002 `MalformedAcceptedArtifact`: stored compiled IR record is not a valid accepted artifact envelope.
- ERR-003 `DigestMismatch`: source/artifact/header/event/runtime digests disagree.
- ERR-004 `InvalidVerificationProof`: accepted artifact proof flags, gate count, or capability evidence fail policy.
- ERR-005 `StorageAdmissionWriteFailed`: source/artifact/header/event/index persistence fails before durable acknowledgement.
- ERR-006 `PartialAcceptedRunRejected`: recovery observes an incomplete accepted-run boundary and refuses to treat it as accepted.
- ERR-007 `StrictRawCompiledBypassRejected`: raw `WorkflowParts` or raw `CompiledWorkflow` is presented to strict admission without a persisted accepted artifact.
- ERR-008 `StorageArtifactStoreUnavailable`: strict/journaled CLI cannot construct runtime with a storage-backed artifact store.

## Contract Signatures

These are contract shapes, not implementation requirements for exact names.

- `fn strict_cli_run_from_yaml(source: StrictYamlSource, durability: StrictDurability) -> Result<RunOutcome, CliAdmissionError>`
- `fn strict_cli_submit_from_yaml(source: StrictYamlSource, durability: StrictDurability) -> Result<AcceptedRunReceipt, CliAdmissionError>`
- `fn persist_accepted_artifact(compiled: CompiledWorkflow, source_digest: SourceDigest) -> Result<AcceptedArtifactReceipt, StorageAdmissionError>`
- `fn persist_accepted_run_boundary(source: WorkflowSourceRecord, artifact: AcceptedArtifact, header: RunHeaderRecord) -> Result<AcceptedRunReceipt, StorageAdmissionError>`
- `fn construct_strict_runtime(journal: DurableJournal, artifacts: StorageBackedArtifactStore) -> Result<Runtime, RuntimeAdmissionError>`
- `fn admit_by_artifact_digest(digest: WorkflowDigest, policy: RuntimePolicy) -> Result<RunAdmission, RuntimeAdmissionError>`

## Verus-Owned Clauses

- VERUS-001 covers INV-002 digest-binding refinement over source/artifact/header/event/admission identities.
- VERUS-002 covers INV-003/PRE-005 policy typing: raw compiled values cannot inhabit strict accepted-admission witness type.
- VERUS-003 covers ERR-001..ERR-004 decision totality for pure admission validation.

## TLA+-Owned Clauses

- TLA-001, TLA-002, TLA-003, INV-001, INV-005, POST-001, POST-002, POST-004.

## Theorem-Owned Clauses

- None at contract time. Verus should own Rust-local proof obligations; Lean/Aeneas/Hax are non-goals unless the accepted artifact format bead introduces a tiny theorem kernel beyond Verus.

## Non-goals

- No production code, tests, or proof code in this State 3 contract task.
- No performance claim beyond requiring no unmeasured regressions for release gates.
- No generated Rust/maxperf changes in this bead unless later dependency contracts expand scope.
