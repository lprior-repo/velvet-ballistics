# Proof Evidence: vb-qi37.4

updated_at: 2026-05-17T04:41:55Z
state: 5
attempt: 3

## Evidence Summary

- Direct TLA+ evidence: PASS for `TLA-ACK-001` and `TLA-STATE-002` via exact command.
- Direct Verus evidence: PASS for `VERUS-CAP-003`, `VERUS-GATE-004`, and `VERUS-DIGEST-005` via exact commands.
- Canonical Moon proof gate: PASS for `CANONICAL-PROOF-GATE-016` via `moon run :verify-proof`.
- Production/test/dependency/CI/source-checkout edits: none.

## Workspace Evidence

- Command: `pwd -P`.
- Exit: 0.
- Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4`.

## TLA+ Evidence

- Obligations: `TLA-ACK-001`, `TLA-STATE-002`.
- Artifacts: `specs/admission_header_before_ack.tla`, `specs/admission_header_before_ack.cfg`.
- Command: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`.
- Exit: 0.
- Output summary: TLC 2.19 parsed the model, computed 6 initial states, generated 25 states, found 13 distinct states, checked 2 temporal property branches, reported depth 3, and ended with `Model checking completed. No error has been found.`
- Checked invariants: `TypeOK`, `FailurePreventsAck`, `DuplicateRejectsNoLiveState`, `AckRequiresPersistence`, `LiveStateRequiresPersistence`, `NoLiveStateBeforeDurableAdmission`.
- Checked temporal properties: `FailureEventuallyRejected`, `SuccessEventuallyAcked`.
- Model bounds and simplifications: finite `ErrorCodes = {HeaderPersistenceFailed, QueueFull}`, singleton `NoCode`, `duplicate_run \in BOOLEAN`, states `{Pending, Persisted, Rejected, Acked}`, Fjall persistence abstracted as `PersistHeader`.

## Verus Admission Evidence

- Obligations: `VERUS-GATE-004`, `VERUS-DIGEST-005`.
- Artifact: `verification/verus/admission_artifact_model.rs`.
- Command: `verus verification/verus/admission_artifact_model.rs`.
- Exit: 0.
- Output: `verification results:: 6 verified, 0 errors`.
- Verified proof functions: `proof_success_requires_runtime_gate_count`, `proof_wrong_gate_count_denies`, `proof_false_required_flag_denies`, `proof_success_preserves_digest_binding`, `proof_digest_mismatch_denies`, plus `main`.
- Model bounds and simplifications: pure finite/int model, `required_gate_count() == 15`, required proof flags are `bounded`, `taint_safe`, `retry_safe`, `durable`, and `replayable`; byte digest construction and production field extraction are trusted shell boundaries.

## Verus Capability Evidence

- Obligation: `VERUS-CAP-003`.
- Artifact: `verification/verus/capability_artifact_model.rs`.
- Command: `verus verification/verus/capability_artifact_model.rs`.
- Exit: 0.
- Output: `verification results:: 8 verified, 0 errors`.
- Verified proof functions: `proof_exact_match_requires_name_and_action`, `proof_prefix_or_action_mismatch_denies`, `proof_exact_profile_requires_cardinality`, `proof_missing_or_excess_grants_deny`, `proof_certificate_preserves_required_capabilities`, `proof_non_empty_contract_not_erased`, `proof_gate12_rejects_invalid_schema`, plus `main`.
- Model bounds and simplifications: abstract capability names/actions as integers, abstract profile cardinality as counts, name length bounded by `0 < name_len <= 128`; production `CapabilitySet` extraction and accepted-artifact decoding are trusted shell boundaries.

## Tool Discovery

- `which java || true`: exit=0; `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- `which tlc || true`: exit=0; `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `which verus || true`: exit=0; `/home/lewis/.local/bin/verus`.
- `cargo kani --version`: exit=0; `cargo-kani 0.67.0`.
- `cargo fuzz --version`: exit=0; `cargo-fuzz 0.13.1`.
- `cargo +nightly miri --version`: exit=0; `miri 0.1.0 (e0e95a7187 2026-04-04)`.

## Canonical Moon Proof Gate Evidence

- Obligation: `CANONICAL-PROOF-GATE-016`.
- Command: `moon run :verify-proof`.
- Exit: 0.
- Output summary: Moon task `velvet-ballastics:verify-proof` ran `scripts/rust-verification-gauntlet.sh proof`, completed in 2s 574ms, and reported `[PASS] All proof checks passed`.
- Harnesses reported PASS: `KANI-EXPR-BYTECODE-001`, `KANI-SLOT-REF-001`, `KANI-CONSTANT-POOL-001`, `KANI-ACCESSOR-REF-001`, and `INV-007-NODEDUP-001`.
- Wrapper note: `Verus proofs (VERUS-EXPR-STACK-001, VERUS-SLOT-MAX-001) are WAIVED -- toolchain not installed`; this is an existing wrapper note for vb_compile proof lanes and does not replace the direct Verus evidence above for `VERUS-CAP-003`, `VERUS-GATE-004`, or `VERUS-DIGEST-005`.

## Deferred Or Not Run

- `KANI-ADMIT-006`: NOT_RUN; planned for later `moon run :verify-deep` owner state.
- `FUZZ-ARTIFACT-007`: NOT_RUN; planned for later fuzz/deep evidence owner state.
- `LOOM-JOURNAL-012`: NOT_RUN; planned for later Loom/Shuttle or explicit waiver.
- `INT-HEADER-008`, `INT-RECOVERY-009`, `STATIC-NO-YAML-010`, `MUT-ERR-011`, `GATE-CI-013`, `INT-DUPLICATE-014`, `INT-CAPACITY-015`: NOT_RUN; later integration/static/mutation/CI closure lanes.
- Prior stale blocked-tooling evidence is superseded by State 5 attempt 3 canonical wrapper PASS evidence above.

## Assumptions Ledger

- TLA+ persistence success is modeled by `PersistHeader`; actual disk flush and Fjall semantics remain integration evidence.
- TLA+ duplicate-run rejection is modeled by initial `duplicate_run`; production duplicate lookup remains integration/Kani evidence.
- TLA+ capacity failure is modeled as `QueueFull`; production active-run capacity accounting remains integration/Kani evidence.
- Verus digest ids are abstract integers; byte-level digest construction and serialization are trusted shell boundaries.
- Verus gate/proof flags and capability profiles are abstract inputs; production extraction and validation paths must be covered by later bounded/integration evidence.
- The canonical Moon proof wrapper now passes in this workspace; later realization obligations remain owned by State 8/11 as listed above.
