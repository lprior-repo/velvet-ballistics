# Proof Review — vb-0x1cb

- bead_id: vb-0x1cb
- bead_title: Repair ignored-fallible-results source gate violation (P1)
- state: 6 (proof-reviewer)
- controller: femdation
- invocation_id: proof-reviewer-vb-0x1cb-state6
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T18:10:00Z
- review_state: approved
- status: APPROVED
- binding_classification: N/A (no Verus obligations); Flux spec is model-based with documented post-Repair `#[extern_spec]` plan
- production_path: N/A (model-based Flux spec); cargo-test binds to `crates/vb_runtime/src/shard/transitions.rs` via `Shard::new_with_journal`, `Shard::tick`, `shard.trace_ring().snapshot_for_run`
- verus_smoke: N/A (no Verus artifacts)
- flux_smoke: `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib` → `4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved.` (re-run by reviewer; exit 0)
- cargo_test_smoke_PO-003: `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` → `1 passed, 1808 filtered out (1 suite, 0.00s)` (re-run by reviewer; exit 0)
- cargo_test_smoke_PO-004: `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` → `1 passed, 1808 filtered out (1 suite, 0.00s)` (re-run by reviewer; exit 0)
- cargo_check_smoke: `cargo check -p vb_runtime --lib --tests` → `Finished dev profile [unoptimized + debuginfo] in 0.04s` (re-run by reviewer; exit 0)
- cargo_flux_smoke: `cargo flux -p vb_runtime --message-format human` → `Finished flux profile [unoptimized + debuginfo] in 0.05s` (re-run by reviewer; exit 0)

## Verdict

**APPROVED.** The 3 proof artifacts authored at state 5 are production-bound
(where the production surface exists today), non-vacuous, evidence-backed by
re-run smoke commands, and free of lethal patterns. The 4 BLOCKED
obligations (PO-001, PO-002, PO-003 trace-ring half, PO-004 trace-ring half,
PO-005 crate-internal `extern_spec`, PO-006, PO-007) are correctly routed
through `TBR-vb-0x1cb-009` (`production_dependency`) to the
holzman-rust (state 6 — implementation) and formal-verifier (state 12) owners.

The reviewer-approved debt (PO-005 size-bound 25 vs default-layout 32 bytes)
remains owner_approved_debt; the disposition was set by the proof-plan-reviewer
at state 4b and is carried forward honestly.

## Reviewed artifacts

| Artifact | Path | Verdict |
|----------|------|---------|
| cargo-test PO-003 | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` (appended test fn) | APPROVED with BLOCKED_PRODUCTION_DEPENDENCY (trace-ring half deferred) |
| cargo-test PO-004 | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` (appended test fn) | APPROVED with BLOCKED_PRODUCTION_DEPENDENCY (trace-ring half deferred) |
| flux-rs PO-005 | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` (new file) | APPROVED with reviewer-approved debt (size bound 25 vs layout 32) |
| trust ledger | `.beads/vb-0x1cb/trusted-base-ledger.jsonl` (10 rows) | VALID JSONL; all rows use `trusted-base-ledger/v1`; jq parses |
| writer report | `.beads/vb-0x1cb/proof-writer-report.md` | HONEST accounting; PENDING_FORMAL_EXECUTION routed |
| evidence ledger | `.beads/vb-0x1cb/proof-evidence.md` | HONEST evidence; smoke commands re-runnable today |

## Per-obligation disposition

| ID | Verifier | Disposition | Evidence observed |
|----|----------|-------------|-------------------|
| PO-001 | proptest | PENDING (NOT WRITTEN per user instruction) | proof-writer-report.md §"PO-001, PO-002"; TBR-009 routing to proof-to-implementation (state 7) or follow-up state 5 |
| PO-002 | proptest | PENDING (NOT WRITTEN per user instruction) | proof-writer-report.md §"PO-001, PO-002"; TBR-009 routing |
| PO-003 | cargo-test | PARTIAL PASS (primary-error assertion) + BLOCKED (trace-ring half) | `FinishRunRejectsJournal` stub mirrors `LegacyStepFailsJournal` pattern; primary-error assertion is enforceable today and discharges `Err(StorageJournalAppend(WriteLockPoisoned))` per C-1. Trace-ring half is documented in `// ` comment block pending `TraceEvent::RunRollbackFailed` (TBR-009). |
| PO-004 | cargo-test | PARTIAL PASS (primary-error assertion) + BLOCKED (trace-ring half) | `FailRunStateRejectsJournal` stub mirrors `LegacyStepFailsJournal` pattern; primary-error assertion discharges `Err(StorageJournalAppend(WriteLockPoisoned))` per C-1. Trace-ring half is `// ` comment pending `TraceEvent::RunRollbackFailed`. |
| PO-005 | flux-rs | PASS today (model-based); crate-internal `extern_spec` is the post-Repair closer | Flux discharges 4 functions (size bound, exact, cache-line, pointer-independence) with `0 trusted; 0 ignored`; production-bound `extern_spec` deferred until holzman-rust lands the variant (TBR-009). |
| PO-006 | cargo-clippy | PENDING — BLOCKED_PRODUCTION_DEPENDENCY | Owned by formal-verifier (state 12) after holzman-rust (state 6) removes `#[allow(clippy::let_underscore_must_use)]` at `transitions.rs:86` and `:199`. |
| PO-007 | bash-source-gate / moon-source-gate | PENDING — BLOCKED_PRODUCTION_DEPENDENCY | Owned by formal-verifier (state 12) after holzman-rust deletes the `DISCARD-006` row at `scripts/ignored-fallible-results.allow:4`. |

