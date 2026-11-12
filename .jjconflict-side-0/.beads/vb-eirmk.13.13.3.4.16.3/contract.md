# Recovery-owned arena contract r419

Status: CONTRACT PROPOSAL ONLY for the local5090 recovery repair after its current C4 job ends; no implementation, execution acceptance, self-approval, tracker update or proof-planning stage. Delivery remains contract → implementation → behavioral tests → CODE → separate TEST.

Scope: deterministic cold replay must preserve the full arena it actually mutated and seed validated secret roots before evaluation. No persisted-format, dependency, feature, gate, proof, host-authority or public runtime-execution API change. Preserve all existing storage public signatures and their legacy fail-closed behavior.

Own artifact root: `/home/lewis/src/isoloated/velvet-ballistics-reference-master11-repair-luna`; Git and JJ roots were independently read and matched before writing. Runtime peer `/home/lewis/src/isoloated/velvet-ballistics-union-runtime-5090-r256` and kernel peer `/home/lewis/src/isoloated/velvet-ballistics-union-kernel-5090-r256` were read only. Their worktree bytes, not HEAD alone, identify the evidence; companion JSON records hashes.

The exact peer acceptance dependency is `shipping_runtime_arena_recovery.rs`, SHA256 `1fd0e5647456fb97835acb5d46f5c65f3cee14f40dd652a8a09cb1396dec38d9`, under the shipping workspace. Parent reports its secret case RED at Step0 replay evaluation and clean composite case RED with `CannotResume(slot_values)`; this contract does not claim an independently executed reproduction.

## Existing-carrier decision

`vb_storage::recovery::RecoveryFrameSeedProduct` cannot carry a store: its `ResumableRecoveryFrameSeedProduct` has only a private `seed`, and its `NonResumableRecoveryFrameSeedProduct` has `seed` plus `cannot_resume`. `RecoveryFrameSeed` contains summary/dimensions/PC/steps/slot values/pending actions/unsupported flags, with no arena/checkpoint. Adding a field there would alter an existing public DTO and is not the selected design.

`RecoveryHydration` is only `Summary` or `FrameSeed`; `RecoveryFrameSeedBuilder` returns the same raw seed; neither retains the replay store. `CompleteScalarReplay` carries only frame plus seed and intentionally excludes composite allocations. Its private driver owns a store but drops it; broadening that scalar product/domain is outside this repair.

The other exported snapshot carrier, `RunSnapshot`, contains run/sequence/workflow plus encoded slot and taint byte vectors, not a full store. Packing an arena into those vectors would invent a persisted format and is prohibited. The bounded scan of production recovery carriers found no existing public outcome that transports the actual post-replay arena.

Runtime already has the required downstream transport: `ResumableRecoveryParts.restored_store: Option<ValueStoreCheckpointV1>` → `ResumableRecoveryProduct::into_recover_command()` → `RecoverRunCommand::with_restored_store()` → `recovered_store::prepare()`/`validate_full_store()`. Reuse this path and its existing full-checkpoint and bound-prefix validation. Do not add another journal record, DTO field or recovery command store field.

## Narrow additive storage boundary

Because no existing storage outcome can carry the store, propose one additive non-serialized storage outcome named `RecoveredReplayOutcome` and one additive entrypoint named `recover_runtime_frame_seed_from_events_with_replay_outcome`. These names describe the proposed API, not APIs already present in the inspected source.

The proposed entrypoint accepts exactly the existing context entrypoint's arguments: `events: &[JournalEvent]`, `workflow: &CompiledWorkflow`, `inputs: &[(SlotIdx, SlotValue, Taint)]`, and `store: Option<&ValueStore>`. It returns `RecoveryResult<RecoveredReplayOutcome>`. No parser, host authority or input rebinding enters this storage API.

