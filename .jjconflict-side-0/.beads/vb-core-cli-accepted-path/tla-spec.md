# TLA+ Temporal Model Plan: Accepted CLI Admission

## Boundary

- Temporal/workflow behavior: strict CLI run/submit acceptance ordering, accepted-run atomic boundary, runtime admission by digest, failure-before-ack behavior, and no raw compiled bypass in strict policy.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: postcard decoding, exact digest bytes, typed error construction, constructor APIs, and source parser/compiler correctness.
- External systems abstracted: Fjall storage as atomic or failing write boundary; CLI process as action source; runtime shard as state machine consumer.

## TLA+-Owned Clauses

- TLA-001: Strict run admission ordering.
- TLA-002: Strict submit durable acknowledgement ordering.
- TLA-003: Failure before accepted-run boundary reaches rejection without acknowledgement.
- INV-001: strict/journaled admitted run has persisted accepted artifact.
- INV-005: partial accepted-run boundary is never observable as accepted.
- POST-004: invalid artifact rejects before run state insertion and acknowledgement.

## Proposed Model Shape

- Module/model path: `verification/tla/AcceptedCliAdmission.tla` and `verification/tla/AcceptedCliAdmission.cfg`.
- Variables:
  - `policyByRun`: run id -> `Strict`, `Journaled`, or `Relaxed`.
  - `sourcePersisted`: set of run ids with workflow source persisted.
  - `artifactPersisted`: run id -> artifact digest or absent.
  - `headerPersisted`: run id -> compiled digest or absent.
  - `acceptedEventPersisted`: set of run ids with `RunAccepted`.
  - `admitted`: set of run ids admitted into runtime state.
  - `rejected`: set of run ids terminally rejected.
  - `digestMatch`: relation over source/artifact/header/runtime digest identities.
  - `artifactValid`: run id -> boolean abstraction for envelope/proof/gate/capability validity.
- Init action: `Init` with all persistence/admission sets empty and fixed finite run/digest domains for model checking.
- Next/actions: `ParseYaml`, `Compile`, `PersistSource`, `PersistAcceptedArtifact`, `PersistHeader`, `PersistRunAccepted`, `AtomicPersistAcceptedRun`, `RuntimeAdmitByDigest`, `RejectBeforeAck`, `RelaxedRun`, `StorageFailure`, `MalformedArtifact`.
- State constraints: finite runs, finite digests, finite policies; no run can be both admitted and rejected.
- Symmetry sets: runs and digest atoms may be symmetric if proof-writer keeps policy partitions explicit.
- Bounded model limits: at least 3 runs, 3 digests, all policies, valid and invalid artifact booleans, storage failure interleavings.

## Properties

- Safety invariants:
  - `StrictAdmittedHasArtifact`: every strict/journaled admitted run has persisted accepted artifact.
  - `AcceptedEventHasBoundary`: every strict/journaled `RunAccepted` has source, accepted artifact, and header persisted.
  - `DigestBindingTotal`: strict/journaled accepted/admitted runs have matching artifact/header/runtime digest.
  - `NoRawStrictBypass`: strict/journaled runs cannot transition directly from compile to admitted without accepted artifact persistence.
  - `NoPartialAcceptedRun`: partial persistence is not acknowledged as accepted.
  - `RejectBeforeStateInsertion`: invalid artifacts cannot be in `admitted`.
- Liveness/eventuality:
  - `EventuallyAcceptedOrRejected`: every strict/journaled run attempt eventually reaches accepted/admitted or rejected under fair storage/runtime actions.
  - `FailureEventuallyRejected`: storage/proof/digest failures eventually reach rejected without `RunAccepted`.
- Fairness assumptions: weak fairness must be encoded in the TLA+ spec for enabled persistence, rejection, and runtime admission actions; no fairness on permanently disabled/failing storage actions.
- Deadlock freedom: TLC must either check deadlock directly or prove a reviewed terminal-state treatment. `CHECK_DEADLOCK FALSE` without terminal-state evidence is not accepted.
- Refinement to Rust/runtime behavior: CLI `cmd_run`/`cmd_submit` actions refine parse/compile/persist actions; `vb_storage` write APIs refine persistence actions; `Runtime::new_with_journal` plus storage-backed artifact store refines `RuntimeAdmitByDigest`; lifecycle run insertion refines `admitted`.

## Evidence Command

- `tlc -config verification/tla/AcceptedCliAdmission.cfg verification/tla/AcceptedCliAdmission.tla`
- Required observable evidence: TLC exits 0, reports no invariant violations, reports no non-terminal deadlock or approved terminal-only deadlock handling, and checks configured `PROPERTY` entries for `EventuallyAcceptedOrRejected` and `FailureEventuallyRejected`.
- If Moon verification tasks are used as rollup evidence, they must include or attach the raw TLC output from the exact command above.

## Waivers

- None. This bead has temporal acceptance and acknowledgement ordering; TLA+ applies.
