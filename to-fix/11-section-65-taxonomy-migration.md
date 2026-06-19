# Section 65 SideEffect / RetrySafety Taxonomy Migration

**Bead:** vb-MAJOR-6 (to be filed) — umbrella defect for the broken-taxonomy drift.
**Master contract:** `velvet-ballistics-MASTER.md` lines 3263–3281 (Phase 38) and 3293–3346 (Idempotency verification rules).
**Status:** Round 4 drift confirmed; production is on a 5×3 cardinality, master requires 7×4.

---

## 1. Drift summary (the defect)

| Surface | Production cardinality | Master cardinality | File:line of drift |
|---|---|---|---|
| `SideEffect` enum | 5: `None, Writes, Sends, Creates, Destroys` | 7: `Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell` | `crates/vb_core/src/action.rs:96-107` |
| `RetrySafety` enum | 3: `Safe, KeyRequired, Unsafe` | 4: `Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown` | `crates/vb_core/src/action.rs:113-120` |
| `IdempotencyViolation::MissingKey` payload | `Debug`-formatted broken variant name | Master variant name required | `crates/vb_core/src/action.rs:138-139` |
| `is_compile_idempotency_gate_accepted` | matches `(None, _, _)` and `(Safe|KeyRequired, IdempotentExternal)` | needs 7×4 decision table from master lines 3293–3346 | `crates/vb_compile/src/mod_compile_core.rs:146-160` |
| `check_idempotency_gates` | reason strings reference broken variants | must cite master variants | `crates/vb_compile/src/mod_compile_core.rs:162-215` |
| `is_statically_idempotent_contract` | same 5×3 decision table | needs 7×4 decision table | `crates/vb_validate/src/idempotency_contract.rs:140-187` |
| `kani_idempotency_parity.rs` | iterates 5×3×3 = 45 cases | must iterate 7×4×3 = 84 cases | `crates/vb_compile/src/kani_idempotency_parity.rs:30-46` |
| `kani_idempotency_contract.rs` | symbolic generators produce 5×3 | must produce 7×4 | `crates/vb_validate/src/kani_idempotency_contract.rs` (35 hits) |
| `kani_idempotency_gates.rs` | symbolic generators + `MissingKey(SideEffect::Writes)` literal | must use master variant names | `crates/vb_core/src/kani_idempotency_gates.rs:43-58, 196, 220, 730, 765` |
| Dead test module | `crates/vb_compile/src/enums/{mod,tests/*}.rs` not declared in `lib.rs` and has malformed import | must be wired in | `crates/vb_compile/src/enums/mod.rs:1-24` |
| Test files | 28 files reference broken variant names (compile-error surface) | must use master names | grep shows: `idempotency_parity.rs`, `idempotency_contract_red.rs`, `action/tests.rs`, `integration_capability_behavior.rs`, `kani_idempotency_gates.rs`, `kani_idempotency_contract.rs`, `primitives/retry/tests.rs`, `timer_deadline_primitive_tests.rs`, `vb_test_core_yaml_chain_behavior.rs`, `engine/tests.rs`, `action_specs.rs`, `engine/execute_tests.rs`, `engine/execute/execute_tests.rs`, `kani_workflow_arbitrary.rs`, `admission/tests.rs`, `primitives/retry.rs`, `gate_12_14_15_tests.rs`, `action_dispatch_root_migrated.rs`, `idempotency_contract.rs`, `admission.rs`, `engine/drive_tests.rs`, etc. |

`bd list` shows no MAJOR-6 bead (verified: 44 issues, none with `idempotency` or `taxonomy` in title). A new umbrella bead must be filed.

---

## 2. Migration order (minimizes the broken window)

The hard constraint is that any compile error in the canonical enum will cascade into ≥28 dependent files. The window can be collapsed by:

1. **File the MAJOR-6 bead** (no code change; instant) so the umbrella is tracked.
2. **One-shot enum rename** in `vb_core/src/action.rs`: replace the 5-variant `SideEffect` and 3-variant `RetrySafety` with the 7×4 master taxonomies in a single commit. The production enum is the source of truth; everything else follows.
3. **Bridge update in the same commit**: update `mod_compile_core.rs`, `idempotency_contract.rs`, `kani_idempotency_gates.rs`, `kani_idempotency_contract.rs` so the three gate functions and the two symbolic Kani generators use the new variants.
4. **Cascade the rename** to the 28 dependent test/bench files: this is mechanical, and since the enum is the single point of truth, every call site must move together. Land as a single commit so `cargo check` never sees a half-typed codebase.
5. **Wire the dead test module** (`enums/mod.rs`) and fix the malformed `use` import.
6. **Update the 3 broken-taxonomy test files** (`idempotency_parity.rs`, `kani_idempotency_parity.rs`, `idempotency_contract_red.rs`) to assert the 7×4 taxonomy; the existing `5×3` assertions become 7×4.
7. **Add a Kani harness** asserting `SideEffect` has ≥7 variants.
8. **Update `MissingKey` to serialize master variant names** so error messages cite `ExternalWrite` not `Writes`.

Total broken window: one commit that swaps the enum source-of-truth and updates all call sites together, plus one follow-up commit that lands the Kani harness and the test-module wiring. The "broken window" between the two is zero because the work compiles at every step.

---

## 3. Work items

### WI-1 — File MAJOR-6 umbrella bead (admin, no code)

- **Defect:** No bead exists for the Section 65 drift. There is no audit trail linking the broken-taxonomy references to a tracked defect.
- **Fix:** File a new P0 bug bead titled `P0: migrate Section 65 SideEffect/RetrySafety taxonomy to master (7×4)` with description pointing at `to-fix/11-section-65-taxonomy-migration.md` and the master contract lines 3263-3346. Add children `MAJOR-6.1` … `MAJOR-6.6` matching the six work items below, each with `blocks:MAJOR-6`.
- **File:line of change:** none; this is a beads change.
- **Test impact:** none.
- **Acceptance:** `bd list | grep MAJOR-6` shows the parent + 6 children. Parent is `blocks:`-depended-on by all 6 children.
- **Risk:** none (admin).
- **Hours:** 0.25 h
- **Bead ID:** new `vb-MAJOR-6` (parent) + `vb-MAJOR-6.1` … `vb-MAJOR-6.6` (children)

### WI-2 — Rename `SideEffect` (5→7) and `RetrySafety` (3→4) in `vb_core::action`

- **Defect:** `crates/vb_core/src/action.rs:96-107` declares `None, Writes, Sends, Creates, Destroys` and `crates/vb_core/src/action.rs:113-120` declares `Safe, KeyRequired, Unsafe`. Master requires `Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell` and `Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown`. Discriminants stay 0..6 and 0..3.
- **Fix:** Edit `action.rs:96-107` to declare the 7-variant enum with discriminants `Pure=0, LocalRead=1, LocalWrite=2, ExternalRead=3, ExternalWrite=4, Process=5, UnsafeShell=6`. Edit `action.rs:113-120` to declare the 4-variant enum with discriminants `Idempotent=0, RequiresIdempotencyKey=1, NotRetrySafe=2, Unknown=3`. Keep `#[derive(...)]`, `#[repr(u8)]`, `#[non_exhaustive]` attributes. Keep `#[error("...")]` messages stable.
- **File:line of change:**
  - `crates/vb_core/src/action.rs:96-107` (SideEffect)
  - `crates/vb_core/src/action.rs:113-120` (RetrySafety)
- **Test impact:** every file that references the old variant names will fail to compile until the call-site sweep in WI-3 is done. Land WI-2 + WI-3 + WI-4 in the same commit.
- **Acceptance:** `cargo check -p vb_core` succeeds. `cargo doc -p vb_core` documents the new variants.
- **Risk:** High blast radius. The enum is in the canonical type crate, so every downstream crate breaks until the sweep lands. Mitigation: land WI-2 + WI-3 + WI-4 in a single commit; never push a half-typed tree.
- **Hours:** 1.0 h (mechanical edit; not heroic, but the file is hot)
- **Bead ID:** `vb-MAJOR-6.2`

### WI-3 — Cascade variant renames across 28 dependent files