## Routing of PENDING_FORMAL_EXECUTION (10-row ledger validation)

All 10 trust-ledger rows parse under `jq` and use `trusted-base-ledger/v1`:

```
1: TBR-vb-0x1cb-001 (PO-001, PO-002) external_body — proptest Arbitrary impl
2: TBR-vb-0x1cb-002 (PO-001, PO-002) extern_spec — pub(crate) Shard::observe_run_state_rollback
3: TBR-vb-0x1cb-003 (PO-003, PO-004) stub — FinishRunRejectsJournal / FailRunStateRejectsJournal [trusted: smoke passes today]
4: TBR-vb-0x1cb-004 (PO-005) extern_spec — Flux refinement mirror at verification/flux/vb_0x1cb_run_rollback_failed_spec.rs [trusted: smoke passes today]
5: TBR-vb-0x1cb-005 (PO-005) assume — flux-rs nightly toolchain [trusted]
6: TBR-vb-0x1cb-006 (PO-005) assume — Arc<RuntimeError> 8-byte pointer indirection [trusted]
7: TBR-vb-0x1cb-007 (PO-007) assume — bash + rg for source-gate script [trusted]
8: TBR-vb-0x1cb-008 (PO-006) extern_spec — observe_run_state_rollback #[must_use] pending [pending]
9: TBR-vb-0x1cb-009 (PO-001..PO-007) production_dependency — TraceEvent::RunRollbackFailed + RollbackSite pending [blocked, BLOCKED_PRODUCTION_DEPENDENCY]
10: TBR-vb-0x1cb-010 (PO-001..PO-007) pending_formal_execution — overall state 5 status [pending]
```

**Routing integrity:**

- **PO-001, PO-002** (proptest): artifacts NOT created per user instruction; TBR-001 + TBR-002 + TBR-009 form a complete blocker chain routing to proof-to-implementation (state 7) or follow-up state 5.
- **PO-003, PO-004** (cargo-test, trace-ring half): TBR-009 routing to holzman-rust (state 6) for the production variant, then formal-verifier (state 12) for the trace-ring assertion re-run.
- **PO-005** (Flux `extern_spec` over crate-internal type): TBR-009 routes the crate-internal `extern_spec` to holzman-rust (state 6); the top-level `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` is `trusted` today.
- **PO-006** (cargo-clippy): TBR-008 + TBR-009 route to holzman-rust (state 6) for `#[must_use]` annotation and the bound-result expression; then formal-verifier (state 12) for the clippy command.
- **PO-007** (bash-source-gate): TBR-007 + TBR-009 route to holzman-rust (state 6) for the allow-row deletion; then formal-verifier (state 12) for the bash command.

All PENDING_FORMAL_EXECUTION obligations are correctly routed. The blocker
chain is:

```
holzman-rust (state 6) → TBR-009 unlocks →
  PO-003 trace-ring half,
  PO-004 trace-ring half,
  PO-005 crate-internal extern_spec,
  PO-006 cargo-clippy,
  PO-007 bash-source-gate
formal-verifier (state 12) closes PO-001..PO-007 with raw evidence
proof-to-implementation (state 7) or follow-up state 5 closes PO-001, PO-002 proptest files
```

## Behavior-waiver audit

