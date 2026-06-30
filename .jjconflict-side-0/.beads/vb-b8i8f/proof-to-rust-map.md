# Proof-to-Rust Map: vb-b8i8f

## Bridge Metadata

| Field | Value |
|-------|-------|
| Bead | vb-b8i8f |
| State | 7 (proof-to-implementation bridge) |
| Agent | proof-to-implementation |
| Invocation | vb-b8i8f-state7-proof-to-implementation-attempt1 |
| Schema | proof-to-rust-map/v1 |
| Source checkout | /home/lewis/src/velvet-ballistics (control plane, read-only) |
| Workspace | /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f |
| Previous state review | State 6, vb-b8i8f-state6-proof-reviewer-attempt3, REJECTED |

## Blocker Status Summary

| Blocker | Status | RRO rows affected |
|---------|--------|-------------------|
| vb-b8i8f-BLOCK-001 (kind 28 excluded from validation.rs:10..=27) | ✅ RESOLVED — validation.rs range extended to 10..=28 | RRO-004, RRO-009, RRO-014, RRO-019..022 |
| vb-b8i8f-BLOCK-002 (Full Shard Kani construction requires SharedRuntimeJournal→Fjall chain) | ⚠️ STILL BLOCKED for Kani runtime harnesses | RRO-006..008 |
| GOD RULE 2 (Verus requires/ensures on production exec fn) | 🔴 Deferred to State 11 — see RRO-001..005 | RRO-001..005 |
| Flux trust abuse + dead code + missing dep | 🔴 Deferred to State 11 — see RRO-011..015 | RRO-011..015 |