- **Defect:** 28 test/bench/source files reference broken variant names. The grep for `SideEffect::(None|Writes|Sends|Creates|Destroys)|RetrySafety::(Safe|KeyRequired|Unsafe)` counts ≥470 hits across the workspace.
- **Fix:** Apply the mapping table below to every match. Use a single `cargo check` + `cargo clippy` cycle per file group to keep the rename atomic.

  | Old | New |
  |---|---|
  | `SideEffect::None` | `SideEffect::Pure` |
  | `SideEffect::Writes` | `SideEffect::ExternalWrite` |
  | `SideEffect::Sends` | `SideEffect::Process` |
  | `SideEffect::Creates` | `SideEffect::ExternalRead` (best-fit; see semantic-loss note) |
  | `SideEffect::Destroys` | `SideEffect::UnsafeShell` (best-fit) |
  | `RetrySafety::Safe` | `RetrySafety::Idempotent` |
  | `RetrySafety::KeyRequired` | `RetrySafety::RequiresIdempotencyKey` |
  | `RetrySafety::Unsafe` | `RetrySafety::NotRetrySafe` |

  **Semantic-loss note:** the 5→7 mapping is not 1:1. `Creates` and `Destroys` had no master equivalent. The conservative mapping collapses them to `ExternalRead` (provision allocates → read identity) and `UnsafeShell` (deprovision = shell call). If domain experts disagree, file a follow-up bead to split the mapping; the migration is a rename, not a redesign.

- **File:line of change (representative):**
  - `crates/vb_core/src/action/tests.rs:102` (test fixtures)
  - `crates/vb_core/src/kani_idempotency_gates.rs:43-58, 196, 220, 730, 765`
  - `crates/vb_core/src/engine/tests/integration_capability_behavior.rs:48`
  - `crates/vb_validate/src/idempotency_contract.rs:148, 154, 174`
  - `crates/vb_validate/src/kani_idempotency_contract.rs:35` (symbolic generators)
  - `crates/vb_validate/tests/idempotency_contract_red.rs:100-117, 565` (119 hits)
  - `crates/vb_runtime/src/action/tests.rs:15, 16, 154, 155, 225, 226, 351, 352, 463, 464, 486, 487, 522, 523`
  - `crates/vb_runtime/src/primitives/retry.rs:5` + `tests.rs:35`
  - `crates/vb_runtime/src/engine/{tests,execute_tests,execute/execute_tests,drive_tests}.rs`
  - `crates/vb_cli/src/action_specs.rs:16`
  - `crates/vb_cli/tests/admission_evidence_integration/chunk_001.rs`
  - `crates/vb_storage/src/admission.rs:4` + `admission/tests.rs:6`
  - `crates/vb_validate/src/gates/tests.rs`, `gate_12_14_15/tests.rs`
  - `crates/vb_validate/tests/capability_contract_schema.rs`
  - `crates/vb_validate/benches/capability_schema.rs`
  - `crates/workspace_tests/tests/{timer_deadline_primitive_tests,vb_test_core_yaml_chain_behavior,gate_12_14_15_tests}.rs`
  - `crates/workspace_tests/benches/action_dispatch.rs`, `action_dispatch_root_migrated.rs`
  - `crates/workspace_tests/tests/proptest_compile_error_codes.rs`, `integration_validation_tests.rs`, `bdd_validation_tests.rs`, `cancel_kill_lattice_tests.rs`, `vb_vt2f_direct_runtime_api_acceptance.rs`, `vb_test_runtime_lifecycle_state_behavior.rs`, `vb_c1s0_orchestration_runtime_tests.rs`, `proptest_validation.rs`
  - `crates/vb_core/src/kani_workflow_arbitrary.rs:8`
  - `crates/vb_compile/src/{kani_idempotency_parity,mod_compile_core}.rs`
  - `crates/vb_compile/tests/idempotency_parity.rs:46 hits`
  - `crates/vb_runtime/tests/{vb_jggy_lifecycle_tests,vb_qi37_12_2_resume_error_propagation,vb_5m8w_step_budget_suspension_runtime,durable_retry_red_phase,durable_resume_red_phase,durability_matrix_integration}.rs`
