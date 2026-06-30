# Test Plan: vb-qi37.12 State 7 — Silent Discard Elimination

## Summary

- Startup doctrine read and applied: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md`; both require behavior-first public API planning, BDD scenarios, proptest/fuzz/Kani/mutation coverage, and exact-value/exact-error assertions. The `.agents` copy controls on conflict.
- Testing philosophy read and applied: `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`; plan tests behavior through public APIs, state not interactions, with integration as the widest layer.
- Review inputs accepted: `.beads/vb-qi37.12/proof-review.md` has `STATUS: APPROVED`; `.beads/vb-qi37.12/contract-verification-review.md` has `STATUS: APPROVED`.
- Contract inputs: `.beads/vb-qi37.12/contract.md`, `.beads/vb-qi37.12/traceability-matrix.jsonl`, `.beads/vb-qi37.12/proof-obligations.jsonl`, `.beads/vb-qi37.12/proof-obligations.planned.jsonl`, `.beads/vb-qi37.12/delivery-scope.jsonl`.
- Behaviors identified: 15 traceability-backed behaviors.
- Trophy allocation: minimum 36 unit/boundary tests / 8 integration scenario groups / 1 e2e-release gate / 5 static-or-formal gates. Integration remains widest by risk, but State 9 repair requires at least 5 unit-level exact tests per public contract signature; this plan allocates 6 per signature for the 6 signatures in `contract.md` lines 54-59.
- Proptest invariants: 7.
- Fuzz targets: 4.
- Kani harnesses: 0 required active lanes; 2 reopen-if-implementation-introduces-bounded-state candidates.
- Mutation threshold: minimum 90% killed overall; 100% killed for release-critical branches listed below.
- Hard assertion rule: no planned test may assert only `is_ok()` or `is_err()`; every assertion must match the exact success value, exact error variant, and required diagnostic fields.

## 1. Behavior Inventory

| ID | Contract / traceability | Behavior | Primary public API/surface | Layer |
|---|---|---|---|---|
| B01 | PRE-001 | Scoped production fallible sites classify every result-bearing operation as `must_propagate`, `must_accumulate`, `typed_optional`, or `typed_best_effort_discard` when the inventory gate runs. | quality inventory APIs, static scan report | static + integration |
| B02 | PRE-002 | Best-effort discard classification rejects release-critical durability, recovery, acknowledgement, or validation paths when the gate evaluates exceptions. | quality inventory APIs | unit + static |
| B03 | PRE-003 | Runtime, storage, and compiler fallible APIs return typed errors or accumulated errors when called with injected failures. | `FjallJournal`, `ProcessLock`, `Shard`, compiler validation APIs | integration |
| B04 | PRE-004 | Recovery decode surfaces return typed corruption errors and distinguish absent optional payloads when persisted bytes are corrupt or truncated. | `JournalEvent`, replay/recovery decode paths | unit + fuzz + integration |
| B05 | POST-001 | Strict journal/storage persist failure prevents success acknowledgement when a runtime/storage mutation attempts to commit. | `append_strict`, `append_strict_batch`, `persist_strict`, runtime journal append | integration + TLA linkage |
| B06 | POST-002 | Process lock open/flock/metadata failures return `JournalError::ProcessLockIo`, `JournalError::ProcessLockHeld`, or a documented non-critical best-effort metadata path. | `ProcessLock::acquire`, `FjallJournal::open` | integration |
| B07 | POST-003 | Engine drive, action, ask, wait, retry, cancel, resume, and terminal failure paths preserve causal diagnostics to caller-visible runtime errors and evidence. | `Shard::apply_drive_result`, resume/error conversion APIs | integration |
| B08 | POST-004 | Compiler validation accumulates multiple validation causes instead of converting any validation failure into success. | `validate_workflow_ast`, `validate_input_schemas`, strict YAML validation | unit + integration |
| B09 | POST-005 | Recovery-critical optional accessors cannot hide corrupt decode failures as successful absence. | `JournalEvent::slot_value` plus recovery decode/replay APIs | fuzz + integration |
| B10 | POST-006 | Deliberate discard exceptions are named, typed, inventoryable, and limited to non-critical best-effort scope. | quality inventory APIs, typed discard report | static + integration |
| B11 | INV-001 | Required persist failure cannot be followed by success ack or in-memory-only externally visible mutation. | storage/runtime mutation flow | integration + TLA linkage |
| B12 | INV-002 | Diagnostic envelope transformations preserve operation, boundary, optional run id, optional record kind, and source cause across boundaries. | `RuntimeError`, `ResumeError`, `JournalError`, compiler errors | unit + integration + Verus linkage |
| B13 | INV-003 | Corrupt, truncated, or inconsistent persisted data cannot hydrate as empty successful recovery state. | recovery/replay APIs and fuzz target | fuzz + integration + TLA/Verus linkage |
| B14 | INV-004 | Ignored results, `.ok()` erasures, wildcard errors, and log-and-continue paths fail unless justified by typed discard inventory. | focused static scan and report | static |
| B15 | INV-005 | Invalid YAML, IR schema, unsupported profile events, unresolved references, and invalid schemas are rejected with retained causes. | compiler public compile/validate APIs | unit + integration + static |

## 2. Trophy Allocation

| Behavior IDs | Layer | Required tests/gates | Rationale |
|---|---|---|---|
| B02, B04, B08, B12, B13, B15 | Unit / calc | Pure validation, decode classification, diagnostic mapping, compiler schema/reference/profile checks. | These are deterministic local transformations with exact error variants and boundary values. |
| B01, B03, B05, B06, B07, B09, B10, B11, B13, B15 | Integration | Real filesystem/temp Fjall journal, process lock contention, runtime shard failure injection, compiler end-to-end validation, inventory report validation. | Silent discard risk lives at component boundaries; real dependencies beat mocks. Use temp dirs/fakes only for controlled failure injection. |
| B01-B15 | Static/formal gates | `rg`/classified scan, clippy source gates, TLA, Verus, fuzz discovery, `moon ci` release gate. | Static/formal evidence proves absence of ignored-result classes beyond examples. |
| B03, B05, B07, B11, B15 | E2E/release | Canonical `moon ci`; one acceptance smoke covering compile → runtime/storage failure path if implementation exposes CLI/API seam. | Keep large tests few; final release gate is State 11 owner. |

Deviation from target ratio is intentional: this bead is release-critical boundary repair, so integration/static/formal evidence dominates. Unit tests still cover every pure decision and exact error taxonomy mapping.

## 3. BDD Scenarios

### B01 — Fallible-site classification covers all scoped production candidates

- Test name: `given_scoped_production_fallible_sites_when_inventory_runs_then_each_site_has_classification`
- Given: the scoped files from `delivery-scope.jsonl` and raw candidates for `let _ =`, `.ok()`, wildcard error matches, and logging continuations.
- When: the focused inventory scanner and classifier run.
- Then: every production candidate has exactly one disposition: `must_propagate`, `must_accumulate`, `typed_optional`, `typed_best_effort_discard`, or non-production/test-only.
- And: the report states `Unclassified release-critical silent discards: 0`.
- Assertions: compare exact candidate count/report fields, not a vague successful command status.
- Traceability: PRE-001; proofs `VERUS-CLS-003`, `SCAN-DISCARD-006`.

### B02 — Best-effort discard cannot cover release-critical paths

- Test name: `given_best_effort_discard_on_release_critical_path_when_gate_runs_then_gate_rejects`
- Given: an inventory record for journal persist, recovery decode, success acknowledgement, or compiler validation marked `typed_best_effort_discard`.
- When: the classification validator evaluates the record.
- Then: validation returns the exact typed classification error for release-critical best-effort misuse.
- And: operation name, path, rationale, and criticality are preserved in the diagnostic.
- Traceability: PRE-002; proofs `VERUS-CLS-003`, `SCAN-DISCARD-006`.

### B03 — Fallible runtime/storage/compiler APIs return typed errors

- Test name: `given_runtime_storage_compiler_failure_when_api_returns_then_result_is_typed_error_or_accumulated_error`
- Given: injected storage persist failure, runtime journal append failure, and compiler validation failures.
- When: each public boundary returns.
- Then: storage returns the exact `JournalError` variant, runtime returns exact `RuntimeError::StorageJournalAppend` or preserved runtime error, and compiler returns `CompileErrors` containing all expected causes.
- Traceability: PRE-003; proofs `VERUS-DIAG-004`, `TEST-JOURNAL-007`, `TEST-RUNTIME-008`.

### B04 — Corrupt recovery payload returns typed corruption

- Test name: `given_corrupt_recovery_payload_when_decoded_then_typed_corruption_error_is_returned`
- Given: corrupt/truncated bytes for a recovery-critical slot/event payload and a separate genuinely absent optional payload.
- When: recovery decode or replay runs.
- Then: corrupt or truncated slot/event payload bytes return exactly `JournalError::CorruptEventPayload` with decode/source context preserved; corrupt/missing/inconsistent replay data returns exactly `JournalError::ReplayCorruption`; no implementation-chosen third variant is acceptable for these cases.
- And: genuinely absent optional payload is distinguishable from corrupt bytes.
- Traceability: PRE-004; proofs `VERUS-DEC-005`, `FUZZ-DECODE-009`.

### B05 — Strict persist failure prevents success acknowledgement

- Test name: `given_strict_journal_persist_failure_when_runtime_mutates_then_no_success_ack_is_emitted`
- Given: a required mutation uses `append_strict`, `append_strict_batch`, or runtime journal append with a storage backend whose persist boundary fails.
- When: the mutation attempts to commit.
- Then: the caller receives exact typed persist failure.
- And: no success acknowledgement, terminal success state, index update, or recovery-visible success exists.
- Traceability: POST-001; proofs `TLA-ACK-001`, `TEST-JOURNAL-007`.

### B06 — Process lock failures surface typed lock diagnostics

- Test name: `given_process_lock_metadata_write_failure_when_acquire_runs_then_typed_lock_error_or_documented_best_effort_path_is_observed`
- Given: temp journal path scenarios for open failure, flock contention, non-would-block flock error, and PID metadata write/read failure.
- When: `ProcessLock::acquire` or `FjallJournal::open` runs.
- Then: open/flock failures return exact `JournalError::ProcessLockIo` or `JournalError::ProcessLockHeld` with path and holder metadata rules.
- And: PID metadata read/write failures are accepted only as documented non-critical best-effort metadata, never as lock acquisition failure concealment.
- Traceability: POST-002; proof `SCAN-DISCARD-006`.

### B07 — Engine-drive failure preserves cause

- Test name: `given_engine_drive_error_when_apply_drive_result_handles_it_then_terminal_failure_retains_cause`
- Given: runtime drive returns an engine error during action/resume/ask/wait/cancel/terminal flow.
- When: the shard applies the drive result and flushes evidence.
- Then: caller-visible runtime result preserves the cause as `RuntimeError::EngineDriveFailed` or a more specific typed diagnostic.
- And: journal/evidence retains run id, operation, record kind when applicable, and source cause.
- Traceability: POST-003; proofs `TLA-ACK-001`, `VERUS-DIAG-004`, `TEST-RUNTIME-008`.

### B08 — Compiler validation accumulates all causes

- Test name: `given_multiple_compiler_validation_errors_when_validation_runs_then_all_causes_remain_in_compile_errors`
- Given: a workflow AST/YAML with multiple independent schema/reference/profile validation failures.
- When: compiler validation runs.
- Then: `CompileErrors` contains every expected `CompileError` variant, in deterministic order where the API promises order.
- And: no error is dropped by early success, `.ok()`, or wildcard suppression.
- Traceability: POST-004; proof `SCAN-DISCARD-006`.

### B09 — Optional accessors do not hide recovery corruption

- Test name: `given_optional_accessor_decode_failure_when_used_by_recovery_then_recovery_rejects_typed_corruption`
- Given: a `SlotWrittenEvent` with `value: Some(corrupt_bytes)` on a recovery-critical path and `value: None` on an optional non-critical path.
- When: recovery/replay hydrates state.
- Then: corrupt bytes reject with exactly `JournalError::CorruptEventPayload` on the accessor/decode boundary or exactly `JournalError::ReplayCorruption` after replay/summary reconstruction; absent optional payload returns only the documented successful absence path, never a corruption variant.
- And: no empty successful recovery frame is returned for corrupt bytes.
- Traceability: POST-005; proofs `TLA-REC-002`, `VERUS-DEC-005`, `FUZZ-DECODE-009`.

### B10 — Typed discard exceptions are mechanically inventoryable

- Test name: `given_documented_typed_discard_exception_when_inventory_runs_then_exception_contains_operation_rationale_and_noncritical_scope`
- Given: known non-critical best-effort metadata paths such as process-lock PID metadata read/write.
- When: inventory classification report is rendered.
- Then: each exception names operation, path, rationale, non-critical scope, and why durability/recovery/validation is unaffected.
- Traceability: POST-006; proofs `VERUS-CLS-003`, `SCAN-DISCARD-006`, `GATE-RELEASE-010`.

### B11 — Required persist failure cannot acknowledge

- Test name: `given_required_persist_fails_when_mutation_attempts_ack_then_ack_is_absent_and_typed_error_returned`
- Given: required persist returns failure after mutation staging but before acknowledgement.
- When: runtime/storage caller observes the result and recovery reads persisted state.
- Then: the result is typed failure and recovery cannot observe acknowledged success for the failed mutation.
- Traceability: INV-001; proofs `TLA-ACK-001`, `TLA-DEADLOCK-011`, `TEST-JOURNAL-007`.

### B12 — Diagnostic fields survive boundary conversion

- Test name: `given_storage_runtime_compiler_error_conversion_when_error_crosses_boundary_then_required_context_is_preserved`
- Given: storage, runtime, resume, and compiler errors with operation, boundary, run id, record kind, and source cause populated where applicable.
- When: each error crosses its public conversion boundary.
- Then: exact required fields remain observable in typed variants and evidence.
- Traceability: INV-002; proofs `VERUS-DIAG-004`, `TEST-RUNTIME-008`.

### B13 — Corrupt persisted data fails closed

- Test name: `given_corrupt_or_truncated_persisted_data_when_recovery_runs_then_empty_success_frame_is_not_returned`
- Given: corrupt/truncated event payloads, corrupt slot values, inconsistent sequence data, and missing required recovery data.
- When: recovery summary/frame-seed construction or journal replay runs.
- Then: corrupt/truncated decode returns exactly `JournalError::CorruptEventPayload`; replay inconsistency, missing required recovery data, sequence gaps, or mixed run data returns exactly `JournalError::ReplayCorruption`; no silent empty success is returned for corruption and no wildcard recovery error class is accepted.
- Traceability: INV-003; proofs `TLA-REC-002`, `VERUS-DEC-005`, `FUZZ-DECODE-009`.

### B14 — Static gate rejects unclassified ignored results

- Test name: `given_source_contains_ignored_result_when_static_gate_runs_then_unclassified_discard_fails`
- Given: a controlled fixture or synthetic inventory entry containing unclassified `let _ =`, `.ok()`, wildcard error, or log-and-continue on a production fallible result.
- When: the static gate runs.
- Then: the gate returns exact failure naming path, line/pattern, and missing classification.
- Traceability: INV-004; proofs `VERUS-CLS-003`, `SCAN-DISCARD-006`.

### B15 — Invalid compiler inputs never become success

- Test name: `given_invalid_yaml_ir_schema_or_reference_when_compiler_validates_then_errors_are_not_dropped`
- Given: YAML alias/tag/multi-document cases, malformed schema bounds, invalid defaults, unresolved references, unsupported accessors, unsupported profile events, and invalid IR schemas.
- When: compiler validation runs through public compile/validate APIs.
- Then: exact `CompileError` variants/codes are returned and accumulated where multiple independent errors exist.
- Traceability: INV-005; proofs `SCAN-DISCARD-006`, `GATE-RELEASE-010`.

## 4. Unit Test Plan

| Group | Target behavior | Inputs | Exact assertions | Traceability |
|---|---|---|---|---|
| Classification lattice | B01, B02, B10, B14 | all classification enum values; release-critical vs non-critical operations | exact accepted/rejected classification and error payload fields | PRE-001, PRE-002, POST-006, INV-004 |
| Diagnostic conversion | B03, B07, B12 | `JournalError`, `ResumeError`, runtime engine errors, compiler errors | exact `RuntimeError`/`ResumeError` variants and preserved source fields | PRE-003, POST-003, INV-002 |
| Recovery decode classification | B04, B09, B13 | absent bytes, valid bytes, corrupt bytes, truncated bytes | absence vs corrupt distinction; exact typed corruption/no-data/replay error | PRE-004, POST-005, INV-003 |
| Recovery summary/frame seed | B13 | empty event list, mixed run ids, corrupt slot bytes, unsupported object/list slots, max slot/step boundaries | exact `RecoveryError` variants or explicit unsupported flags; no unwrap/expect in new tests | INV-003 |
| Compiler profile/schema/reference validation | B08, B15 | alias, tag, empty doc, multi-doc, unknown schema fields, invalid bounds/defaults, invalid references/accessors | exact `CompileError`/`CompileErrors` variants/codes and count | POST-004, INV-005 |

## 5. Integration Test Plan

| Scenario | Setup | Action | Required observable outcome | Traceability |
|---|---|---|---|---|
| Strict journal persist failure | temp Fjall journal or failure-injection adapter at strict persist boundary | call `append_strict`/`append_strict_batch`/`persist_strict` | exact `JournalError::StoragePersistFailed` or current storage typed error; no durable ack/index success | POST-001, INV-001 |
| Empty strict batch | real temp journal | `append_strict_batch(&[])` | exact `Ok(())`, no persist call requirement, no event written | POST-001 |
| Process lock contention | two handles to same temp db path | acquire lock twice | second returns exact `JournalError::ProcessLockHeld` with path and holder PID optionality rules | POST-002 |
| Process lock open failure | path with unwritable/malformed lock location | acquire lock | exact `JournalError::ProcessLockIo` with path/source | POST-002 |
| Journal replay corruption | persisted malformed record or corrupt event payload fixture | `events_for_run` / recovery replay | exact decode/replay corruption; no empty vector success for corrupt data | PRE-004, INV-003 |
| Runtime journal append failure | shard with journal append failure injected | action/ask/wait/resume/cancel terminal transition | exact runtime typed error and evidence retaining run/operation/source | PRE-003, POST-003, INV-002 |
| Runtime engine drive failure | deterministic engine failure path | apply/drive run | exact terminal failure diagnostic; no dropped source cause | POST-003, INV-002 |
| Compiler whole-input validation | public compiler API with invalid YAML/schema/references | compile/validate | exact `CompileErrors` list with every expected cause | POST-004, INV-005 |
| Inventory report validation | full scoped scan output | render/validate classified report | exact candidate counts and zero unclassified release-critical silent discards | PRE-001, POST-006, INV-004 |

## 6. Proptest Invariants

### P01 — Classification lattice admits no implicit discard
- Invariant: for any production fallible site, `accepted(site)` implies classification is not unclassified and not release-critical best-effort discard.
- Strategy: generate operation criticality, crate/surface, result-discard pattern, classification enum.
- Anti-invariant: release-critical site with `typed_best_effort_discard` must always fail.
- Traceability: PRE-001, PRE-002, POST-006, INV-004.

### P02 — Diagnostic envelopes preserve required fields
- Invariant: converting storage/resume/runtime/compiler diagnostic envelopes preserves operation, boundary, optional run id, optional record kind, and source cause when present.
- Strategy: generate envelope structs with optional fields and source labels.
- Anti-invariant: transformations that clear source or boundary must fail equality/property assertion.
- Traceability: PRE-003, POST-003, INV-002.

### P03 — Decode class never conflates corrupt with absent
- Invariant: any byte class generated as corrupt/truncated maps to typed corruption, never successful absence.
- Strategy: generate enum class `{Absent, Valid(encoded SlotValue), Corrupt(bytes), Truncated(prefix)}`.
- Anti-invariant: corrupt/truncated input producing `Ok(None)` or empty success fails.
- Traceability: PRE-004, POST-005, INV-003.

### P04 — Recovery summaries are run-homogeneous
- Invariant: any event list containing more than one `RunId` returns exact replay divergence, never a merged summary.
- Strategy: non-empty event vectors with controlled one-run/two-run distributions.
- Anti-invariant: mixed run ids returning `RecoveryHydration::Summary` fails.
- Traceability: INV-003.

### P05 — Compiler validation is monotonic over independent errors
- Invariant: adding an independent invalid schema/reference/profile construct does not remove existing diagnostic causes.
- Strategy: compose invalid YAML/AST fragments with tagged expected errors.
- Anti-invariant: error count decreases or an expected variant disappears.
- Traceability: POST-004, INV-005.

### P06 — Static scan classification is total over raw candidates
- Invariant: for any raw candidate record from the scoped pattern grammar, validation either classifies it or emits exact unclassified error; no candidate is dropped from the report.
- Strategy: generate candidate records with path, line, pattern, crate, production/test flag.
- Anti-invariant: report length less than raw candidate length fails.
- Traceability: PRE-001, INV-004.

### P07 — Sequence replay preserves contiguous order or errors exactly
- Invariant: event replay with contiguous sequences yields ordered events; gaps, corrupt records, or wrong run ids produce exact typed replay/decode error.
- Strategy: generate per-run event sequences with valid/gap/out-of-order/corrupt classes.
- Anti-invariant: gap/corrupt class yielding success fails.
- Traceability: POST-005, INV-003.

## 7. Fuzz Targets

### F01 — `vb_qi37_12_persisted_payload_decode`
- Input type: bytes.
- Risk: panic, OOM, corrupt payload hydrating as empty success, decode error erased by `Option`.
- Corpus seeds: empty bytes, single byte, valid encoded `SlotValue`, truncated valid encoding, random high-bit bytes, oversized-but-bounded payload, legacy slot payload without extra taint.
- Oracle: no panic/sanitizer crash; corrupt/truncated classes return typed error or explicit unsupported state, never empty successful recovery.
- Traceability: PRE-004, POST-005, INV-003; obligation `FUZZ-DECODE-009`.

### F02 — Strict YAML profile parser
- Input type: UTF-8-ish bytes converted to string/lossy fixture where API accepts `&str`.
- Risk: parser event errors dropped, aliases/tags/multi-doc accepted, empty source accepted.
- Corpus seeds: alias, anchor, tag, empty string, multi-doc stream, deeply nested sequence/mapping within configured resource bounds.
- Traceability: POST-004, INV-005.

### F03 — Compiler reference strings
- Input type: arbitrary strings embedded in AST expressions/references.
- Risk: unknown roots/names/accessors accepted or mapped to less informative diagnostic.
- Corpus seeds: `$slot.0`, `$slot.x`, `$slot.1.0`, `$slot.1.name`, `$vars.data.field`, `$unknown.x`, bare references, empty segments.
- Traceability: POST-004, INV-005.

### F04 — Inventory raw scan line parser/classifier
- Input type: strings/records representing raw scan lines.
- Risk: malformed scan lines dropped, production paths misclassified as test-only, release-critical discard exception accepted without rationale.
- Corpus seeds: lines with `let _ =`, `.ok()`, `Err(_)`, `tracing::error!`, paths under `crates/vb_storage/src`, `crates/vb_runtime/src`, `crates/vb_compile/src`, and `tests`.
- Traceability: PRE-001, PRE-002, POST-006, INV-004.

## 8. Kani Harness Plan

Active State 7 decision: no Kani harness is required by the approved proof plan. `proof-obligations.planned.jsonl` marks `NA-KANI-012` as not applicable because Verus plus TLA/static/fuzz/test evidence owns the finite kernels.

Reopen triggers and candidate harnesses if State 8 implementation introduces bounded arithmetic/state code not already covered by Verus:

| Candidate | Property | Bound | Rationale | Mapped clauses |
|---|---|---|---|---|
| `vb_qi37_12_discard_classification_totality` | every bounded classification enum value either rejects or maps to exactly one accepted typed class; no implicit discard state | all enum variants, bounded criticality/path class | Useful if classification implementation moves from report-only to Rust enum logic. | PRE-001, PRE-002, INV-004 |
| `vb_qi37_12_decode_class_not_absent_for_corrupt` | corrupt/truncated decode class cannot equal absent success | bounded byte class enum, small byte arrays | Useful if concrete decode classifier gains hand-written branching not proven by Verus. | PRE-004, POST-005, INV-003 |

If reopened, Kani evidence must be exact `cargo kani --harness <name>` output and must not replace fuzzing or integration tests.

## 9. Static and Formal Gates

| Gate | Command / evidence | Required assertion | Traceability |
|---|---|---|---|
| Artifact presence | `test -s` for contract, reviews, traceability, obligations, delivery scope, test plan | all required artifacts non-empty | State 7 gate |
| JSONL validity | `jq -c .` over traceability, proof obligations, planned obligations, delivery scope | parse exits 0 | State 7 gate |
| Approved reviews | exact one `STATUS: APPROVED` in proof and contract-verification reviews | both approvals present | State 7 prerequisite |
| Silent discard scan | focused scan + classified report | 690 candidates classified and zero unclassified release-critical silent discards, or updated exact counts with zero unclassified after implementation deltas | PRE-001, PRE-002, POST-006, INV-004 |
| TLA | `TMPDIR=target/tmp tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla` | no invariant/temporal/deadlock error; no explicit `Stutter`, no `CHECK_DEADLOCK FALSE` | POST-001, INV-001, INV-003 |
| Verus | three exact Verus commands from obligations | `1 verified, 0 errors` for classification, diagnostic, decode kernels | PRE-001, PRE-004, INV-002, INV-003, INV-004 |
| Focused storage tests | `rtk cargo test -p vb_storage decode_rejects -- --nocapture`; `rtk cargo test -p vb_storage process_lock -- --nocapture` | exact pass counts may update, but all tests pass and assertions match exact variants | POST-001, POST-002, INV-001, INV-003 |
| Focused runtime tests | `rtk cargo test -p vb_runtime diagnostic -- --nocapture` | all tests pass and diagnostics preserve source fields | PRE-003, POST-003, INV-002 |
| Fuzz | `cargo fuzz list`; `cargo fuzz run vb_qi37_12_persisted_payload_decode --target x86_64-unknown-linux-gnu -- -runs=1000` with absolute temp/target if needed | target listed; 1000 runs no crash; corrupt/truncated oracles enforced | PRE-004, POST-005, INV-003 |
| Release | `moon ci` | exits 0 after State 8+ implementation/test artifacts are complete | POST-006, INV-004, INV-005 |

## 10. Mutation Checkpoints

Threshold: `cargo-mutants` must kill at least 90% of mutants overall for touched crates/modules and 100% of listed release-critical branch mutants. Surviving mutants in listed branches are blockers unless documented as equivalent mutants by reviewer.

| Mutated branch/operator | Must be caught by scenario | Expected kill reason |
|---|---|---|
| `append_strict` ignores `persist_strict()` result | B05/B11 | test expects exact persist error and no ack |
| `append_strict_batch` skips persist for non-empty batch | B05/B11 | durable event/ack invariant fails |
| process lock maps all flock failures to success | B06 | contention/open failure exact variant missing |
| PID metadata best-effort path becomes release-critical success claim | B06/B10 | inventory and process-lock assertions reject critical discard |
| `JournalEvent::slot_value` / recovery decode maps postcard error to absent on recovery path | B04/B09/B13 | corrupt bytes must typed-error or unsupported, not absent success |
| `events_for_run` ignores `decode_record` error | B13 | corrupt persisted record must fail closed |
| runtime `From<JournalError>` loses source | B03/B12 | required source field missing from `RuntimeError::StorageJournalAppend` |
| `ResumeError::JournalAppendFailedWithSource` maps to generic error | B07/B12 | exact source runtime error no longer preserved |
| `apply_drive_result` converts engine error to success/continue | B07 | terminal failure diagnostic missing |
| compiler schema validation returns `Ok(())` when errors vector non-empty | B08/B15 | exact `CompileErrors` count/variants missing |
| YAML alias/tag/multi-doc rejection returns success | B15/F02 | exact profile error missing |
| reference validator accepts unsupported accessor or unknown root | B15/F03 | exact reference error missing |
| inventory classifier drops raw candidate from report | B01/B14/P06 | candidate count and zero-unclassified proof fail |

## 11. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Test layer | Clauses |
|---|---|---|---|---|
| Classification happy path | `must_propagate` critical site | accepted with exact classification | unit/property | PRE-001 |
| Classification release-critical best-effort | critical site + `typed_best_effort_discard` | exact classification rejection | unit/property/static | PRE-002, INV-004 |
| Classification non-critical best-effort | PID metadata read/write with rationale | accepted and rendered with operation/rationale | integration/static | POST-006 |
| Strict append success | valid event + successful persist | exact success and durable event visible | integration | POST-001 |
| Strict append persist failure | valid event + persist failure | exact `JournalError` and no ack | integration | POST-001, INV-001 |
| Empty strict batch | no events | exact `Ok(())`, no required persist | unit/integration | POST-001 |
| Process lock acquired | free temp db lock | exact acquired guard | integration | POST-002 |
| Process lock held | second nonblocking lock | exact `ProcessLockHeld` | integration | POST-002 |
| Process lock I/O fail | lock path cannot open/flock | exact `ProcessLockIo` | integration | POST-002 |
| Decode absent | `value: None` optional non-critical | documented absent path | unit | PRE-004, POST-005 |
| Decode valid | postcard-encoded supported slot | exact `SlotValue`/taint | unit/integration | INV-003 |
| Decode corrupt/truncated | invalid bytes | exact corruption/replay error or unsupported flag; never empty success | unit/fuzz/integration | PRE-004, INV-003 |
| Recovery empty events | empty slice | exact `RecoveryError::NoRecoveryData { run: RunId::new(0) }` | unit | INV-003 |
| Recovery mixed runs | events with two run ids | exact `ReplayDivergence` detail | unit/property | INV-003 |
| Runtime storage error conversion | `JournalError` source | exact `RuntimeError::StorageJournalAppend { source }` | unit/integration | PRE-003, INV-002 |
| Resume source preservation | `JournalAppendFailedWithSource` | exact original runtime error | unit/integration | POST-003, INV-002 |
| Engine drive failure | `RuntimeEngineResult::Err` | exact terminal failure diagnostic/evidence | integration | POST-003 |
| Compiler valid input | valid YAML/schema/references | compiled/validated success with expected object, not merely success status | integration | POST-004 |
| Compiler multiple invalid inputs | independent invalid schema/reference/profile fields | `CompileErrors` exact count and variants | unit/integration/property | POST-004, INV-005 |
| Static scan clean report | current scoped source | exact classified count and zero release-critical unclassified | static | PRE-001, INV-004 |
| Static scan bad fixture | synthetic unclassified ignored result | exact gate failure with path/pattern | static/integration | INV-004 |

## 12. Traceability Coverage Matrix

| Contract clause | Required scenarios/gates | Proof/obligation linkage |
|---|---|---|
| PRE-001 | B01, P01, P06, static scan | `VERUS-CLS-003`, `SCAN-DISCARD-006` |
| PRE-002 | B02, P01, mutation classification critical branch | `VERUS-CLS-003`, `SCAN-DISCARD-006` |
| PRE-003 | B03, B12, runtime/storage/compiler integration | `VERUS-DIAG-004`, `TEST-JOURNAL-007`, `TEST-RUNTIME-008` |
| PRE-004 | B04, F01, P03, Kani reopen candidate | `VERUS-DEC-005`, `FUZZ-DECODE-009` |
| POST-001 | B05, B11, strict journal integration, TLA gate | `TLA-ACK-001`, `TEST-JOURNAL-007` |
| POST-002 | B06, process-lock integration, metadata discard inventory | `SCAN-DISCARD-006` |
| POST-003 | B07, B12, runtime diagnostic integration | `TLA-ACK-001`, `VERUS-DIAG-004`, `TEST-RUNTIME-008` |
| POST-004 | B08, B15, compiler validation unit/integration/property | `SCAN-DISCARD-006` |
| POST-005 | B09, B13, F01, P03 | `TLA-REC-002`, `VERUS-DEC-005`, `FUZZ-DECODE-009` |
| POST-006 | B10, P01, P06, static scan, release gate | `VERUS-CLS-003`, `SCAN-DISCARD-006`, `GATE-RELEASE-010` |
| INV-001 | B05, B11, TLA, strict persist integration | `TLA-ACK-001`, `TLA-DEADLOCK-011`, `TEST-JOURNAL-007` |
| INV-002 | B07, B12, P02, runtime diagnostic tests | `VERUS-DIAG-004`, `TEST-RUNTIME-008` |
| INV-003 | B04, B09, B13, F01, P03, P04, P07 | `TLA-REC-002`, `VERUS-DEC-005`, `FUZZ-DECODE-009` |
| INV-004 | B01, B02, B10, B14, P01, P06, static scan, mutation | `VERUS-CLS-003`, `SCAN-DISCARD-006` |
| INV-005 | B08, B15, F02, F03, P05, release gate | `SCAN-DISCARD-006`, `GATE-RELEASE-010` |

## 13. Test Implementation Constraints For State 8+

- Use public APIs and component boundaries; do not test private implementation details unless a private pure function is the only calc seam and cannot be reached otherwise.
- Prefer real temp filesystem/Fjall dependencies; use failure-injection fakes only where real I/O failure is nondeterministic or impossible to induce hermetically.
- No sleeps. Use deterministic polling/events/channels only if async/concurrent behavior is introduced.
- Every test name must be behavior sentence style.
- Every error-path assertion must match exact variant and relevant fields.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic in new tests unless repository test policy explicitly allows and reviewer accepts; prefer pattern matching and explicit failure messages through test result returns.
- Existing tests that use weaker assertions should not be copied as style precedent; this plan is the gate.

## 14. State 7 Repair Addendum After State 9 Rejection

### 14.1 Repair Transition

- Re-entry reason: State 9 rejected the original State 7 plan for underallocated unit/boundary coverage, incomplete per-signature boundary matrices, flexible recovery error wording, mutation checkpoints mapped only to scenario families, hollow proptest allowance, plan-parity gaps, weak assertion debt, and fuzz oracle wildcard acceptance.
- Repair rule: this addendum supersedes any earlier lower-count or flexible wording in this plan. If a prior row says a recovery error may be implementation-defined, the repaired exact taxonomy below wins.
- Scope: plan artifact only. No production code, tests, fuzz harnesses, proof source, dependency files, or CI files are to be edited in State 7 repair.

### 14.2 Minimum Unit/Boundary Allocation

- Public contract signatures: 6 (`classify_fallible_site`, `close_or_persist_strict`, `acquire_process_lock`, `decode_recovery_slot_value`, `apply_drive_result`, `validate_workflow_ast`).
- Reviewer floor: at least 5 unit-level exact tests per signature = 30.
- Repaired allocation: 36 named unit/boundary tests, 6 per signature. Integration tests remain required for real filesystem/Fjall/runtime/compiler seams, but they do not count toward the 36-test floor.
- Assertion floor: every named test below must assert an exact success value or exact error variant plus required fields. Bare `is_ok()`, bare `is_err()`, wildcard success/error acceptance, and tautological equality are forbidden.

### 14.3 Per-Signature Boundary Matrix — 36 Required Unit/Boundary Tests

#### `classify_fallible_site(site: FallibleSite) -> Result<DiscardClassification, ContractError>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `classify_fallible_site_returns_must_propagate_when_release_critical_persist_result_is_checked` | minimum valid critical classification | production storage persist site, classification `must_propagate` | `Ok(DiscardClassification::MustPropagate)` and diagnostic report fields preserve path/operation/criticality | PRE-001, INV-004 |
| `classify_fallible_site_rejects_best_effort_when_release_critical_recovery_decode_is_marked_discard` | one-above allowed discard criticality | production recovery decode site, classification `typed_best_effort_discard` | exact `Err(ContractError::ReleaseCriticalBestEffortDiscard { operation, path, criticality })` or repository equivalent named contract error; no success | PRE-002, INV-004 |
| `classify_fallible_site_accepts_best_effort_when_noncritical_pid_metadata_has_rationale` | maximum valid best-effort | process-lock PID metadata read/write, non-critical, named rationale | `Ok(DiscardClassification::TypedBestEffortDiscard { operation, rationale, noncritical_scope })` | POST-006 |
| `classify_fallible_site_rejects_unclassified_when_production_candidate_has_no_disposition` | empty/None disposition | production candidate with no classification | exact unclassified contract error naming path, line/pattern, crate, and operation | PRE-001, INV-004 |
| `classify_fallible_site_marks_test_only_when_candidate_is_outside_production_scope` | one-below production scope | test fixture path with discard pattern | exact non-production/test-only disposition; must not decrement production classified count | PRE-001 |
| `classify_fallible_site_rejects_overflow_when_candidate_line_or_count_exceeds_report_bound` | overflow/resource bound | maximum line/count plus one or generated oversized candidate metadata | exact bounds/invalid-candidate error; no wraparound, saturation success, or dropped report row | PRE-001, INV-004 |

