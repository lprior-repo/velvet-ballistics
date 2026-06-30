# Wave 4 — Agent 12 — Ad-Hoc Kani-Harness Deep Dive (Chunk 12)

**Scope:** 6 bug IDs from `/tmp/wave4-chunk-12.txt`.
**Method:** `bd show <id>` for description, locate related Kani harness under
`verification/kani/` or `crates/*/src/verification/kani/`, check GOD RULE 1
compliance (Arbitrary vs hardcoded shape), bounded unwinding, and kani-list
wiring. Read-only — no source edits, no beads.

**Workspace:** `/home/lewis/src/velvet-ballistics`
(Git root verified via `git rev-parse --show-toplevel`).

**Baseline kani-list snapshots:** `.evidence/kani-list/{vb_core,vb_runtime,vb_storage,vb_validate,vb_verification,vb_yaml}.json`.
Fresh run added for `vb_compile.json` and re-run for `vb_runtime.json` (no Kani
output delta — same 21 harnesses).

## Kani-Harness Field Conventions Used

- `arbitrary`: `Y` if harness uses `kani::any()` / `kani::Arbitrary` /
  bounded generators for structural inputs (WorkflowParts / RunFrame);
  `N` if it hardcodes a `WorkflowParts { … }` struct literal or fixed
  `RunFrame::new(…)` shape; `N/A` if no harness exists for the path.
- `hardcoded-shape`: `Y` / `N` / `N/A` — same definition.
- `orphan?`: `Y` if the harness file exists on disk but is NOT wired into
  the crate root (`#[cfg(kani)] pub mod …;` declaration) AND/OR is
  absent from `cargo kani list --format json` output. `N` if wired and
  present. `N/A` if no harness exists.
- `kani-cmd`: the exact `bash scripts/kani-list.sh <pkg>` invocation used.
- `kani-result`: high-level outcome (`OK 65 std`, `OK 21 std`, `OK 7 std`,
  `N/A`).

---

## Findings Table