## Obligation Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|-------------------|------------------|-------------------|------------------------|----------|-----------------|------------|
| PO-VERUS-001 | Cancel/Kill live-only spec model | true | `Shard::handle_cancel`, `Shard::handle_kill` | `cancel_kill_lattice_tests.rs` | `verification/verus/cancel_kill_lattice.rs` | verus | `verus --crate-type=lib verification/verus/cancel_kill_lattice.rs` | State 11 |
| PO-VERUS-002 | Single terminal winner spec model | true | `Shard::handle_cancel`, `Shard::handle_kill` | `cancel_kill_lattice_tests.rs`, `cancel_kill_lattice_props.rs` | `verification/verus/cancel_kill_lattice.rs` | verus | `verus --crate-type=lib verification/verus/cancel_kill_lattice.rs` | State 11 |
| PO-VERUS-003 | Stale authority spec model | true | `Shard::handle_timer`, `Shard::handle_ask_answer` | `cancel_kill_lattice_tests.rs` | `verification/verus/cancel_kill_lattice.rs` | verus | `verus --crate-type=lib verification/verus/cancel_kill_lattice.rs` | State 11 |
| PO-VERUS-004 | Kind 28 admission spec model | true | `validation::is_known_record_kind`, `validation::validate_kind_family` | `cancel_kill_lattice_props.rs` | `verification/verus/storage_kind_family.rs` | verus | `verus --crate-type=lib verification/verus/storage_kind_family.rs` | State 11 |
| PO-VERUS-005 | Replay contiguity spec model | true | `replay::events_for_run`, `codec::validate_replayed_event` | `proptest_storage.rs` | `verification/verus/storage_kind_family.rs` | verus | `verus --crate-type=lib verification/verus/storage_kind_family.rs` | State 11 |
| PO-KANI-001 | Cancel/Kill live-only bounded check | true | `Shard::handle_cancel`, `Shard::handle_kill` | `cancel_kill_lattice_tests.rs` | `kani_cancel_kill_lattice.rs` | kani | `cargo kani -p vb_runtime` | State 11 |
| PO-KANI-002 | Single terminal winner bounded check | true | `Shard::handle_cancel`, `Shard::handle_kill` | `cancel_kill_lattice_tests.rs` | `kani_cancel_kill_lattice.rs` | kani | `cargo kani -p vb_runtime` | State 11 |
| PO-KANI-003 | Stale authority bounded check | true | `Shard::handle_timer`, `Shard::handle_ask_answer` | `cancel_kill_lattice_tests.rs` | `kani_cancel_kill_lattice.rs` | kani | `cargo kani -p vb_runtime` | State 11 |
| PO-KANI-004 | Kind 28 admission bounded check | true | `validation::is_known_record_kind`, `validation::validate_kind_family` | `cancel_kill_lattice_props.rs` | `kani_record_kind.rs` | kani | `KANI_FEATURES=legacy-kani cargo kani -p vb_storage` | Post BLOCK-001 |
| PO-KANI-005 | Replay with killed bounded check | true | `replay::events_for_run`, `codec::validate_replayed_event` | `proptest_storage.rs` | `kani_record_kind.rs` | kani | `KANI_FEATURES=legacy-kani cargo kani -p vb_storage` | Post BLOCK-001 |
| PO-FLUX-001 | Cancel/Kill return type refinement | true | `Shard::handle_cancel`, `Shard::handle_kill` | `cancel_kill_lattice_tests.rs` | `flux_cancel_kill.rs` | flux-rs | `bash scripts/flux-check-package.sh vb_runtime` | State 11 |
| PO-FLUX-002 | Terminal membership refinement | true | `Shard::terminal_runs` | `cancel_kill_lattice_tests.rs` | `flux_cancel_kill.rs` | flux-rs | `bash scripts/flux-check-package.sh vb_runtime` | State 11 |
| PO-FLUX-003 | Stale authority refinement | true | `Shard::handle_timer`, `Shard::handle_ask_answer` | `cancel_kill_lattice_tests.rs` | `flux_cancel_kill.rs` | flux-rs | `bash scripts/flux-check-package.sh vb_runtime` | State 11 |
| PO-FLUX-004 | Kind range refinement | true | `validation::validate_kind_family`, `validation::is_known_record_kind` | `cancel_kill_lattice_props.rs` | `flux_validation.rs` | flux-rs | `bash scripts/flux-check-package.sh vb_storage` | State 11 |
| PO-FLUX-005 | Playback contiguity refinement | true | `replay::events_for_run` | `proptest_storage.rs` | `flux_validation.rs` | flux-rs | `bash scripts/flux-check-package.sh vb_storage` | State 11 |
| PO-PROP-001 | RunKilled validation properties | true | `JournalEvent::RunKilled`, `RecordKind::RunKilled` | `cancel_kill_lattice_props.rs` | `cancel_kill_lattice_props.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` | none (verified) |
| PO-PROP-002 | RecordKind uniqueness properties | true | `RecordKind` all variants | `cancel_kill_lattice_props.rs` | `cancel_kill_lattice_props.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` | none (verified) |
| PO-PROP-003 | RunKilled field consistency | true | `JournalEvent::RunKilled`, `JournalEvent::RunCancelled` | `cancel_kill_lattice_props.rs` | `cancel_kill_lattice_props.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` | none (verified) |
| PO-PROP-004 | Kind 28 round-trip properties | true | `encode_record`, `decode_record`, `decode_journal_event` | `proptest_storage.rs` | `proptest_storage.rs` | proptest | `cargo test -p vb_storage -- proptest` | State 11 |
| PO-PROP-005 | Replay sequence properties | true | `replay::events_for_run` | `proptest_storage.rs` | `proptest_storage.rs` | proptest | `cargo test -p vb_storage -- replay` | State 11 |
| PO-FUZZ-001 | Kind validation fuzz | true | `validation::validate_kind_family` | `cancel_kill_lattice_props.rs`, `proptest_storage.rs` | `fuzz/fuzz_targets/kind_validation.rs` | cargo-fuzz | `cargo +nightly fuzz run kind_validation -- -max_len=8 -runs=100000` | State 11 |
| PO-FUZZ-002 | Journal decode fuzz | true | `decode_record`, `decode_journal_event` | `proptest_storage.rs`, `cancel_kill_lattice_props.rs` | `fuzz/fuzz_targets/journal_decode.rs` | cargo-fuzz | `cargo +nightly fuzz run journal_decode -- -max_len=4096 -runs=100000` | State 11 |

## Contract Clause → Proof Obligation Traceability

| Contract Clause | Obligation IDs | Status |
|----------------|----------------|--------|
| C1 (Public Kill API) | PO-PROP-001 | Planned; Runtime::kill_run missing |
| C2 (Cancel/Kill Missing + Already-Terminal) | PO-VERUS-001, PO-KANI-001, PO-FLUX-001, PO-PROP-001 | Mixed: proptest PASS, formal layers rejected |
| C3 (Single Terminal Journal Event) | PO-VERUS-002, PO-KANI-002, PO-FLUX-002, PO-PROP-002 | Mixed: proptest PASS, formal layers rejected |
| C4 (Stale Action/Timer Cleanup) | PO-VERUS-003, PO-KANI-003, PO-FLUX-003, PO-PROP-003 | Mixed: proptest PASS, formal layers rejected |
| C5 (Durable Kill Storage Admission) | PO-VERUS-004, PO-KANI-004, PO-FLUX-004, PO-PROP-004, PO-FUZZ-001 | Mixed: Kani PASS, proptest BLOCKED, formal layers rejected, fuzz PENDING |
| C6 (Replay Integrity) | PO-VERUS-005, PO-KANI-005, PO-FLUX-005, PO-PROP-005, PO-FUZZ-002 | Mixed: Kani PASS, proptest BLOCKED, formal layers rejected, fuzz PENDING |