#### `close_or_persist_strict(journal: &mut FjallJournal) -> Result<(), JournalError>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `close_or_persist_strict_returns_unit_when_dirty_journal_persist_succeeds` | valid success | dirty journal with persist success | exact `Ok(())` and durable event/index visible through public read seam | POST-001, INV-001 |
| `close_or_persist_strict_returns_unit_when_journal_has_no_pending_required_write` | empty/zero | clean journal / empty strict batch | exact `Ok(())`; no event written and no acknowledgement invented | POST-001 |
| `close_or_persist_strict_returns_storage_persist_failed_when_flush_fails` | one-above success/failure boundary | injected strict persist/flush failure | exact `Err(JournalError::StoragePersistFailed { .. })` preserving storage boundary/source | POST-001, INV-001 |
| `close_or_persist_strict_does_not_ack_when_persist_fails_after_mutation_staging` | required fail-closed | staged mutation then persist failure | exact storage error and no success ack, terminal success, index update, or recovery-visible success | POST-001, INV-001 |
| `close_or_persist_strict_preserves_error_when_persist_fails_at_max_valid_batch` | max valid | maximum supported strict batch size with final persist failure | exact `StoragePersistFailed`; no partial success acknowledgement and no arithmetic/index overflow | POST-001, INV-001 |
| `close_or_persist_strict_rejects_or_errors_without_ack_when_batch_exceeds_supported_bound` | overflow/one-above max | max supported batch plus one, or configured oversized write | exact capacity/storage error if API bounds exist, otherwise exact `StoragePersistFailed`; never `Ok(())` with dropped records | POST-001, INV-001 |

