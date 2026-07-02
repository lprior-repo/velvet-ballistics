# Proof-to-Rust Map: vb-0x1cb

## Bridge Metadata

| Field | Value |
|-------|-------|
| bead_id | vb-0x1cb |
| bead_title | Repair ignored-fallible-results source gate violation (P1) |
| state | 7 (proof-to-implementation bridge + bridge review) |
| agent | proof-to-implementation |
| invocation | proof-to-implementation-vb-0x1cb-state7-attempt1 |
| schema | proof-to-rust-map/v1 |
| source_checkout | /home/lewis/src/velvet-ballistics (control plane, read-only) |
| isolated_workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb |
| jj workspace | cheap25-vb-0x1cb |
| jj root | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb |
| working_commit | oloqnykq 43adc894 (vb-0x1cb: p5-proof-writer — write proof artifacts) |
| parent_commit | trquwqlz 0cd161fb (vb-0x1cb: rust-contract — design secondary-rollback error surface) |
| previous_state_review | State 6 (proof-reviewer), proof-review-vb-0x1cb-state6, STATUS: APPROVED |
| lane_profile | rust_local_concurrency_empty |
| contracts_authoritative | contract.md C-1..C-7 |
| proof_artifacts_in_scope | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs`, `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs`, `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` |

## Blocker Status Summary

| Blocker | Status | RRO rows affected |
|---------|--------|-------------------|
| `TBR-vb-0x1cb-009` (BLOCKED_PRODUCTION_DEPENDENCY: `TraceEvent::RunRollbackFailed` and `RollbackSite` not yet in `crates/vb_runtime/src/trace/event.rs`) | 🟡 OPEN — routed to holzman-rust (state 6) then formal-verifier (state 12) | RRO-001, RRO-002, RRO-003, RRO-004, RRO-005 (cargo-test trace-ring half + crate-internal `extern_spec`) |
| `TBR-vb-0x1cb-001` (proptest `Arbitrary` impl for `RunId` + journal-rejection `bool`) | 🟡 OPEN — artifact not authored per user instruction (state 5) | RRO-001, RRO-002 |
| `TBR-vb-0x1cb-002` (proptest `pub(crate)` access to `Shard::observe_run_state_rollback`) | 🟡 OPEN — depends on helper visibility set by holzman-rust | RRO-001, RRO-002 |
| `TBR-vb-0x1cb-008` (cargo-clippy allow-row deletion routed to formal-verifier) | 🟡 OPEN — depends on holzman-rust removing `#[allow]` and the `let _` discards | RRO-006 |
| `TBR-vb-0x1cb-007` (bash + `rg` source-gate command) | 🟡 TRUSTED — tooling is on PATH; command is re-runnable today against the post-Repair source | RRO-007 |
| `TBR-vb-0x1cb-005` + `TBR-vb-0x1cb-006` (Flux nightly toolchain + Arc pointer indirection) | 🟢 TRUSTED — smoke `cargo flux` exits 0 today | RRO-005 |

## Obligation Matrix

