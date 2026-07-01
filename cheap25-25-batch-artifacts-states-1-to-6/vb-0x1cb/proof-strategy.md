# Proof Strategy — vb-0x1cb

- bead_id: vb-0x1cb
- title: Repair ignored-fallible-results source gate violation (P1)
- state: 4 (proof-planner)
- scope_kind: contract (canonical surface)
- lane_profile: rust_local_concurrency_empty
- controller: femdation
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- captured_at: 2026-07-01T16:05:00Z
- inputs_read: STATE.md, codebase-map.md, contract.md, delivery-scope.jsonl,
  proof-seeds.jsonl (S1..S7), traceability-matrix.jsonl (R1..R9)

## 1. Scope (binding chain)

Downstream holzman-rust + test-planner + black-hat-reviewer MUST satisfy each clause
of `contract.md` and the obligations in `proof-obligations.planned.jsonl`. The planner
binds the contract to verifier lanes without claiming pass/fail — proof-reviewer owns
disposition (4b), proof-writer owns artifacts (5..7), formal-verifier owns closure
(12). Nothing in this strategy approves behavior.

Production surface (single typed-error/observability surface):

- `crates/vb_runtime/src/shard/transitions.rs:100` (in `Shard::finish_run`) — primary
  discard site. Replaced by a bound `match` expression that calls a new helper
  `Shard::observe_run_state_rollback(run, RollbackSite::FinishRun, error, secondary)`.
- `crates/vb_runtime/src/shard/transitions.rs:202` (in `Shard::fail_run_state`) —
  mirror call site. Same helper invoked with `RollbackSite::FailRunState`.
- `crates/vb_runtime/src/shard/transitions.rs:86` and `:199` — the
  `#[allow(clippy::let_underscore_must_use)]` annotations are removed. With the
  expression replaced, `let _ = …` is gone, so `clippy::let_underscore_must_use`
  no longer triggers.
- `crates/vb_runtime/src/trace/event.rs` — new variant
  `TraceEvent::RunRollbackFailed { run: RunId, site: RollbackSite,
  primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> }` plus the
  non-terminal `is_terminal_for_run` arm.
- New `RollbackSite` enum (`#[non_exhaustive]`) with `FinishRun` and `FailRunState`,
  both `Copy + Eq + Hash`. Lives in `crates/vb_runtime/src/trace/event.rs`
  alongside the variant that owns it.
- `scripts/ignored-fallible-results.allow:4` — the DISCARD-006 row
  (`follow_up=vb-ttki3|…`) is deleted entirely. The header comment block stays.
  `follow_up=vb-ttki3` MUST NOT be reused in any new allow row — the bead it
  pointed at is unrelated (`moon ci` after forced push, per `to-fix/wave4/agent-12-adhoc-kani-harness.md`).

The discard sites DO NOT currently use `Ok(_)|Err(_)=>{}` (DISCARD-004). That
literal pattern lives in the bead description only and is stale (per
`codebase-map.md` §"Bead description drift"). The repair is bound to the
two actual `let _ = self.run_state_insert(run, state);` call sites at lines
100 and 202 — not to a phantom `match` arm at line 146.

## 2. Lane profile (rust_local_concurrency_empty)