#### `acquire_process_lock(db_path: &Path) -> Result<ProcessLock, JournalError>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `acquire_process_lock_returns_guard_when_lock_file_can_be_opened_and_flocked` | valid success | temp db path with free lock | exact `Ok(ProcessLock)` observable by second acquisition failing held | POST-002 |
| `acquire_process_lock_returns_process_lock_held_when_lock_is_already_held` | contention max valid lock state | second nonblocking acquire on same path | exact `Err(JournalError::ProcessLockHeld { path, holder_pid })`; holder PID may be `None` only through typed optional metadata path | POST-002 |
| `acquire_process_lock_returns_process_lock_io_when_lock_file_open_fails` | one-below filesystem permission/path validity | unwritable/malformed lock path | exact `Err(JournalError::ProcessLockIo { path, source })` | POST-002 |
| `acquire_process_lock_returns_process_lock_io_when_flock_returns_non_would_block_error` | non-contention I/O error | flock/open handle returns I/O error other than would-block | exact `ProcessLockIo`, not held and not success | POST-002 |
| `acquire_process_lock_accepts_noncritical_metadata_read_failure_only_as_typed_optional` | metadata best-effort boundary | lock acquired but PID metadata read is invalid/unreadable | lock success may occur only with typed optional metadata absence; report must classify it non-critical | POST-002, POST-006 |
| `acquire_process_lock_never_claims_full_metadata_success_when_pid_metadata_write_fails` | write failure/overflow metadata | PID metadata write/truncate/rewind failure | exact `ProcessLockIo` for critical metadata write, or explicitly inventoried non-critical best-effort result; never full metadata success | POST-002, POST-006 |