## Obligation-by-Obligation Source Mapping

### PO-VERUS-001 (Cancel/Kill Live-Only — Rejected, GOD RULE 2 gap)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-001 |
| Verus artifact | `verification/verus/cancel_kill_lattice.rs` (18 lemmas proven, model-only) |
| Production target | `Shard::handle_cancel` at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:115` |
| | `Shard::handle_kill` at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:134` |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` (lines 115-132) |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_kill` (lines 134-149) |
| | `crates/vb_runtime/src/shard/types.rs::ShardCommand::Cancel` (line 163), `ShardCommand::Kill` (line 170) |
| | `crates/vb_runtime/src/shard/types.rs::Shard::runs` (IndexMap<RunId, RunState>) |
| | `crates/vb_runtime/src/shard/types.rs::Shard::terminal_runs` (IndexSet<RunId>) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::hp1_cancel_running_run_transitions_to_cancelled` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::test_kill_live_run` (to be added, State 8-10) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_valid_event_passes_validation` |
| Refinement harness refs | `verification/verus/cancel_kill_lattice.rs` (spec_model, lines 1-287) |
| Current gap | GOD RULE 2: Zero `requires`/`ensures` on production `handle_cancel`/`handle_kill`. Verus spec proves `spec_terminalize` model-internally but does not constrain the production functions. `#[verifier::external_body]` on `classify_run_has_correct_semantics` returns `true` unconditionally — vacuous trust anchor. |
| Required fix (State 11) | Add `requires`/`ensures` to `handle_cancel` and `handle_kill` referencing `spec_terminalize`, `spec_is_live`, and `RUN_ABSENT_RETURNS_ERR` from Verus spec model. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-VERUS-002 (Single Terminal Winner — Rejected, GOD RULE 2 gap)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-002 |
| Production target | `Shard::handle_cancel` at `chunk_002.rs:121-129` (terminal_runs.insert + counters.inc_failed) |
| | `Shard::handle_kill` at `chunk_002.rs:140-146` (terminal_runs.insert + counters.inc_failed) |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` lines 121-131 |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_kill` lines 139-148 |
| | `crates/vb_runtime/src/shard/types.rs::Shard::terminal_runs` (IndexSet<RunId>) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::inv1_terminal_never_regresses_after_cancel` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::inv1_completed_run_terminal_never_regresses` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_unique` |
| Refinement harness refs | `verification/verus/cancel_kill_lattice.rs::lemma_single_terminal_winner` |
| Current gap | Same GOD RULE 2 gap. `spec_single_terminal_winner` is proven for the model enum but not attached to production IndexSet<RunId>. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-VERUS-003 (Stale Authority Rejection — Rejected, GOD RULE 2 gap)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-003 |
| Production target | `Shard::handle_timer` at `chunk_002.rs:78` |
| | `Shard::handle_ask_answer` at `chunk_002.rs:16` |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer` (lines 78-113) |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_ask_answer` (lines 16-76) |
| | `crates/vb_runtime/src/shard/types.rs::Shard::pending_timers` (IndexMap<RunId, PendingTimer>) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::hp4_action_after_cancel_returns_error` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::test_timer_after_kill_returns_error` (to be added, State 8-10) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_carries_attempt` |
| Refinement harness refs | `verification/verus/cancel_kill_lattice.rs::lemma_stale_authority_rejected` |
| Mapping status | planned |
| behavior_affecting | true |

