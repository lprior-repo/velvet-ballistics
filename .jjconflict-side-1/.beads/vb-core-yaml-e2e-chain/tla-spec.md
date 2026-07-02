# TLA+ Temporal Model Plan

## Boundary

- Temporal/workflow behavior: YAML-origin strict run lifecycle, accepted-artifact persistence/admission, strict durability before acknowledgement, events/inspect projection, restart/replay/recovery without YAML reparsing, and fail-closed corrupt/mismatch paths.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/tests: byte-level digest equality, postcard decode details, pure recovery summary predicates, parser correctness, and Rust error enum plumbing.
- External systems abstracted: Fjall storage as durable key/event store with success/failure actions; CLI as client actions; wall-clock and OS process restart as nondeterministic restart action.
- Non-applicability rationale: not applicable; this bead is lifecycle/protocol-heavy and requires temporal modeling.

## TLA+-Owned Clauses

- PRE-006 -> `YamlE2eChain::PersistBeforeAck`
- POST-001 -> `YamlE2eChain::SuccessfulRunHasEvidence`
- POST-003 -> `YamlE2eChain::RecoveryWithoutYamlParse`
- POST-005 -> `YamlE2eChain::RecoveredStateRefinesJournal`
- INV-003 -> `YamlE2eChain::StrictAdmissionOnlyAcceptedArtifact`
- INV-004 -> `YamlE2eChain::JournalPrefixDurable`
- INV-005 -> `YamlE2eChain::NoYamlParseAfterAdmission`
- INV-007 -> `YamlE2eChain::InspectEventsReflectJournal`

## Model Shape

- Module/model path: `verification/tla/YamlE2eChain.tla` with config `verification/tla/YamlE2eChain.cfg`.
- Variables:
  - `phase[run]`: one of `Cold`, `YamlValidated`, `SourcePersisted`, `ArtifactPersisted`, `RunHeaderPersisted`, `Accepted`, `Admitted`, `Running`, `Suspended`, `Finished`, `Failed`, `Restarted`, `Recovered`.
  - `sourceStored[run]`, `artifactStored[run]`, `acceptedEnvelope[run]`, `runHeaderStored[run]`.
  - `journal[run]`: finite sequence of event atoms including `RunAccepted`, `RunAdmission`, `RunFinished`, `RunFailed`.
  - `ack[run]`: externally visible acknowledgement/status set.
  - `yamlParserUsedAfterAdmission[run]`: boolean violation flag.
  - `digestOk[run]`, `artifactOk[run]`, `capabilityOk[run]`, `gateOk[run]`.
  - `inspectStatus[run]`, `eventsProjection[run]`.
- Init action: `Init` starts all runs at `Cold` with no durable evidence, no acknowledgements, and parser violation false.
- Next/actions: `ValidateYaml`, `RejectYaml`, `PersistSource`, `PersistArtifact`, `PersistRunHeader`, `AppendRunAccepted`, `AdmitAcceptedArtifact`, `RejectAdmission`, `StartRuntime`, `SuspendRuntime`, `FinishRuntime`, `FailRuntime`, `CrashRestart`, `RecoverFromJournal`, `RejectRecovery`, `Inspect`, `Events`, `IllegalYamlReparse`.
- State constraints: finite `RUNS`, bounded event sequence length, finite digest/proof/capability booleans, at most one active restart per run for TLC bounds.
- Symmetry sets: runs may be symmetric when bounded to multiple run ids; digest roles are not symmetric.
- Bounded model limits: start with `RUNS = {r1,r2}`, event length <= 8, boolean fault injection for source mismatch/artifact mismatch/gate mismatch/capability mismatch/durability failure/replay divergence.

## Properties

- Safety invariants:
  - `StrictAdmissionOnlyAcceptedArtifact`: `phase \in {Admitted,Running,Suspended,Finished,Recovered}` implies source, artifact, header, RunAccepted, accepted envelope, gate, proof, and capability predicates hold.
  - `PersistBeforeAck`: any ack for accepted/admitted/running/finished state implies the required durable prefix exists.
  - `NoYamlParseAfterAdmission`: once `Accepted` or later, `yamlParserUsedAfterAdmission = FALSE` always.
  - `InspectEventsReflectJournal`: inspect/events output is a projection of durable journal prefix, not a fabricated state.
  - `MismatchFailsClosed`: source/artifact/gate/capability/replay faults lead to `Failed` or rejected state, never `Admitted`/`Recovered` success.
- Liveness/eventuality:
  - Under no injected storage/runtime/replay fault and weak fairness for persistence/runtime/recovery actions, a validated run eventually reaches `Finished` or `Suspended` and after restart eventually reaches `Recovered` or terminal `Failed` with typed reason.
  - Inspect/events eventually reflect a durable terminal state after `Finished`.
- Fairness assumptions: weak fairness on enabled persistence, runtime progress, journal append, inspect/events projection, and recovery actions; no fairness on injected corruption/failure actions.
- Deadlock freedom: TLC must run with deadlock checking enabled or with an explicit terminal-stutter/progress property that is listed in the config. A config containing `CHECK_DEADLOCK FALSE` without such a property does not satisfy this contract.
- Temporal-property encoding: `NoFaultRunEventuallyAdmittedOrFailsTyped`, `NoFaultRunEventuallyFinishedOrSuspended`, and `AfterRestartEventuallyRecoveredOrTypedFailure` must be encoded as TLC `PROPERTY` entries or must carry an approved temporal waiver with owner, expiry, limitation, and compensating E2E evidence. A safety-only invariant run is insufficient.
- Refinement to Rust/runtime behavior: CLI and storage/runtime events refine TLA+ actions by `RunId`, source digest role, artifact digest role, accepted artifact proof fields, journal sequence, and inspect/events output. YAML parser calls after `RunAccepted` are outside refinement and violate `NoYamlParseAfterAdmission`.

## Evidence Command

- `tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla`
- Expected evidence: TLC exits 0, reports `Model checking completed. No error has been found.`, does not disable deadlock checking without an explicit progress property, and checks the configured temporal `PROPERTY` entries or references approved waiver `TLA-TEMPORAL-WAIVER` in `proof-obligations.jsonl`.
- If TLC binary is unavailable in repo environment, formal-verifier must record tool absence and use the repository proof-gauntlet template instead of silently passing.

## Waivers

- None. Temporal coverage is required for this bead.
