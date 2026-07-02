# Black-Hat Review — vb-cn2v4

STATUS: APPROVED

## Bead
- **bead_id**: vb-cn2v4
- **title**: Keys reject zero `RunId` (P1 bug)
- **working-copy commit**: `xrpxwkvz a47b72c6` (vb-cn2v4 state11: holzman-rust impl - reject zero RunId)
- **isolated_workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
- **reviewer**: formal-verifier (direct child of femdation; no sub-agents)
- **review date**: 2026-07-01

## Scope of Review

The change is the C1-C8 contract enforcement from `contract.md`:
- A private `fn require_non_zero_run(run: RunId) -> Result<(), JournalError>` in `crates/vb_storage/src/keys.rs` is the single source of truth for the zero-RunId rejection.
- Six public encoders (`run_header_key`, `run_event_key`, `run_snapshot_key`, `index_status_key`, `index_workflow_key`, `index_action_key`) and three private helpers (`run_only_key`, `sequenced_run_key`, `index_*_key`) all delegate to `require_non_zero_run`.
- 18 unit/integration tests in three files (`keys/tests.rs`, `fjall_keyspace_manifest_tests.rs`, `vb_eepg_bdd_tests.rs`) were flipped from `Ok(...)` expectations to `Err(JournalError::InvalidRunId { run })` expectations.
- The Kani harness `kani_typed_partitioned_ids.rs::assert_key_contracts` was reorganised to distinguish the rejection path from the happy path.

## Attack Vectors Considered

| Vector | Threat | Verdict | Reasoning |
|---|---|---|---|
| A1 | Guard not present at any of the six call sites | Defended | Each call site is a `require_non_zero_run(run)?;` at function top; 61 unit + 23 integration + 33 BDD tests all pass and cover the rejection arm. |
| A2 | Guard present but not called *first* in `index_status_key` (must precede `state.to_u8_checked`) | Defended | State 11 holzman-rust places the guard at function top (before `state.to_u8_checked`); the `index_status_key_rejects_other_state_in_collision_range` and `index_status_key_accepts_other_state_above_collision_range` tests flip `RunId::new(0)` to `RunId::new(1)` to keep the collision path exercised separately (per implementation.md §Test-flip Manifest). |
| A3 | `RunId::ZERO` constructor invariant violated | Defended | C9 explicitly preserves `RunId::new` / `RunId::ZERO`; the guard is on the encoder, not the constructor. |
| A4 | `JournalError::InvalidRunId` variant mutated (e.g. extra field) | Defended | C3 forbids new variants; the existing `InvalidRunId { run: RunId }` is reused. Test assertions reference the original variant shape. |
| A5 | `headers.rs::run_header` defence-in-depth guard removed inadvertently | Defended | C4 explicitly permits KEEP or REMOVE; State 11 KEEPS the guard (per implementation.md §Manual Check Decision). The 3 companion tests in `keys/tests.rs` (`run_header_key_accepts_nonzero_run_id`, `index_status_key_with_zero_state_and_timestamp_nonzero_run`, `run_prefix_key_rejects_zero_run_id`) and the 3 proptest guards in `fjall_keyspace_manifest_tests.rs` (run_event_ordering, cross_keyspace_non_collision, index_action_ordering) preserve non-zero coverage. |
| A6 | Decoder-side `KeyDecodeError::InvalidRunId` source of truth broken | Defended | C8 leaves the decoder untouched; `decode_storage_key` continues to surface `InvalidRunId` for `run == 0` bytes (keys.rs:372-374, 381-383, 400-402, 412-414, 423-425). The new encoder rejection closes the asymmetric gap; the decoder is the source of truth. |
| A7 | `decode_storage_key` reachable path to `InvalidRunId` removed | Defended | Decoder is byte-level; `RunId::new(0)` bytes are still rejected. The 69 `restate_doctor_storage_scan_decode_tests` all pass; this includes `parse_decode_error_zero_run_id_is_typed_error` which exercises the decoder path. |
| A8 | Out-of-scope surfaces accidentally tightened | Defended | C9 enumeration (recovery diagnostics, workspace tests using `RunId::new(0)` without reaching a key encoder, `vb_qi37_16_5_lifecycle_journal_storage.rs` TLA+ mirror, `all_key_functions_are_deterministic`, `symbolic_code_table`) — none are in the encoder change scope. State 11 jj diff confirms the 6 files modified are exactly the encoder + test surfaces. |
| A9 | Test flips hide the underlying behavior change | Defended | Tests now assert `Err(InvalidRunId { run: RunId::new(0) })`; if the guard is removed, the encoder returns `Ok(bytes)` and the test panics. Mutation-resistance proptest (PO-006, planned) is the second-line defence. |
| A10 | Verus / Kani / proptest obligations waived or skipped | NOT_WAIVED | Per `formal-waivers.jsonl` (empty) and `verifier-lane-decisions.jsonl`, the six Verus/Kani/proptest obligations remain `status: planned` / `owner_state: 4`. They are NOT closed in State 12; they are planner-owned for the next bead. This is an honest scope boundary, not a waiver. |
| A11 | Pre-existing red test (`proptest_admission_with_budget_has_runtime_capacity_rejection_surface`) introduced by this change | NOT_INTRODUCED | Evidence `evidence/vb_core_preexisting_red_test.log` shows the failure on the parent commit (State 3 rust-contract). Out of scope per C9; documented in State 11 implementation.md §Residual Risks. |
| A12 | `cargo fmt` drift | NOT_INTRODUCED | State 11 implementation.md §Residual Risks #2 documents three pre-existing format drifts in unrelated files. The 6 touched files in State 11 are fmt-clean (source-only clippy on `vb_storage --lib --bins --all-features` is green per `evidence/clippy_vb_storage.log`). |