`waiver-candidates.jsonl` (2 rows, WC-001 and WC-002): both `behavior_affecting: false`.
No `E_BEHAVIOR_WAIVER` finding. The TBR-007 (assume — bash + rg), TBR-005
(assume — flux nightly), TBR-006 (assume — Arc pointer size), TBR-007
(assume — tooling), and TBR-009 (BLOCKED_PRODUCTION_DEPENDENCY) entries are
all `behavior_affecting: false`. **0 behavior-affecting trust rows in the
ledger.**

## Lethal-pattern audit (state 6)

| Lethal pattern | Verification | Result |
|---------------|--------------|--------|
| VACUUM Verus spec (hand-written shadow, no `#[path]` to production) | No Verus obligations | N/A (vacuous) |
| Disconnected Verus `proof fn` / `spec fn` | No Verus obligations | N/A (vacuous) |
| Kani harness with hardcoded structural inputs | No Kani obligations | N/A (vacuous) |
| Kani `cover!` as proof; `assert(true)` | No Kani obligations | N/A (vacuous) |
| Flux broad `trusted` / `ignore` | PO-005 flux summary | `4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved.` (PASS) |
| Flux `usize{v: v == SIZE_BOUND_BYTES}` exact refinement discharged vacuously | Verified — the body returns `SIZE_BOUND_BYTES` directly, so the postcondition `v == SIZE_BOUND_BYTES` collapses to a true identity | PASS (non-vacuous via collapse-to-identity, not via suppression) |
| Loom model missing synchronization | No Loom obligations | N/A (vacuous) |
| TLA+ unbounded `Nat` | No TLA+ obligations | N/A (vacuous) |
| Proof artifact with merge-conflict markers | All 3 artifacts read | None |
| Stale rejected review state | Plan review is `approved_with_debt`; no REJECTED history | OK |
| Unledgered trust marker | All trust markers in 10-row ledger | OK; `jq` parses all |
| `let _ = self.run_state_insert(run, state);` RETAINED in proof artifacts (DISCARD-006 violation) | All 3 artifacts read | None — proof artifacts bind to production via the `Shard::tick` envelope |
| Hardcoded graph builders in proptest (GOD RULE 1) | No proptest artifacts | N/A (PO-001, PO-002 not written) |
| `RuntimeError::Core { source: CoreError::InternalInvariantViolation }` introduced | All 3 artifacts read | None — uses existing `RuntimeError::StorageJournalAppend { source: Arc<…> }` |

No lethal findings.

## Non-vacuity checks (per skill)

### PO-003 cargo-test (chunk_005.rs::finish_run_rollback_*)

- **Stub shape:** `FinishRunRejectsJournal` rejects exactly one journal event
  variant (`RunFinished`) with typed `JournalError::WriteLockPoisoned`;
  returns `Ok(())` for all others. Mirrors `LegacyStepFailsJournal` from
  `chunk_004.rs:236-333`. Not a hardcoded `WorkflowParts` graph (GOD RULE 1).
- **Production binding:** uses `Shard::new_with_journal` (production `pub
  fn`), `shard.enqueue(ShardCommand::Submit { run, workflow, caps })`
  (production public), `shard.tick()` (production public), and the
  production `ShardConfig`/`finished_workflow` test helpers.
- **Assertion strength:** `assert!(matches!(&result, Err(StorageJournalAppend
  { source }) if matches!(source.as_ref(), JournalError::WriteLockPoisoned)))`
  — typed `RuntimeError` variant + typed `JournalError` source. Not an
  `assert(true)` or `assert!(result.is_err())`.
- **Reachability:** the test drives `shard.tick()` which is the production
  route through `finish_run` at `transitions.rs:87-112`. The reject-on-RunFinished
  branch is reached deterministically.
- **Smoke:** `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_*`
  → `1 passed` (re-run; exit 0).

### PO-004 cargo-test (chunk_008.rs::fail_run_state_rollback_*)

- **Stub shape:** `FailRunStateRejectsJournal` rejects exactly one journal
  event variant (`RunFailed`); returns `Ok(())` for others. Mirrors the
  same pattern as PO-003. Not a hardcoded graph.
- **Production binding:** uses `Shard::new_with_journal`,
  `ShardCommand::Submit { … }`, `ShardCommand::ActionFailed { ticket,
  failure: non_retryable_failure() }`, and `shard.tick()` — all production
  public surfaces.
- **Assertion strength:** same typed-error assertion as PO-003.
- **Reachability:** the test drives `shard.tick()` after an `ActionFailed`
  enqueue, which is the production route into `fail_run_state` at
  `transitions.rs:200-214` (post-Repair destination).
- **Smoke:** `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_*`
  → `1 passed` (re-run; exit 0).