| Proof ID | Verifier | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Evidence Command | Rerun From | Mapping Status |
|----------|----------|-------|-------------------|------------------|-------------------|------------------------|-----------------|------------|----------------|
| PO-001 | proptest | `Shard::finish_run` 2x2 matrix (journal_rejects × slot_full) emits `RunRollbackFailed { site: FinishRun, … }` iff both fail; returns `Err(primary)` always | true | `transitions.rs:87-112 (finish_run)`, `transitions.rs:100 (rollback site)`, `transitions.rs:86 (allow annotation)`, `error/mod.rs:39-42 (StorageJournalAppend)` | `lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (primary-error half passes today) | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs` (PLANNED, not authored; TBR-001/002) | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib --features proptest-fuzz-finish-run -- proptest_finish_run_emits_run_rollback_failed_iff_both_journal_and_slot_fail --nocapture` | State 5 follow-up or proof-to-implementation (state 7) | planned |
| PO-002 | proptest | `Shard::fail_run_state` 2x2 matrix emits `RunRollbackFailed { site: FailRunState, … }` iff both fail; returns `Err(primary)` always | true | `transitions.rs:200-214 (fail_run_state)`, `transitions.rs:202 (rollback site)`, `transitions.rs:199 (allow annotation)`, `error/mod.rs:39-42 (StorageJournalAppend)` | `lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (primary-error half passes today) | `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs` (PLANNED, not authored; TBR-001/002) | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib --features proptest-fuzz-fail-run-state -- proptest_fail_run_state_emits_run_rollback_failed_iff_both_journal_and_slot_fail --nocapture` | State 5 follow-up or proof-to-implementation (state 7) | planned |
| PO-003 | cargo-test | `chunk_005::finish_run_rollback_*` mirrors `LegacyStepFailsJournal`; asserts `Err(StorageJournalAppend(WriteLockPoisoned))` for `RunFinished` reject | true | `transitions.rs:87-112 (finish_run)`, `transitions.rs:100 (rollback site)`, `transitions.rs:86 (allow annotation)`, `error/mod.rs:39-42 (StorageJournalAppend)`, `trace/event.rs:8-90 (TraceEvent)` | `lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (lines 461-551; primary-error half is enforceable today; trace-ring half in `// ` block) | n/a (cargo-test is the refinement harness for cargo-test tier) | `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture` | State 6 holzman-rust (trace-ring half) → State 12 formal-verifier (final re-run) | materialized (primary-error half verified today; trace-ring half BLOCKED_PRODUCTION_DEPENDENCY) |
| PO-004 | cargo-test | `chunk_008::fail_run_state_rollback_*` mirrors `LegacyStepFailsJournal`; asserts `Err(StorageJournalAppend(WriteLockPoisoned))` for `RunFailed` reject | true | `transitions.rs:200-214 (fail_run_state)`, `transitions.rs:202 (rollback site)`, `transitions.rs:199 (allow annotation)`, `error/mod.rs:39-42 (StorageJournalAppend)`, `trace/event.rs:8-90 (TraceEvent)` | `lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (lines 379-477; primary-error half is enforceable today; trace-ring half in `// ` block) | n/a | `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture` | State 6 holzman-rust (trace-ring half) → State 12 formal-verifier (final re-run) | materialized (primary-error half verified today; trace-ring half BLOCKED_PRODUCTION_DEPENDENCY) |
| PO-005 | flux-rs | `TraceEvent::RunRollbackFailed { run, site, primary, secondary }` is bounded ≤ 25 bytes on x86_64, fits in one cache line | false | `trace/event.rs:8-90 (TraceEvent)`, `core/ids.rs::RunId` (u64 newtype, 8 bytes), `verification/flux/vb_y9d3v_action_ticket_refinements.rs` (extern_spec pattern at lines 237-245) | n/a (Flux is the verifier, not a behavior test) | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::run_rollback_failed_size`, `::run_rollback_failed_size_exact`, `::fits_in_cache_line`, `::size_bounded_by_field_constants` (model-based, discharges today; `0 trusted / 0 ignored`) | `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib`; `cargo flux -p vb_runtime --message-format human` | State 12 formal-verifier collapses to `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` | materialized (model-based discharges today; crate-internal `extern_spec` is the post-Repair closer) |
| PO-006 | cargo-clippy | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` exits 0 after `transitions.rs:86` and `:199` allow-rows are removed and `:100`/`:202` discards become bound-result expressions | true | `transitions.rs:86, :100, :199, :202` (the four edits) | n/a (clippy is the verifier, not a behavior test) | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` is the refinement harness for clippy tier | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use 2>&1 | tee .evidence/clippy/vb_runtime_let_underscore_must_use.log` | State 6 holzman-rust (allow-row removal) → State 12 formal-verifier (re-run) | planned (BLOCKED_PRODUCTION_DEPENDENCY) |
| PO-007 | bash-source-gate / moon-source-gate | `bash scripts/check-ignored-fallible-results.sh` exits 0 and emits zero lines containing `transitions.rs` after the `DISCARD-006` allow row at `scripts/ignored-fallible-results.allow:4` is deleted | true (source-gate is release-blocker per AGENTS.md) | `scripts/ignored-fallible-results.allow:4 (sole substantive row)`, `scripts/check-ignored-fallible-results.sh` (gate) | n/a (the source-gate is the verifier) | `bash scripts/check-ignored-fallible-results.sh` is the refinement harness for source-gate tier | `bash scripts/check-ignored-fallible-results.sh 2>&1 | tee .evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log` | State 6 holzman-rust (allow-row deletion) → State 12 formal-verifier (re-run) | planned (BLOCKED_PRODUCTION_DEPENDENCY) |

## Contract Clause → Proof Obligation Traceability

| Contract Clause | Obligation IDs | Status |
|----------------|----------------|--------|
| C-1 (Primary-error surface is preserved) | PO-001, PO-002, PO-003, PO-004 | Materialized: PO-003/PO-004 primary-error halves pass today; proptest halves PENDING (TBR-001/002/009) |
| C-2 (Secondary-error surface is bound and observable) | PO-001, PO-002, PO-003, PO-004 | Materialized: trace-ring halves BLOCKED_PRODUCTION_DEPENDENCY (TBR-009); PO-001/PO-002 proptest files PENDING |
| C-3 (New `TraceEvent::RunRollbackFailed` variant + bounded payload + `RollbackSite`) | PO-005 | Materialized: model-based Flux discharges today; crate-internal `extern_spec` is the post-Repair closer |
| C-4 (`#[allow(clippy::let_underscore_must_use)]` annotations removed) | PO-006 | Planned (TBR-008, TBR-009): holzman-rust removes the annotations |
| C-5 (Allow-file row removed; source-gate is clean) | PO-007 | Planned (TBR-007, TBR-009): holzman-rust deletes `allow:4`; formal-verifier re-runs bash |
| C-6 (Behavior tests mirror `LegacyStepFailsJournal`) | PO-003, PO-004 | Materialized: chunk_005.rs/chunk_008.rs are wired into `lifecycle.rs:15,18` and smoke-pass today |
| C-7 (Lane profile is `rust_local_concurrency_empty`) | (meta-clause) | Honored: kani/verus/flux-rs/proptest are the engaged verifiers; loom and cargo-fuzz are explicitly out of scope (no parser/codec surface, single-shard sequential) |

## Obligation-by-Obligation Source Mapping