#### `decode_recovery_slot_value(event: &JournalEvent) -> Result<Option<SlotValue>, JournalError>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `decode_recovery_slot_value_returns_none_when_optional_payload_is_absent` | None/empty optional | `value: None` on documented optional non-critical path | exact `Ok(None)` only for absence, with no corruption diagnostic | PRE-004, POST-005 |
| `decode_recovery_slot_value_returns_slot_value_when_payload_is_valid_minimal` | minimum valid | smallest valid encoded `SlotValue` | exact `Ok(Some(expected_slot_value))` including taint/value fields | PRE-004, INV-003 |
| `decode_recovery_slot_value_returns_slot_value_when_payload_is_valid_max_supported` | maximum valid | largest supported bounded encoded `SlotValue` | exact `Ok(Some(expected_slot_value))`; no truncation, overflow, or field loss | INV-003 |
| `decode_recovery_slot_value_returns_corrupt_event_payload_when_bytes_are_corrupt` | corrupt one-above valid encoding | non-postcard/random/high-bit bytes | exact `Err(JournalError::CorruptEventPayload { .. })` with decode source/context; never `Ok(None)` | PRE-004, POST-005, INV-003 |
| `decode_recovery_slot_value_returns_corrupt_event_payload_when_valid_encoding_is_truncated` | one-below valid length | every prefix shorter than a known valid encoding | exact `CorruptEventPayload`; never absent/success | PRE-004, POST-005, INV-003 |
| `decode_recovery_slot_value_rejects_oversized_payload_without_empty_success` | overflow/resource bound | oversized-but-bounded payload above decode limit or generated large garbage | exact `CorruptEventPayload` or documented size-limit `JournalError`; never `Ok(None)` and never panic/OOM | INV-003 |

