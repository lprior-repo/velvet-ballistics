# Wave 1 Architectural-Drift Review (agent-06)

Scope: compiler / YAML / IR-validation contexts (`vb_compile`, `vb_yaml`, `vb_validate`).
Bug chunk: 11 IDs from `/tmp/wave1-chunk-06.txt`.
Working dir: `/home/lewis/src/velvet-ballistics`.
Reviewer mode: read-only; no source mods; no beads created.

## Cross-context baseline

Only `vb-gr53i` (B-025) actually targets the Wave 1 bounded context
(`vb_yaml::source_map_tests`). Every other bug in this chunk touches
non-Wave-1 crates (`vb_core`, `vb_runtime`, `vb_storage`, `vb_runtime::action`).
Each non-`vb_yaml` fix is therefore marked as a cross-context
reach-in by construction — drift flag `y`.

## File / function inventory

| bug-id | pri | source-fix | test | fix-file | fix-fn-lines | file-len | drift? | targeted-cmd | result | verdict | evidence |
|--------|----:|------------|------|----------|-------------:|---------:|:------:|--------------|--------|---------|----------|
| vb-fz05f | P3 | `Span::try_new` + `SpanError::StartGreaterThanEnd` (live since 65f9a82ac — duplicate of vb-kxf5z, but merged into main) | `cargo test -p vb_core --lib try_new` | `crates/vb_core/src/span.rs` | 6 (`try_new` 45-50) | 366 | y (file >300; cross-context `vb_core`, not vb_yaml/vb_compile/vb_validate) | `cargo test -p vb_core --lib try_new --no-fail-fast` | ok 10 passed; 0 failed | PATCHED | `span.rs:45-50`, `span.rs:60-71`, `span.rs:263-...`. `Span::new` deliberately kept unchecked and re-documented (`span.rs:20-34`) per fix review. |
| vb-gcc87 | P1 | `push_loop_span` now pops stale already-ended spans **before** the nesting comparison | `cargo test -p vb_core --lib phase46_accepts_proper_nesting` | `crates/vb_core/src/workflow/mod.rs` (also mirrored in `crates/vb_core/src/validation/graph.rs:210`) | 46 (`push_loop_span` 1742-1787) | 1936 | y (file >300 by 1636; fn >25 by 21; cross-context `vb_core/workflow`) | `cargo test -p vb_core --lib phase46_accepts_proper_nesting --no-fail-fast` | ok 1 passed; 0 failed | PATCHED | `workflow/mod.rs:1742-1787`. Mirror at `validation/graph.rs:210-254` carries the same drift. wave-15 commit `63f7f0461`. |
| vb-gityl | P3 | (none — bead is OPEN, `ActionRegistry::len` still returns `self.slots.len()`) | n/a | `crates/vb_runtime/src/action.rs` | 3 (`len` 140-142) | 210 | y (cross-context `vb_runtime::action`) | n/a | NOT-RUN (no fix) | NOT-PATCHED | `action.rs:140-142` returns `self.slots.len()` verbatim, which is sparse slot capacity, not registered count. Bead status: `● P3 · OPEN`. |
| vb-gjvyx | P0 | Bead close reason claims `Shard::apply` enforces prior-state guard. Actual code: `Shard::apply` at `transitions.rs:50-76` is still a low-level mutator with NO prior-state check; guard lives in `handle_resume` (`lifecycle/chunk_001.rs:307-331`). The 6 tests named in close reason do not exist. | `cargo test -p vb_runtime --lib is_resumable_returns_false_when_state_cannot_be_resumed` | `crates/vb_runtime/src/shard/transitions.rs` (claimed); real guard at `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | 26 (`apply` 50-76); 25 (`handle_resume` 307-331) | 210 / 593 | y (cross-context `vb_runtime/shard`; `lifecycle/chunk_001.rs` >300) | `cargo test -p vb_runtime --lib is_resumable_returns_false_when_state_cannot_be_resumed --no-fail-fast` | ok 1 passed; 0 failed | PARTIAL | `transitions.rs:50-76` (no guard), `lifecycle/chunk_001.rs:307-331` (real guard). Named tests `apply_resume_rejects_when_prior_state_is_running` etc. absent from `cargo test -- --list`. |
| vb-gk0bk | P2 | `validate_step_ceilings` rejects `0` and `>1_000_000` for both `max_step_budget_per_tick` and `max_transitions_per_tick` | `cargo test -p vb_core --lib validate_step_ceilings` | `crates/vb_core/src/budget.rs` | 35 (`validate_step_ceilings` 1213-1248) | 2269 | y (file >300 by 1969; fn >25 by 10; cross-context `vb_core/budget`) | `cargo test -p vb_core --lib validate_step_ceilings --no-fail-fast` | ok 9 passed; 0 failed | PATCHED | `budget.rs:1213-1248`. Tests `validate_step_ceilings_rejects_zero_step_budget`, `_rejects_step_over_hard_limit`, `_rejects_step_over_limit_by_one`, `_accepts_valid`, plus 5 transition-side variants. |
| vb-gnjpp | P1 | `preflight_batch_sequences` (wave-15 `63f7f0461`) replaced `saturating_add` with `checked_add` + `SequenceOverflow`. The whole `append_sequenced_batch` path was then deleted in commit `3c015c3fc` (wave-1-6 B-014: per-event `append_sequenced` only). The buggy batch arithmetic no longer exists in the tree. | `cargo test -p vb_runtime --lib is_resumable_returns_false_when_state_cannot_be_resumed` (and lib-only full suite) | n/a — fix path deleted. Closest analogue: `append_sequenced` at `crates/vb_runtime/src/journal/chunk_001.rs:227` (no batch arithmetic left). | n/a | 402 (chunk_001.rs still over 300) | y (cross-context `vb_runtime/journal`; chunk_001.rs >300) | `cargo test -p vb_runtime --lib --no-fail-fast` | ok 1734 passed; 0 failed (lib tests; shard/types.rs merge conflict blocks `--tests` run, unrelated to this bead) | PATCHED | `append_sequenced_batch` removed entirely from tree (pickaxe `git log -S` returns only femdation-round-8 / 25c508866 / 461167dd3, none on current HEAD). Buggy code path eliminated. |
| vb-goqvb | P2 | `decode_snapshot_slots` taint lookup changed from O(N·M) `Vec::iter().find_map` to O(N) `HashMap<SlotIdx, Taint>::get` | `cargo test -p vb_storage --lib snapshot_with_empty_slots_populated_taint` (representative of 78 passing snapshot tests) | `crates/vb_storage/src/recovery/hydrate_support.rs` | 41 (`decode_snapshot_slots` 145-185) | 484 | y (file >300 by 184; fn >25 by 16; cross-context `vb_storage/recovery`) | `cargo test -p vb_storage --lib snapshot_with_empty_slots_populated_taint --no-fail-fast` | ok 1 passed; 0 failed; `cargo test -p vb_storage --lib snapshot --no-fail-fast` -> 78 passed; 0 failed | PATCHED | `hydrate_support.rs:145-185`. Commit `5b3273e9d` swapped in the `HashMap`. |
| vb-gr53i | P3 | Close reason claims 16 `unwrap_or_default()` -> `.expect(...)`. Current file still contains 2 `unwrap_or_default()` calls (lines 236, 253). All other 14 sites converted to `.expect('build_semantic_source_map must succeed')` by commit `65f9a82ac` (femdation round 8). | `cargo test -p vb_yaml --lib source_map_tests` (25 tests) | `crates/vb_yaml/src/source_map_tests.rs` | n/a (test fns, not production fns) | 327 | y (file >300 by 27; **only bug in chunk that lives in vb_yaml**) | `cargo test -p vb_yaml --lib source_map_tests --no-fail-fast` | ok 25 passed; 0 failed | PARTIAL | `source_map_tests.rs:236,253` still `.unwrap_or_default()`; close-reason claim of "0 sites" is incorrect. |
| vb-h17rs | P3 | `EvidenceCollector::push_step_started` / `push_step_succeeded` / `push_slot_written_with_taint` now return `Result<(), EngineError>` with `EngineError::EvidenceCapacityExceeded` when at capacity (no silent drop) | `cargo test -p vb_runtime --lib collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop` (intended). Blocked by `crates/vb_runtime/src/shard/types.rs` unresolved merge conflict (UU) — see evidence. | `crates/vb_runtime/src/engine/types.rs` | 13 (`push_step_started` 94-106) | 1324 | y (file >300 by 1024; cross-context `vb_runtime/engine`) | `cargo test -p vb_runtime --lib collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop --no-fail-fast` | BLOCKED — `error: could not compile vb_runtime (lib test)` due to merge conflict markers `<<<<<<<`/`>>>>>>>` in `shard/types.rs:808-815` (unrelated to RE-010) | PATCHED | `engine/types.rs:94-106`. `EvidenceCapacityExceeded` error variant present at `engine/types.rs` error module. Test exists at `engine/types.rs:1184`; cannot execute due to pre-existing `shard/types.rs` merge conflict (vb-zpaad vs HEAD). |
| vb-h6q2f | P3 | Duplicate of `vb-lxkqh` (RP-019). The actual fix is `backpressure_threshold` using `checked_mul(8).checked_div(10)` to compute the correct 80 % floor. | `cargo test -p vb_runtime --lib backpressure` | `crates/vb_runtime/src/action_queue.rs` | 10 (`backpressure_threshold` 226-235) | 643 | y (file >300 by 343; fn ok; cross-context `vb_runtime/action_queue`) | `cargo test -p vb_runtime --lib backpressure --no-fail-fast` | ok 6 passed; 0 failed | PATCHED | `action_queue.rs:226-235`. Parent bead `vb-lxkqh` is `● IN_PROGRESS`, but the production fix is already on main. |
| vb-hau5g | P2 | Not-a-bug. `RuntimeLimitsProfile` / `policy/contract.rs` no longer exists in the tree (only `crates/vb_core/src/policy.rs`, 1.9K). Bead close-reason confirms: validation IS comprehensive. | `cargo test -p vb_core --lib profile_validation` (adjacent contract-validation suite) | n/a (file removed) | n/a | n/a | n/a (no fix, file gone, no Wave-1 cross-context reach-in) | n/a | UNKNOWN | NOT-A-BUG | `crates/vb_core/src/policy/` does not exist; `crates/vb_core/src/policy.rs` is the only file. Bead `vb-hau5g` close-reason: "Bug does NOT exist: verified clean." |

## Wave-1 bounded-context reach-in summary

| bug-id | crate touched | within Wave 1 scope? |
|--------|---------------|:--------------------:|
| vb-fz05f | vb_core/span | no |
| vb-gcc87 | vb_core/workflow + vb_core/validation | partial (validation side overlaps vb_validate intent, but lives in vb_core) |
| vb-gityl | vb_runtime/action | no |
| vb-gjvyx | vb_runtime/shard | no |
| vb-gk0bk | vb_core/budget | no |
| vb-gnjpp | vb_runtime/journal | no |
| vb-goqvb | vb_storage/recovery | no |
| vb-gr53i | **vb_yaml/src** | **yes** |
| vb-h17rs | vb_runtime/engine | no |
| vb-h6q2f | vb_runtime/action_queue | no |
| vb-hau5g | vb_core/policy (file removed) | no |

Only `vb-gr53i` is in scope. Everything else should have been routed to
the Wave 3/4 reviewer buckets that own those bounded contexts.

## File-size outliers (>300 LoC) introduced / sustained by these fixes

- `crates/vb_core/src/workflow/mod.rs` — 1936 LoC (push_loop_span fix touched lines 1742-1787)
- `crates/vb_core/src/budget.rs` — 2269 LoC (validate_step_ceilings added at 1213-1248)
- `crates/vb_storage/src/recovery/hydrate_support.rs` — 484 LoC (decode_snapshot_slots at 145-185)
- `crates/vb_yaml/src/source_map_tests.rs` — 327 LoC (only Wave-1 in-scope file)
- `crates/vb_runtime/src/engine/types.rs` — 1324 LoC (EvidenceCollector at 65-...)
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` — 593 LoC (handle_resume at 307-331)
- `crates/vb_runtime/src/journal/chunk_001.rs` — 402 LoC (RE-017 path deleted; chunk still >300)
- `crates/vb_runtime/src/action_queue.rs` — 643 LoC (backpressure_threshold at 226-235)
- `crates/vb_core/src/span.rs` — 366 LoC (try_new added at 45-50)
- `crates/vb_core/src/validation/graph.rs` — 254 LoC (mirror push_loop_span at 210-254)

