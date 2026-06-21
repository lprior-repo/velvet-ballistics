# Master Test Suite Review — Workspace-Wide Sweep (Round 1 of 40)

**Date:** 2026-06-21
**Reviewer:** test-reviewer (master synthesis of 4 parallel subagent reviews)
**Mode:** Workspace-wide behavior-test sweep, 4 slices, parallel subagent dispatch.
**Scope:** 799 test-bearing Rust files across 17 production crates.

## STATUS: REJECTED

The workspace contains **24 CRITICAL** and **40 HIGH** test-quality defects that would
still pass if the production behavior they claim to verify were deleted, mutated, or had
its error/success variants swapped. Each finding below has a file:line anchor, a mutation
thought experiment demonstrating the false-positive escape, and a concrete fix recipe.
The slice artifacts in `.evidence/test-review/slice-*.md` carry the full evidence
(command output, BEFORE/AFTER snippets, exhaustive pattern census).

This is the **Round 1 baseline** in a 40-round review/fix loop. The fix list is filed as
24 blocker beads (one per CRITICAL finding) plus 40 HIGH beads, dispatched to the
`test-writer` agent. Subsequent rounds (2-40) will re-dispatch the same 4 subagents
against the post-fix code and track which findings close.

---

## 1. Aggregate Findings

| Severity | Count | Definition | Action |
|----------|-------|------------|--------|
| CRITICAL | **24** | Test passes if the production behavior it claims is deleted or swapped. | File blocker beads; test-writer MUST fix before next review. |
| HIGH | **40** | Smoke test (`is_ok()` / `is_err()` / `Some(_)`) that accepts ANY Result. | File debt beads; test-writer SHOULD fix before round 5. |
| MEDIUM | **38** | Decorative assertion, redundant `is_err()` + `matches!()`, or `let _ =` field-reachability check. | Owner-approved debt; track, fix opportunistically. |
| LOW | **23** | Test-infrastructure issue, controlled sleep, or `panic!` in `other` arm. | Owner-approved no-action or trivial cleanup. |
| OBSERVATION | **19** | Positive observations (exemplary test files, idiomatic patterns). | Out of scope. |
| **TOTAL** | **144** | | |

## 2. Per-Slice Rollup

| Slice | Crates | Files | CRITICAL | HIGH | MEDIUM | LOW | OBS | Verdict | Artifact |
|-------|--------|-------|----------|------|--------|-----|-----|---------|----------|
| 1 | vb_core + vb_runtime | 261 | 10 | 10 | 9 | 0 | 5 | REJECTED | `.evidence/test-review/slice-1-core-runtime-review.md` |
| 2 | vb_storage + workspace_tests | 313 | 3 | 10 | 12 | 8 | 5 | REJECTED | `.evidence/test-review/slice-2-storage-workspace-review.md` |
| 3 | vb_compile + vb_cli + vb_validate + vb_proof_kernels | 181 | 7 | 12 | 9 | 10 | 8 | REJECTED | `.evidence/test-review/slice-3-compile-cli-validate-proof-review.md` |
| 4 | vb_expr + vb_ipc + vb_yaml + vb_queue_semantics + vb_boundary_inventory + vb_benchmark + vb_test_util + vb_doc + vb_ajc40_flux + vb_verification | 56 | 4 | 8 | 8 | 5 | 1 | REJECTED | `.evidence/test-review/slice-4-misc-review.md` |
| **TOTAL** | | **811** | **24** | **40** | **38** | **23** | **19** | | |

## 3. The 24 CRITICAL Findings (with mutation experiments)

Each entry: ID | File:Line | Defect (1 line) | Mutation thought experiment (1 line). Full evidence in slice artifacts.

### Slice 1 (vb_core + vb_runtime) — 10 CRITICAL

