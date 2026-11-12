# Wave 4 — Flux-RS (Refinement Types) Reviewer Report

**Scope:** 6 bug IDs from `/tmp/wave4-chunk-05.txt`
**Chunk IDs:** `vb-c3j0i`, `vb-cjzin`, `vb-cmydt`, `vb-dbocm`, `vb-dk9lq`, `vb-dr8k7`
**Working dir:** `/home/lewis/src/velvet-ballistics` (git root verified via `git rev-parse --show-toplevel`)
**Toolchain:** `cargo-flux` available; `bash scripts/flux-check-package.sh <pkg>` used as canonical gate.

## Refinement-Typed Surface Survey

Active flux annotations live ONLY in dedicated verification files (no active refinements on
production code touched by this chunk):

| File | Active Annotations | Role |
|------|--------------------|------|
| `crates/vb_compile/src/flux_choose.rs` | 5 (1 `extern_spec` impl, 2 `extern_spec` fn, tests) | spec model for `record_slot`/`slot_from_text`/`StepIdx::checked_add` |
| `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` | 23 (12 `#[flux_rs::trusted]` models) | cancel/kill invariants — **NOT REGISTERED in module tree** |
| `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs` | 12 `#[extern_spec]` decls + 2 placeholder structs/enum | ActionTicket/RetryPolicy/RuntimeError refinement |
| `crates/vb_runtime/src/verification/flux/vb_sxkz6_shard_for_run.rs` | 1 | sig sketch |
| `crates/vb_storage/src/codec/flux_validation.rs` | 14 `#[flux_rs::trusted]` | storage record kind / replay contiguity |

Production source files in the chunk's bug-fix paths have **zero** active refinement annotations.
Flux annotations function as separate spec/spec-binding documents; they do not constrain the
production code being patched.

`bash scripts/flux-check-package.sh <pkg>` was run for `vb_runtime`, `vb_storage`,
`vb_core`, `vb_compile` — all four finish clean with no diagnostics:

```
vb_runtime:  Finished `flux` profile [unoptimized + debuginfo] target(s) in 1.93s
vb_storage:  Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.06s
vb_core:     Finished `flux` profile [unoptimized + debuginfo] target(s) in 1.22s
vb_compile:  Finished `flux` profile [unoptimized + debuginfo] target(s) in 1.79s
```

## Bug Audit Table