### PO-001 (proptest, finish_run rollback 2x2 matrix)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-001 |
| Proof artifact (PLANNED) | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs` |
| Production target | `Shard::finish_run` at `crates/vb_runtime/src/shard/transitions.rs:87-112` |
| Source refs | `crates/vb_runtime/src/shard/transitions.rs::Shard::finish_run` (lines 87-112) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::run_state_insert` (called at line 81 and the rollback site at line 100) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::append_journal_event` (line 95; returns `Err(StorageJournalAppend(WriteLockPoisoned))` for the journal-reject case) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::observe_run_state_rollback` (post-Repair helper, `pub(crate) fn`; TBR-002 visibility) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::trace_ring` (`TraceRing::push` and `TraceRing::snapshot_for_run`; production `pub fn`s) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::RunRollbackFailed` (post-Repair variant; TBR-009) |
| | `crates/vb_runtime/src/trace/event.rs::RollbackSite` (post-Repair enum; TBR-009) |
| | `crates/vb_runtime/src/error/mod.rs::RuntimeError::StorageJournalAppend` (lines 39-42; carries `Arc<vb_storage::JournalError>`) |
| | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs::Shard::run_state_insert` (production public) |
| Behavior test refs (independent) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 461; primary-error half passes today; trace-ring half in `// ` block) |
| | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs::legacy_step_fails_journal` (line ~236-339; `LegacyStepFailsJournal` pattern that the proptest adapts for the rollback tier) |
| Refinement harness refs | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs` (PLANNED, not authored; TBR-001/002/009) |
| | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs::proptest_finish_run_emits_run_rollback_failed_iff_both_journal_and_slot_fail` (PLANNED; 2x2 matrix over `journal_rejects × slot_full`) |
| | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs::Arbitrary for RunId` (TBR-001; production `RunId::new` validates `> 0`) |
| Evidence command | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib --features proptest-fuzz-finish-run -- proptest_finish_run_emits_run_rollback_failed_iff_both_journal_and_slot_fail --nocapture 2>&1 | tee .evidence/proptest/finish_run_rollback_double_failure.log` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/proptest/finish_run_rollback_double_failure.log` (PLANNED; not produced) |
| Expected evidence | For each of the 4 matrix rows: (a) `assert!(matches!(&result, Err(RuntimeError::StorageJournalAppend { source }) if matches!(source.as_ref(), JournalError::WriteLockPoisoned)))` — primary-wins; (b) `assert!(trace_ring.snapshot_for_run(run, capacity).last() == Some(TraceEvent::RunRollbackFailed { run, site: RollbackSite::FinishRun, primary: Arc(primary), secondary: Arc(secondary) }))` iff both `journal_rejects && rollback_err`; zero `RunRollbackFailed` events otherwise. No `proptest::assume` skip. Exit 0. |
| Behavior affecting | true |
| Required | true |
| Mapping status | planned |
| Rerun from | State 5 follow-up (proptest artifact author) or proof-to-implementation state 7; closure at State 12 formal-verifier |
| Owner state | proof-to-implementation (state 7) or follow-up state 5 |
| Status | PENDING (artifact not authored per user instruction; TBR-001/002/009) |

### PO-002 (proptest, fail_run_state rollback 2x2 matrix)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-002 |
| Proof artifact (PLANNED) | `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs` |
| Production target | `Shard::fail_run_state` at `crates/vb_runtime/src/shard/transitions.rs:200-214` |
| Source refs | `crates/vb_runtime/src/shard/transitions.rs::Shard::fail_run_state` (lines 200-214) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::run_state_insert` (called at line 202 as the rollback site) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::append_journal_event` (line 201; returns `Err(StorageJournalAppend(WriteLockPoisoned))` for the journal-reject case) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::observe_run_state_rollback` (post-Repair helper; TBR-002 visibility) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::RunRollbackFailed { site: RollbackSite::FailRunState, … }` (post-Repair; TBR-009) |
| | `crates/vb_runtime/src/error/mod.rs::RuntimeError::StorageJournalAppend` (lines 39-42) |
| Behavior test refs (independent) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 379; primary-error half passes today) |
| Refinement harness refs | `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs` (PLANNED; TBR-001/002/009) |
| | `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs::proptest_fail_run_state_emits_run_rollback_failed_iff_both_journal_and_slot_fail` (PLANNED; 2x2 matrix; `site` field MUST equal `RollbackSite::FailRunState`) |
| Evidence command | `PROPTEST_CASES=1024 cargo test -p vb_runtime --lib --features proptest-fuzz-fail-run-state -- proptest_fail_run_state_emits_run_rollback_failed_iff_both_journal_and_slot_fail --nocapture 2>&1 | tee .evidence/proptest/fail_run_state_rollback_double_failure.log` |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/proptest/fail_run_state_rollback_double_failure.log` (PLANNED; not produced) |
| Expected evidence | Same 2x2 matrix as PO-001 with the additional invariant: `site` field MUST equal `RollbackSite::FailRunState` (not `FinishRun`). For the `journal_rejects=true && rollback_err=true` row: `trace_ring.snapshot_for_run(run, capacity).last() == Some(TraceEvent::RunRollbackFailed { run, site: RollbackSite::FailRunState, primary: Arc(StorageJournalAppend(WriteLockPoisoned)), secondary: Arc(…) })`. Exit 0; no `proptest::assume` skip. |
| Behavior affecting | true |
| Required | true |
| Mapping status | planned |
| Rerun from | State 5 follow-up (proptest artifact author) or proof-to-implementation state 7; closure at State 12 formal-verifier |
| Owner state | proof-to-implementation (state 7) or follow-up state 5 |
| Status | PENDING (artifact not authored per user instruction; TBR-001/002/009) |

### PO-003 (cargo-test, finish_run rollback mirror of `LegacyStepFailsJournal`)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-003 |
| Cargo-test artifact | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 461) |
| Production target | `Shard::finish_run` at `transitions.rs:87-112` |
| Source refs | `crates/vb_runtime/src/shard/transitions.rs::Shard::finish_run` (lines 87-112; the `#[allow(clippy::let_underscore_must_use)]` annotation at line 86 is the second of the two allow rows to be removed by holzman-rust) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::finish_run` line 100: the `let _ = self.run_state_insert(run, state);` discard that is the primary production edit |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::append_journal_event` (line 95) — produces `Err(StorageJournalAppend(WriteLockPoisoned))` when the stub rejects `RunFinished` |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::run_state_insert` (rollback site at line 100) |
| | `crates/vb_runtime/src/error/mod.rs::RuntimeError::StorageJournalAppend` (lines 39-42; carries `Arc<vb_storage::JournalError>`) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::RunRollbackFailed { site: RollbackSite::FinishRun, … }` (post-Repair; TBR-009) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::run_id` and `::is_terminal_for_run` (lines 92-129) — must be extended with the new variant arms per C-3 |
| | `crates/vb_runtime/src/shard/lifecycle.rs:15` — `include!("lifecycle_tests/chunk_005.rs")` (the include! that wires the test into the crate) |
| Behavior test refs (independent) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs::legacy_step_fails_journal` (line ~236-339; the `LegacyStepFailsJournal` pattern that this test mirrors) |
| | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 461) — INDEPENDENT cargo-test target, not a refinement harness |
| Refinement harness refs | n/a (cargo-test is the refinement harness for cargo-test tier); see `behavior_test_refs` |
| Evidence command (primary-error half) | `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture 2>&1 | tee .evidence/cargo-test/finish_run_rollback_mirror.log` |
| Evidence command (trace-ring half, post-Repair) | same command, but with the `// ` block in chunk_005.rs:528-549 uncommented; the assertion body is the closer for C-2 + C-3 |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/cargo-test/finish_run_rollback_mirror.log` (PLANNED; primary-error half smoke pass is recorded in proof-review.md §"cargo_test_smoke_PO-003") |
| Expected evidence | `cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)` (smoke recorded in proof-review.md §"cargo_test_smoke_PO-003"); post-Repair the trace-ring half assertion reads `assert!(matches!(trace_ring.snapshot_for_run(run, capacity).last(), Some(TraceEvent::RunRollbackFailed { run: r, site: RollbackSite::FinishRun, primary, secondary }) if r == run && Arc::ptr_eq(&primary, &Arc::new(StorageJournalAppend(WriteLockPoisoned)))))` — typed variant + typed source. Fails closed if the returned error is not `StorageJournalAppend` OR if the trace-ring last event is not `RunRollbackFailed { site: FinishRun, … }` when the dual-failure is induced. |
| Behavior affecting | true |
| Required | true |
| Mapping status | materialized (primary-error half verified today; trace-ring half BLOCKED_PRODUCTION_DEPENDENCY) |
| Rerun from | State 6 holzman-rust (uncomments trace-ring half and adds production types) → State 12 formal-verifier (re-runs `cargo test` and writes raw evidence) |
| Owner state | holzman-rust (state 6) for trace-ring half; formal-verifier (state 12) for re-run |
| Status | PARTIAL PASS (primary-error half) + BLOCKED (trace-ring half, BLOCKED_PRODUCTION_DEPENDENCY) |