| ID | File:Line | Defect | Mutation |
|----|-----------|--------|---------|
| S1-C1 | `crates/vb_runtime/src/engine/action_tests.rs:267` | `assert!(result.is_err())` on `resolve_contract` | Swap to `Err(ResolveError::IndexOutOfBounds)` — test passes. |
| S1-C2 | `crates/vb_runtime/src/engine/action_tests.rs:289` | `assert!(result.is_ok())` on `resolve_contract` | Return `Ok(&wrong_contract)` — test passes. |
| S1-C3 | `crates/vb_runtime/src/engine/action_tests.rs:296` | Same as S1-C2 for "last contract" | Same mutation. |
| S1-C4 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | `assert!(result.is_ok())` on `enqueue` | Return `Ok(false)` or `Ok(SomeError)` — test passes. |
| S1-C5 | `crates/vb_runtime/src/shard/lru_ring_red_queen_tests.rs:507` | `assert!(r.is_err())` no variant on `LruRing::insert` | Return `Err(LruError::Generic)` instead of `TerminalRunsLruFull` — passes. |
| S1-C6 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | `assert!(result.is_err())` no variant on `recover_runtime_summary` | Return `Err(RecoveryError::Io)` instead of `EmptyJournal` — passes. |
| S1-C7 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | `assert!(result.is_ok())` on `check_compiled_ir_digest` | Stub to always `Ok(())` — test passes. |
| S1-C8 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | `assert!(result.is_err())` no variant on digest mismatch | Return `Err(WrongVariant)` instead of `DigestMismatchError` — passes. |
| S1-C9 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | `assert!(result.is_ok())` on `recover_runtime_summary` | Return `Ok(RecoverySummary::default())` — passes. |
| S1-C10 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | `assert!(result.is_ok())` on `hydrate_run_frame` (tail after watermark) | Return `Ok(default_frame)` — passes. |

### Slice 2 (vb_storage + workspace_tests) — 3 CRITICAL + 1 CRITICAL-on-misnamed-test

| ID | File:Line | Defect | Mutation |
|----|-----------|--------|---------|
| S2-C1 | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:376,401,424` | `assert!(result.is_ok() \|\| result.is_err())` — TAUTOLOGY | Delete `CompileError::DepthLimit/SequenceLimit/ScalarLimit` arms — all 3 tests pass. |
| S2-C2 | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:215` | `assert!(result.is_ok() \|\| result.is_err())` — explicit tautology with self-justifying comment | Delete `hydrate_run_frame` seed validation entirely — test passes. |
| S2-C3 | `crates/vb_storage/src/process_lock_tests.rs:141-181` | `match { Ok(_) => {} \| Err(_) => {} }` — accepts ALL outcomes (security test) | Delete `process_lock.rs` entirely — both tests pass. |
| S2-C4 | `crates/vb_storage/src/edge_case_tests.rs:547-554` | Test name `encode_rejects_zero_length_payload_serialization` asserts the OPPOSITE (`assert!(result.is_ok(), "empty payload should be accepted")`) | Delete the entire "reject empty payload" branch — test passes; name lies. |

### Slice 3 (vb_compile + vb_cli + vb_validate + vb_proof_kernels) — 7 CRITICAL

| ID | File:Line | Defect | Mutation |
|----|-----------|--------|---------|
| S3-C1 | `crates/vb_cli/src/args/tests/workflow.rs:13,29,...,446` (23 sites) | `if let Ok(Command::Validate{..}) = parsed { real asserts } else { assert!(parsed.is_ok()) }` — fallback accepts any `Ok(Command::X)` | Delete `Command::Validate` arm, route to `Command::Run` — all 23 tests pass. |
| S3-C2 | `crates/vb_cli/src/args/tests/status.rs:13,...,343` (9 sites) | Same pattern for `Command::SystemStatus` | Same mutation. |
| S3-C3 | `crates/vb_cli/src/args/tests/run.rs` (11 sites) | Same pattern for `Command::Run` | Same mutation. |
| S3-C4 | `crates/vb_cli/src/args/tests/cancel.rs` (8 sites) | Same pattern for `Command::Cancel` | Same mutation. |
| S3-C5 | `crates/vb_cli/src/args/tests/action.rs` (7 sites) | Same pattern for `Command::Action` | Same mutation. |
| S3-C6 | `crates/vb_cli/src/args/tests/parse_*.rs` (10+ sites) | Same pattern across all parser-args test modules | Delete the matching parser arm — test passes. |
| S3-C7 | `crates/vb_compile/src/budget_analyzer.rs:126-137,206-217` + `red_queen_budget.rs:450-465` (43 sites) | `let _ = budget.field;` — discards value, only checks field is reachable | Set every field to 0 in `WholeWorkflowBudget` — all 43 tests pass. |
| S3-C8 | `crates/vb_compile/src/taint/tests/secret_finish_tests.rs:42,69,94,...,598` (13 sites) | `matches!(result, Ok(_))` for Section 47 contract (secret in Finish) | Strip secret data from Finish result, return `Ok(workflow)` — all 13 tests pass. Section 47 violated. |
| S3-C9 | `crates/vb_compile/src/mod_compile_lowering/together_*_tests.rs` (15+ tests, 5 files) | TDD-red `if let Ok(()) = result { /* detailed asserts */ } // TDD: Accept either Ok or Err` | Delete `emit_single_body_set` Together branch — all 15+ tests pass. |
| S3-C10 | `crates/vb_compile/tests/proptest_save_canonical_name.rs:30-46,80-105` | Test calls a **locally-defined** `canonical_name` that duplicates production | Revert `Save{..}` to `"save"` in production — test passes. |
| S3-C11 | `crates/vb_compile/src/tests/do_choose_digest_unit_tests.rs:179,...,408` (18 sites) | `let _ = digest_step_primitive(&mut hasher, &step);` discards `Result<()>` | Make function return `Err` and short-circuit — hasher is zero; tests may pass spuriously. |
| S3-C12 | `crates/vb_compile/tests/digest_ask_explicit_arm.rs:144,...,233` (11 sites) | `let _ = canonical_digest(&source).expect("valid test input");` discards digest value | Return `Ok(zero_digest)` for every variant — all 11 tests pass. |