## Findings

- **No blocking findings.**
- **Residual risk**: Verus and Kani obligations (PO-001 through PO-004) and proptest obligations (PO-005, PO-006) remain `planned` and are NOT discharged in this bead. The Verus mirror at `verification/verus/extern_vb_storage_keys.rs` does not yet have the `SpecKeyEncodeError::InvalidRunId` variant (per State 11 implementation.md §Residual Risks #4). The Kani harness source compiles under `#[cfg(kani)]` (per `evidence/kani_typed_partitioned_ids_syntax_check.log`); running the Kani solver is the next bead's responsibility. The two proptests (`encoder_rejects_zero_run_id_for_every_prefix`, `mutation_resistance_require_non_zero_run`) are not yet implemented; the test-writer in the next bead must add them. **This is acceptable because**: (a) the user's State 12 scope is the 3 behavior-test commands, all of which PASS; (b) the production Rust change is fully landed and the unit/integration tests cover the rejection surface; (c) the planner's verifier-lane-decisions.jsonl documents the planned Verus/Kani/proptest obligations as owner_state=4, not this state.
- **Residual risk**: The 117 behavior tests pass on the State 11 holzman-rust commit, but the Verus mirror has not been updated. A future adversarial change could remove `require_non_zero_run` from one call site and the unit tests would still catch it (because each call site has at least one test exercising `RunId::new(0)`), so the user-facing behavior is robust to this risk. The Verus mirror is a second-line defence; the first-line defence (the unit tests) is in place.
- **Residual risk**: Pre-existing global debt (vb_core red test, repo-wide fmt drift in `vb_core/src/lib.rs`, `vb_core/src/time.rs`, `vb_runtime/src/frame_pool/tests.rs`) is out of scope per C9. None of these files are touched by State 11.

## Attack Result

The user-mandated 3 `cargo test` commands pass with 117/117 tests green.
The full vb_storage test surface (1674 tests) is green. The workspace
compiles clean under `--all-targets --all-features`. The contract
clauses C1-C8 are enforced by the production change and verified by the
117 behavior tests. C9 is preserved (no out-of-scope surfaces touched).

The Verus/Kani/proptest lanes are planner-deferred and are not blockers
for this State 12 closure; they are documented as residual risk with
honest scope boundaries.

**State 13 is APPROVED for bookmark-ready handoff.**

## Defects

See `defects.md` (empty — no defects introduced or uncovered by this verification pass).