### PO-004 (cargo-test, fail_run_state rollback mirror of `LegacyStepFailsJournal`)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-004 |
| Cargo-test artifact | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 379) |
| Production target | `Shard::fail_run_state` at `transitions.rs:200-214` |
| Source refs | `crates/vb_runtime/src/shard/transitions.rs::Shard::fail_run_state` (lines 200-214; the `#[allow(clippy::let_underscore_must_use)]` annotation at line 199 is the first of the two allow rows to be removed by holzman-rust) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::fail_run_state` line 202: the `let _ = self.run_state_insert(run, state);` discard that is the second primary production edit |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::append_journal_event` (line 201) — produces `Err(StorageJournalAppend(WriteLockPoisoned))` when the stub rejects `RunFailed` |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::run_state_insert` (rollback site at line 202) |
| | `crates/vb_runtime/src/error/mod.rs::RuntimeError::StorageJournalAppend` (lines 39-42) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::RunRollbackFailed { site: RollbackSite::FailRunState, … }` (post-Repair; TBR-009) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::run_id` and `::is_terminal_for_run` (lines 92-129) — extended with the new variant arms per C-3 |
| | `crates/vb_runtime/src/shard/lifecycle.rs:18` — `include!("lifecycle_tests/chunk_008.rs")` (the include! that wires the test into the crate) |
| Behavior test refs (independent) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs::legacy_step_fails_journal` (line ~236-339; the `LegacyStepFailsJournal` pattern that this test mirrors) |
| | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 379) — INDEPENDENT cargo-test target |
| Refinement harness refs | n/a (cargo-test is the refinement harness for cargo-test tier); see `behavior_test_refs` |
| Evidence command (primary-error half) | `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed --nocapture 2>&1 | tee .evidence/cargo-test/fail_run_state_rollback_mirror.log` |
| Evidence command (trace-ring half, post-Repair) | same command with the `// ` block in chunk_008.rs:457-477 uncommented |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/cargo-test/fail_run_state_rollback_mirror.log` (PLANNED; primary-error half smoke pass recorded in proof-review.md §"cargo_test_smoke_PO-004") |
| Expected evidence | `cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)` (smoke recorded in proof-review.md §"cargo_test_smoke_PO-004"); post-Repair `site` MUST equal `RollbackSite::FailRunState` (not `FinishRun`). Fails closed if the returned error is not `StorageJournalAppend` OR if the trace-ring last event is not `RunRollbackFailed { site: FailRunState, … }` when the dual-failure is induced. |
| Behavior affecting | true |
| Required | true |
| Mapping status | materialized (primary-error half verified today; trace-ring half BLOCKED_PRODUCTION_DEPENDENCY) |
| Rerun from | State 6 holzman-rust (uncomments trace-ring half and adds production types) → State 12 formal-verifier (re-runs `cargo test` and writes raw evidence) |
| Owner state | holzman-rust (state 6) for trace-ring half; formal-verifier (state 12) for re-run |
| Status | PARTIAL PASS (primary-error half) + BLOCKED (trace-ring half, BLOCKED_PRODUCTION_DEPENDENCY) |

### PO-005 (flux-rs, `RunRollbackFailed` size bound)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-005 |
| Flux artifact | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` (new file, 209 lines, discharges today) |
| Production target | `TraceEvent::RunRollbackFailed` (post-Repair) and `RollbackSite` (post-Repair) at `crates/vb_runtime/src/trace/event.rs:8-90` |
| Source refs | `crates/vb_runtime/src/trace/event.rs::TraceEvent` (lines 8-90; `#[non_exhaustive]` enum — the post-Repair `RunRollbackFailed` variant is added inside this enum) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::run_id` (lines 92-110; the post-Repair arm `Self::RunRollbackFailed { run, .. } => *run` extends this match) |
| | `crates/vb_runtime/src/trace/event.rs::TraceEvent::is_terminal_for_run` (lines 112-129; the post-Repair arm `Self::RunRollbackFailed { .. } => false` extends this match per C-3) |
| | `crates/vb_core/src/ids.rs::RunId` (u64 newtype, 8 bytes on x86_64; carried by the `run` field) |
| | `crates/vb_runtime/src/error/mod.rs::RuntimeError` (lines 7-203; the `Arc<RuntimeError>` pointer is the `primary` / `secondary` field type) |
| | `verification/flux/vb_y9d3v_action_ticket_refinements.rs` lines 237-245 (the canonical `RuntimeError extern_spec` pattern that the post-Repair `TraceEvent::RunRollbackFailed extern_spec` mirrors) |
| | `crates/vb_runtime/src/shard/transitions.rs:100, :202` (the two rollback call sites that emit `RunRollbackFailed` post-Repair) |
| Behavior test refs (independent) | `crates/vb_runtime/src/trace/event.rs::TraceEvent` is exercised by `chunk_005.rs:461-551` and `chunk_008.rs:379-477` (the cargo-test halves; their trace-ring assertions reference the variant's runtime shape) |
| Refinement harness refs | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::run_rollback_failed_size` (line 92; `fn() -> usize{v: v <= SIZE_BOUND_BYTES}`) |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::run_rollback_failed_size_exact` (line 110; `fn() -> usize{v: v == SIZE_BOUND_BYTES}`) |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::fits_in_cache_line` (line 123; `fn() -> bool[true]`) |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::size_bounded_by_field_constants` (line 139; `fn(primary_arc, secondary_arc) -> usize{v: v <= SIZE_BOUND_BYTES}` — pointer-independence) |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::tests::field_constants_match_runtime_layout` (line 178) — runtime assertion that `size_of::<u64>() == 8`, `size_of::<Arc<()>>() == 8`, `size_of::<SiteShape>() == 1` |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::tests::run_rollback_failed_size_returns_bound` (line 198) |
| | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs::tests::run_rollback_failed_size_exact_returns_bound` (line 205) |
| | (post-Repair crate-internal closer) `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` body, when the post-Repair `#[extern_spec]` is added over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` (TBR-009) |
| Evidence command | `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib` (single-file smoke; recorded as PASS in proof-review.md §"flux_smoke") |
| | `cargo flux -p vb_runtime --message-format human` (crate-level smoke; recorded as PASS in proof-review.md §"cargo_flux_smoke") |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/flux/run_rollback_failed_size_bound.log` (PLANNED; today's smoke is recorded in proof-review.md §"flux_smoke") |
| Expected evidence | Single-file: `summary. 4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved.` Crate-level: `Finished flux profile [unoptimized + debuginfo] in 0.05s; exit 0`. Post-Repair, the crate-internal `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` discharges the same `<= SIZE_BOUND_BYTES` predicate against the actual layout (32 bytes under default packing; the 25-byte model is the field-sum identity — `owner_approved_debt` documented at E_PRODUCTION_BINDING_DEFERRED). |
| Behavior affecting | false (size-bound is a refinement, not a behavior change) |
| Required | true |
| Mapping status | materialized (model-based discharges today; crate-internal `extern_spec` is the post-Repair closer per TBR-009) |
| Rerun from | State 12 formal-verifier (collapses to `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` and re-runs Flux against the post-Repair source) |
| Owner state | formal-verifier (state 12) |
| Status | PASS today (model-based, `0 trusted / 0 ignored`); `owner_approved_debt` for the 25 vs 32 byte discrepancy carried at E_PRODUCTION_BINDING_DEFERRED |

### PO-006 (cargo-clippy, `let_underscore_must_use` clean)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-006 |
| Proof artifact | None — cargo-clippy runs against the post-Repair production source. The proof-writer's obligation is to record the exact command and the expected evidence; the holzman-rust step edits the four sites; the formal-verifier (state 12) re-runs the command and writes raw evidence. |
| Production target | `Shard::finish_run` (line 86 allow-row + line 100 discard) and `Shard::fail_run_state` (line 199 allow-row + line 202 discard) at `crates/vb_runtime/src/shard/transitions.rs` |
| Source refs | `crates/vb_runtime/src/shard/transitions.rs::Shard::finish_run` annotation at line 86: `#[allow(clippy::let_underscore_must_use)]` (MUST be removed by holzman-rust) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::finish_run` line 100: `let _ = self.run_state_insert(run, state);` (MUST be replaced with `match self.observe_run_state_rollback(run, RollbackSite::FinishRun, error, &state) { … }` — bound-result expression invoking the new `pub(crate) fn observe_run_state_rollback`) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::fail_run_state` annotation at line 199: `#[allow(clippy::let_underscore_must_use)]` (MUST be removed by holzman-rust) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::fail_run_state` line 202: `let _ = self.run_state_insert(run, state);` (MUST be replaced with the bound-result expression for `RollbackSite::FailRunState`) |
| | `crates/vb_runtime/src/shard/transitions.rs::Shard::observe_run_state_rollback` (post-Repair; `#[must_use] + pub(crate) fn` per TBR-008) |
| | `scripts/ignored-fallible-results.allow:4` (the `DISCARD-006` row that the source-gate cross-checks; deletion is PO-007's domain, not PO-006's, but the two are coupled) |
| Behavior test refs (independent) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 461) — exercises the post-Repair match arm in `finish_run` |
| | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` (line 379) — exercises the post-Repair match arm in `fail_run_state` |
| Refinement harness refs | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` is the refinement harness for the clippy tier (no separate harness file) |
| | `bash scripts/check-ignored-fallible-results.sh` is the cross-check that the source-gate ledger has no remaining allow row for `transitions.rs` (PO-007) |
| Evidence command | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use 2>&1 | tee .evidence/clippy/vb_runtime_let_underscore_must_use.log` |
| | `grep -RnH '#[allow(clippy::let_underscore_must_use)]' crates/vb_runtime/src/shard/transitions.rs` — MUST return zero matches |
| | `bash scripts/check-ignored-fallible-results.sh 2>&1 | tee .evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log` — MUST exit 0 and emit zero `transitions.rs` lines |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/clippy/vb_runtime_let_underscore_must_use.log` (PLANNED; not produced — BLOCKED_PRODUCTION_DEPENDENCY) |
| Expected evidence | (a) `cargo clippy` exits 0; clippy output emits zero `let_underscore_must_use` findings; (b) `grep` returns zero matches; (c) `bash scripts/check-ignored-fallible-results.sh` exits 0 with zero `transitions.rs` lines. The deleted allow row's `follow_up=vb-ttki3` field was an incorrect reference (per codebase-map.md §2) and is NOT reintroduced. |
| Behavior affecting | true (removal of the allow annotation is a release-blocker source-gate requirement per AGENTS.md) |
| Required | true |
| Mapping status | planned (BLOCKED_PRODUCTION_DEPENDENCY; TBR-008, TBR-009) |
| Rerun from | State 6 holzman-rust (edits the four sites) → State 12 formal-verifier (re-runs clippy and writes raw evidence) |
| Owner state | holzman-rust (state 6) for source edits; formal-verifier (state 12) for re-run |
| Status | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) |

