# Machine Gate Report — vb-qxjgx

STATUS: PASS (bead-local); DEFERRED_GLOBAL (pre-existing workspace debt)

## Bead-Local Gates

- PASS: `cargo test -p vb_storage --tests` → 1678 passed (17 suites, 13.13s)
- PASS: `cargo test -p vb_storage --lib` → 1535 passed (1 suite, 1.23s)
- PASS: `cargo test -p vb_runtime --tests` → 2348 passed, 1 ignored (35 suites, 3.34s)
- PASS: `cargo test -p vb_runtime --lib` → 1807 passed (1 suite, 3.27s)
- PASS: `cargo test -p vb_storage --tests -- step_succeeded_event_maps_to_step_succeeded_kind slot_written_event_maps_to_slot_written_kind_unchanged step_succeeded_and_slot_written_record_kinds_are_distinct legacy_envelope_id_12_with_step_succeeded_payload_is_accepted canonical_id_33_round_trip_step_succeeded slot_written_with_envelope_id_33_is_rejected` → 6 passed, 1672 filtered out
- PASS: `PROPTEST_CASES=10000 cargo test --test proptest_durability_matrix_step_succeeded --release -p vb_runtime` → 5 passed (1 suite, 0.02s)
- PASS: `PROPTEST_CASES=10000 cargo test --test proptest_replay_summary_step_succeeded_split --release -p vb_storage` → 4 passed (1 suite, 0.04s)
- PASS: `cargo check -p vb_storage --all-targets` → Finished `dev` profile (4.98s)
- PASS: `cargo check -p vb_runtime --all-targets` → Finished `dev` profile (2.84s)
- PASS: `cargo clippy -p vb_storage --lib` → No issues found
- PASS: `cargo clippy -p vb_runtime --lib` → No issues found
- PASS: `cargo fmt --check -p vb_storage` → clean (no output)
- DEFERRED_GLOBAL: `cargo fmt --check -p vb_runtime` → 3 pre-existing diffs at frame_pool/tests.rs:85, 114, 139; raw output saved at `.beads/vb-qxjgx/evidence/mg-cargo-fmt.txt`; **NOT modified by this bead** (verified via `jj diff`); pre-existing global debt
- BLOCKED_TOOLING: `cargo kani` workspace-wide → TBR-001 (pre-existing unclosed-delimiter in `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7`); NOT caused by this bead; compensating evidence: 1678 + 2348 cargo test PASS + 6 back-compat tests + 9 proptest properties
- PASS: `rg "(unwrap\(\)\|expect\(\|panic!\|todo!\|unimplemented!\|dbg!\|unsafe )"` on 6 production files (records.rs, events.rs, codec/{validation,kind_parity,mod}.rs, durability_matrix.rs) → 0 matches in production code

## Workspace Status

- `cargo test -p vb_storage --tests` PASS (bead-local)
- `cargo test -p vb_runtime --tests` PASS (bead-local)
- `cargo check -p vb_storage --all-targets` PASS (bead-local)
- `cargo check -p vb_runtime --all-targets` PASS (bead-local)
- `cargo clippy -p vb_storage --lib` PASS (bead-local)
- `cargo clippy -p vb_runtime --lib` PASS (bead-local)
- `cargo fmt --check -p vb_storage` PASS (bead-local)
- `cargo fmt --check -p vb_runtime` DEFERRED_GLOBAL (pre-existing frame_pool/tests.rs drift; not in bead scope)
- `cargo check --workspace --all-targets` DEFERRED_GLOBAL (14 errors in pre-existing test code unrelated to this bead; vb_storage + vb_runtime pass independently)

## Bead-Local vs. Global

**Bead-local blockers:** none.

**Bead-local test results:** all gates PASS for the 2 affected packages (vb_storage + vb_runtime). The 5 new kani files compile under `cargo check --features kani-vb-qxjgx-record-kind-split`; the 2 new proptest files pass at PROPTEST_CASES=10000; the 6 back-compat unit tests pass; the cargo test sweep passes (1678 + 2348).

**Global unrelated debt:**
- Pre-existing kani_helpers.rs unclosed-delimiter (TBR-001) — blocks cargo kani workspace-wide. NOT caused by this bead (verified on parent commit ywnswumt 1b72c500). Routes to kani-helpers owner.
- Pre-existing vb_runtime/src/frame_pool/tests.rs fmt drift (3 sites) — NOT modified by this bead (verified via `jj diff`). Routes to frame_pool owner.
- Pre-existing aggregate_resource_budget_properties_red proptest failure — NOT in scope for this bead.

**Classification:** STATUS: PASS for bead-local gates. The 3 global debt items are pre-existing, not introduced by this bead, and have owner_approved_debt disposition per trusted-base-ledger.jsonl (TBR-001, TBR-010) and the black-hat-review.md findings table.
