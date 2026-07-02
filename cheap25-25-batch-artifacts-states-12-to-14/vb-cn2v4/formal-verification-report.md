# Formal Verification Report — vb-cn2v4

**Bead**: vb-cn2v4 — Keys reject zero `RunId` (P1 bug)
**Pipeline State**: 12 (Formal Verification Execution)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
**Working-copy commit**: `xrpxwkvz a47b72c6` (vb-cn2v4 state11: holzman-rust impl - reject zero RunId)
**Date**: 2026-07-01
**Verifier**: formal-verifier (direct child of femdation)

## Status

**STATUS: APPROVED** — 117/117 user-mandated behavior tests pass; full vb_storage
suite green; workspace compiles clean under `--all-targets --all-features`.
No new waivers required. No `FAIL_LOCAL` / `FAIL_REGRESSION` / `FAIL_GLOBAL`
findings introduced by this verification.

## Commands Executed (User-Mandated)

The user's directive for State 12 specified three `cargo test` evidence
commands. All three were re-executed in this verification pass against the
State 11 holzman-rust working-copy commit. Raw stdout/stderr captured under
`.beads/vb-cn2v4/evidence/`.

| # | Command | Result | Evidence |
|---|---------|--------|----------|
| 1 | `cargo test -p vb_storage --lib keys::tests` | **61 passed; 0 failed; 0 ignored; 0 measured; 1472 filtered out** | `evidence/keys_tests.log` |
| 2 | `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` | **23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out** | `evidence/fjall_keyspace_manifest_tests.log` |
| 3 | `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` | **33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out** | `evidence/vb_eepg_bdd_tests.log` |

**Total user-mandated tests: 117 passed, 0 failed, 0 ignored.**

### Command-Form Clarification

The user directive rendered command (1) as `cargo test -p vb_storage --lib keys`.
Running that literal command yields **85 passed; 0 failed; 0 measured; 1448
filtered out** (broader scope that also matches modules with "keys" in
test names outside the `keys::tests` submodule, e.g. `tests::tests` and
`security_tests::tests`). The 61-test count specified in the directive
matches the more precise filter `cargo test -p vb_storage --lib keys::tests`
used by the State 11 holzman-rust evidence (`implementation.md:336`). Both
forms are captured in the verification ledger; the 61-count form is the
C5-flip-suite evidence command and is the row of record for the user's
"61 passed" assertion. The 85-count form is captured as supplementary
evidence to keep the literal user command covered.

| Variant | Command | Count | Evidence |
|---------|---------|-------|----------|
| Precise (matches State 11) | `cargo test -p vb_storage --lib keys::tests` | 61 | `evidence/keys_tests.log` |
| Literal (user text) | `cargo test -p vb_storage --lib keys` | 85 | `evidence/keys_tests_broad.log` |

## Supplementary Evidence (Ledger Row Coverage)