The new outcome has private storage-owned fields for the existing `RecoveryFrameSeedProduct`, the existing optional successor `(StepIdx, u64)` cursor, and `Option<ValueStore>` containing the actual arena paired with the replay result. One consuming `into_parts` accessor may transfer these three existing types to runtime; no public constructor or mutable fields are needed. The outcome has no wire serialization and adds no error enum or fabricated authority witness.

`Some(store)` means an actual retained/restored/replayed store is owned by this result. `None` is permitted only where no arena was produced or required; it must never accompany newly produced arena handles with a resumable classification. If no deterministic step ran, runtime retains the already-validated retained store rather than replacing it with an unrelated empty store.

Keep every old storage function, signature and raw DTO unchanged, including `recover_runtime_frame_seed_from_events_with_replay_inputs` returning `(RecoveryFrameSeedProduct, Option<(StepIdx, u64)>)`. Old store-less projections must retain their previous refusal of newly allocated composite handles. Never implement a compatibility wrapper by dropping the new store while returning its newly resumable seed unchanged.

The new capability is owning the real replay arena. It is not permission to declare arbitrary handles resumable, clear `slot_values`/`store_missing`, invent a store from handles or reinterpret the legacy scalar replay product.

## Same-pass ownership invariant

Core already provides `ReplayEngine::with_initial_inputs`, `replay_frame_through(target, &mut ValueStore)` and `replay_frame_up_to(target, &mut ValueStore)`. Reuse those real calculations; no second replay algorithm or runtime-side reconstruction from slot handles.

Extend the private storage replay outcome so each `ThroughTarget` or `BeforeTargetFallback` owns the frame AND the exact store mutated by that pass. Derive the slot entries and cursor from that same frame before transferring the store into the additive outcome. The returned seed and arena must never come from different replay attempts.

For successful through-target replay, retain the full extended store, including symbol prefix, unused/retired allocations, lists, objects, blobs, taints, allocation order and capacity bound. For a nondeterministic target, discard the partially mutated failed-through scratch state exactly as today, reset from the same validated input checkpoint, perform the before-target pass, and retain THAT fallback frame/store pair.

Only completed `ThroughTarget` may supply a successor cursor. `BeforeTargetFallback` supplies none; later journal-boundary precedence remains authoritative. Runtime continues applying successor PC only and preserves the existing hydrated executed-count semantics; this repair does not copy replay execution counters into the hydrated frame.

Use an already-prepared retained full store without composing it again. When a genuinely fresh scratch store is required, use the existing workflow capacity and existing compiler-prefix seeding/preparation path appropriate to that workflow; never invent a new default bound, omit the prefix or seed an already full store twice. `ValueStore::checkpoint_v1` and `from_checkpoint_v1` are the existing transport/validation operations.

Validate replay-created references against the actual post-replay owned arena. Then merge durable `SlotWritten` overrides in the same existing order, preserving their runtime IDs/taints; validate final references against the resulting authoritative full arena. If an event refers to arena contents not reconstructable from retained state or deterministic replay, retain the genuine existing fail-closed result. Do not remap journal IDs or manufacture missing contents.

## Runtime input and store binding

Preserve current order: load journal prefix and identity, load/validate accepted artifact and workflow, validate admission, validate host secret-name presence, validate retained input admission/artifact/workflow/checkpoint, and only then prepare the replay context. Identity, authority or malformed-input failures must not be masked by successful replay or a later error.

The actual helper is `Shard::prepare_secret_inputs(&self, &CompiledWorkflow, &[(SlotIdx, SlotValue)]) -> RuntimeResult<Vec<(SlotIdx, SlotValue)>>`. It validates presence and fills/checks exact `Symbol(binding.requirement())` pairs; it neither returns taints nor owns a store. Its current callers are fresh submission and enqueue preflight, not cold replay. Reuse this helper rather than writing another secret-name lookup path.