### PO-VERUS-004 (Kind 28 Admission — Rejected, GOD RULE 2 gap)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-004 |
| Production target | `is_known_record_kind` at `validation.rs:23` |
| | `validate_kind_family` at `validation.rs:42` |
| Source refs | `crates/vb_storage/src/codec/validation.rs::is_known_record_kind` (line 23, matches! pattern including 28 post-BLOCK-001) |
| | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42, MAGIC_JOURNAL_EVENT branch 10..=28 post-BLOCK-001) |
| | `crates/vb_storage/src/codec/validation.rs::validate_known_kind` (line 35) |
| | `crates/vb_storage/src/codec/mod.rs::encode_record` (line 21, calls validate_kind_family) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_valid` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_is_known_record_kind` (BLOCKED by compile error) |
| Refinement harness refs | `verification/verus/storage_kind_family.rs::lemma_kind_28_is_known` (model-only) |
| | `verification/verus/storage_kind_family.rs::lemma_kind_28_journal_family_ok` (model-only) |
| Current gap | GOD RULE 2: `spec_is_known_record_kind(28)` proved model-internally. No Verus contract on production `is_known_record_kind(28)`. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-VERUS-005 (Replay Contiguity — Rejected, GOD RULE 2 gap)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-005 |
| Production target | `validate_replay_sequence` / `events_for_run` at `replay.rs` |
| Source refs | `crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run` (line 53) |
| | `crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run_bounded` (line 54) |
| | `crates/vb_storage/src/codec/mod.rs::validate_replayed_event` (line 73) |
| | `crates/vb_storage/src/codec/mod.rs::next_seq` (line 66) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs::prop_replay_contiguity_mixed_kinds` (BLOCKED by compile error) |
| Refinement harness refs | `verification/verus/storage_kind_family.rs::lemma_replay_contiguity_with_killed_invariant` (model-only) |
| Mapping status | planned |
| behavior_affecting | true |

### PO-KANI-001 (Cancel/Kill Live-Only Kani Harnesses — Rejected, dead code + vacuous)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-006 |
| Kani artifact | `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` (20 harnesses, 380 lines) |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_kill` |
| | `crates/vb_runtime/src/shard/types.rs::Shard::runs` (IndexMap) |
| | `crates/vb_runtime/src/shard/types.rs::Shard::pending_timers` (IndexMap) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` (all cancel/kill scenarios) |
| Refinement harness refs | `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` (10 production-exercising harnesses: lines 39-166, 364-379) |
| Current gaps | (1) File NOT wired into crate — no `verification/mod.rs`, no `#[cfg(kani)] pub mod kani;` in `lib.rs`. Dead code. (2) 10 of 20 harnesses use local boolean variables modeling IndexMap/IndexSet semantics — structurally vacuous. (3) BLOCK-002: full Shard construction requires SharedRuntimeJournal→Fjall chain. (4) Two harnesses have zero assertions (`check_cancel_safe_for_boundary_run_ids`, `check_kill_safe_for_boundary_run_ids`). |
| Required fix | (1) Create `verification/mod.rs` + `verification/kani/mod.rs` wiring. (2) Remove 10 boolean-model harnesses or replace with real IndexMap/IndexSet Kani harnesses. (3) Reduce to 10 production-exercising harnesses. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-KANI-002 (Single Terminal Winner Kani — Rejected, dead code + vacuous)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-007 |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` lines 121-129 |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_kill` lines 139-146 |
| | `crates/vb_runtime/src/shard/types.rs::Shard::terminal_runs` (IndexSet) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::inv1_terminal_never_regresses_after_cancel` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_distinct_from_cancelled` |
| Refinement harness refs | `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` harnesses: `check_terminal_runs_insert_idempotent`, `check_terminal_runs_contains_after_insert`, `check_cancel_then_kill_no_double_terminal`, `check_kill_then_cancel_no_double_terminal` (lines 168-217) |
| Current gaps | Same dead code + vacuity gaps as PO-KANI-001. 5/5 harnesses for this tier use boolean models. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-KANI-003 (Stale Authority Kani — Rejected, dead code + vacuous)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-008 |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer` (lines 78-113) |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_ask_answer` (lines 16-76) |
| | `crates/vb_runtime/src/shard/types.rs::Shard::pending_timers` (IndexMap) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::hp4_action_after_cancel_returns_error` |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_carries_attempt` |
| Refinement harness refs | `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` harnesses: `check_stale_timer_after_cancel_pending_timers_empty`, `check_stale_timer_after_kill_pending_timers_empty`, `check_stale_ask_after_cancel_run_not_found`, `check_stale_ask_after_kill_run_not_found` (lines 218-297) |
| Current gaps | Same dead code + vacuity gaps as PO-KANI-001. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-KANI-004 (Kind 28 Admission Kani — PASSING, wired into vb_storage)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-009 |
| Kani artifact | `crates/vb_storage/src/kani_record_kind.rs` (wired via `lib.rs:44`) |
| Source refs | `crates/vb_storage/src/codec/validation.rs::is_known_record_kind` (line 23) |
| | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42) |
| | `crates/vb_storage/src/codec/validation.rs::validate_known_kind` (line 35) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_valid` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_is_known_record_kind` |
| Refinement harness refs | `crates/vb_storage/src/kani_record_kind.rs::check_kind_28_known` (line ~XX, asserts `is_known_record_kind(28)==true`) |
| | `crates/vb_storage/src/kani_record_kind.rs::check_kind_28_journal_family` (asserts `validate_kind_family(MAGIC_JOURNAL_EVENT,28)==Ok`) |
| | `crates/vb_storage/src/kani_record_kind.rs::check_kind_28_snapshot_family_rejected` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_kind_28_blob_family_rejected` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_unknown_kind_rejected` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_all_existing_kinds_known` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_journal_family_exhaustive` |
| Evidence command | `KANI_FEATURES=legacy-kani cargo kani -p vb_storage` |
| Evidence workdir | `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f` |
| Evidence artifact | `.evidence/kani/vb_storage/kani_record_kind_success.log` |
| Status | Wired into `lib.rs:44` with `#[cfg(kani)] mod kani_record_kind;`. Production-bound. GOD RULE 1 compliant. Uses `kani::any()` for exhaustive kind-space. Non-vacuous. BLOCK-001 RESOLVED enables these harnesses to pass. |
| Mapping status | materialized (wired, needs re-execution post BLOCK-001 fix) |
| behavior_affecting | true |