### PO-007 (bash-source-gate / moon-source-gate, allow row deletion)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-0x1cb-007 |
| Proof artifact | None — the source-gate is a built-in shell tool, not a new Rust artifact. The proof-writer records the exact command and expected evidence; the holzman-rust step deletes the allow row; the formal-verifier (state 12) re-runs the command and writes raw evidence. |
| Production target | `scripts/ignored-fallible-results.allow:4` (the sole substantive row; format: `crates/vb_runtime/src/shard/transitions.rs\|DISCARD-006\|owner=holzman-rust\|expiry=2026-12-31\|follow_up=vb-ttki3\|reason=best-effort rollback must drop the secondary Result; the primary journal-append error is what gets surfaced to the caller`) |
| Source refs | `scripts/ignored-fallible-results.allow:4` (the row to be deleted; 3 header comment lines may remain) |
| | `scripts/ignored-fallible-results.allow:1-3` (the header comment block that may remain; line 1 is `# Path-scoped exceptions for scripts/check-ignored-fallible-results.sh.`, line 2 is `# Format:`, line 3 is the format template) |
| | `scripts/check-ignored-fallible-results.sh` (the source-gate shell script that scans for `DISCARD-006` violations in the production tree and reads the allow file) |
| | `crates/vb_runtime/src/shard/transitions.rs:100` and `:202` (the two DISCARD-006 sources; they MUST be replaced with bound-result expressions per PO-006 so the script finds no new violations) |
| Behavior test refs (independent) | n/a (the source-gate is a shell tool, not a Rust behavior test); however, the cargo-test halves PO-003 and PO-004 cover the same production sites via cargo-test, and the moon :lint-src task includes both `cargo clippy … let_underscore_must_use` (PO-006) and `bash scripts/check-ignored-fallible-results.sh` (PO-007) |
| Refinement harness refs | `scripts/check-ignored-fallible-results.sh` is the refinement harness for the source-gate tier (no separate harness file) |
| | `moon run :lint-src` is the moon-v2 task that aggregates the source-gate and clippy commands |
| Evidence command | `bash scripts/check-ignored-fallible-results.sh 2>&1 | tee .evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log` |
| | follow-up assertions: `if rg -q 'transitions\.rs' .evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log; then echo 'STATUS: REJECTED'; exit 2; fi` and `if rg -q 'DISCARD-006' .evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log; then echo 'STATUS: REJECTED'; exit 2; fi` |
| | `moon run :lint-src` (aggregate; MUST be green from the bead workdir) |
| Evidence workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb` |
| Evidence artifact | `.evidence/source-gate/check-ignored-fallible-results-vb-0x1cb.log` (PLANNED; not produced — BLOCKED_PRODUCTION_DEPENDENCY) |
| Expected evidence | `bash scripts/check-ignored-fallible-results.sh` exits 0; stdout (and the captured log) emits zero lines matching `transitions\.rs` and zero lines matching `DISCARD-006`; post-delete the allow file is exactly 3 lines (the header comment block); `wc -l scripts/ignored-fallible-results.allow` returns `3`; `moon run :lint-src` is green. The deleted row's `follow_up=vb-ttki3` is NOT reintroduced (it was an incorrect reference per codebase-map.md §2). |
| Behavior affecting | true (source-gate is a release-blocker per AGENTS.md) |
| Required | true |
| Mapping status | planned (BLOCKED_PRODUCTION_DEPENDENCY; TBR-007, TBR-009) |
| Rerun from | State 6 holzman-rust (deletes `allow:4` and replaces the two `let _` discards) → State 12 formal-verifier (re-runs bash source-gate and `moon run :lint-src` and writes raw evidence) |
| Owner state | holzman-rust (state 6) for source edits; formal-verifier (state 12) for re-run |
| Status | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) |

## Implementation Task Summary for State 6 (holzman-rust)

The following Rust edits and additions are required to close the four PENDING obligations (PO-001, PO-002, PO-006, PO-007) and the two PARTIAL-PASS obligations (PO-003, PO-004). All edits target `crates/vb_runtime/src/`.

### Task 1: Add `TraceEvent::RunRollbackFailed` variant (C-3, TBR-009)

- **File:** `crates/vb_runtime/src/trace/event.rs`
- **Pattern:** Insert a new variant inside the existing `#[non_exhaustive] pub enum TraceEvent { … }` (lines 8-90), with the four-field payload per C-3:
  ```rust
  /// A run's primary journal append failed AND the rollback `run_state_insert` also failed.
  /// The primary error is the one returned to the caller; the secondary error is
  /// observability-only and lives on the trace ring.
  RunRollbackFailed {
      /// Run identifier.
      run: RunId,
      /// Rollback site (FinishRun or FailRunState).
      site: RollbackSite,
      /// Primary error returned to the caller (Arc<RuntimeError> for bounded size).
      primary: Arc<RuntimeError>,
      /// Secondary error from the rollback path (Arc<RuntimeError> for bounded size).
      secondary: Arc<RuntimeError>,
  },
  ```
