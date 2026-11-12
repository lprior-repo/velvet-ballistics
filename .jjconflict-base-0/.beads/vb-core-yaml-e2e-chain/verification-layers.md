# Verification Layers

## Boundary

- Verus-owned kernel: digest role validation, mismatch classification, deterministic recovery summary predicates, and pure recovery state/refinement invariants.
- TLA+ temporal model: YAML-origin lifecycle, strict accepted-artifact admission, persist-before-ack, events/inspect projection, restart/replay/recovery, and fail-closed temporal behavior.
- Theorem projection: optional digest-role theorem only if Verus is insufficient.
- Runtime shell: CLI commands, Fjall, Postcard, parser execution, process restart, and filesystem.
- External systems excluded from formal proof: OS crash semantics beyond modeled restart, Fjall internals beyond durable store abstraction, and parser implementation internals beyond compile-boundary tests/fuzzing.

## Layer Assignment

- PRE-001 -> static-scan + integration + TLA+ `NoYamlParseAfterAdmission`.
- PRE-002 -> focused strict YAML tests + cargo-fuzz/Bolero or proptest malformed YAML corpus + mutation; must observe `StrictYamlRejected`-class evidence.
- PRE-003 -> Verus + proptest + unit/integration digest mismatch tests.
- PRE-004 -> Verus + proptest + unit/integration artifact mismatch tests.
- PRE-005 -> integration + Kani/proptest for proof/gate combinations + TLA+ strict admission invariant.
- PRE-006 -> TLA+ persist-before-ack + integration fail-injected persistence tests.
- PRE-007 -> TLA+ recovery boundary + static-scan dependency boundary + integration restart/replay tests.
- POST-001 -> BDD E2E + CLI integration + TLA+ success evidence invariant.
- POST-002 -> BDD E2E + CLI events/inspect assertions + traceability snapshot.
- POST-003 -> TLA+ + static-scan + BDD restart/replay with parser-call sentinel or dependency-boundary evidence.
- POST-004 -> unit/integration/proptest corruption tests + Verus mismatch classification.
- POST-005 -> TLA+ refinement + recovery integration + proptest journal/snapshot variants.
- POST-006 -> static-scan dependency boundary + source clippy/lint gate.
- INV-001 -> Verus + proptest + focused storage tests.
- INV-002 -> Verus + proptest + focused admission/recovery tests.
- INV-003 -> TLA+ + integration + mutation.
- INV-004 -> TLA+ + fail-injected storage integration.
- INV-005 -> TLA+ + static-scan + BDD restart/replay.
- INV-006 -> Verus + proptest + recovery integration.
- INV-007 -> TLA+ + CLI events/inspect integration.
- INV-008 -> Verus + optional Lean + proptest mismatched role cases.

## Verus Scope

- Rust targets:
  - `vb_storage::journal::admission::verify_content_digest`
  - `vb_storage::recovery::recover::verify_digests`
  - `vb_storage::recovery::replay::summary::reject_workflow_digest_mismatch`
  - recovery summary/frame seed pure abstraction around `recover_runtime_frame_seed_from_events_with_workflow`
- Spec/proof functions: `verification/verus/yaml_e2e_digest_roles.rs` owns the current abstract kernel. Proof names are the Verus items in that file for source/artifact role separation, mismatch classification, invalid-artifact denial, and deterministic same-input recovery classification abstraction.
- Invariants: source digest equality, artifact digest equality, role separation, deterministic summary from equal event/workflow inputs, mismatch maps to typed error.
- Trusted boundary: validated bytes and digest inputs from storage/CLI; Fjall reads; Postcard decode; parser output.
- Shell exclusions: I/O, YAML parser internals, Fjall flush internals, CLI formatting, wall-clock, process restart.
- Evidence command: `verus verification/verus/yaml_e2e_digest_roles.rs`.
- Expected evidence: `verification results:: 6 verified, 0 errors`.
- Verus shell-linkage waiver: Verus proves the pure abstraction only and intentionally excludes BLAKE3 implementation, Fjall I/O, Postcard decode, CLI formatting, YAML parsing, and runtime scheduling. Owner: proof-planner/formal-verifier. Expiry: before State 6 retry approval. Compensating evidence: required Kani admission matrix, storage/runtime typed-error tests, static boundary scan, Miri codec run, and E2E CLI/recovery evidence.

## TLA+ Scope

- Module/model path: planned `verification/tla/YamlE2eChain.tla`; config `verification/tla/YamlE2eChain.cfg`.
- Variables: `phase`, `sourceStored`, `artifactStored`, `acceptedEnvelope`, `runHeaderStored`, `journal`, `ack`, `yamlParserUsedAfterAdmission`, `digestOk`, `artifactOk`, `capabilityOk`, `gateOk`, `inspectStatus`, `eventsProjection`.
- Actions: `ValidateYaml`, `PersistSource`, `PersistArtifact`, `PersistRunHeader`, `AppendRunAccepted`, `AdmitAcceptedArtifact`, `StartRuntime`, `SuspendRuntime`, `FinishRuntime`, `CrashRestart`, `RecoverFromJournal`, `RejectRecovery`, `Inspect`, `Events`, fault-injection actions.
- Safety invariants: strict accepted-artifact admission, persist-before-ack, no YAML parse after admission, faithful inspect/events projection, mismatch fails closed.
- Temporal properties: no-fault eventual admission-or-typed-fail, no-fault eventual finish/suspend, post-restart eventual recovery or typed failure, and deadlock freedom or explicit terminal-stutter progress property.
- Fairness/deadlock stance: weak fairness on enabled persistence/runtime/recovery/projection actions; no fairness on fault injection; State 6 rejects TLA evidence if `CHECK_DEADLOCK FALSE` is present without an explicit progress property/waiver.
- Refinement boundary: Rust CLI/storage/runtime journal events and outputs refine model actions by run id, digest roles, accepted envelope, proof/gate/capability flags, and journal sequence.
- Evidence command: `tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla`.

## Runtime Shell Evidence

- Focused CLI E2E: `cargo test -p velvet_ballistics --test cli_integration -- --nocapture` with downstream-added targeted tests for YAML strict run/submit/events/inspect/recovery chain.
- Storage/admission/recovery unit/integration: `cargo test -p vb_storage -- --nocapture` and `cargo test -p velvet-ballistics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture`.
- Runtime admission: `cargo test -p vb_runtime -- --nocapture`.
- Strict YAML rejection: `cargo test -p vb_compile -- --nocapture` with focused malformed YAML cases.
- Kani strict admission matrix: `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix`; currently BLOCKED until the harness is wired into a Kani-discoverable `vb_runtime` target.
- Source lint/static boundary: `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` plus a downstream static scan proving runtime crates do not depend on YAML parser crates for recovery.
- Release gate: `moon ci`.

## Waivers

- No TLA+ waiver.
- Lean mandatory proof waived until State 4 unless Verus cannot express digest role separation.
- Miri/Postcard codec is required for this release-critical parser/codec bead unless an independent reviewer approves a clause-specific waiver. Command: `cargo +nightly miri test -p vb_storage`.