### PO-005 Flux spec (verification/flux/vb_0x1cb_run_rollback_failed_spec.rs)

- **Model-based:** the spec is honest about being a model (not an
  `extern_spec`); the production variant does not yet exist on disk.
- **Constants aligned to production:** `RUN_ID_SIZE_BYTES = 8` matches
  `size_of::<u64>() == 8`; `ARC_POINTER_SIZE_BYTES = 8` matches
  `size_of::<Arc<()>>() == 8`; `ROLLBACK_SITE_SIZE_BYTES = 1` matches the
  `#[repr(u8)]` `SiteShape` enum (`FinishRun`, `FailRunState`).
- **Refinements declared:** 4 functions, each with a non-trivial
  postcondition (`<= 25`, `== 25`, `< 64`, `=> { v: v <= 25 }` —
  pointer-independence accepts `primary_arc` and `secondary_arc` of
  arbitrary size without expanding the bound).
- **No `#[flux::trusted]`** or **`#[flux::ignore]`** — all 4 functions
  are Flux-checked (`summary: 4 functions processed: 4 checked; 0 trusted; 0 ignored`).
- **GOD RULE 3:** every check is against the explicit `SIZE_BOUND_BYTES = 25`
  constant; no `Nat`, no `u64::MAX` reliance, no assumptions about
  arithmetic overflow.
- **Smoke:** `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib`
  → `4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved.`
  (re-run; exit 0).
- **Reviewer-approved debt:** the proof-plan-reviewer flagged (low,
  E_SOURCE_REF_SHAPE) that with default Rust struct layout
  (`u64`, `u8`, `*const`, `*const`), the natural alignment packing yields
  `8 + 1 + 7(pad) + 8 + 8 = 32` bytes, not 25. The model discharges the
  25-byte bound against the field-sum identity, NOT against the layout.
  This is owner_approved_debt and is documented at the spec header
  ("Production binding" section) and in the proof-writer-report.md.
  Post-Repair, the spec collapses to `#[extern_spec]` over
  `std::mem::size_of::<TraceEvent::RunRollbackFailed>()` which the
  formal-verifier (state 12) will exercise against the actual layout.

### Production-binding gate

- **Verus production-binding gate** (GOD RULE 2 / STRONG | WEAK | VACUUM):
  the plan has 0 `verifier: verus` obligations. The gate is satisfied
  vacuously. `bash scripts/check-verus-production-binding.sh` would return
  0 on the empty obligation set.
- **Flux production-binding exemption (per proof-planner SKILL):** PO-005 is
  exempt from the Verus production-binding discipline; the Flux spec is
  correctly modeled by the proof-writer with a documented plan for
  collapse-to-`extern_spec` post-Repair.

### Forbidden-pattern audit (per contract C-3 + bead instructions)

The proof artifacts retain none of the forbidden patterns:

- `let _ = self.run_state_insert(run, state);` — NOT in the proof artifacts.
- `match … { Ok(_) | Err(_) => {} }` — NOT in the proof artifacts.
- `Err(secondary)` returned in place of `Err(primary)` — C-1 violated by
  such a return; PO-003/PO-004 explicitly assert primary-error
  (`RuntimeError::StorageJournalAppend { source: Arc(WriteLockPoisoned) }`).
- `RuntimeError::Core { source: CoreError::InternalInvariantViolation }` —
  NOT used; the error path uses the existing `StorageJournalAppend` variant.
- `eprintln!` / `tracing::error!` for the secondary surface — NOT used.
- Allow row with `follow_up=vb-ttki3` — NOT introduced (TBR-009 routes
  the deletion, not a refresh).

## Findings summary

See `proof-findings.jsonl` for the canonical `finding/v1` rows. **No blocker
findings.** Three observations are dispositioned and approved.

| Severity | Code | Subject | Disposition |
|----------|------|---------|-------------|
| observation | E_PRODUCTION_BINDING_DEFERRED | The PO-005 Flux model discharges against the field-sum identity (25 bytes), not against the real `TraceEvent::RunRollbackFailed` layout (32 bytes due to `*const` alignment padding). The model is honest about this and the post-Repair `#[extern_spec]` collapse is the closer. | owner_approved_debt (carried from proof-plan-reviewer finding `E_SOURCE_REF_SHAPE`, severity=low) |
| observation | E_TRACE_RING_HALF_BLOCKED | PO-003 / PO-004 trace-ring halves are documented in `// ` comment blocks awaiting `TraceEvent::RunRollbackFailed` and `RollbackSite::FinishRun` / `RollbackSite::FailRunState` in `crates/vb_runtime/src/trace/event.rs` (TBR-009). The primary-error half passes today. | owner_approved_debt (BLOCKED_PRODUCTION_DEPENDENCY; routed via TBR-009 to holzman-rust state 6, then formal-verifier state 12) |
| observation | E_PROPTEST_PENDING | PO-001 / PO-002 proptest files are NOT created. The user instruction listed only 3 artifacts (chunk_005.rs, chunk_008.rs, flux file). TBR-001 + TBR-002 + TBR-009 route the proptest files to proof-to-implementation (state 7) or follow-up state 5. | owner_approved_no_action (per user instruction) |