| bug-id   | pri | kani-harness                                                                                                                  | arbitrary | hardcoded-shape | orphan? | kani-cmd                                          | kani-result  | verdict      | evidence                                                                                                                                                                                                                                                                                                                                                                          |
|----------|-----|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------|-----------------|---------|---------------------------------------------------|--------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| vb-qp6qh | P3  | **none for introspection.rs** (file removed from `crates/vb_runtime/src/shard/`); nearest neighbour `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` (orphan) | N (uses `make_minimal_run_state` w/ hardcoded `WorkflowParts {…}` + `RunFrame::new(RunId::new(1), …).unwrap()`) | Y | Y (file not in `kani-list.json` — `kani_shard_lifecycle.rs` aggregator itself is not declared in `crates/vb_runtime/src/lib.rs`) | `bash scripts/kani-list.sh vb_runtime` | OK 21 std (no shard-lifecycle harness wired) | UNKNOWN | bd show vb-qp6qh; `find crates/vb_runtime -name 'introspection*'` → empty; `crates/vb_runtime/src/kani_shard_lifecycle.rs:17` `#![cfg(kani)]` w/o `pub mod` in lib.rs; `.evidence/kani-list/vb_runtime.json` lists 21 harnesses, none under `kani_shard_lifecycle_harnesses`.                                                                                                                                  |
| vb-qxt3f | P0  | **none in fuzz crate**                                                                                                                                                              | N/A       | N/A             | N/A     | `bash scripts/kani-list.sh velvet-ballistics-fuzz` (no Kani adapter — crate is libFuzzer, not Kani) | N/A          | UNKNOWN | bd show vb-qxt3f; `fuzz/Cargo.toml:32-50` depends on `libfuzzer-sys`, `cargo-fuzz`; no `#[cfg(kani)]` anywhere in `fuzz/`; Kani runbook explicitly excludes fuzz targets.                                                                                                                                                                                                                                              |
| vb-rf62m | P0  | **none (CI issue)**                                                                                                                                                                  | N/A       | N/A             | N/A     | N/A                                               | N/A          | UNKNOWN | bd show vb-rf62m; bug is `.moon/tasks/all.yml` cache hash; no production-code path. Kani is irrelevant to Moon v2 dependency-hash configuration.                                                                                                                                                                                                                                                                          |
| vb-ttki3 | P0  | **none (CI issue)**                                                                                                                                                                  | N/A       | N/A             | N/A     | N/A                                               | N/A          | UNKNOWN | bd show vb-ttki3; bug is `moon ci` after forced push; AC = "moon ci exits 0"; Kani harness coverage does not exercise Moon v2 task-graph.                                                                                                                                                                                                                                                                                            |
| vb-ub4md | P0  | `crates/vb_compile/src/kani_foreach_parity.rs` (build_foreach_parts hardcoded); **`kani_resource_contract_*.rs` (6 files) are ORPHAN — not declared in `lib.rs`**                                                                                                                                                | N (build_foreach_parts hardcodes indices 0–3, fixed slot_count=4, fixed ResourceContract literals) | Y | **mixed**: `kani_foreach_parity` wired and in `.evidence/kani-list/vb_compile.json` (line `kani_foreach_parity::foreach_all_nodes_reachable` etc.); `kani_resource_contract_*` (6) NOT in lib.rs and NOT in kani-list output. Also `kani_digest_ask_*.rs` (5) + `kani_digest_step_primitive_no_panic.rs` are gated behind `feature="test-util"` and absent from default kani-list. | `bash scripts/kani-list.sh vb_compile` (also `KANI_FEATURES=test-util` for the ask-harnesses) | OK 65 std (default); 0 of 6 `kani_resource_contract_*` listed; 0 of 5 `kani_digest_ask_*` listed | PARTIAL | bd show vb-ub4md → fix at `crates/vb_compile/src/lib.rs:99` `use mod_compile_errors as errors;` (verified); `kani_foreach_parity.rs:49-137` `build_foreach_parts(...)` ignores its 3 params and returns fixed `WorkflowParts{ name:"foreach_harness", digest:[0u8;32], nodes:4 hardcoded, slot_count:4, …}` — direct GOD RULE 1 violation; `rtk grep 'kani_resource_contract' .evidence/kani-list/vb_compile.json` returns 0 matches — orphan files; `crates/vb_compile/src/kani_resource_contract_entry_point.rs:23,65,76` uses `expect(...)` (Holzman violation in harness). |
| vb-uy8p5 | P1  | `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs` (13 proofs, wired); **`kani_shard_lifecycle_harnesses.rs` ORPHAN**; **`kani_journal_duplicate.rs` (vb_storage) ORPHAN**; `vb_fzgdn_timer_harnesses.rs` declared but not in default kani-list | Y (kani_attempt_fence uses `any_bounded_ticket`, `any_do_run_state` w/ bounded `kani::any()` + `kani::assume()`; `#[kani::unwind(3)]` per proof) | N (other harnesses use hardcoded shapes; `vb_fzgdn_timer_harnesses.rs:292,336,380` hardcoded `WorkflowParts {…}`; `kani_shard_lifecycle_harnesses.rs:139` hardcoded) | **mixed**: `kani_attempt_fence` wired (line `kani_attempt_fence_harnesses::proof_stale_attempt_rejected` … in kani-list). `kani_shard_lifecycle_harnesses.rs` + `kani_ask_answer_lifecycle.rs` + `kani_resume_state_machine.rs` + `kani_admission_ordering.rs` are wired only via orphan `kani_shard_lifecycle.rs` aggregator (no `pub mod` in lib.rs). | `bash scripts/kani-list.sh vb_runtime` (also `KANI_FEATURES=kani-shard-command-queue,vb-y9d3v-attempt-fence`) | OK 21 std; `kani_shard_lifecycle_harnesses` NOT in output | PARTIAL | bd show vb-uy8p5 → 3 test failures in shard impl_tests/chunk_001, lru_ring_capacity_tests, and proptest_vb_god2f_action_completion — all test-only, not Kani; `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:5` explicitly states `GOD RULE 1: No hardcoded shapes` and is bound to `vb_y9d3v-attempt-fence` feature (in kani-list); sibling `kani_shard_lifecycle_harnesses.rs` is reachable only via orphan `kani_shard_lifecycle.rs` (no lib.rs entry). |