| Lane | Decision | Why |
|------|----------|-----|
| rust-local (cargo test) | engaged | Two behavior tests at `lifecycle_tests/chunk_005.rs` and `chunk_008.rs` mirroring `chunk_004.rs:240 LegacyStepFailsJournal`. Asserts primary error preservation + `RunRollbackFailed` trace insertion (when the dual failure occurs). |
| proptest | engaged | Exhaustive variation of the `{journal_rejects × slot_full}` 2×2 to prove that `RunRollbackFailed` is emitted iff both failures occur, not on the happy or recovered-rollback paths. Applied to both `finish_run` and `fail_run_state` helpers. |
| flux-rs | engaged | `#[extern_spec]` over the new `TraceEvent::RunRollbackFailed` variant with `#[refined_by(run, site, primary, secondary)]` and a size invariant that `size_of::<RunRollbackFailed>() <= size_of::<RunId> + size_of::<RollbackSite> + 2 * size_of::<Arc<…>>()` (≤ 25 bytes on 64-bit). Lives alongside the existing `vb_y9d3v_action_ticket_refinements.rs` extern_spec pattern. |
| kani | engaged-as-stub | The `#[cfg(kani)]` stub for `append_journal_event` (`impl_parts/chunk_001.rs:206`) returns `Ok(())`; under `cargo kani` the rollback branch is unreachable, so the new dual-failure path cannot be exercised. Proptest + behavior tests carry the dual-failure obligation; kani is named `not_applicable` for the behavior-affecting seeds S1, S2, S4, S5 and `not_applicable` for the bounded-payload seed S3 (a `kani::any()`-style payload bound would degenerate to a compile-time `size_of` assertion which is what `cargo expand | rg` already proves). The stub MUST remain unchanged. |
| cargo-clippy | engaged | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` must exit 0 after the annotations are removed. The new helper is `#[must_use]` so dropping the helper's return value triggers `unused_must_use` and would be a clippy regression on its own. |
| moon-source-gate | engaged | `moon run :lint-src` is green when both `bash scripts/check-ignored-fallible-results.sh` exits 0 AND the `JustifiedException|…|transitions.rs|…` rows are absent post-repair. |
| bash `scripts/check-ignored-fallible-results.sh` | engaged | Same evidence as `moon-source-gate` per `.moon/tasks/all.yml:75-85`; emitted as the canonical "post-rename stdout" assertion. |
| loom | NOT_APPLICABLE | Single-shard sequential (`Shard::tick` drains one command per tick). `JournalWriteBatch` is `!Send + !Sync`. No concurrent interleaving across the rollback sites — proptest covers both dual-failure cases without scheduler exploration. `limitation_kind: no_concurrency_in_scope`. |
| verus | NOT_APPLICABLE | Bead instruction (this prompt) and the `rust-contract` decision surface deliberately omits Verus for this bead; binding via `#[path = …]` STRONG or `production_inner/_production.rs` mirror would require re-deriving the action_ticket_refinements extern_spec file, expanding proof debt beyond the source-gate repair. Flux carries the bounded-payload refinement, proptest carries behavior, cargo test carries LegacyStepFailsJournal mirroring. `limitation_kind: not_required_by_contract`. |
| miri | NOT_APPLICABLE | All scoped files carry `#![forbid(unsafe_code)]`; zero unsafe blocks across `transitions.rs`, `event.rs`, `lifecycle_tests/chunk_005.rs`, `lifecycle_tests/chunk_008.rs`, `impl_parts/chunk_001.rs`. `limitation_kind: no_unsafe_in_scope`. |
| cargo-fuzz | NOT_APPLICABLE | No codec/parser/byte-level hostile input boundary added in this bead; `TraceEvent::RunRollbackFailed` carries typed `RunId + RollbackSite + Arc<RuntimeError>`, not raw bytes. `limitation_kind: no_codec_in_scope`. |
| TLA+ | globally removed | TLA+ lane has been globally removed across the repo; temporal/state-machine obligations are covered by loom + proptest. |

## 3. Risk classification

| Risk | Severity | Trigger | Lane | Notes |
|------|----------|---------|------|-------|
| release-blocker (DISCARD-006 source gate) | HIGH | `moon :lint-src` fails — CI red. | moon-source-gate + bash | Permit discharge requires `bash scripts/check-ignored-fallible-results.sh` exits 0 and emits no `transitions.rs` lines on stdout. |
| primary-mask regression | HIGH | The secondary `RuntimeError` from the rollback `run_state_insert` must NOT be returned in place of the primary journal-append `RuntimeError`. | cargo test + proptest | `LegacyStepFailsJournal` mirror rejects `StepSucceeded`; the new `FinishRunRejectsJournal` mirror rejects `RunFinished` with `WriteLockPoisoned` and asserts `Err(StorageJournalAppend(…))`, not a Core/rollback variant. |
| observability loss (silent dual-failure) | HIGH | `let _ = …` hidden the rollback error from any operator. | cargo test + proptest + flux-rs | New `TraceEvent::RunRollbackFailed` carries `Arc<RuntimeError>` for both errors with bounded size. |
| clippy::must_use regression | MEDIUM | New helper `observe_run_state_rollback` MUST be `#[must_use]`; dropping the helper's value would reintroduce the same class of bug. | cargo-clippy | clippy with `-D clippy::let_underscore_must_use` exits 0 only if the helper is invoked with `let _ = …` removed and `?` or `match` used. |
| bounded payload drift | LOW | `RunRollbackFailed` size must stay well below one cache line so a saturated `trace_ring` does not perturb hot-path allocation. | flux-rs | `#[extern_spec]` + `#[refined_by]` carry the bound; black-hat-reviewer verifies the variant body uses `Arc<RuntimeError>` and not `Box<RuntimeError>`/`String`. |
| runtime diagnostic route drift | LOW | The contract rejects `RuntimeError::Core { source: InternalInvariantViolation { reason: &'static str } }` and `eprintln!/tracing::error!(…)` for the secondary surface; route MUST go through `TraceEvent::RunRollbackFailed`. | cargo test (assertion of `trace_ring.last()`), black-hat-reviewer | `diagnostics.rs:61-64` and `tests_diagnostics.rs:73,123` are NOT touched per C-3/C-7. |
| `follow_up` linker rot | LOW | `vb-ttki3` per `to-fix/wave4/agent-12-adhoc-kani-harness.md` is the `moon ci` after-forced-push blocker, unrelated to this source gate. Reusing it in any new allow row would silently disable the allow-row ledger audit. | code-review (black-hat) + source-gate | Author MUST NOT include `follow_up=vb-ttki3` in any new allow row. This bead deletes the only row that referenced it. |