- **Test impact:** the broken-taxonomy tests are *updated* (not removed). Their assertions flip from 5×3 to 7×4. See WI-5 for details.
- **Acceptance:** `cargo check --workspace` succeeds. `cargo clippy --workspace --all-targets` is clean. `cargo test --workspace --no-run` compiles.
- **Risk:** High volume; one missed reference = one `cargo test` failure. Mitigation: `cargo check --workspace` + `cargo clippy --workspace --all-targets` between every ten files; CI runs `moon ci` and will fail fast.
- **Hours:** 4.0 h (mechanical; ~28 files × 5-10 minutes each at the current rate)
- **Bead ID:** `vb-MAJOR-6.3`

### WI-4 — Rewrite the 3 production gate functions against the 7×4 decision table

- **Defect:**
  - `crates/vb_compile/src/mod_compile_core.rs:146-160` matches `(SideEffect::None, _, _)` and `(_, Safe|KeyRequired, IdempotentExternal)`. Master lines 3293-3346 require per-`SideEffect` rules: `Pure` → accept; `LocalRead`/`ExternalRead` → accept iff `Idempotent` or `IdempotentExternal+RequiresIdempotencyKey`; `ExternalWrite`/`LocalWrite` → require key proof; `Process`/`UnsafeShell`/`Unknown` → reject.
  - `crates/vb_compile/src/mod_compile_core.rs:162-215` `check_idempotency_gates` issues reason strings that cite broken variants.
  - `crates/vb_validate/src/idempotency_contract.rs:140-187` `is_statically_idempotent_contract` has the same 5×3 decision table.
- **Fix:** Rewrite each match arm to the master 7×4 table. The new rules:
  ```rust
  // is_compile_idempotency_gate_accepted (rejection criterion; see master L3297-3304)
  match (contract.side_effect, contract.retry_safety) {
      (SideEffect::Pure, _) => true,
      (SideEffect::LocalRead | SideEffect::ExternalRead,
       RetrySafety::Idempotent | RetrySafety::RequiresIdempotencyKey) => true,
      (SideEffect::LocalWrite | SideEffect::ExternalWrite,
       RetrySafety::RequiresIdempotencyKey) => true,
      (SideEffect::Process | SideEffect::UnsafeShell, _) => false,
      (SideEffect::LocalWrite | SideEffect::ExternalWrite,
       RetrySafety::Idempotent | RetrySafety::NotRetrySafe | RetrySafety::Unknown) => false,
      (SideEffect::LocalRead | SideEffect::ExternalRead,
       RetrySafety::NotRetrySafe | RetrySafety::Unknown) => false,
      _ => false,
  }
  ```
  `check_idempotency_gates` reason strings become `"side-effecting action declares SideEffect::Process without idempotency proof"`, etc., citing master variant names.
  `is_statically_idempotent_contract` keeps parity with the compile gate (cross-crate parity is the test the Kani harness enforces).
- **File:line of change:**
  - `crates/vb_compile/src/mod_compile_core.rs:140-215`
  - `crates/vb_validate/src/idempotency_contract.rs:140-187`
- **Test impact:** `idempotency_parity.rs` and `idempotency_contract_red.rs` assertions flip. The Kani harness `kani_idempotency_parity.rs` iterates 7×4×3 = 84 cases (up from 45). See WI-5.
- **Acceptance:**
  - `cargo test -p vb_compile --test idempotency_parity` passes against the 84-case table.
  - `cargo test -p vb_validate --test idempotency_contract_red` passes.
  - `bash scripts/verify-verus.sh` (if applicable) still passes.
  - Kani parity harness (WI-7) proves compile-gate and validate-gate agree on all 84 cases.
- **Risk:** Decision-table logic is the heart of Phase 38. A wrong arm produces a silent gate bypass — a workflow with an unsafe side effect compiles when it should not. Mitigation: Kani parity harness (WI-7) is exhaustive over 84 cases, plus black-hat review with `proptest` for the symbolic table.
- **Hours:** 3.0 h (rewrite + 84-case test update + cross-crate parity check)
- **Bead ID:** `vb-MAJOR-6.4`

