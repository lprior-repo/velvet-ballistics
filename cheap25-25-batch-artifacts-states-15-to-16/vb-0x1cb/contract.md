# Contract — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: contract (canonical surface)
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This is the single canonical surface statement for `vb-0x1cb`. The downstream proof-planner, test-planner, holzman-rust implementer, and black-hat reviewer MUST satisfy these clauses. Failure to satisfy any clause is a `STATUS: REJECTED` event.

## Scope

Replace the two `let _ = self.run_state_insert(run, state);` statements at `crates/vb_runtime/src/shard/transitions.rs:100` and `:202` with a typed, bound-result expression that surfaces the secondary `RuntimeError` via the runtime diagnostic path. Remove the `#[allow(clippy::let_underscore_must_use)]` annotations at lines 86 and 199 and remove the `DISCARD-006` allow row from `scripts/ignored-fallible-results.allow`. Mirror the `LegacyStepFailsJournal` test pattern in two new behavior tests.

## Canonical decision

The runtime diagnostic path is the dual channel of typed errors (`RuntimeError`) and observability (`TraceEvent`). Per the bead focus, the secondary error from the rollback branch is bound and surfaced via:

- **`TraceEvent::RunRollbackFailed { run, site, primary, secondary }`** — observability channel.
- The function return type stays `RuntimeResult<()>` returning the primary error.

Alternative `RuntimeError::Core { source: CoreError::InternalInvariantViolation { reason } }` path is rejected for blast-radius reasons (would force `diagnostics.rs:47-105` and 7 test files to be re-audited).

## Clauses

### C-1. Primary-error surface is preserved

The `Result::Err(_)` returned by `Shard::finish_run` and `Shard::fail_run_state` carries the **primary** error from `append_journal_event(RuntimeJournalEvent::RunFinished { … } | RunFailed { … })`. The function MUST NOT return the secondary error in any circumstance.

Witness: behavior test mirroring `LegacyStepFailsJournal` at `chunk_004.rs:240-319`. The test asserts `Err(RuntimeError::StorageJournalAppend { source: Arc(vb_storage::JournalError::WriteLockPoisoned) | QueueFull | … })` regardless of whether the rollback succeeds or fails.

### C-2. Secondary-error surface is bound and observable

When the rollback `run_state_insert(...)` returns `Err(secondary)` after a primary error, the secondary MUST be bound into a named value (no `let _`, no `Ok(_)|Err(_)=>{}`) and MUST be visible on the runtime diagnostic path.

Visibility path: `self.trace_ring.push(TraceEvent::RunRollbackFailed { run, site, primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> })`.

Witness: behavior test asserts the trace ring contains exactly one `RunRollbackFailed { … }` per dual-failure event, with `primary == Arc::new(Err(p))` and `secondary == Arc::new(Err(s))`.

### C-3. New `TraceEvent` variant is added with bounded payload

A new variant `TraceEvent::RunRollbackFailed { run, site, primary, secondary }` is added inside the existing `#[non_exhaustive]` enum at `crates/vb_runtime/src/trace/event.rs`. `Arc<RuntimeError>` is used to keep allocation bounded; `RuntimeError` is already `Clone + PartialEq + Eq` so the variant rides on the existing derive.

`TraceEvent::run_id(&self)` MUST be extended with an explicit arm:

```rust
Self::RunRollbackFailed { run, .. } => *run,
```

`TraceEvent::is_terminal_for_run(&self, target: RunId)` MUST NOT classify `RunRollbackFailed` as terminal. Explicit non-inclusion: `Self::RunRollbackFailed { .. } => false`.

A new `RollbackSite` enum (`#[non_exhaustive]`) is added with two variants: `FinishRun`, `FailRunState`. Both are `Copy + Eq + Hash`. No `&'static str` reason fields.

Witness: `cargo doc -p vb_runtime --no-deps` exits 0. Source-lint: `clippy::large_enum_variant` exits 0 (the new variant is bounded).

### C-4. `#[allow(clippy::let_underscore_must_use)]` annotations removed

The annotations at `transitions.rs:86` (above `finish_run`) and `:199` (above `fail_run_state`) MUST be removed.

### C-5. Allow-file row removed

