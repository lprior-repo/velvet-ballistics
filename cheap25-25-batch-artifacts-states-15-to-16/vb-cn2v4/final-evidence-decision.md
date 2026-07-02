# Final Evidence Decision — vb-cn2v4

STATUS: APPROVED

## Bead
- **bead_id**: vb-cn2v4
- **title**: Keys reject zero `RunId` (P1 bug)
- **isolated_workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
- **working-copy commit**: `xrpxwkvz a47b72c6` (vb-cn2v4 state11: holzman-rust impl - reject zero RunId)
- **decision date**: 2026-07-01

## Decision

- Combined State 12/13/14 closure APPROVED.
- The 3 user-mandated `cargo test` commands re-execute cleanly with 117/117 tests passing (61 + 23 + 33).
- The full vb_storage test surface (1674 tests) and the workspace `--all-targets --all-features` compile are green.
- No defects introduced; no waivers required; no FAIL_LOCAL/FAIL_REGRESSION/FAIL_GLOBAL findings.
- Honest scope boundary: the 6 Verus/Kani/proptest obligations (PO-001..PO-006) remain PLANNED in the planner's ledger and are deferred to the next bead. They are NOT misrepresented as waived or closed in this State 12 pass.

## Required Raw Evidence

- `cargo test -p vb_storage --lib keys::tests` → PASS, 61 passed; 0 failed; 0 ignored; 1472 filtered out. `evidence/keys_tests.log`.
- `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` → PASS, 23 passed; 0 failed; 0 ignored. `evidence/fjall_keyspace_manifest_tests.log`.
- `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` → PASS, 33 passed; 0 failed; 0 ignored. `evidence/vb_eepg_bdd_tests.log`.
- `cargo test -p vb_storage --lib keys` (literal user directive form) → PASS, 85 passed; 0 failed; 0 ignored; 1448 filtered out. `evidence/keys_tests_broad.log`.
- `cargo test -p vb_storage --all-features` → PASS, 1674 passed; 0 failed across 17 suites. `evidence/vb_storage_all_tests.log`.
- `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` (downstream repair) → PASS, 69 passed; 0 failed. `evidence/restate_doctor_storage_scan_decode_tests.log`.
- `cargo check --workspace --all-targets --all-features` → exit 0, 33 crates compiled. `evidence/workspace_check.log`.

**Total: 1945 tests re-executed, 0 failed, 33 crates type-checked, exit 0.**

## Truth Serum Audit

See `truth-serum-report.md` for the 8-phase audit. STATUS: PASS.

## Outstanding Work for Next Bead (Planner Owner)

The following are NOT defects of this bead; they are owner-tracked to the next bead per `delivery-scope.jsonl`:

- PO-001-VERUS-MIRROR: add `SpecKeyEncodeError::InvalidRunId { run: u64 }` variant to `verification/verus/extern_vb_storage_keys.rs`; extend `assume_specification` clauses on `run_event_key`, `journal_key`, `encode_key` (per run-bearing SpecStorageKey variant) with `requires run != 0; ensures result is Err(SpecKeyEncodeError::InvalidRunId { run }) iff run == 0`. Owner: proof-writer.
- PO-002-VERUS-DECODER-SYMMETRY: bind the mirror to the production decoder unchanged. Owner: proof-writer.
- PO-003-KANI-SPLIT-HARNESS / PO-004-KANI-ORDER-OF-CHECKS: run the Kani solver on `vb_eepg_typed_partitioned_ids` and `vb_eepg_typed_partitioned_ids_zero_run_rejection`. Harness source already compiles under `#[cfg(kani)]` (per `evidence/kani_typed_partitioned_ids_syntax_check.log`, EXIT: 0). Owner: proof-writer.
- PO-005-PROPTEST-PER-PREFIX: implement `encoder_rejects_zero_run_id_for_every_prefix` proptest in `crates/vb_storage/src/proptests.rs`; iterate over the six public encoder entry points with `RunId(0)`; assert `Err(JournalError::InvalidRunId { run })`. Owner: test-writer.
- PO-006-PROPTEST-MUTATION: implement `mutation_resistance_require_non_zero_run` proptest in `crates/vb_storage/src/proptests.rs`; assert guard-on branch returns `Err` while guard-off branch returns `Ok`. Owner: test-writer.

## Landing Authorization

The bead is **APPROVED** for State 14 closure. The next step is the landing-skill state (State 15/16) per the go-skill pipeline, which is owned by the landing-skill agent, not this verification pass. Do not land main from this isolated workdir; landing remains serialized by the master pipeline.