### WI-5 — Wire `crates/vb_compile/src/enums/` test module and fix the malformed import

- **Defect:**
  - `crates/vb_compile/src/enums/mod.rs:1-24` declares `mod side_effect_tests; mod retry_safety_tests;` under `#[cfg(test)]` but the parent `crates/vb_compile/src/lib.rs:1-180` does **not** declare `mod enums;`. The directory is dead.
  - `crates/vb_compile/src/enums/tests/side_effect_tests.rs:12-13` and `retry_safety_tests.rs:12-13` have a syntactically malformed `use`:
    ```rust
    use vb_core::{
    use vb_core::action::ActionName;   // ← should be a single `use` block
    ```
    This is a typo that was never compiled because the module was never declared.
  - The tests assert the master 7×4 taxonomy, which is exactly the contract the migration is implementing. Once WI-2 lands, the tests should pass.
- **Fix:**
  1. In `crates/vb_compile/src/lib.rs:18-26`, add `#[cfg(test)] mod enums;` next to the existing `mod tests;` declaration.
  2. In `crates/vb_compile/src/enums/tests/side_effect_tests.rs:12-17`, replace the malformed import with:
     ```rust
     use vb_core::action::{verify_idempotency, ActionName};
     use vb_core::{
         ActionContract, ActionId, Idempotency, RetrySafety, RunFrame, RunId,
         SideEffect, SlotIdx, SlotValue, StepIdx, Taint,
     };
     ```
  3. In `crates/vb_compile/src/enums/tests/retry_safety_tests.rs:12-17`, same fix.
  4. Verify the `match () { _ if true => 1, … }` exhaustive-counting pattern still works with the new variant names (it should — it was written against the master names).
- **File:line of change:**
  - `crates/vb_compile/src/lib.rs` (add `mod enums;` near line 70)
  - `crates/vb_compile/src/enums/tests/side_effect_tests.rs:12-17`
  - `crates/vb_compile/src/enums/tests/retry_safety_tests.rs:12-17`
- **Test impact:** the dead module is now live. The 12+10 tests in the two files start compiling and running. If they pass, the migration is contractually correct.
- **Acceptance:** `cargo test -p vb_compile enums::side_effect_tests::` and `cargo test -p vb_compile enums::retry_safety_tests::` both pass. The `side_effect_has_exactly_seven_master_plan_variants` and `retry_safety_has_exactly_four_master_plan_variants` tests pass — this is the strongest assertion in the suite.
- **Risk:** Low. The module was deliberately written against master; the only risk is the malformed import, which is a one-line fix. If the import fix surfaces a deeper type problem in `vb_core`, the migration order ensures WI-2 has already landed.
- **Hours:** 0.5 h
- **Bead ID:** `vb-MAJOR-6.5`

### WI-6 — Update the 3 broken-taxonomy test files

- **Defect:**
  - `crates/vb_compile/tests/idempotency_parity.rs:36-296` iterates the 5×3 cardinality: `SideEffect::None, Writes, Sends, Creates, Destroys` and `RetrySafety::Safe, KeyRequired, Unsafe`. The 45-case parity assertion (line 212) is hardcoded to 45.
  - `crates/vb_compile/src/kani_idempotency_parity.rs:30-46` iterates the same 5×3×3 = 45 cases; KANI-PARITY-006 comment says "all 45 combinations".
  - `crates/vb_validate/tests/idempotency_contract_red.rs:100-117, 119-905` exercises broken variants; 119 hits.
- **Fix:**
  1. `idempotency_parity.rs`: replace the variant arrays with the 7×4 taxonomy; change the parity count from 45 (= 5×3×3) to 84 (= 7×4×3). Add explicit per-variant-class assertions to cover `Pure`, `Process`, `UnsafeShell`, `Unknown` (currently absent). Update the doc comment "all 45 combinations" → "all 84 combinations".
  2. `kani_idempotency_parity.rs`: same variant swap; bump `#[kani::unwind(8)]` → `#[kani::unwind(8)]` is fine, but the loop bound is `side_effects.len() * retry_safeties.len() * idempotencies.len() = 84`, which is still tractable for Kani. Update KANI-PARITY-006 comment.
  3. `idempotency_contract_red.rs`: rewrite the `at_least_once_violation` / `deterministic_pure_violation` / `retry_unsafe_violation` helpers to use the new variant names; update the 119 hard-coded test fixtures. The test count is preserved (no test removal, only update).