The `scripts/ignored-fallible-results.allow` MUST have its sole substantive row deleted. The header comment block (3 lines) MAY remain. Post-delete:

```
$ wc -l scripts/ignored-fallible-results.allow
3
$ bash scripts/check-ignored-fallible-results.sh
…
JustifiedException|…
…
$ # zero JustifiedException rows for transitions.rs; exits 0
```

The deleted row's `follow_up=vb-ttki3` field was an incorrect reference (per `codebase-map.md` §2: vb-ttki3 is "moon CI after forced push", unrelated). After this bead closes there is no follow-up to encode.

Witness: `bash scripts/check-ignored-fallible-results.sh` exits 0; the script's stdout emits ZERO lines containing `transitions.rs`.

### C-6. Behavior tests mirror `LegacyStepFailsJournal`

Two behavior tests are added under `crates/vb_runtime/src/shard/lifecycle_tests/`:

- `chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` — rejects `RunFinished` with a typed `JournalError::WriteLockPoisoned`; asserts `Err(StorageJournalAppend(WriteLockPoisoned))`; verifies `trace_ring.last() == RunRollbackFailed { … }` only when the rollback also fails.
- `chunk_008.rs` (new file) — `fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` — same pattern for `RunFailed` with `RollbackSite::FailRunState`.

Both tests:

- live under `lifecycle_tests/` (the source-gate skip list at `check-ignored-fallible-results.sh:62-72` allows this path; a `tests.rs` at the crate root would re-trigger DISCARD).
- include `SharedRuntimeJournal` stub matching `LegacyStepFailsJournal` (rejects one journal event, returns `Ok(())` for the rest).
- inject `ShardConfig::shared(..)` that pins `runs` capacity to 1 so the second `run_state_insert` fails (the stub for `reserve_run_state_slot` is implicit; the test planner crafts a config where `runs` already contains the run when the rollback tries to insert — which currently fails only on overflow). The dual-failure case is OPTIONAL for v1; the **primary-error** assertion is mandatory.

If `chunk_008.rs` is added (it does not currently exist on disk), `crates/vb_runtime/src/shard/lifecycle.rs` MUST include it via `mod tests { include!(...); }` style include!; the existing `lifecycle.rs` is a re-export shim.

Witness: `cargo test -p vb_runtime --lib -- lifecycle_tests::chunk_005::finish_run_rollback_*` and `lifecycle_tests::chunk_008::fail_run_state_rollback_*` exit 0.

### C-7. Lane profile is rust_local_concurrency_empty

Verifiers engaged: `kani`, `verus`, `flux-rs`, `proptest`. Verifiers NOT engaged: `loom` (single-shard sequential), `cargo-fuzz` (no parser/codec surface). Each proof seed in `proof-seeds.jsonl` MUST cite exactly one or more of the engaged verifiers and provide its model boundary.

Lane decisions and the per-row `verifier-lane-decision/v1` artifact are owned by the proof-planner downstream, NOT by this contract. This clause pins the bead's *scope of verifier engagement* only.

## Forbidden patterns under the contract

- `let _ = self.run_state_insert(run, state);` — DISCARD-006 violation under the gate.
- `match self.run_state_insert(run, state) { Ok(_) | Err(_) => {} }` — same class.
- `Err(secondary)` returned instead of `Err(primary)` — primary masking.
- New `RuntimeError` variant added.
- New `RuntimeError::Core { source: CoreError::InternalInvariantViolation { .. } }` match arm in `diagnostics.rs`.
- `#[allow(clippy::let_underscore_must_use)]` retained on either rollback site.
- `eprintln!("…")` or `tracing::error!(…)` for the secondary surface.
- Allow-file row reintroduced with a stale `follow_up`.

## Cross-references

- `domain-model.md` §Invariants I1–I6.
- `type-contracts.md` §1.1, §1.2, §2.2.
- `workflow-model.md` §1, §2.
- `error-taxonomy.md` §2, §3.
- `boundary-map.md` §2.1, §2.4, §3.
- `hazard-analysis.md` §H-INV-1..4, §H-DIAG-1, §H-REL-1.
- `proof-seeds.jsonl` rows S1–S7.
- `traceability-matrix.jsonl` rows R1–R9.