#### `apply_drive_result(run: RunId, state: RunState, result: RuntimeEngineResult<RuntimeSignal>) -> RuntimeResult<()>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `apply_drive_result_returns_unit_when_engine_signal_is_valid_for_current_state` | valid success | current run/state plus valid continue/terminal signal | exact `Ok(())` and evidence/journal fields match expected run/operation, not just success | POST-003, INV-002 |
| `apply_drive_result_returns_engine_drive_failed_when_engine_result_is_error` | one-above success | deterministic engine error | exact `Err(RuntimeError::EngineDriveFailed { run, source, .. })` or public runtime equivalent preserving source | POST-003, INV-002 |
| `apply_drive_result_returns_storage_journal_append_when_evidence_persist_fails` | persist failure after runtime transition | runtime signal requires journal/evidence append and storage fails | exact `Err(RuntimeError::StorageJournalAppend { source })`; no terminal success emitted | PRE-003, INV-001, INV-002 |
| `apply_drive_result_preserves_cancel_retry_resume_wait_action_cause_fields` | action-kind matrix max valid enum | action/ask/wait/retry/cancel/resume/terminal failure variants | exact runtime diagnostic keeps run id, operation, record kind where applicable, and source cause | POST-003, INV-002 |
| `apply_drive_result_rejects_mismatched_run_or_state_without_success_ack` | one-below valid state relation | run/state mismatch or invalid transition | exact typed runtime state/drive error; no success ack or evidence claiming success | POST-003, INV-001 |
| `apply_drive_result_never_overflows_or_wraps_when_run_or_step_is_at_boundary` | overflow/underflow | maximum run/step/attempt counters if exposed by public types | exact typed bounds/state error or valid bounded success; no wraparound to success/zero | POST-003, INV-002 |