---

## Bug-by-Bug Reasoning

### vb-qp6qh — RS-214-core-introspection-epoch-saturation
- **Production path:** `crates/vb_runtime/src/shard/introspection.rs:110`.
- **Reality:** File does not exist (`find` returns 0 matches); bug-hunt
  artifact `/bug-hunt-2026-06-21/findings/runtime-shard/RS-214-core-introspection-epoch-saturation.md`
  also missing. Bug is CLOSED in beads but production code was removed
  rather than patched with `checked_add` per the bead description.
- **Kani coverage:** None for the introspection module. The closest
  shard-related harness (`kani_shard_lifecycle_harnesses.rs`) is orphan
  — its aggregator `kani_shard_lifecycle.rs` is `#![cfg(kani)]` but
  **not declared** in `crates/vb_runtime/src/lib.rs:50-87`.
- **Verdict:** UNKNOWN — no Kani harness ever covered the saturation
  invariant; closure was achieved by code deletion, not by an
  implementation-bound proof.

### vb-qxt3f — fuzz/Cargo.toml 33 missing source files
- **Bug scope:** Manifest entrypoints only; Section 37 broken
  (cargo-fuzz `[[bin]]` paths reference files not in `fuzz/src/bin/`).
- **Kani coverage:** None. The fuzz crate uses `libfuzzer-sys` (line 33
  of `fuzz/Cargo.toml`); there is no `#[cfg(kani)]` module anywhere in
  `fuzz/`. Kani runbook forbids running fuzz targets through Kani.
- **Verdict:** UNKNOWN — bug is orthogonal to Kani; no harness in scope.

### vb-rf62m — moon ci missing dependency hash
- **Bug scope:** `.moon/tasks/all.yml` cache/dependency graph for
  `velvet-ballistics:test` depending on uncached `:check`.
- **Kani coverage:** None. Moon v2 task cache semantics are unrelated
  to any production-code path that a Kani harness could exercise.
- **Verdict:** UNKNOWN — CI bug; not a verification-artifact concern.

### vb-ttki3 — Fix moon ci failures after forced push
- **Bug scope:** CI re-validation; AC = `moon ci` exits 0.
- **Kani coverage:** None. Same reason as vb-rf62m.
- **Verdict:** UNKNOWN — CI bug.

### vb-ub4md — VERIFY-NEW-1: NonDeterministicKind import path
- **Bug scope:** `crates/vb_compile/src/references/validate.rs:20`
  → E0432 unresolved `crate::NonDeterministicKind`. Fix verified at
  `lib.rs:99-100` (`use mod_compile_errors as errors;`).
- **Production file:** `crates/vb_compile/src/references/validate.rs`
  no longer exists; `references/tests.rs` is the only remaining file
  in `references/`. `NonDeterministicKind` symbol no longer appears
  under that name in the source tree (`rtk grep -ri nondeterministic
  crates/vb_compile` → 0).
- **Closest Kani harnesses:** vb_compile has 65 wired harnesses; of
  those, `kani_foreach_parity.rs` is the most relevant for the
  lowering surface, but it constructs a hardcoded `WorkflowParts`
  via `build_foreach_parts(...)` (line 49) which **ignores** its
  `_node_count`, `_slot_count`, `_const_count` parameters and emits
  fixed `name: "foreach_harness"`, `digest: [0u8; 32]`, hardcoded
  4-node body, `slot_count: 4`, and a fully populated
  `ResourceContract { max_steps:256, max_slots:256, … }` literal.
  That is a **direct GOD RULE 1 violation** — proves nothing
  general about lowering semantics.