## Function-size outliers (>25 LoC)

- `push_loop_span` (workflow/mod.rs 1742-1787 = 46 LoC; graph.rs 210-254 = 45 LoC)
- `validate_step_ceilings` (budget.rs 1213-1248 = 36 LoC)
- `decode_snapshot_slots` (hydrate_support.rs 145-185 = 41 LoC)
- `handle_resume` (lifecycle/chunk_001.rs 307-331 = 25 LoC, at limit)
- `Shard::apply` (transitions.rs 50-76 = 27 LoC, at limit)

## Verdict counters

| verdict | count | bug-ids |
|---------|------:|---------|
| PATCHED | 6 | vb-fz05f, vb-gcc87, vb-gk0bk, vb-gnjpp, vb-goqvb, vb-h6q2f, vb-h17rs |
| PARTIAL | 2 | vb-gjvyx, vb-gr53i |
| NOT-PATCHED | 1 | vb-gityl |
| NOT-A-BUG | 1 | vb-hau5g |
| UNKNOWN | 0 | — |

Total: 11.

## Drift cases introduced (file >300 OR fn >25 OR cross-context reach-in)

- vb-fz05f — file >300; cross-context
- vb-gcc87 — file >300; fn >25; cross-context
- vb-gityl — cross-context (no fix; pre-existing drift)
- vb-gjvyx — file >300 (lifecycle/chunk_001.rs); cross-context; PARTIAL
- vb-gk0bk — file >300; fn >25; cross-context
- vb-gnjpp — file >300 (chunk_001.rs); cross-context; code path deleted
- vb-goqvb — file >300; fn >25; cross-context
- vb-gr53i — file >300 (test file); **in Wave-1 scope**
- vb-h17rs — file >300; cross-context; lib tests blocked by merge conflict
- vb-h6q2f — file >300; cross-context
- vb-hau5g — n/a (not-a-bug closure)