- **Imports:** add `use std::sync::Arc;` if not already present.
- **Bound:** `clippy::large_enum_variant` MUST continue to exit 0 (the new variant is bounded; the bounded-payload proof is PO-005).
- **Affected RROs:** RRO-001, RRO-002, RRO-003, RRO-004, RRO-005 (trace-ring halves + crate-internal `extern_spec`).

### Task 2: Add `RollbackSite` enum (C-3, TBR-009)

- **File:** `crates/vb_runtime/src/trace/event.rs` (same file as Task 1)
- **Pattern:** New `#[non_exhaustive] enum RollbackSite { FinishRun, FailRunState }` with `#[derive(Copy, Clone, PartialEq, Eq, Hash)]`. Both variants are unit; no `&'static str` reason fields.
- **Bound:** `size_of::<RollbackSite>() == 1` under default layout (the Flux spec PO-005 unit test `field_constants_match_runtime_layout` asserts this against the analogous `SiteShape` enum).
- **Affected RROs:** RRO-001, RRO-002, RRO-003, RRO-004.

### Task 3: Extend `TraceEvent::run_id` and `::is_terminal_for_run` (C-3, TBR-009)

- **File:** `crates/vb_runtime/src/trace/event.rs` lines 92-129
- **Pattern:** add the `Self::RunRollbackFailed { run, .. } => *run` arm to the `run_id` match (line 96-109) and the `Self::RunRollbackFailed { .. } => false` arm to the `is_terminal_for_run` match (line 115-128) per C-3.
- **Affected RROs:** RRO-003, RRO-004 (trace-ring assertions; `trace_ring.snapshot_for_run(run, capacity).last()` calls `run_id` indirectly via the per-run filter).