- **Additional orphan harnesses:** Six `kani_resource_contract_*.rs`
  files (`cross_field_collision`, `digest_determinism`,
  `digest_field_sensitivity`, `dual_path_equivalence`,
  `entry_point`, `migration_digest`) exist on disk but are NOT
  declared in `crates/vb_compile/src/lib.rs` and are absent from
  `.evidence/kani-list/vb_compile.json`. They are dead code — the
  harness references for vb-xi2f K07 obligations but never execute.
  `kani_resource_contract_entry_point.rs` additionally violates
  Holzman (`expect("valid representative YAML source for Kani")`
  at line 23, `expect("valid source must compile successfully")`
  at lines 65 and 76).
- **Verdict:** PARTIAL — fix landed in `lib.rs:99-100` and `cargo
  check -p vb_compile --lib --all-targets` passes, but the most
  relevant Kani harness (`kani_foreach_parity`) uses hardcoded
  shapes (GOD RULE 1 violation) and 6 of the resource-contract
  harnesses are orphan.

### vb-uy8p5 — 3 vb_runtime test failures
- **Bug scope:** Three test failures:
  1. `shard::impl_::tests::shard_config_validate_display_lists_all_errors`
     in `crates/vb_runtime/src/shard/impl_tests/chunk_001.rs:497`
  2. `shard::lru_ring_capacity_tests::clear_does_not_grow_arena_across_ten_cycles`
     in `crates/vb_runtime/src/shard/lru_ring_capacity_tests.rs:297`
  3. `verification::proptest::proptest_vb_god2f_action_completion`
     in `crates/vb_runtime/src/verification/proptest/proptest_vb_god2f_action_completion.rs:154`
- **Kani coverage for shard:** `kani_attempt_fence_harnesses.rs` is
  the only vb_runtime Kani harness wired into kani-list output (13
  proofs under feature `vb-y9d3v-attempt-fence`). It uses
  `any_bounded_ticket()` + `any_do_run_state()` generators with
  `kani::any()` + `kani::assume()` guards (`#[kani::unwind(3)]` per
  proof) — **GOD RULE 1 compliant** and bounded. `make_minimal_run_state`
  (line 101) constructs a hardcoded `WorkflowParts`, but this is the
  fixture-builder, not the structural input to the harness — every
  proof overrides it with `kani::any()` driven values.
- **Orphan coverage:**
  - `kani_shard_lifecycle_harnesses.rs` (788 lines) — reachable only
    via `kani_shard_lifecycle.rs` which has no `pub mod` entry in
    `crates/vb_runtime/src/lib.rs`.
  - `kani_ask_answer_lifecycle.rs`, `kani_resume_state_machine.rs`,
    `kani_admission_ordering.rs` — same orphan chain.
  - `vb_fzgdn_timer_harnesses.rs` — feature-gated, hardcoded
    `WorkflowParts` (lines 292, 336, 380), `.unwrap()` at lines
    293, 337, 381.
  - `vb_storage/src/verification/kani/kani_journal_duplicate.rs`
    — not declared in `crates/vb_storage/src/lib.rs` and not in
    `.evidence/kani-list/vb_storage.json`.
- **Verdict:** PARTIAL — the one wired harness
  (`kani_attempt_fence_harnesses.rs`) is well-formed; the rest of
  the shard-lifecycle harnesses that would cover lru_ring /
  config_display paths are orphan (no lib.rs entry) and several
  use hardcoded shapes plus `.unwrap()`.

---

## Counts

