# Independent admission code review

**Bead**: vb-n7umv (discovered-from:vb-ezymz; independently allocated ID)  
**State**: independent CODE review; separate TEST review not supplied  
**Reviewer**: black-hat-reviewer / independent Codex reviewer  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-ezymz-code-review-sol-20261021  
**Attempt**: 1  
**Exact source**: 9f0557e59ab55203cc94196d67395c3bb3efe92f (codex/vb-ezymz-admission-candidate)  
**Admission delta parent**: 948f19cb7cd92afec3d95e9fb285216cc6689d41  
**Contract**: candidate .beads/vb-ezymz/contract.md revision5; type-contracts.md, boundary-map.md, hazard-analysis.md, test-plan.md and evidence/current/README.md.

## Gate Result

**STATUS: REJECTED** under the explicitly applied black-hat mechanical review rule, solely for FINDING-001 (LOW/P3). No new functional admission defect was confirmed. This is not a reclassification of the already rejected artifact dependency as an admission bug.

The new source is substantially consistent with its scoped admission contract. Overall vb-ezymz completion additionally requires the separately owned recovery/artifact integration gates listed below; this review does not approve those unseen changes or close the parent bead.

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---|---|
| C1 positional postcard wire, empty compatibility, complete decode and kind/count rejection before storage open | Pass in inspected code and focused tests | submit.rs:35-48 calls map_runtime_inputs before commit_submission; run_compiled.rs:106-151 uses take_from_bytes plus trailing check. Public boundary suite6/6 passes, including unchanged existing metadata after invalid input. |
| C2 typed immutable retained inputs, distinct identities, canonical input digest, schema rejection | Pass for scoped producer | RunInputsRecord private fields and constructor; validation rejects duplicate/undeclared/range/kind/unbacked arena handles; compute_input_digest slot-sorts typed pairs and rejects duplicate slots. inputs module13/13 passes. |
| C3 atomic source/artifact/inputs/header/indexes/events; bound receipt tag36 | Pass for scoped producer | RunSubmission::stage and stage_acceptance use one JournalWriteBatch; accepted event keeps workflow digest, receipt keeps artifact+input digests. Bound36/historical24/schema-one ordinal1 tests3/3 pass. |
| C4 Strict SyncAll; Journaled loss window; None metadata without events | Pass in inspected code and focused tests | commit selects batch.strict().commit only for Strict; Relaxed skips events. All three policies covered by retained CLI execution and storage tests. This is not injected hardware crash proof. |
| C5 reject collisions and discard failing staged batch | Pass in inspected code and focused tests | commit_run_submission holds write_lock across existing header/input/events check and commit; errors drop the owned batch. Injected precommit failure, empty keyspaces/indexes, retry and collision preservation covered in13-test lane. |
| C6 persisted values affect numeric execution after original files removed | Pass for explicitly scoped execution seam | CLI retained test2/2 passes:15 mode/value combinations, original source/input files removed, two reopen cycles; independently verifies retained artifact, seeds original values and run_until_blocked finishes with each exact value. |
| C6 full production recovery entry point | Integration dependency, not approved here | Contract explicitly assigns vb-netau. Candidate has no runtime input loader integration beyond preserving new digest field in DTO summaries. Test seam does not demonstrate Runtime::recover_* integration. |
| C7 pure artifact preparation before the atomic batch | Pass for admission producer; artifact implementation remains separately rejected | prepare_records calls prepare_compiled_artifact with no I/O, then RunSubmission validates it; no artifact persistence occurs before the combined batch. |
| No runtime YAML parsing introduced | Pass for admission delta | YAML compilation stays in cold CLI compile_submission. Runtime source delta only carries explicit compatibility binding marker. Persisted execution test reconstructs numeric IR without source compilation. |

The runtime path still marking historical RuntimeJournalEvent conversions LegacyUnbound is explicit compatibility behavior, not evidence that new CLI receipts are input-bound. New CLI RunSubmission writes Digest(...) using stable tag36.

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|---|---|---|---|
| events/wire_serialize_run.rs::serialize_admission | 36 physical lines,45-80 (previously20) | 25 | Fail: FINDING-001 |
| inputs/submission.rs::commit_run_submission | 22 physical lines,134-155 | 25 | Pass |
| inputs/submission.rs::stage | 19 physical lines,87-105 | 25 | Pass |
| inputs/digest.rs::compute_input_digest | 22 physical lines,14-35 | 25 | Pass |
| New submission/retention functions' parameter count | At most5 | 5 | Pass in inspected signatures |

Legacy long match/dispatch functions remain outside a broad refactor: journal conversion, RecordKind id/from_id/is_journal_event, events run_id/seq/record_kind/slot_value/attempt/is_valid, existing journal open, recovery derive/types, and CLI event_to_json. Their existing length is flagged as inherited review debt, not newly claimed correctness defects or justification for a broad rewrite. The bounded repair here concerns the serializer that crossed the limit through new nested binding serialization.

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---|
| Zero unsafe / unwrap / expect / panic / todo / unimplemented / dbg in new producer modules | No occurrence observed in source inspection; no whole-workspace lint claim. |
| Checked arithmetic and access | New input code uses checked conversion, safe get_mut, bounded slot count and explicit allocation failure handling. No new unchecked indexing observed. |
| Typed state and expected failures | RunSubmission members private; RunInputsRecord immutable and fallible; RunInputBinding distinguishes LegacyUnbound from Digest. Typed JournalError variants propagate. |
| Pure calculations separated from I/O | Input mapping, digest/schema checks and binding are pure; CLI reads/open and Fjall commit form shell boundary. |
| Resource bounds | Input envelope1MiB; mapping bounded by declared/u16 slot range; serialized record limits enforced before batch publication. This is not an exhaustive allocator proof. |
| Performance claims | None asserted; no benchmark is presented or required as performance evidence by this review. |