## 4. Seed → obligation mapping

| Proof Seed | REQ | Contract Clause | Domain Claim (abridged) | Required Verifier Lanes | Obligations |
|------------|-----|----------------|------------------------|--------------------------|-------------|
| S1 | REQ-vb-0x1cb-001 | C-2 | Secondary `RuntimeError` from `Shard::finish_run` rollback bound into named value + traced. | proptest, cargo-test | PO-001, PO-003 |
| S2 | REQ-vb-0x1cb-002 | C-1 | `Shard::finish_run` and `fail_run_state` return the primary `RuntimeError` regardless of rollback. | cargo-test, proptest | PO-001, PO-002, PO-003, PO-004 |
| S3 | REQ-vb-0x1cb-003 | C-3 | `TraceEvent::RunRollbackFailed` has bounded size. | flux-rs | PO-005 |
| S4 | REQ-vb-0x1cb-004 | C-3 | `observe_run_state_rollback` is `#[must_use]` + exhaustive; cannot silently drop the dual-failure case. | cargo-test (clippy slice + call_count), proptest | PO-001, PO-002, PO-006 |
| S5 | REQ-vb-0x1cb-005 | C-1, C-2 | `trace_ring` contains exactly one `RunRollbackFailed` per dual-failure call. | proptest, cargo-test | PO-001, PO-002, PO-003, PO-004 |
| S6 | REQ-vb-0x1cb-006 | C-5 | `bash scripts/check-ignored-fallible-results.sh` exits 0 for `transitions.rs` without the allow row. | moon-source-gate, bash | PO-007 |
| S7 | REQ-vb-0x1cb-007 | C-4, C-5 | `#[allow(clippy::let_underscore_must_use)]` annotations removed; follow-up scan finds zero retention. | cargo-clippy, bash | PO-006, PO-007 |

## 5. Obligation Summary (5..7)

| ID | Verifier | Target | Mode | Command | Status |
|----|----------|--------|------|---------|--------|
| PO-001 | proptest | `Shard::finish_run` rollback helper dual-failure matrix | verify-proof | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib proptest_finish_run_emits_run_rollback_failed_iff_both_fail -- --nocapture` | planned |
| PO-002 | proptest | `Shard::fail_run_state` rollback helper dual-failure matrix | verify-proof | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib proptest_fail_run_state_emits_run_rollback_failed_iff_both_fail -- --nocapture` | planned |
| PO-003 | cargo-test | `lifecycle_tests::chunk_005::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | verify-behavior | `cargo test -p vb_runtime --lib -- lifecycle_tests::chunk_005::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture` | planned |
| PO-004 | cargo-test | `lifecycle_tests::chunk_008::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | verify-behavior | `cargo test -p vb_runtime --lib -- lifecycle_tests::chunk_008::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture` | planned |
| PO-005 | flux-rs | `TraceEvent::RunRollbackFailed` size-bound extern_spec | verify-refinement | `bash scripts/flux-check-package.sh vb_runtime` (extern_spec inside `crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs`, run from workspace root via `cargo flux -p vb_runtime --message-format human`) | planned |
| PO-006 | cargo-clippy | `transitions.rs` after annotation removal + allow row gone | verify-lint | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` AND `bash scripts/check-ignored-fallible-results.sh` | planned |
| PO-007 | moon-source-gate, bash | Source-gate postcondition (zero `transitions.rs` lines, exit 0) | verify-source-gate | `bash scripts/check-ignored-fallible-results.sh` AND `moon run :lint-src` | planned |

## 6. Production binding declarations (Flux row only)

PO-005 binds via Flux `#[extern_spec]`. The extern crate resolves against the
real production types `vb_runtime::trace::TraceEvent`, `vb_runtime::RuntimeError`,
`vb_core::ids::RunId`, and the new `vb_runtime::trace::RollbackSite` enum.

Binding mechanism: **`#[extern_spec]` in
`crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs`
that mirrors the production `TraceEvent` enum and adds a `#[refined_by]` size
bound on the new `RunRollbackFailed` variant.** This is Flux's native
production-binding pattern, consistent with `vb_y9d3v_action_ticket_refinements.rs`
already merged.