- **bugs-checked:** 6
- **verdicts:**
  - UNKNOWN : 4 (vb-qp6qh, vb-qxt3f, vb-rf62m, vb-ttki3)
  - PARTIAL : 2 (vb-ub4md, vb-uy8p5)
  - PATCHED : 0
  - NOT-PATCHED : 0
  - PASS / FAIL per harness N/A (no harness execution was performed —
    bug sweep scope is harness-quality audit, not harness re-run)

- **hardcoded-shape harnesses (GOD RULE 1 candidates) — 7 distinct files:**
  1. `crates/vb_compile/src/kani_foreach_parity.rs:105` (`build_foreach_parts` returns fixed `WorkflowParts{...}`, params unused)
  2. `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs:139` (`make_minimal_run_state` hardcoded `WorkflowParts`)
  3. `crates/vb_runtime/src/verification/kani/kani_engine_signals.rs:28` (hardcoded `WorkflowParts`)
  4. `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:73` (`any_do_run_state` builds fixed 1-Do-node `WorkflowParts` — fixture, not structural input)
  5. `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs:72` (hardcoded `WorkflowParts`)
  6. `crates/vb_runtime/src/verification/kani/vb_fzgdn_timer_harnesses.rs:292,336,380` (3 hardcoded `WorkflowParts`)
  7. `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:88` (`RunFrame::new(RunId::new(1), …, 1)` — hardcoded `RunFrame`)

- **orphan harnesses (file on disk but not wired into crate root or not in kani-list output) — 12 distinct files:**
  - vb_runtime: `kani_shard_lifecycle_harnesses.rs`, `kani_ask_answer_lifecycle.rs`, `kani_resume_state_machine.rs`, `kani_admission_ordering.rs` (chained through orphan `kani_shard_lifecycle.rs`); `kani_sxkz6_shard_for_run.rs` (feature `kani-sxkz6-shard-for-run` not enabled in default kani-list); `kani_cancel_kill_lattice.rs` (feature `kani-shard-command-queue` not enabled for cancel-kill lane).
  - vb_storage: `kani_journal_duplicate.rs` (not declared in `crates/vb_storage/src/lib.rs`).
  - vb_compile: `kani_resource_contract_cross_field_collision.rs`, `kani_resource_contract_digest_determinism.rs`, `kani_resource_contract_digest_field_sensitivity.rs`, `kani_resource_contract_dual_path_equivalence.rs`, `kani_resource_contract_entry_point.rs`, `kani_resource_contract_migration_digest.rs` (6 files — none referenced from `crates/vb_compile/src/lib.rs:14-94`).
  - vb_compile (feature-gated, absent from default kani-list): `kani_digest_ask_empty_prompt.rs`, `kani_digest_ask_field_ordering.rs`, `kani_digest_ask_prompt_sensitivity.rs`, `kani_digest_ask_timeout_sensitivity.rs`, `kani_digest_ask_timeout_sentinel.rs`, `kani_digest_step_primitive_no_panic.rs` (require `feature = "test-util"`).

- **Arbitrary/bounded Kani harnesses (GOD RULE 1 compliant) — wired:**
  - `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs` (13 proofs, all `#[kani::unwind(3)]`, all use `kani::any()` with `kani::assume()` bounds)
  - `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` (all inputs `kani::any()`)
  - `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs` (all inputs `kani::any()`)
  - `crates/vb_compile/src/kani_lower_control.rs` (`lower_ask_rejects_max_id_without_overflow`, `lower_choose_fanout_bound`, etc.)
  - `crates/vb_compile/src/kani_foreach_parity.rs` (uses hardcoded `WorkflowParts` fixture but kani::any() for indices in `build_foreach_parts` params — partial compliance)

---

## Top-3 NOT-PATCHED / Partial Reasons

