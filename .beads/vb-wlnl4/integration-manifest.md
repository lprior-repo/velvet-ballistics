# Integration Manifest — vb-wlnl4 (CORRECTED)

Integration engineer: DeepSeek v4 Pro via OMP (this agent)
Candidate branch: `codex/vb-wlnl4-local-integration-candidate`
Base: `e36e4c38` (reviewed lifecycle fixture repairs)

## Correction to prior manifest

The prior manifest claimed the C6/Ask/retry/executor WIP branches were
"preserved as parent, not merged". **This is FALSE.** All WIP branches are
actual ancestors of the integration commit and their code IS in the tree:
`vb-7r7jv` (executor), `vb-r5c1c` (retry checkpoint), `vb-qzxto` (AskTicket),
`vb-clp3q` (artifact fixtures), `vb-mdi3w` (retained input recovery).

## Digest repair (c789b1b35) — MERGED + repaired

`codex/vb-a2l02-digest-boundaries-candidate` (c789b1b3) merged over 1e7aeee2
via JJ (merge base = schema3 376979e05; disjoint file sets, clean merge).
Partial 29b is only history, never landed standalone.

Reviewer findings repaired in-tree:
- **F-4 (fail-closed)**: `digest_string`/`digest_len` return typed
  `CompileError::CanonicalYaml` on over-u32 length; `digest_author_value` /
  `TriggerAst` / `ScalarValue` unknown variants return error (no `0xFF`
  sentinel, no `unwrap_or(u32::MAX)` saturation).
- **F-5 (at_once/pages/items)**: optional u32 fields resolve `None -> 1`
  matching lowering `unwrap_or(1)` (part_02.rs:188, part_03.rs:218-219).
  B7 tests reverted to `assert_eq!`; collect pages/items tests renamed to
  `*_none_equiv_some1` with `assert_eq!`.
- **F-6 (size)**: bead-linked cold-path justification added for
  `canonical_digest`/`digest_step_primitive`.
- **T-2 (oracles)**: added AuthorValue text/sequence and sequence/mapping
  boundary collision oracles (`author_value_text_vs_sequence_boundary`,
  `author_value_sequence_vs_mapping_boundary`). 9/9 collision tests pass.

## C6 retained-input recovery (F-3/T-1) — FIXED

- `submit` helper parsed YAML as JSON. Fixed to `serde_saphyr::from_str`.
- Recovery runtime used `RuntimePolicy::Strict` against a `--durability
  journaled` admission (`admission_policy_mismatch`). Fixed to `Journaled`.
- Test now green end-to-end (submit → delete source/input files → cold
  FjallJournal reopen → retained input digest vs admission → `Runtime`
  recover → `Resumable`).

## Executor conformance (F-1) — partially wired, dispatch descoped

- `validate_registered_recovery` was dead code. Now called in
  `Runtime::build_runtime_recovery_product` (fails closed when a recovered
  registered action cannot be receipt-reconciled). Dead-code warning gone.
- Legacy recovery now sets `ReconcileOnly` (not `Live`) when the boundary has
  a `PendingAction`, so replayed runs never re-execute a live side effect.
- `execute_registered_action` / `reconcile_registered_action` remain `pub`
  with no in-tree dispatch caller. Full grant lifecycle is **vb-hk1j8**
  (separate bead) — not fabricated here. Storage conformance suite
  (`vb_7r7jv_executor_conformance`) covers the catalog/receipt adapter.

## Admission fixtures — capability grammar

`runtime_fjall_pending_action_recovery` fixture used capability name
`fjall.pending-action.recovery` (hyphens) which violates the capability
grammar (`is_capability_name_grammar_valid`: `a-z 0-9 _ .` only). Renamed to
`fjall.pending_action.recovery`. 10 of 13 failing tests now pass.

## Remaining blockers (precise, preserved source + evidence)

1. **Durable retry end-to-end (F-2)** — `cmd_retry` still returns
   "durable retry execution is unavailable" (`run_ops.rs:59`). No production
   caller of `RunCheckpointV1::capture`, no producer of `RetryCheckpoint`
   journal events. `durable_retry_red_phase.rs` documents the gap
   (vb-qi37.16.3). This is a large feature, not a planner/checkpoint unit
   fix; the checkpoint data structures and recover integration are complete,
   but capture-on-failure + CLI `retry --step` load/re-execute are unimplemented.

2. **Two admission ABI tests** (`runtime_fjall_pending_action_recovery`),
   previously mislabeled "artifact checksum mismatch / hash pending":
   - `runtime_submit_drive_without_action_abi_authority_fails_before_ticket_persist`:
     Gate 12 contract-completeness rejects Do-without-contract at submit
     (`ACTION_CONTRACT_MISSING`), test expects submit-ok + tick
     `action_abi_digests_missing`. Gate-12 vs ABI-recording semantics conflict.
   - `runtime_submit_drive_persists_recoverable_fjall_pending_action_abi_digest`:
     pending action ticket not rehydrated after cold reopen (runtime-drive
     event sequence vs `pending_action_ticket_from_events` seed detection).
   FIXED: `strict_submit_rejects_unbound_action_abi_before_ticket_persist` —
   wired `AdmissionActionAbiDigestMismatch` check in
   `execution_binding::compare_execution_tuple` (was declared but never produced).

## Verification (true-exit runner, full logs in evidence/)

- vb_compile lib: 494 passed / 2 fixed then 496 passed (0 failed)
- digest_framing_collision: 9 passed
- vb_runtime lib: 1934 passed, 0 failed
- vb_runtime recovery: 33 passed
- C6 retained input: 1 passed
- vb_runtime lib: 0 warnings (dead-code `validate_registered_recovery` gone)