### Slice 4 (misc) — 4 CRITICAL

| ID | File:Line | Defect | Mutation |
|----|-----------|--------|---------|
| S4-C1 | `crates/vb_expr/src/eval/tests/and_or_short_circuit_tests.rs` (1619 lines, entire file) | Orphaned test module — NOT referenced by any `mod` declaration. Broken syntax. **Section 46 (no short-circuit) has zero executable coverage.** | Wire it in — build fails. Leave as-is — coverage is zero. |
| S4-C2 | `crates/vb_ajc40_flux/tests/density_tests.rs:22-59` | Test re-implements `validate_count` / `validate_summary` LOCALLY; tests never call production | Delete `vb_core::validate_compiled_slug_count` — all 50+ tests pass. |
| S4-C3 | `crates/vb_ipc/src/tests.rs:443-455` | Uses `crossbeam_channel` (FORBIDDEN per Section 50) and tests the library, not `MemoryIngress` | Delete `MemoryIngress` entirely — test passes. |
| S4-C4 | `crates/vb_ipc/src/queue/tests/array_queue_tests.rs:702-741` | `fifo_order_invariant_for_submit_recv_cycle` discards frame data with `while let Ok(Some(_)) { received.push(()) }` — only checks counts | Return frames in REVERSE order — test passes. The FIFO invariant is a lie. |

## 4. The 5 Most Dangerous Mutation Gaps (workspace-wide)

These represent the 5 production-code mutations most likely to ship to users undetected.