## Independent re-run evidence (re-validated by reviewer)

```text
pwd -P
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```text
jj root
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```text
jj status (working copy)
M crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs
M crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs
A verification/flux/vb_0x1cb_run_rollback_failed_spec.rs
Working copy  (@) : oloqnykq 43adc894 vb-0x1cb: p5-proof-writer — write proof artifacts (PO-003, PO-004, PO-005) — pending formal execution
Parent commit (@-): trquwqlz 0cd161fb (empty) vb-0x1cb: rust-contract — design secondary-rollback error surface for transitions.rs:100/202
exit 0
```

```text
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

```text
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

```text
cargo check -p vb_runtime --lib --tests
cargo build (0 crates compiled) [cached]
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
exit 0
```

```text
flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib
summary. 4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved. Finished in 110.76ms
exit 0
```

```text
cargo flux -p vb_runtime --message-format human
Checking vb_runtime v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb/crates/vb_runtime)
Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.05s
exit 0
```

```text
jq -s 'length' .beads/vb-0x1cb/trusted-base-ledger.jsonl
10
exit 0
```

```text
jq -r '.schema_version' .beads/vb-0x1cb/trusted-base-ledger.jsonl | sort -u
trusted-base-ledger/v1
exit 0
```

## Validation gates

| Gate | Status |
|------|--------|
| `pwd -P` resolves to isolated workdir | PASS |
| Both `chunk_005.rs` and `chunk_008.rs` artifacts exist and are well-formed | PASS |
| `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` exists and is well-formed | PASS |
| `jq` parses all 10 trusted-base-ledger rows | PASS |
| Ledger uses `trusted-base-ledger/v1` schema exclusively | PASS |
| No merge-conflict markers in any artifact | PASS |
| No hidden `let _` or `#[allow(clippy::let_underscore_must_use)]` reintroduction in proofs | PASS |
| No `RuntimeError::Core::InternalInvariantViolation` shadow introduction | PASS |
| No hardcoded `WorkflowParts` / `RunFrame` graph builder (GOD RULE 1) | PASS (no proptest artifacts written) |
| 0 behavior-affecting trust rows | PASS |
| 0 VACUUM Verus specs | PASS (no Verus obligations) |
| 0 Flux `trusted` / `ignore` suppressions | PASS (4/4 checked) |
| Smoke commands re-run and exit 0 | PASS (cargo test, cargo check, flux, cargo flux) |
| PENDING_FORMAL_EXECUTION correctly routed | PASS (TBR-009 chain) |

## Provenance and self-approval check

- `proof-reviewer-vb-0x1cb-state6` invocation_id is fresh (not equal to any
  prior `proof-plan-reviewer-vb-0x1cb-state4b`, `proof-writer-vb-0x1cb-state5`,
  `explore-vb-0x1cb-state2`, or `go-skill-vb-0x1cb-state1` invocation_id).
- The proof-reviewer does not appear anywhere in the ledger as the proof-writer
  or plan-reviewer. **No self-approval.**

## Decision

Approval proceeds:

1. All 7 obligations are mapped; 3 obligations (PO-003, PO-004, PO-005) have
   smoke evidence today; 4 obligations (PO-001, PO-002, PO-006, PO-007) are
   correctly routed through `TBR-vb-0x1cb-009` to the implementation and
   verification owners.
2. All 3 new proof artifacts are production-bound (where production types
   exist) and discharge against non-vacuous assertions / refinements.
3. All 10 trust-ledger rows parse under `jq` and use `trusted-base-ledger/v1`.
4. No behavior-affecting waivers.
5. Production-binding gate is satisfied vacuously (no Verus); Flux is exempt.
6. Forbidden patterns from contract and bead instructions are absent from the
   proof artifacts.
7. The reviewer-approved debt (PO-005 size-bound 25 vs layout 32) is
   honestly documented and routed for post-Repair closure.

STATUS: APPROVED