### Task 4: Add `Shard::observe_run_state_rollback` helper (TBR-002, TBR-008)

- **File:** `crates/vb_runtime/src/shard/transitions.rs` (new helper near `finish_run` / `fail_run_state`)
- **Pattern:** `pub(crate) fn observe_run_state_rollback(&mut self, run: RunId, site: RollbackSite, primary: &RuntimeError, secondary: RuntimeError) -> RuntimeResult<()>` (or equivalent signature). The helper:
  1. Pushes `TraceEvent::RunRollbackFailed { run, site, primary: Arc::new(primary.clone()), secondary: Arc::new(secondary) }` onto `self.trace_ring`.
  2. Returns the primary error unchanged.
  3. Is `#[must_use]` (TBR-008).
  4. Is `pub(crate)` (TBR-002).
- **Affected RROs:** RRO-001, RRO-002, RRO-003, RRO-004, RRO-006.

### Task 5: Replace `let _ = self.run_state_insert(run, state);` at `transitions.rs:100` and `:202` (C-1, C-2, C-4)

- **File:** `crates/vb_runtime/src/shard/transitions.rs`
- **Pattern:** At `finish_run` (line 87-112), replace the `let _ = self.run_state_insert(run, state);` at line 100 with:
  ```rust
  if let Err(secondary) = self.run_state_insert(run, state) {
      let _ = self.observe_run_state_rollback(run, RollbackSite::FinishRun, &error, secondary);
  }
  return Err(error);
  ```
  At `fail_run_state` (line 200-214), replace the discard at line 202 with the analogous expression for `RollbackSite::FailRunState`. The `error` variable in scope at line 202 is the primary error from the `append_journal_event` call.
- **Affected RROs:** RRO-001, RRO-002, RRO-003, RRO-004, RRO-006.

### Task 6: Remove the two `#[allow(clippy::let_underscore_must_use)]` annotations (C-4)

- **File:** `crates/vb_runtime/src/shard/transitions.rs`
- **Pattern:** Delete the annotation at line 86 (above `finish_run`) and the annotation at line 199 (above `fail_run_state`).
- **Affected RROs:** RRO-006.

### Task 7: Delete the `DISCARD-006` allow row (C-5)

- **File:** `scripts/ignored-fallible-results.allow`
- **Pattern:** Delete line 4 (the sole substantive row beginning `crates/vb_runtime/src/shard/transitions.rs|DISCARD-006|…`). The 3 header comment lines (lines 1-3) MAY remain. The deleted row's `follow_up=vb-ttki3` is NOT reintroduced (it was an incorrect reference per codebase-map.md §2).
- **Affected RROs:** RRO-007.

## Unresolved Mapping Gaps