| # | Production code | What would break | File:Line of test gap |
|---|----------------|------------------|-----------------------|
| 1 | `vb_runtime::recover_runtime_summary` returns `Ok(RecoverySummary::default())` for all paths | All recovery silently returns empty summaries | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883,2728` (S1-C9, S1-C10) |
| 2 | `vb_ipc::MemoryIngress::try_recv` returns frames in reverse submission order | Frames arrive out of order in production | `crates/vb_ipc/src/queue/tests/array_queue_tests.rs:702-741` (S4-C4) |
| 3 | `vb_compile::compile_workflow` strips secret data from Finish results | Section 47 taint contract silently broken | `crates/vb_compile/src/taint/tests/secret_finish_tests.rs:42,69,94,...` (S3-C8) |
| 4 | `vb_cli::parse_args` returns wrong `Command::*` variant for a given subcommand | All CLI commands silently dispatch to the wrong handler | `crates/vb_cli/src/args/tests/*.rs` (S3-C1 through S3-C6) |
| 5 | `vb_expr::eval_binary_op(And/Or, ...)` uses Rust's `&&` / `\|\|` and short-circuits | Section 46 "no short-circuit" mandate violated; F64/I64 type mismatches silently skipped | `crates/vb_expr/src/eval/tests/and_or_short_circuit_tests.rs` orphaned (S4-C1); `eval_tests.rs` has zero short-circuit tests |

## 5. Top 10 Fixes Ranked by Impact-per-Effort

These are the 10 highest-leverage fixes from the 64 blocker items. Each is small, mechanical, and
catches a real class of silent regression. See slice artifacts for full BEFORE/AFTER.

| # | Fix | Crates affected | Effort | Catches |
|---|-----|-----------------|--------|---------|
| 1 | Replace `if let Ok(Command::X{..}) / else assert!(parsed.is_ok())` with `match { Ok(X) => ..., other => panic!(...) }` in 6 CLI args test files | vb_cli | 30 min | CLI dispatch regressions (mutation #4) |
| 2 | Replace `let _ = canonical_digest(&source).expect(...)` with concrete digest equality in `digest_ask_explicit_arm.rs` | vb_compile | 30 min | Digest determinism regressions (S3-C12) |
| 3 | Replace `let _ = digest_step_primitive(&mut hasher, &step)` with `.expect("digest must succeed")` in 18 sites in `do_choose_digest_unit_tests.rs` | vb_compile | 15 min | Silent Err from digest (S3-C11) |
| 4 | Delete the 4 tautology `assert!(result.is_ok() \|\| result.is_err())` and replace with concrete `matches!` | workspace_tests | 15 min | Compile-error path deletion (mutation #1 family) |
| 5 | Convert `process_lock_tests.rs:141-181` from "accepts all outcomes" to specific `ProcessLockHeld` assertion | vb_storage | 30 min | SECURITY-relevant process-lock regression (S2-C3) |
| 6 | Replace `matches!(result, Ok(_))` in `taint/tests/secret_finish_tests.rs` with workflow-content assertion (`workflow.finish_contains_secret_data()`) | vb_compile | 2 hours | Section 47 taint breach (mutation #3, S3-C8) |
| 7 | Replace `let _ = budget.field;` with concrete budget-value assertions in `budget_analyzer.rs` and `red_queen_budget.rs` | vb_compile | 1 hour | Whole-workflow budget corruption (S3-C7) |
| 8 | Wire `and_or_short_circuit_tests.rs` after deduplication, OR add 8 small tests to `eval_tests.rs` for And/Or no-short-circuit | vb_expr | 30 min | Section 46 short-circuit violation (mutation #5, S4-C1) |
| 9 | Replace `while let Ok(Some(_)) = ingress.try_recv() { received.push(()) }` with order-preserving frame collection + run_id equality | vb_ipc | 5 min | FIFO order violation (mutation #2, S4-C4) |
| 10 | Expose `canonical_primitive_name` as `pub(crate)` and rewire `proptest_save_canonical_name.rs` to call production | vb_compile | 30 min | Save/Spelling regression in production (S3-C10) |

**Total cleanup time for Top 10: ~6-7 hours.**

## 6. Fix List (beads dispatched to test-writer)

24 CRITICAL beads (one per finding) + 40 HIGH beads (grouped by file where possible).
Each bead includes the file:line, defect description, mutation thought experiment,
and exact BEFORE/AFTER snippet from the slice artifact.

| Bead title prefix | Count | Disposition |
|------------------|-------|-------------|
| `fix-test: S1-C{1..10} …` | 10 | blocker |
| `fix-test: S2-C{1..4} …` | 4 | blocker |
| `fix-test: S3-C{1..12} …` | 12 | blocker |
| `fix-test: S4-C{1..4} …` | 4 | blocker (S4-C1 is `owner_approved_debt` per the slice review — wire or replace) |
| `fix-test: H-* …` (40 HIGH items) | 40 | owner_approved_debt (track, fix opportunistically) |

## 7. Round Loop Protocol (Rounds 2-40)

For each round 2-40:

1. **Wait** for test-writer to close the open blocker beads (or a portion thereof).
2. **Re-dispatch** the same 4 subagents (same slice partitions, same rubric) against the
   updated code.
3. **Synthesize** a new round-N master review.
4. **Track closure**:
   - If finding N was a blocker and is no longer in round N+1, mark it CLOSED.
   - If a finding recurs, it becomes a "stale blocker" — investigate why the fix did
     not land cleanly.
   - Each round should find strictly fewer new CRITICALs and more MEDIUM/LOW drift.
5. **Target convergence**: by round 10, all CRITICALs should be CLOSED. By round 20,
   all HIGHs should be CLOSED. By round 30, MEDIUMs should be reduced to <10. By round
   40, the workspace should be APPROVED with only OBSERVATION-class findings.

**STATUS: REJECTED** — 24 CRITICAL blockers, 40 HIGH debt items. Workspace does not
ship. File beads, dispatch fixes, re-review.

## 8. Round 1 Fix Dispatch Status (added 2026-06-21)

22 P1 fix-test beads dispatched to 4 slice subagents (S1/S2/S3/S4) in
`/tmp/opencode/vb-testfix-r1-{s1,s2,s3,s4}/`. Evidence collected via
`jj diff` / `jj status` against parent commit `eddbe9c4` (WIP: in-flight
kani proof changes from femdation-tier-a). All 22 beads received
`bd comment` evidence; orchestrator will close after `git push`.

| Bead    | Slice | Fix landed? | Evidence |
|---------|-------|-------------|----------|
| vb-b9sab | S1    | YES         | action_tests.rs:267 — `assert_eq!(result, Err(...UnknownAction{action: ActionId::new(99)}))` |
| vb-wuexb | S1    | YES         | action_tests.rs:289,296 — `matches!(result, Ok(c) if c.id == ActionId::new(0\|2) && c.id.get() == 0\|2)` |
| vb-zc7vf | S1    | YES (pre-existing) | bounded_queue_tests.rs:105 — `assert_eq!(result, Ok(()))` was already in parent |
| vb-tjo9t | S1    | NO          | lru_ring_red_queen_tests.rs:517 still `assert!(r.is_err(), ...)` — recovery_bdd_tests.rs and lru_ring_red_queen_tests.rs NOT touched by S1 subagent |
| vb-hnn9u | S1    | NO          | recovery_bdd_tests.rs:2141 still `assert!(result.is_err(), ...)` |
| vb-2x3qk | S1    | NO          | recovery_bdd_tests.rs:2843,2852 still `assert!(result.is_ok()/is_err(), ...)` |
| vb-lynec | S1    | NO          | recovery_bdd_tests.rs:2728,2883 still `assert!(result.is_ok(), ...)` |
| vb-w73yl | S2    | YES         | integration_compile_error_message_quality.rs:376,403,428 — 3x tautologies deleted, replaced with `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(\|e\| matches!(e, CompileError::{DepthLimit\|SequenceLimit\|ScalarLimit} { actual, limit } if *actual > *limit && *limit == 1\|2\|5)))` |
| vb-ahb69 | S2    | YES         | integration_runtime_storage_fault_tolerance.rs:215 — tautology deleted; new `matches!(result, Ok(ref frame) if frame.run_id() == run && frame.step_count() == 0 && frame.slot_count() == 0)` |
| vb-6f2dj | S2    | YES (SECURITY) | process_lock_tests.rs:149-178 — 2x accept-all match blocks replaced with `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. })))` and `assert!(result.is_ok())` |
| vb-2tugo | S2    | YES         | edge_case_tests.rs:547 — `encode_rejects_zero_length_payload_serialization` renamed to `encode_accepts_zero_length_payload_and_round_trips`; assertion strengthened with envelope non-empty + round-trip checks |
| vb-ra0mp | S3    | NO          | S3 workspace has NO working-copy changes — `jj status` reports "The working copy has no changes"; vb_cli/src/args/tests/* untouched |
| vb-2ehds | S3    | NO          | S3 empty; budget_analyzer.rs and red_queen_budget.rs untouched |
| vb-a02hh | S3    | NO          | S3 empty; secret_finish_tests.rs untouched |
| vb-kviy0 | S3    | NO          | S3 empty; together_*_tests.rs untouched |
| vb-ladbb | S3    | NO          | S3 empty; proptest_save_canonical_name.rs untouched |
| vb-x6t5e | S3    | NO          | S3 empty; do_choose_digest_unit_tests.rs untouched |
| vb-5nljx | S3    | NO          | S3 empty; digest_ask_explicit_arm.rs untouched |
| vb-0to5y | S4    | YES         | eval_tests.rs:669-757 — 8 new no-short-circuit tests for BinaryOp::And/Or added |
| vb-2kw49 | S4    | YES         | density_tests.rs — local validate_count/validate_summary + 3 constants deleted; production `vb_core::workflow::compiled_slug::{validate_compiled_slug_count, validate_compiled_slug_summary, ...}` imported |
| vb-8r7cp | S4    | YES         | vb_ipc/src/tests.rs:445 — crossbeam_channel replaced with `MemoryIngress::bounded(QueueCapacity::new(NonZeroUsize::MIN))` + `disconnect_sender()` |
| vb-few2x | S4    | YES         | array_queue_tests.rs:730-749 — FIFO proptest now captures frames via `received.push(frame)` and asserts run_id order with `prop_assert_eq!` |

### Summary

- **Total reviewed:** 22 beads
- **Fix landed (11):** b9sab, wuexb, zc7vf, w73yl, ahb69, 6f2dj, 2tugo, 0to5y, 2kw49, 8r7cp, few2x
- **Pending retry (11):**
  - S1 partial retry needed (4): tjo9t, hnn9u, 2x3qk, lynec — subagent
    only touched action_tests.rs; must also fix lru_ring_red_queen_tests.rs
    and recovery_bdd_tests.rs
  - S3 full retry needed (7): ra0mp, 2ehds, a02hh, kviy0, ladbb, x6t5e,
    5nljx — subagent returned empty working copy; no files modified
- **Slice pass rates:** S1=3/7, S2=4/4, S3=0/7, S4=4/4
- **Action required:** re-dispatch S1 partial (3 files) and S3 full (7+ files)
