# Trusted Base Plan — vb-pg2wq

STATUS: PLANNED. No verifier, test, fuzz, CI, or proof success is claimed here.

This bead is a test-only assertion rewrite of 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences across 5 proptest functions in 4 files under `crates/vb_storage/tests/`. No production source under `crates/vb_storage/src/` is modified, and no `Cargo.toml` is modified. The trusted base is therefore narrow: the existing production contract being pinned, the existing Kani harness already modeling the contract, and the test-tooling/runtime tooling pinned by the workspace.

## Trusted/abstracted surfaces

### TB-vb-pg2wq-proptest-strategy-preserved

- **Surface**: proptest input strategy `run in 1u64..1000u64, seq in 0u64..100u64` (ps001/ps003/ps008/ps009/ps004b) and the PS_004 no-persist variant `run in 1u64..1000u64` with `seq` fixed at 0.
- **Why trusted**: proptest strategy is the input distribution; the field-bound matches! guard depends on the strategy producing diverse `run`/`seq` tuples. If the strategy were narrowed or replaced with a single-shape constant, the field-bound guard would degenerate to a constant check and lose its regression-resistance property. The strategy MUST be preserved verbatim per contract.md §Obligation 5.
- **Reference**: contract.md Obligation 5; traceability-matrix.jsonl rows 1-6.
- **Repair trigger**: if a future bead narrows the proptest strategy, the field-bound guard's regression-resistance is lost and PO-vb-pg2wq-001/002 must be re-justified with the new strategy.

### TB-vb-pg2wq-secondary-invariants-preserved

- **Surface**: secondary assertions in `ps004_no_persist` (`prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`, `prop_assert_eq!(events.len(), 1)`) and `ps004_empty_commit_after_rej` (`prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`).
- **Why trusted**: the field-bound matches! guard for DuplicateEvent only pins the `Err` payload. The secondary assertions are independently regression-resistant against sibling regressions (e.g., `self.aborted = true` no longer set; commit path no longer returning `BatchAborted`; replay set drift). These MUST be preserved verbatim per contract.md §Obligation 4.
- **Reference**: contract.md Obligation 4; contract.md §File 3 Function A lines 47-53 and §File 3 Function B lines 93-97.
- **Repair trigger**: if any secondary assertion is removed in a future bead, the test's regression surface shrinks; PO-vb-pg2wq-002 must be re-justified with the new assertion set.

### TB-vb-pg2wq-source-lint-tooling

- **Surface**: pinned nightly toolchain `nightly-2026-04-28` (per `.moon/tasks/all.yml`), `cargo fmt --all --check`, `cargo clippy --tests -p vb_storage` with the workspace lint set (`-D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing`), `bash scripts/check-test-integrity.sh`, and `rtk rg` for the weak-pattern scan.
- **Why trusted**: PO-vb-pg2wq-003 depends on these tools being available and producing the expected exit codes. The toolchain pin and lint set are workspace-controlled; changes to either require re-justification of PO-vb-pg2wq-003.
- **Reference**: `.moon/tasks/all.yml` lines 35-44 (fmt), 46-62 (lint-src), 121-130 (check); `scripts/check-test-integrity.sh`.
- **Repair trigger**: if the pinned nightly toolchain changes, the source-lint obligation must be re-pinned; if `scripts/check-test-integrity.sh` is removed or its scope changes, the pattern-discipline guarantee breaks and PO-vb-pg2wq-003 must be re-justified.

## Trusted Kani harness (already exists; not modified by this bead)

- **Surface**: `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` models `JournalError::DuplicateEvent { run, seq }` with the field-bound guard `r == run && s == seq`.
- **Why trusted**: the Kani harness is the existing proof binding that the test rewrite strengthens. The harness is not modified by this bead; if a future bead changes the Kani harness, the runtime↔Kani alignment shifts and the strengthening argument changes.
- **Reference**: codebase-map.md lines 318-324; contract.md §Obligation 6; proof-seeds.jsonl row 8 (`vb-pg2wq-seed-kani-binding-strengthened`); traceability-matrix.jsonl row 8.
- **Repair trigger**: if the Kani harness is removed or rewritten without the field-bound guard, the runtime↔Kani binding argument no longer holds and the test rewrite must be re-justified.

## Production contract being pinned (not modified)

- **Surface**: `crates/vb_storage/src/batch/append_event.rs:61-67` — `self.journal.events.contains_key(key)?` branch returns `Err(JournalError::DuplicateEvent { run: event.run_id(), seq: event.seq() })` and sets `self.aborted = true`.
- **Why trusted**: the field-bound matches! guard in the test rewrite asserts the production code returns EXACTLY this tuple. If production regresses to return e.g. `DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) }` or to mutate to a sibling variant, the test fails. The contract is trusted as the source of truth that the test pins; the test rewrite does not modify it.
- **Reference**: contract.md §Production contract (lines 32-33); traceability-matrix.jsonl rows 7-8; codebase-map.md lines 156-188.
- **Repair trigger**: if production is changed (e.g., to return the tuple in a different order, or to add new fields), the field-bound guard may need to be updated and the proof binding re-justified. This is explicitly OUT OF SCOPE for this bead per contract.md §Obligation 6.

## Repair triggers summary

- If a future bead narrows the proptest input strategy, PO-vb-pg2wq-001/002 must be re-justified.
- If any secondary assertion in ps004_no_persist or ps004_empty_commit_after_rej is removed, PO-vb-pg2wq-002 must be re-justified.
- If the pinned nightly toolchain changes, PO-vb-pg2wq-003 must be re-pinned.
- If `scripts/check-test-integrity.sh` is removed or its scope changes, PO-vb-pg2wq-003 must be re-justified.
- If the Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` is removed or rewritten without the field-bound guard, the runtime↔Kani binding argument no longer holds and the test rewrite must be re-justified.
- If `crates/vb_storage/src/batch/append_event.rs:61-67` is modified to return a different tuple or a sibling variant, the field-bound guard must be updated and the binding re-justified (out of scope for this bead).

## Non-behavior waiver posture

No behavior-affecting waiver is made. `waiver-candidates.jsonl` is empty; this is intentional and consistent with the test-only scope of the bead. The `E_BEHAVIOR_WAIVER` failure mode is avoided by design: every planned obligation is bounded to the test surface, and the production contract is preserved verbatim.