No STRONG / WEAK_MIRROR / WEAK_EXTERN declaration is required at plan time —
the proof-planner SKILL exempts Flux rows from the Verus production-binding
gate. The black-hat-reviewer (PO-005 disposition) confirms the `extern_spec`
target and the `unimplemented!()` body follow the action_ticket pattern.

## 7. Behavioral test pattern (PO-003/PO-004)

Both `finish_run` and `fail_run_state` behavior tests MUST mirror
`LegacyStepFailsJournal` from `lifecycle_tests/chunk_004.rs:236-339`:

1. Construct a `SharedRuntimeJournal` stub `FinishRunRejectsJournal` (resp.
   `FailRunStateRejectsJournal`) that returns `Err(RuntimeError::StorageJournalAppend {
   source: Arc::new(vb_storage::JournalError::WriteLockPoisoned) })` for the
   `RuntimeJournalEvent::RunFinished { … }` (resp. `RunFailed { … }`) variant
   and `Ok(())` for all others.
2. Build the shard with `Shard::new_with_journal(small_config(), shared)`.
3. Submit a finished-workflow run (or fail-trigger) so the shard is in a state
   where `finish_run` / `fail_run_state` will be invoked at the next rollback
   site.
4. Tick and assert the returned `Err(RuntimeError::StorageJournalAppend { source:
   Arc(WriteLockPoisoned) })` — primary surface preserved.
5. Assert `trace_ring.last() == Some(TraceEvent::RunRollbackFailed { run, site:
   RollbackSite::FinishRun, primary: Arc<…primary>, secondary:
   Arc<…secondary_or_recovered> })`.
6. The dual-failure assertion (secondary actually fires) is OPTIONAL for v1
   per C-6; the primary-error assertion is mandatory. The proptest PO-001/PO-002
   covers the dual-failure reachability without saturating the slot map
   (it exercises the helper directly on a `Result<Option<RunState>, RuntimeError>`
   value, not through the full shard tick path).

## 8. Open questions deferred to proof-reviewer / proof-writer

1. Where to place the `RollbackSite` enum: `crates/vb_runtime/src/trace/event.rs`
   (next to its only consumer) versus `crates/vb_runtime/src/shard/transitions.rs`
   (next to the helper). Either satisfies C-3; default: `event.rs` so the
   `TraceEvent` payload type and its enum companion live together.
2. Whether the `Arc<RuntimeError>` allocation in the single-failure recovery
   case (rollback OK) leaks a heap allocation pointlessly. Decision: only
   allocate `Arc`s inside the `Err(secondary)` arm; the `Ok(_)` recovery arm
   takes the early return before any `Arc::new`.
3. Whether `is_terminal_for_run` and `run_id` for `RunRollbackFailed { run, … }`
   should reuse `run_id` (returning `*run`, since the trace IS for that run)
   or exclude the variant from `run_id`. Decision: reuse `*run` so existing
   trace-routing code finds the run; the `is_terminal_for_run` for
   `RunRollbackFailed` returns `false` per C-3.

## 9. Out-of-scope (deferred to other beads)

- The `tracing::error!` line 100 of `transitions.rs` referenced in older
  revision history; not present in current code.
- Any DiagnosticCode / symbolic_code extension in `crates/vb_runtime/src/error/diagnostics.rs`.
  The contract explicitly forbids the `Core::InternalInvariantViolation` arm
  (C-3 + Forbidden Patterns). Existing codes (`0x2001` QUEUE_FULL,
  `0x2008` STORAGE_JOURNAL_APPEND_FAILED, `0x1309` INTERNAL_INVARIANT) are
  untouched.
- Possible extension of `trace_ring` capacity. Per `codebase-map.md`, ring
  size is unchanged in this bead.

## 10. Handoff

State 4b (`proof-plan-reviewer`): owns `verifier-lane-review.jsonl` and
`proof-plan-review.md`. Approves/flags obligation truthfulness, binding
completeness, and command specificity. Zero reviewer dispositions in this
file by design.

State 5 (`proof-writer`): authors the
- new Flux extern_spec file
  `crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs`;
- proptest modules under `crates/vb_runtime/src/shard/tests/proptest_chunk_*.rs`
  OR the same `lifecycle_tests/chunk_*.rs` site;
- new `RollbackSite` enum + `TraceEvent::RunRollbackFailed` variant in
  `crates/vb_runtime/src/trace/event.rs`;
- holzman-rust edits to `transitions.rs:100,202` and annotation removals at
  `:86, :199` and the allow row in `scripts/ignored-fallible-results.allow:4`.

State 7 (`proof-to-implementation`): produces the proof→Rust bridge from
these obligations to source refs.

State 12 (`formal-verifier`): executes the verifier commands and reports the
raw evidence; not a planning artifact.

This strategy takes no disposition. Reviewer owns accept/reject; formal
verifier owns PASS/FAIL evidence.