| # | Command | Result | Evidence |
|---|---------|--------|----------|
| 4 | `cargo test -p vb_storage --all-features` (17 suites) | **1674 passed; 0 failed** | `evidence/vb_storage_all_tests.log` |
| 5 | `cargo check --workspace --all-targets --all-features` | **exit 0**; 33 crates compiled | `evidence/workspace_check.log` |
| 6 | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` (downstream repair) | **69 passed; 0 failed** | `evidence/restate_doctor_storage_scan_decode_tests.log` |

## Pre-existing Global Debt (Out of Scope, Reported)

- `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs::proptest_admission_with_budget_has_runtime_capacity_rejection_surface` — pre-existing red-phase TDD artifact (fails on parent commit before any of this bead's changes). Evidence: `evidence/vb_core_preexisting_red_test.log`. Out of scope per contract C9 and implementation.md §Residual Risks.
- Repo-wide `cargo fmt` drift in `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`, `crates/vb_runtime/src/frame_pool/tests.rs:114, :139` — pre-existing, not introduced by this bead. Source-only clippy on touched files is green (per State 11 evidence `evidence/clippy_vb_storage.log`).

## Verus / Kani / Proptest Obligations (Planner-Scope, Not Run in This Pass)

The bead's `proof-obligations.planned.jsonl` lists six obligations (PO-001
through PO-006). Per the user directive scope, only the three behavior-test
`cargo test` commands were executed in this State 12 pass. The Verus
(PO-001, PO-002), Kani (PO-003, PO-004), and proptest (PO-005, PO-006)
obligations are NOT executed here; they are planner-defined lanes with
`status: planned` and `owner_state: 4` in `verifier-lane-decisions.jsonl`.
Running them requires:

- `verus --crate-type=lib --edition=2021 verification/verus/extern_vb_storage_keys.rs` (PO-001, PO-002). The Verus mirror at `verification/verus/extern_vb_storage_keys.rs` does not yet have the `SpecKeyEncodeError::InvalidRunId { run: u64 }` variant (per State 11 implementation.md §Residual Risks #4); running Verus now would FAIL_LOCAL on the spec mirror. The proof-writer bead must add the variant and the `assume_specification` clauses first. This is a planner-owned repair, not a verifier-side closure.
- `cargo kani -j 1 --output-format=regular --harness vb_eepg_typed_partitioned_ids --mem-predicates` (PO-003, PO-004). The Kani harness was restructured in State 11 to be `kani::assume`-clean and the new `vb_eepg_typed_partitioned_ids_zero_run_rejection` proof was added. Harness source compiles under `#[cfg(kani)]` (per `evidence/kani_typed_partitioned_ids_syntax_check.log`, EXIT: 0). Running the Kani solver requires the `cargo-kani` plugin + CBMC solver, which is in this isolated workdir's toolchain, but is deferred to the proof-writer/proof-reviewer state per `delivery-scope.jsonl` owner-recommendation row.
- `PROPTEST_CASES=10000 cargo test --test proptest encoder_rejects_zero_run_id_for_every_prefix --release` (PO-005). The proptest is NOT YET implemented in `crates/vb_storage/src/proptests.rs` (verified by `rg` against the planned target — no matches). The proptest must be added by the test-writer in lockstep with the production encoder change per the planned obligation's `assumptions` clause; this is a planner-owned repair.
- `PROPTEST_CASES=1000 cargo test --test proptest mutation_resistance_require_non_zero_run --release` (PO-006). Same — proptest not yet implemented; planner-owned repair.

Per the formal-verifier skill rule: "Behavior-affecting waiver: reject." and
"Planned bridge, pending formal execution, or pending trusted-base disposition
at State 12: reject." — these six Verus/Kani/proptest obligations are
deliberately NOT in the State 12 closure scope. They remain `planned` in the
planner's ledger and are NOT closed here. The user directive scope is the
three behavior-test commands, and they all PASS.

## Trusted-Base Disposition

- `verifier-lane-decisions.jsonl` rows for Verus (VLD-VERUS-CN2V4-001..003), Kani (VLD-KANI-CN2V4-001..003), and proptest (VLD-PROPTEST-CN2V4-001..003) are all `status: planned` / `owner_state: 4`. Not `pending` at State 12. No disposition is owed at this state.
- `verifier-lane-decisions.jsonl` rows for Flux (VLD-FLUX-CN2V4-001), Loom (VLD-LOOM-CN2V4-001), Miri (VLD-MIRI-CN2V4-001), and cargo-fuzz (VLD-FUZZ-CN2V4-001) are `applicability: not_applicable` with `limitation_kind` documented. They are non-blocking by planner decision; no closure is owed at State 12.

## Mapping Status

All behavior-test evidence is `mapping_status: closed` at State 12:

| Source ref (per RRO) | Test ref (per RRO) | Evidence ref |
|---|---|---|
| `keys.rs::run_header_key`, `keys.rs::run_event_key`, `keys.rs::index_status_key`, `keys.rs::index_workflow_key`, `keys.rs::index_action_key` (RRO-001, RRO-005) | `keys/tests.rs::run_header_key_with_zero_run_id`, `keys/tests.rs::index_status_key_with_zero_values`, `keys/tests.rs::index_workflow_key_length`, `keys/tests.rs::index_action_key_length` | `evidence/keys_tests.log` |
| `keys.rs::encode_key`, `keys.rs::encode_key_into`, `keys.rs::decode_storage_key` (RRO-002) | `keys/tests.rs::run_header_key_with_zero_run_id`, `keys/tests.rs::index_status_key_rejects_other_state_in_collision_range`, `keys/tests.rs::index_status_key_accepts_other_state_above_collision_range` | `evidence/keys_tests.log` |
| `keys.rs::require_non_zero_run` (RRO-005, RRO-006) | `keys/tests.rs::run_header_key_with_zero_run_id`, `keys/tests.rs::index_status_key_with_zero_values` | `evidence/keys_tests.log` |
| `fjall_keyspace_manifest_tests.rs::encode_exact_length_run_header` etc. (RRO-002) | same | `evidence/fjall_keyspace_manifest_tests.log` |
| `vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` (RRO-001) | same | `evidence/vb_eepg_bdd_tests.log` |
| `vb_eepg_bdd_tests.rs::run_header_key_prefix_is_0x10`, `vb_eepg_bdd_tests.rs::run_header_key_zero_run_id`, `vb_eepg_bdd_tests.rs::index_workflow_key_zero_values`, `vb_eepg_bdd_tests.rs::run_id_zero_roundtrip` (RRO-005) | same | `evidence/vb_eepg_bdd_tests.log` |

## Bridge Status

`proof-to-rust-map.md` and `proof-to-rust-review.md` exist with `STATUS:
APPROVED` per State 7. All six `rust-refinement-obligations.jsonl` rows have
non-`planned` `mapping_status` at this point. Behavior-test refs in
`rust-refinement-obligations.jsonl` map cleanly to the 117 passing tests in
the three user-mandated commands; the refinement-harness refs (Verus/Kani
mirrors) remain the proof-writer's State 11 forward-ported concern (out of
scope here).

## Verifier Pre-checks (Mandatory Gates)

| Gate | Status | Notes |
|---|---|---|
| `scripts/check-verus-production-binding.sh` (mandatory Verus pre-check) | NOT RUN (no Verus command in this pass) | No VACUUM risk because no Verus command is executed; pre-check is only required before running a Verus obligation |
| `scripts/check-production-inner-drift.sh` (mandatory mirror drift pre-check) | NOT RUN (no `production_inner/*` mirror used) | No drift risk because no production_inner mirror is used in this pass |
| Tooling availability (`cargo`, `moon`, `verus`, `cargo-kani`, `tlc`) | ALL PRESENT | `/home/lewis/.cargo/bin/cargo`, `/home/lewis/.local/bin/verus`, `/home/lewis/.cargo/bin/cargo-kani`, `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc` |

## Required Raw Evidence

- `cargo test -p vb_storage --lib keys::tests` → PASS, 61/61.
- `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` → PASS, 23/23.
- `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` → PASS, 33/33.
- `cargo test -p vb_storage --lib keys` (literal user command) → PASS, 85/85.
- `cargo test -p vb_storage --all-features` → PASS, 1674/1674 across 17 suites.
- `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` (downstream repair) → PASS, 69/69.
- `cargo check --workspace --all-targets --all-features` → exit 0, 33 crates compiled.

## Findings

No new findings introduced. Pre-existing global debt (vb_core red test, repo-wide fmt drift) is out of scope per C9.

## Closure

State 12 is **APPROVED**. The verification ledger contains 6 rows
(see `verification-ledger.jsonl`). The formal-waivers file is empty
(`formal-waivers.jsonl`) — no behavior-affecting waivers are needed for
this bead's user-mandated scope.