#### `validate_workflow_ast(ast: WorkflowAst) -> Result<ValidatedWorkflow, CompileErrors>`

| Required test name | Boundary class | Input shape | Exact assertion | Clauses |
|---|---|---|---|---|
| `validate_workflow_ast_returns_validated_workflow_when_ast_is_minimal_valid` | minimum valid | minimal supported workflow AST | exact `Ok(ValidatedWorkflow { .. })` fields expected by contract, not only success | POST-004, INV-005 |
| `validate_workflow_ast_returns_validated_workflow_when_ast_is_max_supported_valid` | maximum valid | max supported actions/schemas/references within configured bounds | exact validated field counts/order; no overflow/truncation | POST-004, INV-005 |
| `validate_workflow_ast_returns_compile_errors_when_ast_is_empty_or_missing_required_nodes` | empty/zero/None | empty AST, missing entry/action/schema node | exact `CompileErrors` containing required missing-node variants/count | POST-004, INV-005 |
| `validate_workflow_ast_accumulates_schema_reference_and_profile_errors_when_independent_errors_coexist` | multiple independent failures | invalid schema bounds/defaults + unresolved references + unsupported profile events | exact `CompileErrors` count and set/order of variants; none dropped | POST-004, INV-005 |
| `validate_workflow_ast_rejects_one_below_min_schema_bound_and_one_above_max_bound` | min/max boundary violation | numeric/string/list schema bounds below minimum and above maximum | exact schema bound error variants with field path/bound values | POST-004, INV-005 |
| `validate_workflow_ast_rejects_overflow_depth_or_reference_path_without_success` | overflow/resource bound | excessive nesting/reference path/accessor length | exact depth/reference/resource error; no success, panic, truncation, or wildcard diagnostic | POST-004, INV-005 |