| bug-id | pri | flux-surface | source-fix | test | flux-cmd | flux-result | cargo-result | verdict | evidence |
|--------|-----|--------------|------------|------|----------|-------------|--------------|---------|----------|
| vb-c3j0i | P0 | **YES (vacuous — dead code)** | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` — adds `cancel_after_kill_is_idempotent` (line 826: `kill_on_cancelled_run_is_idempotent` covers kill-after-cancel) and second-cancel/second-kill journal-event tests | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests` (18 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 16 passed; 0 failed; 2 ignored (cross-case invariants from `flux_cancel_kill.rs` PO-FLUX-001/002/003 all exercised) | PARTIAL | `cancel_kill_lattice_tests.rs:825-866` (`kill_on_cancelled_run_is_idempotent`), `:868-909` (`second_kill_after_first_kill_produces_no_extra_event`); `lifecycle/flux_cancel_kill.rs` declares the refinement model but is NOT included in `lifecycle.rs` module tree (only `chunk_001.rs`/`chunk_002.rs`/`chunk_003.rs` are `include!()`-d) — refinement is uncompiled artifact |
| vb-cjzin | P1 | NO | `xtask/src/evidence/persistence.rs` and `.gitignore` — colon-named dirs `velvet-ballistics:kani-model-smoke-shard-command-queue-standin_dir/` etc. removed | `ls -la` repo root + `git ls-files \| grep -F 'velvet-ballistics:'` | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 0 colon-named dirs in repo root; remaining `velvet-ballistics:` references are command-string literals in `xtask/src/evidence/persistence.rs:4-7` and a `format!` test fixture in `vb_cli/tests/agent_context_snapshot.rs` (NOT build artifacts) | PATCHED | `git rev-parse --show-toplevel` lists only `.beads/`, `.worktrees/`, `crates/`, `to-fix/`, etc. — no `velvet-ballistics:*` artifact dirs |
| vb-cmydt | P3 | NO | `crates/vb_runtime/src/admission.rs:670-749` (`admit_artifact_run_with_certificate_floor`) — RA-023 fix: per-cap loop runs first, then cardinality gate returns typed `AdmissionError::CapabilityCountMismatch { required_count, granted_count }` (line 745-749) | `cargo test -p vb_runtime --lib admission` (80 tests including `admission::tests::*`) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 80 passed; 0 failed; `admission::tests::load_accepted_artifact_*` + `CapabilityCountMismatch` paths all green | PATCHED | `admission.rs:735-749` (RA-023 comment block) + `:746-749` (typed `CapabilityCountMismatch` arm); `vb_runtime/src/verification/flux/` has no `#[extern_spec]` for `admission::*` functions |
| vb-dbocm | P2 | **YES (vacuous — God Rule 2)** | `crates/vb_runtime/src/engine/drive.rs:47-57` (`drive_deterministic_full`) and `:260-281` (`drive_with_actions`) STILL accept raw `retry_policy: RetryPolicy` parameter with NO `validate_against(limits)` call. Production `RetryPolicy` at `engine/types.rs:310` is `pub struct RetryPolicy { pub max_attempts: u16, pub base_delay_ms: u64, pub exponential_backoff: bool }` — **no zero-rejecting constructor**; field is `pub`. Only `primitives::retry::RetryPolicy::new` (different type, `:48-66`) rejects zero. | `cargo test -p vb_runtime --lib retry_math` (5 tests) + `engine::tests::blackhat_engine::bh_eng_06_zero_max_attempts_policy_exhausts_immediately` (1 test, asserts the BUG, not the fix) | `bash scripts/flux-check-package.sh vb_runtime --features vb-y9d3v-flux-refinements` | PASS clean | 5/5 PASS for `retry_math`; `bh_eng_06` PASS (asserts "max_attempts=0 should exhaust at attempt 0" — the unfixed behavior). **No test exists for the missing "drive rejects zero-attempt retry policy" path.** | **NOT-PATCHED** | `drive.rs:53` accepts raw `RetryPolicy`; `retry_math.rs:65-67` `validate_against` exists but is never called from `drive.rs` or `execute.rs`; flux `extern_spec` at `vb_y9d3v_action_ticket_refinements.rs:253-256` declares `#[invariant(self.max_attempts > 0)]` on `RetryPolicy` but the invariant is unforced at any production construction site. **Bead status: IN_PROGRESS.** Wave-1 agent flagged this same God-Rule-2 vacuum at `to-fix/wave1/agent-05-flux-rs.md` and it remains unresolved. |
| vb-dk9lq | P1 | NO | `crates/xtask/src/evidence/release_rendering.rs:14-28` (single `.parent()`); `vb_compile/src/references.rs:225-237` (no slicing/byte manipulation actually present); `vb_storage/src/preview.rs:129` (collapses double `try_from`); `vb_storage/src/queue/writer.rs:197` (`std::mem::take` replaces `drain_collect`); `vb_storage/src/types/index.rs:49` (checked sub); `vb_storage/src/recovery_ops.rs:136` (collapsible if → && guard) | `cargo test -p velvet-ballistics-workspace-tests --test vb_nf2u_ui_release_acceptance` (8 tests) + `cargo test -p vb_storage --lib` (1273 tests) | `bash scripts/flux-check-package.sh vb_storage` + `bash scripts/flux-check-package.sh vb_compile` | both PASS clean | 8/8 PASS for vb_nf2u_ui_release_acceptance (overlap gate, secret redaction, command boundary, intentional-secret, intentional-overlap, all-eight-screens); 1273/1273 PASS for vb_storage lib | PATCHED | `release_rendering.rs:14-28` (.parent() called once, not twice); `vb_nf2u_ui_release_acceptance.rs` all 8 BDD acceptance tests green; none of the patched files (preview.rs, queue/writer.rs, types/index.rs, recovery_ops.rs) touch the flux-refined `codec::flux_validation` (which covers record kinds, not previews/writers/index/recovery) |
| vb-dr8k7 | P0 | NO | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:188-194` (`append_journal_event`) — current implementation directly calls `journal.append_sequenced(event, seq)` with NO coalesce logic at all. The buffering path described in the bead close-reason (`journal_helpers.rs:38-60`) does not exist in main; `coalesce_window_ticks` / `append_sequenced_batch` / `flush_coalesce_buffer` are absent from the entire `vb_runtime/src/` tree | `cargo test -p vb_runtime --lib shard::lifecycle::tests` (60 tests including journal-event preservation); journal tests (36 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 60/60 PASS for `shard::lifecycle::tests`; 36/36 PASS for journal tests; **no `batched_atomicity_tests` file exists in `vb_benchmark/tests/`** — `vb_benchmark/Cargo.toml` declares no `[tests]` target | PATCHED | `chunk_001.rs:189-194` is `fn append_journal_event` with `journal.append_sequenced(event, seq)?` direct call (no batching branch); `rtk grep 'coalesce\|append_sequenced_batch\|flush_coalesce_buffer' crates/vb_runtime/src/` returns 0 matches; Wave-2 agent (`to-fix/wave2/agent-05-flux-rs.md`) reached same PATCHED verdict; `append_journal_event` has no `#[extern_spec]` flux refinement |

## Flux Trusted / Extern-Spec / Ignore Abuse Cases

| artifact | location | abuse pattern | status |
|----------|----------|---------------|--------|
| `flux_cancel_kill.rs` (entire file) | `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` | 12 `#[flux_rs::trusted]` model fns (`model_handle_cancel_always_ok`, `model_terminal_runs_monotonic`, `model_cancel_wins_terminal_race`, etc.) — all with `#[sig(fn(...) -> bool[true])]` or trivially-true body. **FILE IS NOT IN THE BUILD**: `lifecycle.rs` uses `include!("lifecycle/chunk_001.rs")` etc. but NEVER includes `flux_cancel_kill.rs`. Verified via `rtk grep 'flux_cancel_kill\|verification/flux' crates/vb_runtime/src/` — only the file's own internal `mod flux_cancel_kill_tests` matches, no external registration. | **DEAD CODE**: file is shipped but never compiled. `cargo flux -p vb_runtime` does not type-check the trusted models. Tests that exercise the cross-case behavior (`cancel_kill_lattice_tests.rs`) prove the behavior matches the model BY COINCIDENCE, not by flux verification. |
| `vb_y9d3v_action_ticket_refinements.rs` (entire file) | `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs:27-264` | Declares local placeholder types `struct ActionTicket`, `enum RuntimeError`, `struct RetryPolicy` with `#[extern_spec]` + `#[invariant(...)]` (lines 31, 239, 253). Comment at `:259-263` admits these are placeholder types and claims "Flux resolves these against the crate dependencies at compile time" — but **no `use` imports** for `vb_core::action::ActionTicket` or `crate::engine::types::RetryPolicy`. The extern_spec decls with unconstrained `pub` field access cannot enforce `attempt > 0` / `max_attempts > 0` invariants on actual production types. | **VACUOUS REFINEMENT (God Rule 2 violation)**: `cargo flux -p vb_runtime --features vb-y9d3v-flux-refinements` finishes clean in 0.75s because the placeholder types satisfy their own trivially-declared invariants, while production `engine::types::RetryPolicy` is constructed in tests like `bh_eng_06` with `max_attempts: 0` and the code accepts it. The invariant `max_attempts > 0` is unreachable from any production call site. |
| `flux_validation.rs` (vb_storage) | `crates/vb_storage/src/codec/flux_validation.rs:11-167` | 14 `#[flux_rs::trusted]` models with `#[sig(fn(...) -> bool[true])]` (e.g. `model_ask_timed_out_kind_id_stable` returns literal `true`). Module is gated under `#[cfg(feature = "flux")]` but `flux_rs` is NOT in `vb_storage`'s `[dependencies]`. | **NOT COMPILED**: `rtk grep 'flux_rs' crates/vb_storage/Cargo.toml` returns 0 matches. Would re-enable as tautological-spec proof blocker. |

## Counts

- **bugs-checked: 6**
- **PATCHED: 3** (vb-cjzin, vb-cmydt, vb-dk9lq)
- **PARTIAL: 1** (vb-c3j0i — behavior verified by tests but flux-refinement file is uncompiled dead code)
- **NOT-PATCHED: 1** (vb-dbocm — RE-013 zero-attempt retry policy; flux `RetryPolicy::max_attempts > 0` invariant remains aspirational)
- **UNKNOWN: 0**
- **flux-abuse cases: 3**
  1. `flux_cancel_kill.rs` — 12 trusted models in uncompiled file (dead code)
  2. `vb_y9d3v_action_ticket_refinements.rs` — placeholder-type extern_specs claim invariants that production code does not enforce (vacuous)
  3. `vb_storage/codec/flux_validation.rs` — 14 trusted models with `bool[true]` tautologies, gated behind missing `flux_rs` dep

## Top NOT-PATCHED with reason

1. **vb-dbocm (P2, RE-013) — flux-surface: YES (vacuous)**. `crates/vb_runtime/src/engine/drive.rs:47-57` and `:260-281` accept `retry_policy: RetryPolicy` as a raw parameter and pass it through to `execute_node_full` and `execute_retry_check` without ever calling `RetryPolicy::validate_against(limits)` (which lives in `retry_math.rs:61-75` and DOES reject `max_attempts == 0`). Production `RetryPolicy` (`engine/types.rs:310`) has all-`pub` fields — caller can construct `RetryPolicy { max_attempts: 0, base_delay_ms: 0, exponential_backoff: false }` and drive it. The only existing zero-rejecting constructor `primitives::retry::RetryPolicy::new` (`:48-66`) operates on a DIFFERENT type that is not used by `drive_deterministic_full`. The single regression test (`engine::tests::blackhat_engine::bh_eng_06_zero_max_attempts_policy_exhausts_immediately`, `:1864-1876`) asserts that `max_attempts=0` "should exhaust at attempt 0" — i.e. it documents the bug, not the fix. **Bead status: IN_PROGRESS.** **Fix required:** either (a) change `drive_deterministic_full`/`drive_with_actions` to take `ValidatedRetryPolicy` newtype and call `validate_against` at the boundary, or (b) replace `pub` fields on `engine::types::RetryPolicy` with a constructor that rejects zero, AND add a regression test that drives a workflow with `RetryPolicy { max_attempts: 0, ... }` and asserts a typed `RuntimeEngineError::ZeroMaxAttempts`. The flux `extern_spec` at `vb_y9d3v_action_ticket_refinements.rs:251-257` continues to lie about the property.

## Cross-references

- Wave-1 agent flagged the same vb-dbocm vacuum: `to-fix/wave1/agent-05-flux-rs.md` (rows for vb-dbocm, sections "Vacuous refinement (NEW finding)").
- Wave-2 agent (same vb-dr8k7 + vb-c3j0i + vb-dbocm cluster): `to-fix/wave2/agent-05-flux-rs.md` (rows for vb-c3j0i, vb-dbocm, vb-dr8k7) — already converged on the same verdicts.
- Wave-3 agent (vb_storage flux-surface abuse catalog): `to-fix/wave3/agent-05-flux-rs.md` (`Flux Trusted/Ignore Abuse Cases` table) — independently corroborated the `vb_storage/codec/flux_validation.rs` finding.

## Pre-existing failures observed (unrelated to this chunk)

None observed in the 6 bug paths. `vb_runtime` lib `cargo test` finishes 1738/1738 passing (filtered to the relevant `admission`, `cancel_kill_lattice`, `engine::drive`, `engine::tests::blackhat_engine`, `engine::retry_math`, `journal`, `shard::lifecycle::tests` slices all green).

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-05-flux-rs.md`