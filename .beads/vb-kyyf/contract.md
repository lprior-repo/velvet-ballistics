# vb-kyyf Contract: Cross-Run Determinism and Reproducibility BDD

## Context
- Bead: `vb-kyyf` - `bdd: Cross-run determinism and reproducibility acceptance scenarios`.
- Scope: release-gating executable Given/When/Then contracts for deterministic run/replay/recovery/generated-vs-IR behavior through public surfaces only.
- Cited startup rules: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md` both require contract-first design, TLA+ by default for temporal behavior, Verus for Rust-local pure/core logic, valid JSONL proof obligations, and no implementation/test/proof code.
- Master clauses read: lines 23, 49, 235-240, 790-797, 1073-1107, 1256-1265, 1276-1285, 1318-1344, 1483-1496, 1514.
- Public surfaces from delivery scope: `FjallJournal::events_for_run`, `recover_full_journal`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `verify_digests`, `Runtime::submit_compiled_with_inputs`, `Runtime::inspect_run`, `compare_generated_to_ir`, `validate_generated_subset`, and CLI `velvet-ballastics replay/events/inspect`.

## Assumptions
- Scenario implementation will live in `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` or an equivalent public-surface BDD test target.
- Public CLI support may be incomplete; if a CLI command is unavailable, the scenario must fail with a typed gap diagnostic linked to this bead rather than silently falling back to a private helper.
- Generated mode remains subset-only per master lines 1073 and 1103-1106; unsupported IR families must fail closed with stable diagnostics.
- Allowed nondeterministic fields are only explicitly normalized cold metadata: temp path, process id, wall-clock timestamp, and generated run id. All semantic evidence remains exact.

## Open Questions
- Exact CLI binary test harness shape is not confirmed by State 2; downstream test state must discover existing CLI integration conventions before writing tests.
- Exact executable semantic parity API for generated Rust versus IR is not complete; downstream implementation must not claim full parity from `compare_generated_to_ir` source-pattern checks alone.

## Domain Terms
- Cross-run determinism: same accepted artifact, same input, same durability profile, and same deterministic action completions produce the same normalized terminal observation across independent runs.
- Reproducibility: persisted evidence can be dropped/reopened and replayed repeatedly with the same normalized report.
- Replay safety: replay does not re-execute non-replay-safe external side effects and produces stable typed blocked/error states.
- Semantic parity: generated mode and IR mode produce identical terminal result, taint, journal event signature, suspension state, and typed errors for supported workflows.
- Public surface: CLI, direct Rust API, binary IPC, or documented storage/runtime/codegen public functions; private helper-only proof is not release evidence.

## Preconditions
- PRE-001: Every scenario fixture must use an accepted compiled artifact or a generated-mode-supported workflow; raw YAML cannot be the runtime truth for replay/recovery assertions.
- PRE-002: Every scenario must create isolated stores/runs and cannot share mutable state across cases.
- PRE-003: Every scenario must drive behavior through a documented public surface from the delivery scope.
- PRE-004: Every comparison must define a normalization function for allowed cold nondeterminism and must reject unlisted differences.
- PRE-005: External action replay scenarios must declare action replay class: `DeterministicPure`, `IdempotentExternal`, or `AtLeastOnceExternal`.

## Postconditions
- POST-001: Re-running the same accepted artifact/input in two isolated stores yields identical normalized terminal result, taint, event kind/sequence signature, digest status, and typed diagnostics.
- POST-002: Reopening a persisted store and replaying the same run repeatedly yields identical normalized `events_for_run`, recovery summary/frame seed, and CLI replay/events/inspect report.
- POST-003: Replay after non-idempotent or at-least-once scheduled external action boundaries never re-executes the side effect and returns the same typed blocked/replay-policy outcome on every recovery attempt.
- POST-004: Corrupt, gapped, duplicate, or digest-mismatched records fail deterministically with the same typed storage/replay error on repeated reads.
- POST-005: For generated-mode-supported workflows, IR and generated mode yield identical normalized terminal result, taint, journal signature, suspension, and typed errors; unsupported IR families fail closed with stable diagnostics.
- POST-006: BDD runner output identifies scenario id, Given/When/Then text, public surface, exact mismatch, and evidence artifact path for every pass/fail result.

## Invariants
- INV-001: Public-surface-only invariant: no scenario counts as release evidence if the primary behavior path uses private helpers.
- INV-002: Normalization invariant: only path/run-id/timestamp/process metadata may be normalized; semantic fields must compare exactly.
- INV-003: Journal determinism invariant: per-run sequence numbers are contiguous and monotonic; event kind/order/significant payload digest are stable across repeated observations.
- INV-004: Digest binding invariant: replay checks workflow source digest, compiled workflow digest, action ABI digest, and policy digest; mismatch is a typed failure and not silent continuation.
- INV-005: Replay side-effect invariant: recovery never re-executes non-replay-safe external side effects after a scheduled journal boundary.
- INV-006: Generated parity invariant: supported generated mode is observationally equivalent to IR mode for result, taint, journal events, suspensions, and errors.
- INV-007: Evidence invariant: every scenario emits traceable evidence with bead id `vb-kyyf` and scenario id.

## Error Taxonomy
- ERR-001 `ScenarioSurfaceUnavailable`: required public surface is missing or not invokable.
- ERR-002 `ScenarioUsesPrivateSurface`: scenario attempts to certify behavior through a private helper.
- ERR-003 `NondeterministicObservation`: normalized observations differ across runs/replays.
- ERR-004 `ReplayDigestMismatch`: source/compiled/action ABI/policy digest mismatch is detected.
- ERR-005 `ReplaySequenceViolation`: corrupt, gapped, duplicate, or out-of-order journal evidence is detected.
- ERR-006 `ReplayPolicyBlocked`: replay would re-execute a non-replay-safe side effect and must block with typed outcome.
- ERR-007 `GeneratedIrDivergence`: generated and IR observations differ for a supported workflow.
- ERR-008 `UnsupportedGeneratedSubset`: workflow contains an IR family not yet accepted for generated-mode parity.
- ERR-009 `EvidenceArtifactMissing`: scenario pass/fail cannot be traced to an evidence artifact.

## Contract Signatures
- `fn normalize_observation(raw: PublicObservation) -> Result<NormalizedObservation, DeterminismContractError>`
- `fn compare_cross_run(left: NormalizedObservation, right: NormalizedObservation) -> Result<(), DeterminismContractError>`
- `fn compare_replay(first: NormalizedReplayReport, second: NormalizedReplayReport) -> Result<(), DeterminismContractError>`
- `fn compare_generated_ir(ir: NormalizedObservation, generated: NormalizedObservation) -> Result<(), DeterminismContractError>`
- `fn certify_scenario_evidence(row: ScenarioEvidenceRow) -> Result<(), DeterminismContractError>`

## Executable Given/When/Then Scenarios

### BDD-KYYF-001: isolated identical runs are deterministic
Given the same accepted artifact, same binary input, same durability profile, and two fresh isolated stores.
When the workflow is run through public runtime or CLI surfaces and normalized observations are collected.
Then terminal result, taint, event kind/order/significant payload digest, digest status, and typed diagnostics are identical except allowed cold metadata.

### BDD-KYYF-002: persisted replay is reproducible after reopen
Given a strict or journaled persisted run with durable evidence.
When the store is dropped/reopened and `events_for_run`, recovery summary/frame seed, and CLI `replay/events/inspect` are executed twice.
Then both observations are identical and sequence numbers remain contiguous and monotonic.

### BDD-KYYF-003: non-replay-safe side effects are not re-executed
Given a run journal containing a scheduled non-idempotent or at-least-once external action boundary.
When recovery/replay is attempted repeatedly.
Then no side effect dispatch is repeated and every attempt returns the same typed blocked/replay-policy outcome.

### BDD-KYYF-004: corrupt replay evidence fails deterministically
Given journal/snapshot records with corruption, sequence gaps, duplicates, or digest mismatch.
When replay/recovery is invoked twice through public storage/runtime/CLI surfaces.
Then both attempts fail with the same typed storage/replay error and no silent continuation occurs.

### BDD-KYYF-005: generated mode and IR mode are observationally equivalent for supported workflows
Given a workflow accepted by `validate_generated_subset` and equivalent IR/generator fixtures.
When IR mode and generated mode execute and replay from durable evidence.
Then terminal result, taint, journal signature, suspension state, and typed errors are identical.

### BDD-KYYF-006: unsupported generated subset fails closed
Given a workflow containing a generated-mode-unsupported IR family.
When generated parity certification is requested.
Then the scenario returns `UnsupportedGeneratedSubset` or the existing typed generated-subset rejection and does not count as parity evidence.

### BDD-KYYF-007: evidence artifacts are traceable and strong
Given the release acceptance suite runs the `vb-kyyf` group.
When any scenario passes or fails.
Then runner output includes scenario id, Given/When/Then, public surface, normalized observation digest or mismatch, and evidence artifact path.

## Verus-Owned Clauses
- PRE-004, INV-002: normalization admits only allowed metadata differences and rejects semantic differences.
- INV-003: journal sequence signature comparison preserves monotonic/contiguous constraints over normalized observations.
- POST-001, POST-002, POST-005: pure comparison functions are reflexive/symmetric for equal normalized observations and reject any semantic delta.

## TLA+-Owned Clauses
- INV-003, INV-004, INV-005, POST-002, POST-003, POST-004: replay/recovery state-over-time behavior, side-effect non-reexecution, deterministic terminal/error convergence, and deadlock-free blocked/error terminal outcomes.

## Theorem-Owned Clauses
- None required at contract time. Verus owns the small Rust-local normalization/comparison kernel; TLA+ owns temporal replay/recovery. Lean/Aeneas/Hax is explicitly non-applicable unless proof review finds a tiny algebraic normalization theorem beyond Verus.

## Non-goals
- No production implementation in State 3.
- No test code or proof code in State 3.
- No performance or generated-code speed claim; this bead is correctness/evidence focused.
- No claim of full generated final-IR parity beyond generated-subset workflows accepted by current public validation.