### 14.4 Exact Recovery Error Taxonomy

| Input class | Only accepted result | Forbidden results |
|---|---|---|
| Optional payload truly absent on documented optional/non-critical path | `Ok(None)` | `CorruptEventPayload`, `ReplayCorruption`, generic error, panic |
| Valid supported slot payload | `Ok(Some(expected SlotValue))` | `Ok(None)`, any corruption/replay error |
| Corrupt slot/event bytes on recovery-critical path | `Err(JournalError::CorruptEventPayload { .. })` preserving decode/source context where exposed | `Ok(None)`, `Ok(Some(default))`, `ReplayCorruption` at direct accessor boundary, wildcard/generic error |
| Truncated prefix of otherwise valid encoding | `Err(JournalError::CorruptEventPayload { .. })` | `Ok(None)`, empty recovery success, wildcard/generic error |
| Replay sequence gap, mixed run ids, missing required record, inconsistent recovery summary | `Err(JournalError::ReplayCorruption { .. })` or public replay error that is exactly mapped to that taxonomy | `Ok(empty)`, `Ok(summary)`, `CorruptEventPayload` for non-decode structural inconsistency unless decode failed first |
| Unsupported recovery shape explicitly not implemented | exact documented unsupported recovery error if the public API has one; otherwise `ReplayCorruption` | silent success, default empty frame |

### 14.5 Proptest Invariants — Non-Hollow Requirements

Each proptest must consume generated inputs and compare SUT output to a non-identical oracle/model. `x == x`, `a.saturating_add(b) == a.saturating_add(b)`, source-string existence-only checks, and properties that do not call the classifier/report/decode/validation boundary are invalid.

| ID | Required property test name | Generated data | Required oracle/assertion | Mutation it kills |
|---|---|---|---|---|
| P01 | `proptest_classification_lattice_rejects_release_critical_best_effort_for_all_critical_operations` | operation criticality × discard pattern × classification enum | model says critical + best-effort is invalid; SUT must return exact release-critical classification error with operation/path | criticality predicate inverted or best-effort accepted |
| P02 | `proptest_diagnostic_envelope_conversion_preserves_required_fields_for_all_error_families` | storage/runtime/resume/compiler envelopes with optional run/record/source | after conversion, every present required field equals original model field; clearing source/boundary fails | source field dropped or genericized |
| P03 | `proptest_decode_class_never_maps_corrupt_or_truncated_payload_to_absent` | `Absent`, `Valid`, `Corrupt`, `Truncated`, `Oversized` decode classes | model lattice above decides exact result class; corrupt/truncated/oversized never produce `Ok(None)` | `.ok()` erasure or wildcard decode acceptance |
| P04 | `proptest_recovery_replay_rejects_mixed_run_or_noncontiguous_sequences` | event sequences with run ids and sequence classes | homogeneous contiguous model succeeds; mixed/gap/corrupt model returns exact `ReplayCorruption` | sequence gap ignored or mixed runs merged |
| P05 | `proptest_compiler_validation_error_set_is_monotonic_for_independent_invalid_fragments` | sets of invalid schema/reference/profile fragments with expected error tags | adding independent invalid fragment preserves all previous error tags and increases/keeps superset count; exact variants asserted | early return success or dropped validation error |
| P06 | `proptest_static_scan_report_is_total_over_raw_candidates_and_rejects_critical_best_effort` | raw candidate records plus generated classifications/report rows | rendered report candidate count equals raw production candidate count; unclassified production candidate appears as exact error; critical best-effort rejects | candidate drop, tautological count, release-critical discard accepted |
| P07 | `proptest_strict_persist_model_forbids_ack_after_required_persist_failure` | generated mutation lifecycle steps: stage, persist result, ack attempt, recovery observe | reference state machine allows ack only after successful persist; any failure-before-ack returns exact storage/runtime error and no visible success | ack emitted after failed persist |

### 14.6 Fuzz Oracle Lattice — No Wildcard Acceptance