- **File:line of change:**
  - `crates/vb_compile/tests/idempotency_parity.rs:36-296`
  - `crates/vb_compile/src/kani_idempotency_parity.rs:20-46`
  - `crates/vb_validate/tests/idempotency_contract_red.rs:100-117, 119-905`
- **Test impact:** This is the test-update step. The tests remain — they assert the master taxonomy. No test is removed. CI must pass `cargo test --workspace` and `cargo kani --list` after this lands.
- **Acceptance:**
  - `cargo test --workspace` is green.
  - `bash scripts/kani-list.sh vb_compile` and `bash scripts/kani-list.sh vb_validate` show the parity harnesses with the new 84-case loop.
  - `cargo kani -p vb_compile --harness idempotency_gate_parity` succeeds (use `scripts/check-kani.sh` if it exists; otherwise the `moon run :kani-*` task).
- **Risk:** Test count explosion (45 → 84 cases) may slow Kani. Mitigation: keep `#[kani::unwind(8)]` and run `cargo kani --list --output-format json` to estimate runtime before promoting to CI.
- **Hours:** 3.0 h
- **Bead ID:** `vb-MAJOR-6.6`

### WI-7 — Add Kani harness asserting `SideEffect` has ≥7 variants

- **Defect:** No Kani harness enforces the cardinality of `SideEffect`. The current `kani_idempotency_parity.rs` only checks that two functions agree on the 5×3 table — it does not check that the enum has the right number of variants. If a future PR deletes `Process` or `UnsafeShell`, the parity harness still passes.
- **Fix:** Add a new file `crates/vb_core/src/kani_side_effect_cardinality.rs` (and register it in `crates/vb_core/src/lib.rs` under `#[cfg(kani)]`) with the following harness:
  ```rust
  //! Kani harness: SideEffect has at least the 7 master variants.
  //!
  //! Master plan Section 65: Pure, LocalRead, LocalWrite, ExternalRead,
  //! ExternalWrite, Process, UnsafeShell.
  //!
  //! This proves the enum cannot be silently truncated without breaking CI.

  #![forbid(unsafe_code)]

  use vb_core::action::SideEffect;

  #[kani::proof]
  #[kani::unwind(8)]
  fn side_effect_has_at_least_seven_variants() {
      // kani::any::<u8>() yields values 0..=255. Each distinct discriminant
      // observed represents a distinct variant.
      let raw: u8 = kani::any();
      let v: SideEffect = match raw {
          0 => SideEffect::Pure,
          1 => SideEffect::LocalRead,
          2 => SideEffect::LocalWrite,
          3 => SideEffect::ExternalRead,
          4 => SideEffect::ExternalWrite,
          5 => SideEffect::Process,
          6 => SideEffect::UnsafeShell,
          _ => return, // unknown discriminant: skip
      };
      // Each named variant must be constructible. If the enum is truncated
      // to fewer than 7 variants, one of these arms will not compile.
      kani::assert(
          matches!(v,
              SideEffect::Pure
              | SideEffect::LocalRead
              | SideEffect::LocalWrite
              | SideEffect::ExternalRead
              | SideEffect::ExternalWrite
              | SideEffect::Process
              | SideEffect::UnsafeShell
          ),
          "SideEffect must have all 7 master variants"
      );
  }
  ```
  Register the module in `crates/vb_core/src/lib.rs` next to the other `#[cfg(kani)] pub mod kani_*;` declarations.
- **File:line of change:**
  - new: `crates/vb_core/src/kani_side_effect_cardinality.rs`
  - `crates/vb_core/src/lib.rs` (add `#[cfg(kani)] pub mod kani_side_effect_cardinality;`)