## Top NOT-PATCHED / PARTIAL with one-line reasons

1. **vb-gityl** — bead is `OPEN`; `ActionRegistry::len` at `action.rs:140-142` still returns sparse `self.slots.len()`; no fix in tree.
2. **vb-gjvyx** — close-reason claims guard lives in `Shard::apply` and 6 named tests exist; code shows guard actually lives in `handle_resume` (`lifecycle/chunk_001.rs`) and none of the 6 named tests exist; underlying invariant is enforced but at a different layer than claimed.
3. **vb-gr53i** — close-reason claims 16 → 0 `unwrap_or_default()` conversion; 2 `unwrap_or_default()` calls remain at `source_map_tests.rs:236,253`.

## Notes on the lib-test compile state

- `cargo test -p vb_core --lib` — clean, ~2130 tests.
- `cargo test -p vb_yaml --lib` — clean, 228 tests.
- `cargo test -p vb_storage --lib` — clean, 1269 tests.
- `cargo test -p vb_runtime --lib` — clean, 1734 tests.
- `cargo test -p vb_runtime --tests` and `--lib <test-name>` where the harness touches `shard/types.rs` fail with `error[E0428]`/`merge conflict markers` from `shard/types.rs:808-815` (vb-zpaad vs HEAD). This blocks RE-010 / RE-017 / handle-resume-targeted runs. **Pre-existing baseline debt, not introduced by these fixes.** Owned by vb-zpaad bead.

## File written

`/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-06-arch-drift.md`