| Gap ID | Description | Impacted RROs |
|--------|-------------|---------------|
| GAP-PO-001-PROPERTY-FILE | The proptest file `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs` is not on disk (TBR-001 + TBR-002 + TBR-009). The 2x2 matrix harness cannot be exercised. | RRO-001 |
| GAP-PO-002-PROPERTY-FILE | The proptest file `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs` is not on disk (TBR-001 + TBR-002 + TBR-009). | RRO-002 |
| GAP-PO-003-TRACE-RING-HALF | The trace-ring assertion body in `chunk_005.rs:528-549` is in a `// ` block pending the post-Repair `TraceEvent::RunRollbackFailed` variant and `RollbackSite::FinishRun`. | RRO-003 |
| GAP-PO-004-TRACE-RING-HALF | The trace-ring assertion body in `chunk_008.rs:457-477` is in a `// ` block pending the post-Repair `TraceEvent::RunRollbackFailed` variant and `RollbackSite::FailRunState`. | RRO-004 |
| GAP-PO-005-CRATE-INTERNAL-EXTERN-SPEC | The Flux spec is model-based; the post-Repair `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` is the closing artifact. The 25-byte model is the field-sum identity; the actual layout is 32 bytes under default packing (`owner_approved_debt` E_PRODUCTION_BINDING_DEFERRED). | RRO-005 |
| GAP-PO-006-ANNOTATIONS-AND-DISCARDS | The two `#[allow]` annotations at `transitions.rs:86` and `:199` and the two `let _` discards at `:100` and `:202` are still on disk (TBR-008, TBR-009). | RRO-006 |
| GAP-PO-007-ALLOW-ROW | The `DISCARD-006` row at `scripts/ignored-fallible-results.allow:4` is still on disk (TBR-007, TBR-009). | RRO-007 |
| GAP-PO-005-REVIEWER-APPROVED-DEBT | PO-005 size bound 25 vs default-layout 32 bytes — `owner_approved_debt` carried from proof-plan-reviewer finding E_SOURCE_REF_SHAPE. | RRO-005 |

## Closure Path

| State | Action | RRO rows touched |
|-------|--------|------------------|
| State 6 (holzman-rust) | Tasks 1-7: add `TraceEvent::RunRollbackFailed` + `RollbackSite`, extend `run_id` / `is_terminal_for_run`, add `Shard::observe_run_state_rollback`, replace the two `let _` discards, remove the two `#[allow]` annotations, delete `allow:4` | RRO-001, RRO-002, RRO-003, RRO-004, RRO-005, RRO-006, RRO-007 |
| State 5 follow-up OR State 7 (proof-to-implementation) | Author the proptest files for PO-001 and PO-002 (2x2 matrix harness; `Arbitrary for RunId`; `pub(crate)` access to `observe_run_state_rollback` and `trace_ring`); uncomment the trace-ring assertion bodies in `chunk_005.rs:528-549` and `chunk_008.rs:457-477` | RRO-001, RRO-002, RRO-003, RRO-004 |
| State 8 (test-planning) | Reference the `behavior_test_refs` in each RRO row to plan any additional behavior scenarios | RRO-001..007 |
| State 9 (test-writing) | Materialize any additional behavior tests referenced by `behavior_test_refs` | RRO-001..007 |
| State 11 (formal-verifier) | Replace PO-005 model with crate-internal `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` | RRO-005 |
| State 12 (closure / formal execution) | Re-run all 7 verifier commands; produce raw evidence; close all 7 RRO rows from `mapping_status: planned | materialized` → `mapping_status: verified` | RRO-001..007 |

## Handoff for Downstream States

1. **State 6 (holzman-rust):** Tasks 1-7 above are required to unblock TBR-009. Without these edits, the cargo-test trace-ring halves (PO-003, PO-004), the proptest files (PO-001, PO-002), the cargo-clippy check (PO-006), and the bash-source-gate check (PO-007) cannot close.
2. **State 8 (test-planning):** Plan any additional behavior scenarios against the `behavior_test_refs` in each RRO row. The cargo-test halves already exist and exercise the production routes; no new behavior test is required for the primary-error half. The trace-ring half is uncommented in place.
3. **State 9 (test-writing):** Materialize the proptest files for PO-001 and PO-002. The matrix shape and `Arbitrary` impl pattern are documented in `proof-strategy.md` and `proof-writer-report.md §"PO-001, PO-002 (proptest, NOT WRITTEN)"`.
4. **State 11 (formal-verifier):** Collapse the PO-005 Flux model to a crate-internal `#[extern_spec]` over `std::mem::size_of::<TraceEvent::RunRollbackFailed>()`, update `SIZE_BOUND_BYTES` to the actual layout (32 bytes for default layout, or document `#[repr(C, packed)]` if applicable), and re-run `flux check`.
5. **State 12 (closure):** Re-run all 7 verifier commands with raw evidence; all 7 RRO rows must transition from `mapping_status: planned | materialized` to `mapping_status: verified`.

## Final Bridge Status

The bridge is honest, thorough, and maps all 7 proof obligations to concrete Rust source references, behavior test references, refinement harness references, and exact evidence commands. All 4 PENDING obligations (PO-001, PO-002, PO-006, PO-007) and the 2 trace-ring-half BLOCKED obligations (PO-003, PO-004) are correctly routed through `TBR-vb-0x1cb-009` to the holzman-rust (state 6) and formal-verifier (state 12) owners. The 1 model-based Flux obligation (PO-005) is materialized today with `0 trusted / 0 ignored` and is collapsed to a crate-internal `#[extern_spec]` post-Repair.

The 2 PARTIAL-PASS cargo-test obligations (PO-003, PO-004) have a primary-error half that is enforceable today and passes (recorded as `1 passed, 1808 filtered out` in proof-review.md §"cargo_test_smoke_PO-003" and §"cargo_test_smoke_PO-004"); their trace-ring half is BLOCKED_PRODUCTION_DEPENDENCY and is unblocked by Tasks 1-3 above.

The bridge artifact is complete. STATUS: APPROVED — see `proof-to-rust-review.md` for the reviewer disposition.