Recovery needs a minimal PRIVATE adapter on its owned retained-input carrier: preserve all original `(slot,value,taint)` triples and the prepared full store, use the existing helper's filled pair results, and add only missing metadata-declared secret bindings with `Taint::Secret`. Preserve original ordering of supplied roots and deterministic metadata order for appended roots. Validate any already supplied secret value AND taint before replay; a supplied `Clean` or mismatching secret root must fail through the applicable existing validation path, never be silently relabeled, overwritten or downgraded. Do not recompose prepared symbols after filling compiler-ID secret roots.

Secret initialization currently occurs again in `enqueue_recovered_product` through `initialize_secret_frame`. Keep that consistency/presence check, but it cannot substitute for pre-replay initialization. No raw secret value is loaded, stored or logged; the existing host-confirmed name authority remains unchanged.

The canonical cold `recover_seed_for_loaded_workflow` route must consume the new arena-owning outcome even where the old empty-roots/None-store branch selected a store-less workflow entrypoint, when deterministic replay can create arenas. Preserve the old entrypoints for their existing callers and preserve the existing separate eligible complete-scalar behavior. Do not repeatedly replay merely to recover a store discarded by the first pass.

The runtime private recovered context retains the new owned arena until resumable-product creation. Its full-recovery classification, arena checks, `hydrate_replayed_frame` store argument, and eventual `restored_store` checkpoint must all refer to this same effective store. If no replay arena was produced, use the already-validated retained store. Never publish only the retained-input checkpoint after deterministic replay has extended that arena.

Hydration continues to seed initial roots before recovered/journal slot writes, then restore step states/PC through the existing functions. Keep full seed-shape validation, pending-action/ticket/contract authority, action ABI checks, registered receipt-only reconciliation, terminal lifecycle rejection, collect/manual-gate hydration and enqueue order. Recovery-product construction remains read-only with respect to durable bytes and live actor publication.

## Error contract

Reuse `RecoveryResult`/`RecoveryError` and `RuntimeResult`/`RuntimeError`. Keep workflow-digest and event ordering rejection before trusted replay; preserve `ReplayError::ExpressionEvalFailed { step }` → `RecoveryError::ReplayDivergence { step, detail: "replay expression evaluation failed" }` → existing `RuntimeError::Recovery` envelope. Do not invent an underlying expression-cause field on public replay errors.

Keep existing secret failures: `RuntimeError::SecretUnavailable`, fallible allocation errors, and `CoreError::InternalInvariantViolation { reason: "secret_input_binding" }` where that existing checker applies. Existing earlier retained-record/owned-root validation failures retain precedence. Negative tests must assert the full error at the exercised boundary, not `is_err()` or an invented generic conversion.

Keep full-store/prefix rejection: `validate_bound_symbol_prefix` returns the existing `RunCheckpointV1Error::SymbolPrefixMismatch { index }` at the first bad prefix entry; full-store validation and recovery wrapping remain unchanged. Invalid arena structure/references/taint layout must still be rejected by existing checkpoint/root validators before publication. No new CannotResume waiver or blanket conversion to a successful recovery product.

## Bounded file and caller ownership

Storage implementation fence: `recovery/replay/summary/input_context.rs` for paired frame/store ownership and actual-handle validation; `hydrate.rs` and `derive.rs` for ordered seed construction and arena-carrying result flow; minimal new outcome definition and existing `summary`/`replay`/`recovery` reexport hooks. Do not alter the old public seed DTOs, scalar replay domain, event schema or core replay algorithm.

Runtime fence: `runtime/recovery_product.rs`, `runtime/recovery_seed.rs`, and minimal private retained-input carrier/preparation adapter. Reuse `shard/secrets.rs`, `recovery/full.rs`/`frame.rs`, `recovery/full/product.rs`, `shard/types/recover.rs` and `shard/lifecycle/recovered_store.rs`; change these only if an actual unavoidable private integration hunk is demonstrated. Preserve all fresh/owned routes and sibling C4 work; do not restore older whole files.