Exact Holzmann references read: /home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md and /home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md. Repository docs/rust-governance.md pinned nightly/feature discipline inspected; candidate introduces no new unstable feature or runtime YAML dependency. No broad zero-slippage gate was run under the review assignment's explicit focused-only limit.

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---|
| No Option-based lifecycle state machine | Pass in new submission model: one complete prepared product, no optional partial members. |
| Distinct semantic identities | Explicit accessors keep source, compiled workflow, artifact and input digests separate; consistency checked at RunSubmission boundary. Same digest representation is shared, but this scoped boundary checks roles. |
| No behavioral bool soup | Uses RuntimePolicy/DurabilityMode/RunInputBinding enums. |
| Trusted prepared product before I/O | Binding validation precedes construction; policy verification precedes lock and staging. |
| Low-level deserialized record remains hostile until checked | Envelope/run/digest checks on load, schema check against workflow and receipt input binding available to recovery. Full consumer use is the integration owner's obligation. |
| Mechanical simplicity | One low-priority serializer-size repair; do not expand it into a storage/runtime rewrite. |

## PHASE 5: The Bitter Truth

The source visibly stages one atomic batch; the focused tests actually execute. The retained-value test proves the persisted artifact plus retained inputs can drive the production numeric engine after original files are removed. It does not prove that every production recovery entry point selects and seeds those inputs. Artifact948f's independent rejection is still binding and cannot be laundered by this review's24 passing admission tests.

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---|---|---|---|
| FINDING-001: keep receipt serialization within the explicit function-size gate | LOW / P3 | crates/vb_storage/src/events/wire_serialize_run.rs:45 | Open on exact candidate |

### FINDING-001: Separate receipt binding routing from payload serialization

**Location**: crates/vb_storage/src/events/wire_serialize_run.rs:45-80.

**Problem**: The modified serialize_admission grew from20 to36 physical lines, exceeding the applied black-hat skill's hard25-line function limit. It now destructures the event, branches on binding version and builds two wire payload shapes in one function. No incorrect serialized bytes were observed; this is a mechanical reviewability finding, not a demonstrated functional or security defect.

**Evidence**: Exact candidate source lines45-80 and delta from948f19cb. Independent receipt codec lane3/3 passes both new and historical wire cases.

**Required Fix**: Make a bounded mechanical extraction of routing/payload serialization into small helpers within25 lines and at most5 parameters. Preserve exact tag24 five-field and tag36 six-field layouts and the schema-one compatibility branch. Rerun the three focused receipt codec tests. Do not broaden into unrelated legacy long functions.

## Pinned Integration Requirements (not new admission findings)

1. **Artifact producer948f19cb remains REJECTED**: prior independent report commit832840178a4de0a8d2dc391ad2bdc2ec7607da6b, .beads/vb-9agmk.1/code-review.md. Its runtime submitted-IR/contracts binding and sparse/duplicate registry findings need repaired-source re-review. This admission review does not charge those as new vb-ezymz source bugs.
2. **Full production retained-input recovery**: vb-netau owns loading persisted artifact, receipt/input binding, seeding before first deterministic drive, and public recovery execution coverage without YAML. Review that exact integrated source separately. The candidate's direct engine seam is accurately labeled.
3. **Other CLI legacy artifact callers**: crates/velvet_ballistics/src/run.rs still manually constructs an Unverified proof on this stack. Preserve the previously assigned cross-stack caller migration requirement; submit.rs correctly uses pure prepared artifact. No approval of the complete durable CLI surface is implied.
4. **Independent TEST review and integration moon ci**: separate owners; neither is supplied by this CODE review. Parent vb-ezymz must stay open until its explicit acceptance criteria and dependency gates pass.

## Quality Gates

| Gate | Actual result | Evidence |
|---|---|---|
| cargo test -q -p vb_storage --lib inputs:: | Exit0;13 passed,0 failed,0 ignored,1599 filtered | inputs.log / inputs-command.json |
| cargo test -q -p velvet-ballistics --test submit_input_boundary --test submit_retained_inputs | Exit0;6+2 passed,0 failed,0 ignored,0 filtered | cli.log / cli-command.json |
| cargo test -q -p vb_storage --lib codec::tests::admission_receipt | Exit0;3 passed,0 failed,0 ignored,1609 filtered | receipt-codec.log / receipt-codec-command.json |
| Broad CI / full clippy / formal verification / independent TEST gate | Not run by reviewer | Explicit focused-only assignment; author evidence remains separately labeled, not a fresh reviewer pass. |

All24 tests above were independently run in this isolated workspace at exact9f0557e5. Full argv/exits/source pins are retained in JSON. Commands ran in bounded user systemd scopes with MemoryHigh12G, MemoryMax16G, MemorySwapMax0, Cargo jobs2 and timeout180s (codec90s), RUSTC_WRAPPER cleared. No source/test fixes or independent probe code were needed. Only review evidence files are published.

## Verdict

**STATUS: REJECTED** by the requested black-hat mechanical constraint pending the narrow P3 extraction. No new functional admission defect was confirmed. Even after that repair, artifact/recovery/CLI integration and independent test approval remain necessary before parent bead closure.

## Required Repair Actions

1. LOW: Split the receipt serializer mechanically, preserving both stable layouts, and rerun the three codec cases on the exact revised source.
2. Keep separately owned integration requirements visible; do not reinterpret this scoped review as approval of rejected or unreviewed dependencies.