### PO-KANI-005 (Replay with Killed Kani — PASSING, wired into vb_storage)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-010 |
| Source refs | `crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run` (line 53) |
| | `crates/vb_storage/src/codec/mod.rs::validate_replayed_event` (line 73) |
| | `crates/vb_storage/src/codec/mod.rs::next_seq` (line 66) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs::prop_replay_contiguity_mixed_kinds` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_replay_gap_detection` |
| Refinement harness refs | `crates/vb_storage/src/kani_record_kind.rs::check_replay_contiguity_with_killed` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_replay_sequence_gap_detection` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_replay_duplicate_detection` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_runkilled_fields_preserved` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_runkilled_zero_run_invalid` |
| | `crates/vb_storage/src/kani_record_kind.rs::check_runkilled_zero_attempt_invalid` |
| Evidence command | `KANI_FEATURES=legacy-kani cargo kani -p vb_storage` |
| Mapping status | materialized |
| behavior_affecting | true |

### PO-FLUX-001 (Cancel/Kill Return Type Refinement — Rejected, dead code + #[trusted])

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-011 |
| Flux artifact | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` (194 lines, dead code) |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` (line 115, returns `RuntimeResult<()>`) |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_kill` (line 134, returns `RuntimeResult<()>`) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` all cancel/kill scenarios |
| Refinement harness refs | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::HANDLE_CANCEL_FLUX_SIG` (string constant, not an actual annotation) |
| | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::HANDLE_KILL_FLUX_SIG` (string constant) |
| Current gaps | (1) File not mod-declared in `lifecycle.rs` — dead code. (2) `flux_rs` dependency missing from `vb_runtime/Cargo.toml`. (3) `flux` feature not defined. (4) `#[flux_rs::sig]` annotations encoded as `&str` constants, not actual Rust annotations. (5) Unit test asserts `!HANDLE_CANCEL_FLUX_SIG.is_empty()` — not that any refinement holds. (6) All functions effectively `#[trusted]`. |
| Required fix | Add `flux_rs` dep + feature to Cargo.toml. Wire `flux_cancel_kill` into `lifecycle.rs`. Apply actual `#[flux_rs::sig(...)]` annotations to production `handle_cancel`/`handle_kill`. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FLUX-002 (Terminal Membership Refinement — Rejected, dead code + #[trusted])

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-012 |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_cancel` lines 121-129 |
| | `crates/vb_runtime/src/shard/types.rs::Shard::terminal_runs` (IndexSet<RunId>) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::inv1_terminal_never_regresses_after_cancel` |
| Refinement harness refs | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::TERMINAL_RUNS_MEMBERSHIP` (string constant) |
| | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::SINGLE_TERMINAL_INVARIANT` (string constant) |
| Current gaps | Same dead code + missing dep + trust abuse as PO-FLUX-001. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FLUX-003 (Timer/Ask Post-Terminal Refinement — Rejected, dead code + #[trusted])

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-013 |
| Source refs | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer` (lines 78-113) |
| | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_ask_answer` (lines 16-76) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs::hp4_action_after_cancel_returns_error` |
| Refinement harness refs | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::STALE_TIMER_FLUX_SIG` (string constant) |
| | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::STALE_ASK_FLUX_SIG` (string constant) |
| | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs::STALE_AUTHORITY_INVARIANT` (string constant) |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FLUX-004 (Kind Range Refinement — Rejected, missing dep + #[trusted])

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-014 |
| Flux artifact | `crates/vb_storage/src/codec/flux_validation.rs` (mod-declared in codec/mod.rs:97, but inoperable) |
| Source refs | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42, range 10..=28 post-BLOCK-001) |
| | `crates/vb_storage/src/codec/validation.rs::is_known_record_kind` (line 23) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_valid` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_is_known_record_kind` |
| Refinement harness refs | `crates/vb_storage/src/codec/flux_validation.rs::KIND_28_FLUX_INVARIANT` (string constant) |
| | `crates/vb_storage/src/codec/flux_validation.rs::KIND_28_KNOWN_INVARIANT` (string constant) |
| Current gaps | (1) `flux_rs` dependency missing from `vb_storage/Cargo.toml`. (2) `flux` feature not defined. (3) All functions `#[flux_rs::trusted]`. (4) Unit test at line 108 has no assertion — both match arms are no-ops. Even though `codec/mod.rs` now has `mod flux_validation;` (controller fix), the module cannot compile or verify. |
| Required fix (State 11) | Add `flux_rs` dep + feature. Remove `#[trusted]` from `model_is_known_record_kind` (const fn). Apply `#[flux_rs::sig]` directly to production `is_known_record_kind` in `validation.rs`. Fix unit test to assert. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FLUX-005 (Playback Contiguity Refinement — Rejected, missing dep + #[trusted])

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-015 |
| Source refs | `crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run` (line 53) |
| | `crates/vb_storage/src/codec/mod.rs::validate_replayed_event` (line 73) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs::prop_replay_contiguity_mixed_kinds` |
| Refinement harness refs | `crates/vb_storage/src/codec/flux_validation.rs::REPLAY_CONTIGUITY_INVARIANT` (string constant) |
| | `crates/vb_storage/src/codec/flux_validation.rs::REPLAY_NO_DUPLICATE_INVARIANT` (string constant) |
| | `crates/vb_storage/src/codec/flux_validation.rs::RUNKILLED_FIELD_PRESERVATION` (string constant) |
| | `crates/vb_storage/src/codec/flux_validation.rs::RUNKILLED_KIND_ID_STABLE` (string constant) |
| Mapping status | planned |
| behavior_affecting | true |

### PO-PROP-001 (Live-Only Cancel/Kill Proptest — PASSING, 10/10 tests)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-016 |
| Proptest artifact | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` (213 lines, registered test target) |
| Source refs | `crates/vb_storage/src/events.rs::JournalEvent::RunKilled` (line 213) |
| | `crates/vb_storage/src/records.rs::RecordKind::RunKilled` (line 171, id 28) |
| | `crates/vb_core/src/ids.rs::RunId` |
| | `crates/vb_storage/src/types.rs::EventSeq` |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_valid` (line 25) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_valid_event_passes_validation` (line 44) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_zero_run_invalid` (line 63) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_zero_attempt_invalid` (line 74) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_overflow_seq_invalid` (line 85) |
| Refinement harness refs | N/A (proptest is the refinement harness) |
| Evidence command | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` |
| Evidence workdir | `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f` |
| Evidence artifact | State 6 evidence log showing 10/10 pass |
| Status | PASSING. Non-vacuous. Exercises production `JournalEvent::RunKilled`, `RecordKind::RunKilled`, `is_valid()`, `record_kind()`. Production-bound. |
| Mapping status | verified (10/10 tests pass as of State 6 evidence) |
| behavior_affecting | true |

### PO-PROP-002 (Single Terminal Winner Proptest — PASSING)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-017 |
| Source refs | `crates/vb_storage/src/records.rs::RecordKind` all variants (line 139) |
| | `crates/vb_storage/src/events.rs::JournalEvent::RunKilled` (line 213) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_record_kind_28_is_unique` (line 101) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_journal_kinds_in_valid_range` (line 142) |
| Evidence command | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` |
| Status | PASSING. Non-vacuous. Verifies id 28 uniqueness across all RecordKind variants. |
| Mapping status | verified |
| behavior_affecting | true |

### PO-PROP-003 (Stale Authority Proptest — PASSING)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-018 |
| Source refs | `crates/vb_storage/src/events.rs::JournalEvent::RunKilled` (line 213) |
| | `crates/vb_storage/src/events.rs::JournalEvent::RunCancelled` (line 202) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_carries_attempt` (line 159) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_record_kind_consistent` (line 175) |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs::prop_runkilled_distinct_from_cancelled` (line 191) |
| Evidence command | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props` |
| Status | PASSING. Non-vacuous. Verifies RunKilled distinct from RunCancelled, attempt() field preserved, record_kind() consistent. |
| Mapping status | verified |
| behavior_affecting | true |

### PO-PROP-004 (Kind 28 Round-Trip Proptest — BLOCKED, compile error)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-019 |
| Proptest artifact | `crates/vb_storage/src/proptest_storage.rs` |
| Source refs | `crates/vb_storage/src/codec/mod.rs::encode_record` (line 21) |
| | `crates/vb_storage/src/codec/mod.rs::decode_record` (line 35) |
| | `crates/vb_storage/src/codec/mod.rs::decode_journal_event` (line 54) |
| | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_id_is_stable` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_is_known_record_kind` (currently documents gap with eprintln!) |
| | `crates/vb_storage/src/proptest_storage.rs::prop_runkilled_encode_decode_roundtrip` (currently documents gap with eprintln!) |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_rejected_for_wrong_magic` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_kind_28_rejected_for_blob_magic` |
| Evidence command | `cargo test -p vb_storage -- proptest` |
| Current gap | Pre-existing compile error at `proptest_storage.rs:317` blocks execution. Gap-documentation tests use `eprintln!()` instead of `prop_assert!()` and always pass regardless of kind 28 admission. |
| Required fix | Fix `proptest_storage.rs:317` compile error. Replace `eprintln!` gap docs with `prop_assert!` so tests flip from gap to pass when BLOCK-001 resolves |
| Mapping status | planned |
| behavior_affecting | true |

### PO-PROP-005 (Replay Sequence Proptest — BLOCKED, compile error)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-020 |
| Source refs | `crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run` (line 53) |
| | `crates/vb_storage/src/codec/mod.rs::validate_replayed_event` (line 73) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs::prop_replay_contiguity_mixed_kinds` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_replay_gap_detection` |
| | `crates/vb_storage/src/proptest_storage.rs::prop_replay_duplicate_detection` |
| Evidence command | `cargo test -p vb_storage -- replay` |
| Current gap | Same `proptest_storage.rs:317` compile error blocks execution. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FUZZ-001 (Kind Validation Fuzz — PENDING execution)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-021 |
| Fuzz artifact | `fuzz/fuzz_targets/kind_validation.rs` |
| Source refs | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42) |
| | `crates/vb_storage/src/codec/validation.rs::is_known_record_kind` (line 23) |
| | `crates/vb_storage/src/codec/validation.rs::validate_known_kind` (line 35) |
| Behavior test refs | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` — all kind validation props |
| | `crates/vb_storage/src/proptest_storage.rs` — all kind round-trip props |
| Refinement harness refs | `fuzz/fuzz_targets/kind_validation.rs` (fuzz harness) |
| Evidence command | `cargo +nightly fuzz run kind_validation -- -max_len=8 -runs=100000` |
| Evidence workdir | `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f` |
| Evidence artifact | `.evidence/fuzz/kind_validation_run.log` |
| Expected evidence | Zero crashes/panics across 100k iterations with arbitrary (magic, kind) pairs including boundary values 0x00000000, 0xFFFFFFFF and all six magic constants. |
| Mapping status | planned |
| behavior_affecting | true |

### PO-FUZZ-002 (Journal Decode Fuzz — PENDING execution)

| Field | Value |
|-------|-------|
| RRO ID | RRO-vb-b8i8f-022 |
| Fuzz artifact | `fuzz/fuzz_targets/journal_decode.rs` |
| Source refs | `crates/vb_storage/src/codec/mod.rs::decode_record` (line 35) |
| | `crates/vb_storage/src/codec/mod.rs::decode_journal_event` (line 54) |
| | `crates/vb_storage/src/codec/validation.rs::validate_kind_family` (line 42) |
| Behavior test refs | `crates/vb_storage/src/proptest_storage.rs` — all round-trip props |
| | `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` — all RunKilled validation props |
| Refinement harness refs | `fuzz/fuzz_targets/journal_decode.rs` (fuzz harness) |
| Evidence command | `cargo +nightly fuzz run journal_decode -- -max_len=4096 -runs=100000` |
| Expected evidence | Zero crashes/panics across 100k iterations with arbitrary byte streams. All errors are typed `JournalError` variants (not panics). RunKilled events maintain structural validity. |
| Mapping status | planned |
| behavior_affecting | true |

## Implementation Task Summary for States 8-11

The following Rust implementation tasks are required to close the gaps identified in this bridge:

### Task 1: Add `Runtime::kill_run` (Contract C1)
- **File**: `crates/vb_runtime/src/runtime.rs`
- **Pattern**: Mirrors `cancel_run` at line 174-177
- **Code**:
  ```rust
  pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
      let shard = self.shard_for(run)?;
      shard.enqueue(ShardCommand::Kill { run, reason: None })
  }
  ```
- **Affected RROs**: RRO-016 (PO-PROP-001 behavior test coverage)

### Task 2: Fix cancel/kill return semantics (Contract C2)
- **File**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
- **Change**: `handle_cancel` and `handle_kill` must return `Err` for missing/already-terminal runs instead of `Ok(())`
- **Current behavior**: Both return `Ok(())` when run doesn't exist or is already terminal
- **Required behavior**: Return `Err(RuntimeError::RunNotFound)` or typed terminal error
- **Affected RROs**: RRO-001, RRO-006, RRO-011 (cancel/kill live-only contract)

### Task 3: Wire Kani runtime harnesses
- **Files**: Create `crates/vb_runtime/src/verification/mod.rs` and `verification/kani/mod.rs`
- **Affected RROs**: RRO-006..008

### Task 4: Wire Flux into vb_runtime and vb_storage (State 11)
- **Files**: `vb_runtime/Cargo.toml`, `vb_storage/Cargo.toml`, `lifecycle.rs`, `validation.rs`
- **Affected RROs**: RRO-011..015

### Task 5: Add Verus requires/ensures to production functions (State 11)
- **Files**: `chunk_002.rs`, `validation.rs`
- **GOD RULE 2 gap**: Attach spec models to production exec fn
- **Affected RROs**: RRO-001..005

### Task 6: Fix proptest compile error + convert gap docs to assertions
- **File**: `crates/vb_storage/src/proptest_storage.rs`
- **Affected RROs**: RRO-019..020

### Task 7: Execute fuzz targets
- **Affected RROs**: RRO-021..022

## Handoff for proof-reviewer

The following artifacts form the complete bridge output:

| Artifact | Path | Purpose |
|----------|------|---------|
| proof-to-rust-map.md | `.beads/vb-b8i8f/proof-to-rust-map.md` | Human-readable obligation-to-source mapping |
| rust-refinement-obligations.jsonl | `.beads/vb-b8i8f/rust-refinement-obligations.jsonl` | Machine-readable RRO rows (schemas: rust-refinement-obligation/v1) |
| agent-invocation-ledger.jsonl | `.beads/vb-b8i8f/agent-invocation-ledger.jsonl` | Updated with seq 11 entry |

## Unresolved Mapping Gaps

| Gap ID | Description | Impacted RROs |
|--------|-------------|---------------|
| GAP-VERUS-PRODUCTION-BINDING | GOD RULE 2: Verus specs are mathematically correct but unattached to production exec fn. Requires/ensures needed on handle_cancel, handle_kill, is_known_record_kind, validate_kind_family. | RRO-001..005 |
| GAP-KANI-RUNTIME-WIRING | Kani runtime harness file exists but is dead code — needs module wiring + boolean-model removal. | RRO-006..008 |
| GAP-FLUX-DEAD-CODE | Flux files exist but cannot compile or verify — missing deps, missing features, all functions #[trusted], string-constant annotations. | RRO-011..015 |
| GAP-PROPTEST-COMPILE | Pre-existing compile error at proptest_storage.rs:317 blocks PO-PROP-004/005 evidence collection. | RRO-019..020 |
| GAP-RUNTIME-KILL-API | Runtime::kill_run does not exist. Required before behavior test scenarios can be written for kill operations. | RRO-016 (partial) |

## Closure Path

| State | Action |
|-------|--------|
| State 8 (test-planning) | Plan behavior test scenarios referencing this bridge's `behavior_test_refs` |
| State 9 (test-writing) | Write tests covering C1-C6 with concrete scenarios |
| State 10 (implementation) | Implement Tasks 1-2 (Runtime::kill_run, cancel/kill error semantics) |
| State 11 (formal-verifier) | Implement Tasks 3-7 (Kani wiring, Flux wiring + dep, Verus production binding) |
| State 12 (closure) | All RRO rows must have `mapping_status: verified` |