- **Test impact:** New harness. Adds one Kani proof. Estimated runtime: < 5 seconds.
- **Acceptance:**
  - `cargo kani -p vb_core --harness side_effect_has_at_least_seven_variants` succeeds.
  - The harness is gated behind the `kani` cfg so non-Kani builds are unaffected.
  - The harness fails (does not compile) if a future PR removes one of the 7 variants.
- **Risk:** Low. The harness is a contract enforcer, not a behavioral proof. It does not interact with the production decision table.
- **Hours:** 1.0 h
- **Bead ID:** `vb-MAJOR-6.7` (added as a 7th child since WI was numbered 1-6 in the bead-ready plan, but the brief said "6 work items"; this is a 7th, derived from the brief's explicit "Must add a Kani harness" constraint).

### WI-8 — Update `IdempotencyViolation::MissingKey` to serialize master variant names

- **Defect:** `crates/vb_core/src/action.rs:138-139` uses `#[error("action has side-effect {0:?} but no idempotency key")]`. The `Debug` format on `SideEffect` emits the master variant name once the enum is renamed (because `Debug` uses the variant identifier). However, the public diagnostic emitted by the runtime may go through a separate serialization path. Verify whether the error message reaches the user as `MissingKey(Pure)` or as the integer discriminant.
- **Fix:**
  1. Confirm `Debug` formatting on the renamed enum emits `Pure`/`LocalRead`/etc. (Rust's default `#[derive(Debug)]` does this; no change needed).
  2. Add an explicit assertion test: `crates/vb_core/src/action/tests.rs` already has tests at lines 502, 601, 626, 726, 751, 844, 869, 993 that match on `IdempotencyViolation::MissingKey(SideEffect::Writes)` etc. After WI-3, these become `MissingKey(SideEffect::ExternalWrite)` etc. The format-string test (`"action has side-effect {0:?}"`) now produces strings like `"action has side-effect ExternalWrite but no idempotency key"`, which is exactly what master requires.
  3. Add a positive test: `assert_eq!(format!("{err:?}"), "MissingKey(ExternalWrite)")` for a constructed `IdempotencyViolation::MissingKey(SideEffect::ExternalWrite)`. This is the explicit contract for the master variant name serialization.
- **File:line of change:**
  - `crates/vb_core/src/action.rs:138-139` (no source change, but verify the `#[error]` attribute)
  - `crates/vb_core/src/action/tests.rs` (add the format-string assertion)
- **Test impact:** New positive test. Existing tests at lines 502+ continue to pass after WI-3.
- **Acceptance:** `cargo test -p vb_core action::tests` shows a new test `idempotency_violation_missing_key_serializes_master_variant_name` passing, asserting that `format!("{:?}", err)` contains `ExternalWrite` (not `Writes`).
- **Risk:** None — Debug formatting is determined by the enum definition. The risk is that some external code path goes through `Serialize`/`Deserialize` (the enum has `#[derive(Serialize, Deserialize)]`). Verify the JSON form: `serde_json::to_string(&IdempotencyViolation::MissingKey(SideEffect::ExternalWrite))` should produce a string containing `"ExternalWrite"`. If it produces `"Writes"`, serde is using the discriminant, not the variant name — this is impossible with default `#[derive(Serialize)]` but should be confirmed.
- **Hours:** 0.5 h
- **Bead ID:** `vb-MAJOR-6.8` (8th child)

---

## 4. Hours summary

| Work item | Hours | Bead |
|---|---|---|
| WI-1 File MAJOR-6 umbrella | 0.25 h | vb-MAJOR-6 |
| WI-2 Enum rename (5→7, 3→4) | 1.0 h | vb-MAJOR-6.2 |
| WI-3 Cascade variant renames (28 files) | 4.0 h | vb-MAJOR-6.3 |
| WI-4 Rewrite 3 gate functions | 3.0 h | vb-MAJOR-6.4 |
| WI-5 Wire `enums/` test module + fix import | 0.5 h | vb-MAJOR-6.5 |
| WI-6 Update 3 broken-taxonomy test files | 3.0 h | vb-MAJOR-6.6 |
| WI-7 Kani cardinality harness | 1.0 h | vb-MAJOR-6.7 |
| WI-8 `MissingKey` master variant serialization | 0.5 h | vb-MAJOR-6.8 |
| Buffer (review, CI triage, black-hat) | 2.0 h | — |
| **Total** | **15.25 h** | |

Round to **15 hours (≈ 2 person-days)** for one engineer. The buffer covers `moon ci` re-runs, the Kani parity harness triaging (45→84 cases changes the proof cost), and the inevitable "one site I missed" cycle.

---

## 5. Definition of done

The migration is **done** when **all** of the following hold:

1. **No broken-taxonomy references remain.** `rtk rg 'SideEffect::(None|Writes|Sends|Creates|Destroys)|RetrySafety::(Safe|KeyRequired|Unsafe)' crates/` returns zero matches.
2. **Master cardinality is enforced by tests:**
   - `cargo test -p vb_compile enums::side_effect_tests::side_effect_has_exactly_seven_master_plan_variants` passes.
   - `cargo test -p vb_compile enums::retry_safety_tests::retry_safety_has_exactly_four_master_plan_variants` passes.
   - `cargo kani -p vb_core --harness side_effect_has_at_least_seven_variants` passes.
3. **Cross-crate parity is proven for 84 cases:** `cargo kani -p vb_compile --harness idempotency_gate_parity` proves `is_compile_idempotency_gate_accepted` and `is_statically_idempotent_contract` agree on all 7×4×3 = 84 combinations.
4. **Workspace compiles and tests pass:** `moon ci` is green (or `cargo check --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` if running locally).
5. **The `enums/` test module is live:** `cargo test -p vb_compile enums::` runs and passes. The malformed import is gone.
6. **Diagnostic serialization cites master variants:** the new test `idempotency_violation_missing_key_serializes_master_variant_name` passes, asserting that `Debug` on `IdempotencyViolation::MissingKey(SideEffect::ExternalWrite)` yields `"MissingKey(ExternalWrite)"` (not `"MissingKey(Writes)"`).
7. **MAJOR-6 bead and all 8 children are closed** with `bd close` after verification.
8. **Branch is pushed** to origin (per landing-skill): `git push` succeeds and `git status` shows "up to date with origin".
9. **No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg!` introduced** (per `AGENTS.md` engineering rules).
10. **No new unstable Rust features** outside `crates/*/src/perf/**` or `crates/*/src/generated/**` (per `AGENTS.md` rust governance).

---

## 6. Risk register (consolidated)

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Half-typed commit breaks `cargo check` on every dependent crate | High | Medium | Land WI-2 + WI-3 + WI-4 in one commit; never push a broken tree. |
| 5→7 mapping is not 1:1 (`Creates`, `Destroys` have no master equivalent) | Medium | High | Conservative collapse to `ExternalRead`/`UnsafeShell`. File follow-up bead for domain-expert mapping. |
| Kani parity harness runtime jumps with 84 cases | Medium | Medium | Pre-flight `cargo kani --list --output-format json`; `moon run :kani-vb_compile` first; gate behind `#[cfg(kani)]`. |
| One of the 28 cascade sites is missed | Medium | High | Final `rtk rg` over the entire workspace after the cascade commit. |
| New `enums/` test module conflicts with an existing `mod tests;` | Low | Low | The two are independent module trees (`enums::tests::` vs `tests::`). |
| Black-hat review rejects the gate-rewrite math | Low | Low | WI-4 is decision-table code; proptest covers it. The Kani parity harness is the ultimate arbiter. |

---

## 7. Migration order (final, in one sentence)

**File MAJOR-6 (WI-1) → rename the enums (WI-2) → cascade call sites (WI-3) → rewrite the gates (WI-4) → wire the dead test module (WI-5) → update the broken-taxonomy tests (WI-6) → add the Kani cardinality harness (WI-7) → assert master variant serialization (WI-8) → close all beads → push.**

Land WI-2 through WI-6 in a single commit so `cargo check` never sees a half-typed tree. WI-7 and WI-8 can land as a follow-up commit (they are net-additive; the workspace already compiles before them).