All fuzz targets must classify every generated input into a closed oracle lattice. A wildcard arm (`_ => {}`) is forbidden in fuzz oracle assertions for malformed/recovery-critical decode classes. Unknown error variants must fail the fuzz test with a diagnostic naming the unexpected variant.

| Fuzz target | Input partition | Accepted oracle result | Rejected oracle result |
|---|---|---|---|
| F01 `vb_qi37_12_persisted_payload_decode` | `AbsentOptional` | exact `Ok(None)` only when absence is represented by event metadata, not arbitrary empty bytes | treating corrupt empty/random bytes as absence |
| F01 | `ValidSlotPayload` | exact `Ok(Some(expected_slot_value))` or replay success with expected record count | default slot value, dropped taint/source, generic success |
| F01 | `CorruptBytes` | exact `JournalError::CorruptEventPayload` | `Ok(None)`, empty recovery success, wildcard accepted unknown error |
| F01 | `TruncatedValidPrefix` | exact `JournalError::CorruptEventPayload` | `Ok(None)`, panic/OOM, wildcard accepted unknown error |
| F01 | `ReplayStructuralInconsistency` | exact `JournalError::ReplayCorruption` | empty successful summary/frame |
| F02 strict YAML parser | alias/tag/multi-doc/empty/depth limit partitions | exact profile/YAML/validation `CompileError` variants per partition | success, generic compile error with no cause, panic/OOM |
| F03 compiler references | known-valid, unknown root, unsupported accessor, empty segment, overlong path | valid references produce exact resolved reference; invalid references produce exact reference/accessor error | success for invalid reference, wildcard diagnostic |
| F04 inventory scan parser | production/test path × pattern × malformed line × classification | exact candidate row, exact unclassified error, or exact non-production disposition | dropped row, critical best-effort accepted, wildcard ignored parse error |

### 14.7 Mutation Checkpoints Mapped To Named Tests

| Mutated branch/operator | Required named test/property/fuzz oracle that must kill it |
|---|---|
| `append_strict` ignores `persist_strict()` result | `close_or_persist_strict_returns_storage_persist_failed_when_flush_fails`; `close_or_persist_strict_does_not_ack_when_persist_fails_after_mutation_staging`; P07 |
| `append_strict_batch` skips persist for non-empty batch | `close_or_persist_strict_preserves_error_when_persist_fails_at_max_valid_batch`; strict journal integration `given_strict_journal_persist_failure_when_runtime_mutates_then_no_success_ack_is_emitted` |
| process lock maps all flock failures to success | `acquire_process_lock_returns_process_lock_held_when_lock_is_already_held`; `acquire_process_lock_returns_process_lock_io_when_flock_returns_non_would_block_error` |
| PID metadata failure becomes full metadata success | `acquire_process_lock_never_claims_full_metadata_success_when_pid_metadata_write_fails`; `classify_fallible_site_accepts_best_effort_when_noncritical_pid_metadata_has_rationale` |
| `JournalEvent::slot_value` / recovery decode maps postcard error to absent | `decode_recovery_slot_value_returns_corrupt_event_payload_when_bytes_are_corrupt`; `decode_recovery_slot_value_returns_corrupt_event_payload_when_valid_encoding_is_truncated`; P03; F01 corrupt/truncated oracle rows |
| `events_for_run` ignores `decode_record` error | `proptest_recovery_replay_rejects_mixed_run_or_noncontiguous_sequences`; integration `given_corrupt_or_truncated_persisted_data_when_recovery_runs_then_empty_success_frame_is_not_returned` |
| runtime `From<JournalError>` loses source | `proptest_diagnostic_envelope_conversion_preserves_required_fields_for_all_error_families`; `apply_drive_result_returns_storage_journal_append_when_evidence_persist_fails` |
| `ResumeError::JournalAppendFailedWithSource` maps to generic error | `apply_drive_result_preserves_cancel_retry_resume_wait_action_cause_fields`; B12 diagnostic conversion scenario |
| `apply_drive_result` converts engine error to success/continue | `apply_drive_result_returns_engine_drive_failed_when_engine_result_is_error`; `apply_drive_result_rejects_mismatched_run_or_state_without_success_ack` |
| compiler schema validation returns `Ok(())` when errors vector non-empty | `validate_workflow_ast_accumulates_schema_reference_and_profile_errors_when_independent_errors_coexist`; P05 |
| YAML alias/tag/multi-doc rejection returns success | F02 strict YAML parser oracle; B15 invalid compiler input scenario |
| reference validator accepts unsupported accessor or unknown root | F03 compiler reference oracle; `validate_workflow_ast_rejects_overflow_depth_or_reference_path_without_success` |
| inventory classifier drops raw candidate from report | `classify_fallible_site_rejects_unclassified_when_production_candidate_has_no_disposition`; P06 |
| hollow property compares expression to itself | P06 must consume generated raw candidate records and classifier/report output; State 9 hollow proptest name is explicitly disallowed |
| fuzz oracle wildcard accepts unknown malformed decode result | F01 oracle must fail on unknown error variants; `given_persisted_payload_fuzz_target_when_oracle_is_scanned_then_malformed_decode_classes_are_exhaustive` must remain exact |

### 14.8 Plan-Parity Requirements For Test Writer Repair

- State 8+ test writer must either implement every named test/property/fuzz oracle above or explicitly mark an item as not implemented with a reviewer-acceptable scope waiver. Silent omission is plan-parity failure.
- Source-string scans may supplement but cannot replace public API behavior tests for the six contract signatures.
- The two red-first tests named in `test-writer-report.md` must not be weakened:
  - `given_recovery_critical_slot_payload_when_accessor_contract_is_scanned_then_decode_error_is_not_erased`
  - `given_persisted_payload_fuzz_target_when_oracle_is_scanned_then_malformed_decode_classes_are_exhaustive`
- Pre-existing weak assertions in unrelated files remain whole-suite static debt; repaired tests for this bead must not introduce new weak assertions and must not depend on reviewer ignoring those hits.

### 14.9 State 7 Repair Completion Evidence

- Mandatory startup read: `/home/lewis/.claude/skills/test-planner/SKILL.md`, `/home/lewis/.agents/skills/test-planner/SKILL.md`; files match and `.agents` controls on conflict.
- Doctrine reference read: `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`.
- Isolation verified from required workspace: `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`.
- Inputs consumed: rejected `.beads/vb-qi37.12/test-plan-review.md`, `.beads/vb-qi37.12/test-suite-review.md`, `.beads/vb-qi37.12/test-repair-guide.md`, existing `.beads/vb-qi37.12/test-plan.md`, `.beads/vb-qi37.12/test-writer-report.md`, and `.beads/vb-qi37.12/contract.md`.
- Repair applied in plan only: added 36 named unit/boundary tests, exact recovery error taxonomy, non-hollow proptest requirements, closed fuzz oracle lattice, named mutation-to-test mapping, and test-writer plan-parity requirements.
- No tests/code edits were made by this State 7 repair.

## Open Questions

- None blocking State 8. If implementation changes add new public APIs, new dependencies, concurrency primitives, unsafe code, or generated code, reopen this plan and add corresponding Loom/Miri/Kani/dependency-audit lanes.