Existing call graph: `Runtime::recover_product` → validated `load_retained_inputs_for_recovery` → `build_runtime_recovery_product` → `recover_seed_for_loaded_workflow` → storage context replay → `recover_slots_through_step` → core `ReplayEngine` with supplied mutable store → replay/event slot merge → typed seed/boundary validation → `hydrate_replayed_frame` → resumable product → existing checkpoint-bearing recover command → existing full-store/prefix restoration. The repair adds arena ownership along this path rather than adding a parallel recovery route.

All new/changed functions must satisfy the repository's 25-line/5-parameter bounds, new files 300 lines, checked arithmetic/indexing/conversion and fallible resource handling; no unsafe, panic/unwrap/expect, suppressions, dependencies, macros hiding business logic or gate edits.

## Behavioral acceptance, unexecuted

1. Run the UNCHANGED registered `computed_composites_and_taints_survive_fjall_reopen_at_action` and `clean_composites_survive_fjall_reopen_at_action` from the exact SHA above on the repaired integrated source. Preserve actual actor/journal scope drop and fresh Fjall reopen, both live and restored complete checkpoint equality, every recorded composite slot/value/taint, identical pending ticket, completion by that ticket, exactly one finish, zero failed events and zero failed-run counter. Do not simplify the source to input-free scalars or weaken any oracle.

2. Positive same-pass evidence: a deterministic through-target allocation returns resolvable list/object contents and taints in the exact returned full checkpoint; include a case where the retained store has no such allocations. Fallback evidence must show the failed-through partial arena is discarded and the fallback frame/store agree, with no successor cursor. Existing deterministic-target and later-journal-boundary cursor controls remain unchanged.

3. Bad-arena negative: malformed nested arena/checkpoint references remain exact typed failures; a durable slot handle whose required arena contents cannot be reconstructed remains fail-closed. Preserve original retained roots/checkpoint and durable bytes on rejection. Prove the repair does not merely accept List/Object tags or clear unsupported flags.

4. Prefix negative: a missing or altered compiler prefix entry fails at the exact first index with existing `SymbolPrefixMismatch { index }` at its validator boundary and the existing runtime wrapper at its caller boundary; the input checkpoint remains unchanged. Correct prefix plus extra valid unused/retired tail allocations stays accepted and byte-preserved.

5. Taint negatives: an already supplied declared-secret root carrying `Taint::Clean`, a mismatching secret value, and an invalid taint layout are rejected through the applicable original exact validator/error route; none is rewritten into valid input. Missing host secret-name availability still yields `SecretUnavailable` before replay/publication. Valid secret roots remain Secret through list/object computations and cold restoration.

6. Retain original admission/artifact/source/input binding failures, owned checkpoint caps, taints and ID stability; journal override ordering; legacy store-less fail-closed behavior; and eligible complete-scalar recovery behavior. Do not claim package/full-workspace/phase acceptance from the two focused cases alone.

7. Required negative controls are bounded behavioral checks of the actual new path: deliberately dropping the returned arena must fail the exact checkpoint/handle acceptance; omitting pre-replay secret seeding must fail the secret case; wrong-store fallback pairing must fail the same-pass assertion. Execute only focused controls authorized for this repair, preserve raw commands/exits and do not claim mutation evidence from inspection alone.

## Provenance and qualification limits

The r410 scout is historical evidence, not a frozen base for the C4 author. During r419 inspection, runtime `recovery/input_binding.rs` had changed under its current owner and contained an incomplete intermediate expression; kernel `summary/derive.rs` and runtime lifecycle composition also retained conflict text. No source was edited or built. Implementation begins only from the owner's coherent frozen post-C4 source with guarded minimal hunks; these live worktree observations do not authorize overwriting that work.

The companion JSON records exact carrier/source hashes and the unchanged peer test SHA. No Rust, tests, proof artifacts, tracker, persistent memory, parent files, processes or chats were changed. This is a reviewable contract proposal; CODE and separate TEST must review the actual eventual implementation and evidence.
