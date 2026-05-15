# Test Plan: vb-core-yaml-e2e-chain

## Summary

- Source skills read: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md`; contents match, and `.agents` wins on conflict. Testing doctrine reference read from `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`.
- Approved gates consumed: `proof-review.md` STATUS: APPROVED and `contract-verification-review.md` STATUS: APPROVED.
- Behaviors identified: 16.
- Trophy allocation: 5 unit/property groups / 8 integration groups / 2 E2E acceptance groups / 3 static/formal gate groups. Integration is intentionally widest because the contract is a cross-crate durable chain.
- Proptest invariants: 7.
- Fuzz targets: 4 candidates; existing bead-specific target not discovered, so fuzz execution is waived unless a target exists later; strict YAML/proptest/Miri compensate.
- Kani harnesses: 1 mandatory harness plus existing TLA+/Verus proof reruns as proof-gate prerequisites.
- Mutation threshold: minimum 90% mutation kill rate for scoped crates/paths; any survivor in digest, admission, durability, recovery, strict YAML, or error mapping code is release-blocking.

## 1. Behavior Inventory

| ID | Behavior | Contract / trace | Primary layer |
|---|---|---|---|
| B01 | YAML-origin execution uses YAML bytes only at the cold compile boundary when strict run starts. | PRE-001, PRE-007, POST-003, INV-005, ERR-011; `given_yaml_source_when_strict_run_starts_then_yaml_is_used_only_by_compile_boundary` | Integration + static |
| B02 | Strict YAML compilation rejects unsupported profile features when duplicate keys, aliases, anchors, tags, multi-doc streams, invalid shape, schema/reference/type/taint/control-flow violations appear. | PRE-002, ERR-001; `given_duplicate_keys_alias_anchor_tag_multi_doc_or_invalid_shape_when_compiled_then_strict_yaml_rejected` | Unit/property |
| B03 | Source persistence rejects source digest mismatch when claimed digest differs from stored YAML source bytes. | PRE-003, INV-001, ERR-002 | Unit/property + integration |
| B04 | Artifact loading/recovery rejects compiled artifact digest mismatch when runtime bytes differ from artifact digest. | PRE-004, INV-002, ERR-003 | Unit/property + integration |
| B05 | Strict runtime admission rejects missing accepted-artifact envelopes, loose raw workflow parts, raw YAML, under-proven artifacts, and malformed envelopes. | PRE-005, INV-003, ERR-004, ERR-005 | Kani + integration |
| B06 | Strict runtime admission rejects capability mismatch when an accepted artifact requires ungranted capabilities. | PRE-005, ERR-006 | Kani + integration |
| B07 | Storage durability failure before acknowledgement returns a typed failure and no runnable state becomes visible. | PRE-006, INV-004, ERR-007 | Integration/E2E |
| B08 | Valid YAML-origin strict run persists source, accepted artifact, run header, RunAccepted, RunAdmission, terminal/suspended runtime state, events, and inspect evidence. | POST-001, POST-002, INV-007 | E2E acceptance |
| B09 | Events and inspect expose digest-bound run id, source digest, artifact digest, accepted sequence, and final/suspended status. | POST-002, INV-007 | Integration/E2E |
| B10 | Restart/replay/recovery reconstructs from persisted headers, artifacts, journal events, snapshots, and compiled bytes without YAML reparsing. | POST-003, POST-005, INV-005, INV-006, ERR-011 | E2E recovery + static |
| B11 | Recovery fails closed with ReplayDivergence when persisted journal/snapshot data diverges from compiled workflow/runtime model. | POST-004, ERR-008, INV-006 | Unit/property + integration |
| B12 | Recovery fails closed with CorruptRecoveryData/CorruptSnapshot/frame corruption when persisted recovery data is corrupt or incomplete. | POST-004, ERR-009 | Unit/property + Miri |
| B13 | Recovery without durable evidence returns NoRecoveryData and never synthesizes an empty success. | POST-004, ERR-010, INV-007 | Unit/property + integration |
| B14 | Replay is deterministic over the same persisted evidence: same recovered state or same typed fail-closed error. | INV-006 | Proptest + integration |
| B15 | Source digest and artifact digest remain distinct roles even when represented by the same digest type. | INV-008 | Verus + Kani + unit/property |
| B16 | Runtime/recovery core remains free of YAML, JSON, and HTTP parser dependencies after admission. | POST-006, INV-005, ERR-011 | Static gate |

## 2. Trophy Allocation

| Layer | Planned groups | Scope | Rationale |
|---|---:|---|---|
| Static/formal base | 3 | clippy/static boundary scan, TLA+/Verus reruns, no parser dependency scan | Required because parser-boundary absence, temporal durability, and digest-role proof cannot be safely inferred from examples. |
| Unit / Calc / property | 5 | strict YAML rejection, digest mismatch classification, artifact mismatch, corrupt recovery data, deterministic recovery | Pure or near-pure classification and corrupt-input logic needs exact expected variants and broad input coverage. |
| Integration | 8 | storage + runtime admission + recovery + events/inspect + CLI error taxonomy | Widest layer: public behavior crosses compile, storage, runtime, CLI, and Fjall evidence boundaries. Use real local storage/fakes only for deterministic failure injection. |
| E2E acceptance | 2 | full YAML-origin strict run; restart/recovery no-YAML chain | Few but mandatory user-facing workflows validate the end-to-end durable chain. |

Deviation from 60/30/5/5 target is intentional: this bead is a durable cross-crate acceptance chain, so integration and E2E carry more contract value than isolated units.

## 3. BDD Scenarios

Every scenario must assert exact values or exact typed variants. Tests that only assert `is_ok()` or `is_err()` are rejected.

### B01: YAML is cold-boundary only

`fn yaml_source_is_used_only_by_compile_boundary_when_strict_run_starts()`
- Given: valid YAML source bytes and a strict YAML-origin run request.
- When: the source is validated/compiled and admitted through the strict runtime path.
- Then: YAML parser APIs are observed only before accepted artifact persistence.
- And: runtime/recovery consumes compiled artifact bytes and persisted records, not YAML source bytes.
- Trace: PRE-001, PRE-007, POST-003, INV-005; PO-003, PO-008, PO-009.

### B02: Strict YAML rejects unsupported profile features

`fn strict_yaml_rejected_when_duplicate_key_or_alias_anchor_tag_multi_doc_or_invalid_shape_present()`
- Given: YAML inputs containing duplicate keys, aliases, anchors, explicit tags, multi-document streams, invalid shape, schema/reference/type/taint/control-flow violations.
- When: `vb_compile` validates/compiles each input.
- Then: each case returns the exact `StrictYamlRejected`-class variant mapped by the crate.
- And: no compiled artifact or accepted artifact is produced.
- Trace: PRE-002, ERR-001; PO-010, PO-011.

### B03: Source digest mismatch fails closed

`fn source_digest_mismatch_returns_typed_error_when_source_is_persisted()`
- Given: source bytes and a different claimed source digest.
- When: the storage source/admission path persists or verifies the source.
- Then: the exact error is `WorkflowSourceDigestMismatch` or `PayloadDigestMismatch` as mapped by the crate.
- And: no accepted source evidence is recorded under the mismatched digest.
- Trace: PRE-003, INV-001, ERR-002; PO-004, PO-006, PO-011.

### B04: Artifact digest mismatch fails closed

`fn compiled_artifact_digest_mismatch_returns_typed_error_when_artifact_is_loaded_or_recovered()`
- Given: persisted artifact bytes and a mismatched compiled artifact digest or RunAccepted digest.
- When: runtime/recovery loads the artifact.
- Then: the exact error is `CompiledIrDigestMismatch`-class.
- And: no recovered or admitted success is returned.
- Trace: PRE-004, INV-002, ERR-003; PO-005, PO-006, PO-011, PO-013.

### B05: Strict admission rejects missing or invalid envelope

`fn strict_admission_rejects_when_accepted_artifact_envelope_is_missing_invalid_or_under_proven()`
- Given: strict runtime admission input with missing envelope, malformed envelope, insufficient gate count, missing proof flags, loose `WorkflowParts`, raw YAML, or raw compiled IR bypass.
- When: `vb_runtime::admission` attempts strict admission.
- Then: exact variants are `AcceptedArtifactMissing` for absent envelope and `AcceptedArtifactInvalid` for malformed/under-proven/gate mismatch cases.
- And: no `RunAdmission` success or runnable state is produced.
- Trace: PRE-005, INV-003, ERR-004, ERR-005; PO-012, PO-011.

### B06: Capability mismatch rejects admission

`fn capability_mismatch_returns_typed_error_when_required_capability_is_not_granted()`
- Given: an accepted artifact requiring a capability absent from runtime grants.
- When: strict admission runs.
- Then: exact variant is `CapabilityMismatch`.
- And: no durable RunAdmission success is appended.
- Trace: PRE-005, ERR-006; PO-012, PO-011.

### B07: Durability failure before ack fails closed

`fn durability_failure_returns_typed_error_and_no_runnable_state_when_persistence_fails_before_ack()`
- Given: a deterministic storage failure before source/artifact/header/journal evidence is flushed.
- When: CLI submit/run attempts a strict YAML-origin execution.
- Then: exact variant is `DurabilityFailure`-class.
- And: inspect/events do not show RunAccepted, RunAdmission, Running, Finished, or Suspended success for that run id.
- Trace: PRE-006, INV-004, ERR-007; PO-002, PO-011.

### B08: Full valid YAML-origin strict run exposes durable evidence

`fn valid_yaml_origin_run_exposes_source_artifact_header_events_and_inspect_when_strict_run_completes()`
- Given: valid YAML source and isolated local storage.
- When: CLI run/submit executes through strict accepted-artifact runtime.
- Then: durable evidence contains source storage, compiled/accepted artifact storage, run header, RunAccepted, RunAdmission, and terminal/suspended event.
- And: inspect/events expose run id, source digest, artifact digest, accepted sequence, and final/suspended status.
- Trace: POST-001, POST-002, INV-007; PO-007, PO-017.

### B09: Events/inspect do not synthesize success

`fn events_and_inspect_do_not_synthesize_success_when_journal_lacks_required_success_prefix()`
- Given: storage state lacking RunAccepted/admission/runtime terminal prefix.
- When: events and inspect query the run.
- Then: no successful status is synthesized.
- And: output is absent, pending, or exact typed failure according to public API contract.
- Trace: POST-002, INV-007; PO-002, PO-007.

### B10: Restart recovery is YAML-free and refines acknowledged state

`fn recovery_uses_persisted_artifact_journal_and_snapshot_when_restart_occurs_after_admission()`
- Given: a YAML-origin run admitted with persisted source/artifact/journal/snapshot evidence.
- When: the process restarts and recovery runs.
- Then: recovered summary/frame seed refines the acknowledged persisted journal/snapshot state.
- And: no YAML parser call or YAML source dependency occurs after admission.
- Trace: POST-003, POST-005, INV-005, INV-006, ERR-011; PO-008, PO-009.

### B11: Replay divergence fails closed

`fn replay_divergence_returns_typed_error_when_journal_snapshot_diverges_from_runtime_model()`
- Given: persisted journal/snapshot data that diverges from compiled workflow/runtime model.
- When: recovery/replay hydrates state.
- Then: exact variant is `ReplayDivergence`.
- And: no partial recovered success is returned.
- Trace: POST-004, ERR-008; PO-006, PO-011.

### B12: Corrupt recovery data fails closed

`fn corrupt_recovery_data_returns_typed_error_when_frame_snapshot_or_journal_is_malformed()`
- Given: corrupt frame, snapshot, or journal bytes/records.
- When: recovery decodes or hydrates persisted data.
- Then: exact variant is `CorruptRecoveryData`, `CorruptSnapshot`, or exact frame-corruption variant mapped to contract ERR-009.
- And: no panic, UB, or recovered success occurs.
- Trace: POST-004, ERR-009; PO-006, PO-011, PO-013.

### B13: No recovery data fails closed

`fn no_recovery_data_returns_typed_error_when_recovery_requested_without_durable_evidence()`
- Given: a run id with no durable header/journal/snapshot evidence.
- When: recovery is requested.
- Then: exact variant is `NoRecoveryData`.
- And: no empty successful frame is synthesized.
- Trace: POST-004, ERR-010; PO-006, PO-011.

### B14: Recovery is deterministic over identical inputs

`fn recovery_returns_same_state_or_same_typed_error_when_persisted_inputs_are_identical()`
- Given: identical persisted source/artifact/journal/snapshot sets.
- When: recovery runs twice in isolated storage contexts.
- Then: both runs return the same recovered summary/frame seed or the same exact typed error.
- Trace: INV-006; PO-006, PO-008.

### B15: Digest roles are distinct

`fn source_digest_used_as_artifact_digest_is_rejected_when_roles_differ()`
- Given: valid source digest bytes supplied in the artifact digest role, or valid artifact digest bytes supplied in the source role.
- When: source persistence, artifact loading, admission, or recovery checks digests.
- Then: role confusion is rejected with exact source/artifact mismatch or admission invalid variant.
- Trace: INV-008; PO-004, PO-005, PO-012.

### B16: Runtime/recovery core has no YAML/JSON/HTTP parser dependency

`fn runtime_recovery_paths_have_no_yaml_json_http_parser_dependency_when_static_boundary_scan_runs()`
- Given: scoped runtime and recovery paths.
- When: clippy and the static boundary scan run.
- Then: no runtime/recovery dependency on YAML parser crates, `serde_json` runtime parsing, or HTTP parsing is present.
- And: any parser reference after admission is a release-blocking failure unless formally isolated to cold compile/input boundary.
- Trace: POST-006, INV-005, ERR-011; PO-009, PO-017.

## 4. Unit / Integration / E2E Plan

### Unit and property groups

| Group | Target | Scenarios | Required assertions |
|---|---|---|---|
| U01 strict YAML profile | `crates/vb_compile/src/strict_yaml.rs`, `crates/vb_compile/src/lib.rs`, `vb_yaml`, `vb_validate` | B02 | Exact `StrictYamlRejected`-class mapping for every unsupported feature; no artifact emitted. |
| U02 source/artifact digest classification | `vb_storage` source/admission/recovery digest checks | B03, B04, B15 | Exact source vs artifact mismatch variant; digest role confusion rejected. |
| U03 recovery corruptions | `crates/vb_storage/src/recovery/**` | B11, B12, B13, B14 | Exact `ReplayDivergence`, `CorruptRecoveryData`/mapped corruption, `NoRecoveryData`; no success/panic. |
| U04 runtime admission matrix | `crates/vb_runtime/src/admission.rs`, crate-local Kani harness | B05, B06, B15 | Missing, invalid, gate/proof mismatch, raw bypass, and capability cases map to exact admission errors. |
| U05 events/inspect projection rules | storage journal projection helpers and CLI-facing status mapping | B09 | Success never appears without durable journal prefix. |

### Integration groups

| Group | Command / path | Scenarios | Required assertions |
|---|---|---|---|
| I01 compile strict YAML | `cargo test -p vb_compile -- --nocapture` | B02 | Report names each malformed class and exact variant. |
| I02 storage digest/recovery | `cargo test -p vb_storage -- --nocapture` | B03, B04, B11-B14 | Focused evidence for all storage/recovery typed errors and deterministic recovery. |
| I03 runtime admission | `cargo test -p vb_runtime -- --nocapture` | B05, B06, B15 | Strict admission rejects all invalid matrix cells; no successful `RunAdmission`. |
| I04 CLI strict durable chain | `cargo test -p velvet_ballastics --test cli_integration -- --nocapture` | B07-B09 | CLI output and stored state expose digest-bound durable evidence or exact typed failures. |
| I05 workspace recovery | `cargo test -p velvet-ballastics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture` | B10 | Restart/recovery uses persisted data only; no YAML parser sentinel after admission. |
| I06 full error taxonomy chain | chained `vb_compile`, `vb_storage`, `vb_runtime`, CLI, workspace recovery commands from PO-011 | B02-B13 | Evidence report maps every ERR-001..ERR-011 to exact crate/public variant. |
| I07 artifact envelope parity | `vb_storage` + `vb_runtime` focused admission tests | B05, B15 | Storage-produced accepted artifact satisfies runtime gate/proof expectations, or mismatch fails with `AcceptedArtifactInvalid`. |
| I08 events/inspect journal fidelity | CLI integration and storage projection tests | B08, B09 | Events/inspect are projections of persisted Fjall journal state; no synthetic success. |

### E2E acceptance groups

| Group | Command | Scenarios | Required assertions |
|---|---|---|---|
| E01 valid YAML-origin strict run | `cargo test -p velvet_ballastics --test cli_integration -- --nocapture` or black-box CLI fixture | B08, B09 | Source digest, artifact digest, run id, accepted sequence, RunAccepted, RunAdmission, terminal status visible in events/inspect. |
| E02 restart/recovery no-YAML chain | `cargo test -p velvet-ballastics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture` | B10, B14 | Recovery refines acknowledged state from persisted artifact/journal/snapshot and proves no post-admission YAML parser dependency. |

## 5. Proptest Invariants

| ID | Function/area | Invariant | Strategy | Anti-invariant / required failure |
|---|---|---|---|---|
| P01 | strict YAML validator | Any unsupported YAML profile feature is rejected before compile output. | Generate YAML strings/ASTs with duplicate keys, aliases, anchors, tags, multi-doc markers, invalid shape/reference/type/taint/control-flow fragments. | Any accepted unsupported feature must fail with `StrictYamlRejected`-class evidence. |
| P02 | source digest verification | For any source byte vector and claimed digest, only the digest of the exact bytes succeeds. | `Vec<u8>` bounded to practical sizes plus digest mutation. | Mismatched digest returns `WorkflowSourceDigestMismatch`/`PayloadDigestMismatch`. |
| P03 | artifact digest verification | For any artifact byte vector and claimed artifact digest, only exact digest succeeds. | Valid artifact seeds plus byte/digest bit flips. | Mismatch returns `CompiledIrDigestMismatch` or exact mapped admission invalid error. |
| P04 | recovery corruptions | Corrupt frame/snapshot/journal bytes never produce recovered success or panic. | Generate valid seed record then mutate byte, ordering, missing field, truncation, duplicated event, divergent snapshot. | Returns `CorruptRecoveryData`, `ReplayDivergence`, or exact mapped typed failure. |
| P05 | deterministic recovery | Same persisted input set yields same recovered state or same typed error. | Generate bounded persisted evidence sets; run recovery twice in isolated stores. | Non-deterministic status/summary/error is failure. |
| P06 | journal prefix durability | Any visible ack/status has a persisted prefix containing required source/artifact/header/admission/runtime events. | Generate event prefix/suffix combinations and ack positions. | Ack without required prefix returns no success or `DurabilityFailure`/projection absence. |
| P07 | digest role separation | Source digest and artifact digest roles are not interchangeable. | Generate two byte domains and valid digests; swap roles. | Role-swap succeeds only if bytes and role contract legitimately match; otherwise exact mismatch/admission invalid. |

## 6. Fuzz Targets

Existing proof plan waived bead-specific fuzz execution because no target was discovered. If a target exists or is added later, these are mandatory candidates:

| Target | Input type | Risk | Corpus seeds | Gate |
|---|---|---|---|---|
| F01 strict YAML parser/profile | bytes/string | panic, OOM, unsupported YAML accepted, invalid diagnostic mapping | empty, huge scalar, duplicate keys, anchors/aliases, tags, multi-doc, deep nesting, invalid UTF-8 if accepted as bytes | `cargo fuzz run <strict_yaml_target> -- -max_total_time=60`; otherwise PO-014 waiver compensation. |
| F02 compiled artifact/postcard decode | bytes | panic/UB, malformed artifact accepted, digest bypass | empty, truncated postcard, wrong version, swapped digest fields, huge vectors | fuzz target if present plus Miri PO-013. |
| F03 recovery snapshot/frame decode | bytes | corrupt recovery success, panic/UB | truncated frame, unknown enum tag, bad length prefix, missing run id, divergent slot state | fuzz target if present plus storage property tests. |
| F04 CLI YAML input boundary | bytes/files | command accepts invalid profile or produces untyped error | valid minimal YAML, malformed YAML, multi-doc, tag/anchor aliases, control-flow invalid cases | CLI/integration target if available. |

## 7. Kani / Formal / Static Gates

| Gate | Obligation | Command | Acceptance |
|---|---|---|---|
| K01 admission matrix | KANI-ADMIT-023 / PO-012 | `TMPDIR=target/tmp cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` | Kani reports one or more successfully verified harnesses, 0 failures; matrix covers missing envelope, under-proven gate/proof, digest role confusion, capability mismatch. |
| T01 lifecycle/recovery model | TLA-LIFE-001, TLA-DUR-002, TLA-REC-003 / PO-001..003 | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` | TLC exits 0 with temporal properties checked, no `CHECK_DEADLOCK FALSE` waiver gap, and state/depth counts recorded. |
| V01 digest roles | VERUS-DIG-004, VERUS-DIG-005 / PO-004..005 | `TMPDIR=target/tmp verus verification/verus/yaml_e2e_digest_roles.rs` | Verus reports at least `8 verified, 0 errors`; shell limitations covered by executable tests. |
| S01 parser boundary + lint | STATIC-BOUNDARY-009 / PO-009 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` plus scoped dependency/static scan | Clippy exits 0; report shows no post-admission runtime/recovery dependency on YAML, JSON, HTTP parser APIs. |
| M01 codec UB | MIRI-CODEC-024 / PO-013 | `cargo +nightly miri test -p vb_storage` | Miri exits 0 or approved waiver records exact tool failure, owner, expiry, and codec/admission compensation. |
| R01 release gate | GATE-RELEASE-025 / PO-017 | `moon ci` | Moon exits 0 or formal evidence separates unrelated regression from bead-local failure; bead-local failure blocks. |

## 8. Mutation Checkpoints

Threshold: `cargo-mutants` scoped mutation kill rate must be ≥90%; any survivor in critical branches below blocks release even if global percentage passes.

| Mutation | Must be killed by |
|---|---|
| Remove duplicate-key/alias/anchor/tag/multi-doc rejection branch. | `strict_yaml_rejected_when_duplicate_key_or_alias_anchor_tag_multi_doc_or_invalid_shape_present` / PO-010. |
| Replace source digest equality with true or artifact digest equality. | B03/B04 digest mismatch tests; P02/P03; Verus/Kani gates. |
| Swap source/artifact digest role variables. | B15, P07, K01, V01. |
| Lower or ignore accepted-artifact gate/proof count. | B05, I07, K01. |
| Ignore missing capability grant. | B06, K01. |
| Acknowledge before journal/header/artifact persistence. | B07, P06, T01. |
| Synthesize inspect/events success without journal prefix. | B09, I08, T01. |
| Convert corrupt recovery data into default recovered state. | B12, P04, M01. |
| Convert absent recovery data into empty success. | B13, P04. |
| Permit YAML parser call during recovery. | B10, B16, S01. |
| Collapse typed errors into generic error. | Full error taxonomy chain PO-011. |

## 9. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| Valid strict YAML run | valid YAML, sufficient capabilities, valid accepted artifact | digest-bound successful durable evidence and inspect/events status | E2E | POST-001, POST-002 |
| Unsupported YAML duplicate keys | duplicate key YAML | `StrictYamlRejected`-class | unit/integration/proptest | PRE-002, ERR-001 |
| Unsupported YAML alias/anchor/tag | alias, anchor, tag YAML | `StrictYamlRejected`-class | unit/integration/proptest | PRE-002, ERR-001 |
| Unsupported YAML multi-doc | multi-document stream | `StrictYamlRejected`-class | unit/integration/proptest | PRE-002, ERR-001 |
| Source digest mismatch | source bytes with wrong source digest | `WorkflowSourceDigestMismatch`/`PayloadDigestMismatch` | unit/property/integration | PRE-003, ERR-002 |
| Artifact digest mismatch | artifact bytes with wrong artifact digest | `CompiledIrDigestMismatch` | unit/property/integration | PRE-004, ERR-003 |
| Missing accepted artifact | absent envelope | `AcceptedArtifactMissing` | integration/Kani | ERR-004 |
| Malformed or under-proven artifact | malformed envelope, insufficient gate/proof flags | `AcceptedArtifactInvalid` | integration/Kani | ERR-005 |
| Capability absent | artifact requires ungranted capability | `CapabilityMismatch` | integration/Kani | ERR-006 |
| Durability failure | storage write/flush failure before ack | `DurabilityFailure`; no runnable state | integration/E2E/TLA | ERR-007, INV-004 |
| Replay divergence | divergent journal/snapshot | `ReplayDivergence` | unit/property/integration | ERR-008 |
| Corrupt recovery data | corrupt frame/snapshot/journal | `CorruptRecoveryData`/mapped variant | unit/property/Miri | ERR-009 |
| No recovery data | unknown run/no durable evidence | `NoRecoveryData` | unit/property/integration | ERR-010 |
| Recovery YAML reparse | parser sentinel after admission or static forbidden dependency | contract failure/static impossibility evidence | E2E/static | ERR-011, POST-006 |
| Same persisted inputs twice | identical persisted source/artifact/journal/snapshot | same state or same typed error | proptest/integration | INV-006 |
| Journal lacks success prefix | missing RunAccepted/admission/runtime prefix | no synthesized success | integration/TLA | INV-007 |

## 10. Traceability to Required Commands

| Requirement IDs | Command |
|---|---|
| STRICT-YAML-012, ERR-STRICT-013 | `cargo test -p vb_compile -- --nocapture` |
| PROP-CORRUPT-006, ERR-SOURCE-014, ERR-ARTIFACT-DIGEST-015, ERR-REPLAY-020, ERR-CORRUPT-021, ERR-NO-DATA-022 | `cargo test -p vb_storage -- --nocapture` |
| ERR-ARTIFACT-MISSING-016, ERR-ARTIFACT-INVALID-017, ERR-CAPABILITY-018 | `cargo test -p vb_runtime -- --nocapture` |
| E2E-CLI-007, ERR-DURABILITY-019 | `cargo test -p velvet_ballastics --test cli_integration -- --nocapture` |
| E2E-REC-008 | `cargo test -p velvet-ballastics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture` |
| ERR-TAXONOMY-013-022 | chained focused commands from PO-011 with report mapping every ERR-001..ERR-011 |
| STATIC-BOUNDARY-009 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` plus static boundary report |
| KANI-ADMIT-023 | `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` |
| MIRI-CODEC-024 | `cargo +nightly miri test -p vb_storage` or approved waiver |
| GATE-RELEASE-025 | `moon ci` |

## 11. Completion Evidence

- Isolation verified before planning: working path is `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`, not `/home/lewis/src/velvet-ballistics` and not nested under it.
- Inputs read: `STATE.md`, approved `proof-review.md`, approved `contract-verification-review.md`, `contract.md`, `traceability-matrix.jsonl`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `delivery-scope.jsonl`.
- Scope honored: no production source, proof code, tests, dependencies, or CI files were edited; only this test-plan artifact and the State 7 transition evidence are written.
- Exit criteria: every public behavior has a BDD scenario; every digest/recovery/profile pure boundary has proptest invariants; every parser/codec boundary has fuzz/Miri treatment; every contract error variant ERR-001..ERR-011 has an explicit scenario; mutation threshold is stated; exact value/error assertions are required.

## 12. State 7 Repair Transition After State 9 Rejection

This section is an authoritative repair addendum to the plan above. If any earlier wording implies weaker density, fuzz, or red-suite handling, this section wins for the next State 8.

### Repair inputs and isolation evidence

- Rejection inputs read for this repair: `.beads/vb-core-yaml-e2e-chain/test-plan.md`, `test-plan-review.md`, `test-suite-review.md`, `test-repair-guide.md`, `test-writer-report.md`, and `contract.md` contract signatures at lines 82-88.
- Skill sources cited for this State 7 repair: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md`; contents match, and `.agents` wins on conflict. The testing philosophy reference read was `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`.
- Isolation command evidence: running from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain` printed `state7-repair-isolation-ok`; this is not `/home/lewis/src/velvet-ballistics` and not nested under it.
- Scope evidence: this repair edits only this plan artifact. It does not edit production code, tests, proofs, dependencies, or CI.

### Expected red behavior from the rejected strict accepted-artifact test

- Failing test to preserve exactly in State 8: `tests/vb_core_yaml_e2e_chain_contract.rs:74` `storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted`.
- Current red result from State 9: contract suite returns `4 passed; 1 failed`; raw blocker is `Error: "artifact checksum mismatch"`.
- This failure is expected red evidence, not an acceptable green outcome. State 8 must not weaken, delete, ignore, or invert the test.
- Valid YAML-origin strict acceptance contract for B08/I07/E01: `submit_artifact(&journal, &workflow_from_yaml, RuntimePolicy::Strict)` must return an accepted artifact, not `artifact checksum mismatch`; the artifact digest and verification digest must equal `workflow.digest()`, verification flags `durable`, `bounded`, `taint_safe`, `retry_safe`, and `replayable` must be true, and `verification.gate_count` must equal `REQUIRED_GATE_COUNT`.
- Error contract for invalid accepted-artifact cases remains exact: malformed/under-proven/gate-count mismatch artifacts must return exact storage/runtime invalid-artifact variants; source digest mismatch must exercise the public storage/source persistence or admission path and assert `WorkflowSourceDigestMismatch` or `PayloadDigestMismatch`, not merely prove two local digest values differ.

### Concrete unit-test density plan: 35 named tests minimum

State 8 must implement or map executable evidence for at least these 35 concrete unit/integration-facing tests, five per public contract signature. Existing tests may count only when the name and assertion satisfy the exact expected output below; `is_ok()`/`is_err()` alone never counts.

| Contract signature | Required concrete tests | Expected exact assertion contract |
|---|---|---|
| `validate_and_compile_yaml` | `validate_and_compile_yaml_returns_artifact_when_minimal_yaml_is_valid`; `validate_and_compile_yaml_rejects_duplicate_keys_with_strict_yaml_rejected`; `validate_and_compile_yaml_rejects_aliases_and_anchors_with_strict_yaml_rejected`; `validate_and_compile_yaml_rejects_explicit_tags_with_strict_yaml_rejected`; `validate_and_compile_yaml_rejects_multi_document_stream_with_strict_yaml_rejected` | Valid case asserts digest-bearing compiled artifact; invalid cases assert exact `StrictYamlRejected`-class variant and no artifact. |
| `persist_source_and_artifact` | `persist_source_and_artifact_persists_source_artifact_and_ref_when_digests_match`; `persist_source_and_artifact_returns_workflow_source_digest_mismatch_when_source_digest_differs`; `persist_source_and_artifact_returns_compiled_ir_digest_mismatch_when_artifact_digest_differs`; `persist_source_and_artifact_rejects_source_digest_used_as_artifact_digest_when_roles_differ`; `persist_source_and_artifact_returns_durability_failure_and_no_ref_when_flush_fails` | Success asserts durable source/artifact/ref fields; failures assert exact mapped variant and no accepted ref/ack. |
| `admit_strict_artifact_run` | `admit_strict_artifact_run_accepts_storage_produced_yaml_artifact_when_gate_count_matches_required`; `admit_strict_artifact_run_returns_accepted_artifact_missing_when_envelope_absent`; `admit_strict_artifact_run_returns_accepted_artifact_invalid_when_gate_count_under_required`; `admit_strict_artifact_run_returns_capability_mismatch_when_required_capability_ungranted`; `admit_strict_artifact_run_rejects_raw_workflow_parts_or_yaml_bypass_with_accepted_artifact_invalid` | Success asserts `RunAdmission` and accepted sequence; failures assert exact admission variant and no runnable state. |
| `append_strict_runtime_event` | `append_strict_runtime_event_appends_run_accepted_before_admission_when_ack_succeeds`; `append_strict_runtime_event_appends_terminal_or_suspended_event_after_admission`; `append_strict_runtime_event_returns_durability_failure_when_event_flush_fails_before_ack`; `append_strict_runtime_event_rejects_event_for_unadmitted_run_without_success_ack`; `append_strict_runtime_event_preserves_monotonic_event_sequence_for_same_run` | Ack cases assert durable sequence/order; failures assert `DurabilityFailure` or exact mapped rejection and no visible success. |
| `inspect_run` | `inspect_run_returns_digest_bound_status_when_required_journal_prefix_exists`; `inspect_run_returns_no_recovery_data_or_absent_status_when_run_has_no_evidence`; `inspect_run_does_not_synthesize_success_when_run_accepted_is_missing`; `inspect_run_reports_source_and_artifact_digest_roles_distinctly`; `inspect_run_reports_terminal_or_suspended_state_from_persisted_events_only` | Assertions inspect exact run id, source digest, artifact digest, accepted sequence, and status; no synthetic success allowed. |
| `events_for_run` | `events_for_run_returns_run_accepted_admission_and_terminal_events_in_order`; `events_for_run_returns_empty_or_exact_absent_error_when_no_journal_exists`; `events_for_run_does_not_include_success_when_admission_event_missing`; `events_for_run_preserves_digest_fields_without_role_swap`; `events_for_run_returns_corrupt_recovery_data_when_event_record_is_malformed` | Assertions compare ordered event kinds and digest fields exactly; corrupt/absent cases assert exact variant. |
| `recover_yaml_origin_run` | `recover_yaml_origin_run_recovers_state_from_persisted_artifact_journal_and_snapshot_without_yaml`; `recover_yaml_origin_run_returns_replay_divergence_when_snapshot_diverges_from_model`; `recover_yaml_origin_run_returns_corrupt_recovery_data_when_snapshot_or_frame_decode_fails`; `recover_yaml_origin_run_returns_no_recovery_data_when_no_durable_evidence_exists`; `recover_yaml_origin_run_is_deterministic_for_identical_persisted_inputs` | Success asserts recovered state refines persisted evidence and no YAML parser dependency; failures assert exact variant; deterministic case asserts identical state or identical error. |

### Fuzz execution repair plan

The earlier fuzz deferral is no longer sufficient for this bead. State 8 must either add executable fuzz targets or record a complete formal waiver with owner, expiry, compensating evidence, and commands. Preferred repair is executable fuzzing:

| Target | Required executable entry | Minimum command | Acceptance |
|---|---|---|---|
| Strict YAML profile | A `cargo-fuzz` target that feeds bytes/source text through strict YAML validation/compile. | `cargo fuzz run strict_yaml_profile -- -max_total_time=60` | No panic/OOM; unsupported duplicate key, alias/anchor, tag, multi-doc, invalid shape classes never produce an artifact. |
| Accepted artifact/postcard decode | A `cargo-fuzz` target that feeds bytes through accepted-artifact/postcard decode and digest/envelope validation. | `cargo fuzz run accepted_artifact_decode -- -max_total_time=60` | Malformed/truncated/swapped-digest bytes return exact malformed/invalid/digest mismatch class; no accepted artifact bypass. |
| Recovery frame/snapshot/journal decode | A `cargo-fuzz` target that feeds corrupt recovery frame/snapshot/journal bytes. | `cargo fuzz run recovery_decode -- -max_total_time=60` | Corrupt input returns `CorruptRecoveryData`, `CorruptSnapshot`, `ReplayDivergence`, or exact mapped failure; no recovered success or panic. |

If tooling cannot run in the environment, the waiver must name the owner `vb-core-yaml-e2e-chain`, expire no later than the next State 9 rerun, include the exact missing command and failure reason, and require compensating proptest plus Miri evidence for the same byte boundaries. A vague "target not discovered" waiver is rejected.

### Next State 8 suite changes mandated by this plan

1. Preserve the accepted-artifact red test and its exact assertions for digest, verification flags, and `REQUIRED_GATE_COUNT`; production must fix the `artifact checksum mismatch` blocker.
2. Add or map the 35 named tests above and report which executable test covers each name.
3. Replace the local-only digest proptest with, or supplement it by, a storage-facing property/integration test that calls the public persistence/admission path and asserts `WorkflowSourceDigestMismatch` or `PayloadDigestMismatch` exactly.
4. Add executable fuzz targets for strict YAML, accepted artifact/postcard decode, and recovery decode, or record the strict waiver described above.
5. Rerun State 9 from Tier 0 after State 8 repair; coverage and mutation review remain blocked until the strict accepted-artifact suite is green.

## Open Questions

- None blocking for State 7. Downstream State 8+ must implement or verify the planned tests and must not treat this plan as execution evidence.