1. **vb-ub4md (PARTIAL) — `kani_foreach_parity.rs` is hardcoded-shape.** The
   `build_foreach_parts(_node_count, _slot_count, _const_count)` function at
   `crates/vb_compile/src/kani_foreach_parity.rs:49-137` ignores all three of
   its parameters and returns a structurally-fixed `WorkflowParts` with
   `name: "foreach_harness"`, `digest: [0u8; 32]`, `nodes: [4 hardcoded]`,
   `slot_count: 4`, and a fully-populated `ResourceContract { max_steps: 256,
   max_slots: 256, … }` literal. Every harness at lines 156, 238, 280, 374,
   391, 405, 420, 493 calls `build_foreach_parts(4, 8, 2)` with literal
   arguments. This is a textbook GOD RULE 1 violation — these harnesses
   prove properties of one specific lowering output, not of the lowering
   function over the structural input domain. Additionally, six
   `kani_resource_contract_*.rs` harnesses (vb-xi2f K07 obligations) are
   orphan — they exist on disk but are not declared in
   `crates/vb_compile/src/lib.rs:14-94` and are absent from
   `.evidence/kani-list/vb_compile.json`. The fix landed in production
   (`lib.rs:99-100`), but the Kani proof evidence for that fix is either
   hardcoded-shape or not wired.

2. **vb-uy8p5 (PARTIAL) — `kani_shard_lifecycle_harnesses.rs` is orphan.**
   The 788-line harness file at
   `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs`
   is the natural coverage for the shard-lifecycle paths exercised by the
   three failing tests in vb-uy8p5 (config_display, lru_ring_capacity,
   god2f action completion). It is reachable only via the file
   `crates/vb_runtime/src/kani_shard_lifecycle.rs:20-31` (`#[path = ...]
   mod ...`), which is itself `#![cfg(kani)]` but **not declared** in
   `crates/vb_runtime/src/lib.rs:62-87` (no `pub mod
   kani_shard_lifecycle;` entry). It does not appear in
   `.evidence/kani-list/vb_runtime.json` (21 harnesses listed, none under
   `kani_shard_lifecycle_harnesses`). Of the harnesses that ARE wired,
   `kani_attempt_fence_harnesses.rs` is the only shard-touching one and
   its proofs cover ticket-generation, not the failing test paths.

3. **vb-qp6qh (UNKNOWN) — no Kani harness ever covered the affected path.**
   The bug references `crates/vb_runtime/src/shard/introspection.rs:110`
   for an epoch-saturation invariant. The file does not exist in the
   current tree (`find` returns 0 matches); the bug-hunt finding file
   is also missing. The bug was closed by deleting the production
   code rather than by patching with `checked_add` per the bead
   description, so there is no implementation-bound Kani harness for
   the saturation invariant. Closest neighbor
   (`kani_shard_lifecycle_harnesses.rs`) is orphan (see #2).

---

## File Path Written

`/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-12-adhoc-kani-harness.md`

## Notes / Caveats

- Read-only: no source files were modified, no beads were created.
- `bash scripts/kani-list.sh vb_runtime` re-run confirmed 21 standard
  harnesses; `bash scripts/kani-list.sh vb_compile` ran fresh and
  produced 65 standard harnesses — both stored under
  `.evidence/kani-list/{vb_compile,vb_runtime}.json`.
- The bug-hunt-2026-06-21 directory referenced by vb-qp6qh is missing
  from the repository (deleted or never committed). Beads shows the
  bug as CLOSED at bead creation timestamp; no recent fix evidence is
  available in-tree.
- Holzman-style `.unwrap()` / `.expect()` calls inside
  `build_foreach_parts`, `make_minimal_run_state`, and
  `representative_source` are visible in kani harness fixtures; they
  do not violate the strict Holzman rule for harness setup as long as
  the bounded `kani::assume()` guards prevent the unwrap path from
  being reachable. The orphan `kani_resource_contract_entry_point.rs`
  uses `.expect("valid representative YAML source for Kani")` for a
  YAML literal — no symbolic input reaches that path, so it is
  effectively safe but stylistically Holzman-questionable.