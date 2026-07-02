# Implementation — vb-vzo9b

## Bead

- bead_id: vb-vzo9b
- title: Tests: replace multi-run recovery disjunction with exact slots (P1)
- controller: holzman-rust (direct child of femdation)
- state: 11
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b
- source_checkout: /home/lewis/src/velvet-ballistics (coordination only, untouched)

## Touched Surface

Single fuzz harness file modified per delivery-scope.jsonl:

- `fuzz/src/journal_target/readback.rs` — exactly one block (lines 192-216) rewritten.
  Production surface untouched:
- `crates/vb_storage/src/recovery/types.rs` (lines 547-605) — UNCHANGED.
- `crates/vb_storage/src/recovery/replay/summary/apply.rs` — UNCHANGED.
- `crates/vb_storage/src/recovery/replay/summary/derive.rs` — UNCHANGED.
- `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` — UNCHANGED.
- `fuzz/src/journal_target.rs`, `fuzz/src/lib.rs`, `fuzz/src/bin/recovery_decode.rs`, `fuzz/Cargo.toml` — UNCHANGED.

## Diff

```diff
--- a/fuzz/src/journal_target/readback.rs
+++ b/fuzz/src/journal_target/readback.rs
@@ -193,7 +193,21 @@
         Ok(hydration) => {
             if !events.is_empty() {
                 let run_summary = hydration.summary();
-                assert!(run_summary.run == run || run_summary.run == vb_core::RunId::new(0));
+                let expected = vb_storage::recovery::RecoveryRuntimeSummary {
+                    run,
+                    first_seq: seq,
+                    last_seq: seq,
+                    workflow: Some(digest),
+                    steps_started: 0,
+                    steps_succeeded: 0,
+                    actions_scheduled: 0,
+                    actions_resolved: 0,
+                    suspensions: 0,
+                    slots_written: 0,
+                    terminal: None,
+                };
+                assert_eq!(run_summary, expected);
             }
         }
         Err(error) => assert_typed_recovery_error(error),
```

Lines 195-216 of the post-fix file (full block context):

```rust
    match vb_storage::recovery::summarize_recovery_events(&events) {
        Ok(hydration) => {
            if !events.is_empty() {
                let run_summary = hydration.summary();
                let expected = vb_storage::recovery::RecoveryRuntimeSummary {
                    run,
                    first_seq: seq,
                    last_seq: seq,
                    workflow: Some(digest),
                    steps_started: 0,
                    steps_succeeded: 0,
                    actions_scheduled: 0,
                    actions_resolved: 0,
                    suspensions: 0,
                    slots_written: 0,
                    terminal: None,
                };
                assert_eq!(run_summary, expected);
            }
        }
        Err(error) => assert_typed_recovery_error(error),
    }
    if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events) {
        assert_typed_recovery_error(error);
    }
```

## Pin Specification Met (all 11 `RecoveryRuntimeSummary` fields)

| # | Field             | Expected value                 | Source                                              |
|---|-------------------|-------------------------------|-----------------------------------------------------|
| 1 | `run`             | `run` (local RunId)           | `summarize_recovery_events` first event's `run_id()` (apply.rs:92) |
| 2 | `first_seq`       | `seq` (`EventSeq::new(1)`)    | `summarize_recovery_events` first event's `seq()` (apply.rs:95) |
| 3 | `last_seq`        | `seq` (`EventSeq::new(1)`)    | `summarize_recovery_events` event loop (apply.rs:124) |
| 4 | `workflow`        | `Some(digest)`                | `apply_summary_event` for `RunAccepted` (apply.rs:25-27) |
| 5 | `steps_started`   | `0`                           | `RecoveryRuntimeSummary` default + no `StepStarted` event |
| 6 | `steps_succeeded` | `0`                           | default + no `StepSucceeded`/`ActionCompletedEnvelope`/`AskTimedOutEvent` |
| 7 | `actions_scheduled` | `0`                         | default + no `ActionScheduled`/`ActionScheduledTicket` |
| 8 | `actions_resolved`| `0`                           | default + no `ActionCompletedEvent`/`ActionFailedEvent`/`ActionCompletedEnvelope`/`ActionAbandoned` |
| 9 | `suspensions`     | `0`                           | default + no `WaitScheduledEvent`/`AskScheduledEvent`/`RetryScheduledEvent` |
| 10 | `slots_written`   | `0`                           | default + no `SlotWrittenEvent`/`ActionCompletedEnvelope` |
| 11 | `terminal`        | `None`                        | default + no `RunCancelled`/`RunKilled`/`RunFinished`/`RunFailedEvent` |

The empty-events sentinel rejection rail is preserved through the existing
`match` arm `Err(error) => assert_typed_recovery_error(error)`, which the
production `summarize_recovery_events` reaches by returning
`Err(RecoveryError::NoRecoveryData { run: RunId::new(0) })` when `events.first()`
is `None` (apply.rs:89-91). `assert_typed_recovery_error` matches `NoRecoveryData`
in its `match` arm (errors.rs:67) so the empty-payload fuzz rail retains typed
error coverage.

## Power-of-Ten and Zero-Panic Rules Affected

| Rule                                              | Status | Note |
|---------------------------------------------------|--------|------|
| #1 Simple control flow                            | PASS   | No recursion, no panic-driven flow, no macro-hidden branching. |
| #2 Fixed loop bounds                              | N/A    | No new loops. Outer match-block remains a single iteration over the (single-event) `events` Vec inside `summarize_recovery_events`; bound unchanged. |
| #3 No post-init dynamic allocation                | N/A    | Code path is test code; `Vec::new()` over `data.len()` is bounded by fuzz input size and was already present. |
| #4 ≤25 logical lines                              | PASS   | Replaced 1-line disjunctive assert with 14-line typed struct literal that is data, not logic; function length unchanged at 35 lines including surrounding match arms. |
| #5 Assertion density                              | PASS   | `assert_eq!` over a `PartialEq + Eq + Debug` typed struct (~11 field equality, panic on diff) replaces a 1-bit disjunction. Invariant strength: weak disjunction → maximum-precision pin. |
| #6 Smallest scope                                  | PASS   | `expected` lives in the immediate `if !events.is_empty()` block; declared at first use. |
| #7 Checked returns                                | PASS   | No `Result`/`Option` introduced; existing `match` still validates `summarize_recovery_events` and existing `if let Err(...)` validates `recover_runtime_frame_seed_from_events`. |
| #8 Limited macros/pointers                        | PASS   | No macros, no FFI, no function pointers. |
| #9 Restricted pointer use                         | PASS   | No `unsafe`, no raw pointers, no `transmute`. |
| #10 Warnings and analysis zero                    | PASS   | `cargo check` and `cargo build --bin recovery_decode` succeed clean. `cargo clippy --bin recovery_decode` reports 0 findings in `readback.rs` (other fuzz crates have pre-existing lint findings that fall under `BLOCK_GLOBAL` and are not in this bead's delivery scope). |

Additional Holzman/Rust strength:

- Zero forbidden constructs in the patch: no `unsafe`, no `unwrap`, no `expect`, no `panic`, no `todo`, no `unimplemented`, no `unreachable!`, no `dbg!`, no checked-index removal, no lossy `as`, no ignored fallible results. The pre-existing `data.first().copied().unwrap_or(0)` at readback.rs:185 is unchanged (out of scope per delivery-scope.jsonl row 1).
- Pre-fix: `RecoveryRuntimeSummary` derives `Debug, Clone, Copy, PartialEq, Eq` (types.rs:546). Post-fix: the new `assert_eq!` requires all four derives; they are present.
- Sentinel-collision guard: the OR-disjunction accepted `run_summary.run == RunId::new(0)` regardless of payload; post-fix the equality pin rejects any payload-derived `run_id() != run`, exposing that fuzz payload was mis-classifying sentinel as success.

## Exact Commands Run and Pass/Fail Status

All commands run from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`.

Pre-fix baseline (carried in evidence/00-baseline-*.txt for reference):

| Command                                                    | Result      |
|------------------------------------------------------------|-------------|
| `cargo test -p vb_storage --lib summarize_recovery_events` | PASS — 12 passed; 0 failed; 0 ignored; 1518 filtered out |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | PASS — 6 passed; 0 failed; 0 ignored; 1524 filtered out |
| `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` (after `cargo clean --manifest-path fuzz/Cargo.toml`) | PASS — `Finished 'recovery_decode' [unoptimized + debuginfo] target(s) in 6.84s` |

Post-fix (the bead task's named evidence commands):

| Command                                                    | Result      |
|------------------------------------------------------------|-------------|
| `cargo test -p vb_storage --lib summarize_recovery_events --no-fail-fast` | PASS — `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1518 filtered out` |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events --no-fail-fast` | PASS — `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1524 filtered out` |
| `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` | PASS — `Compiling velvet-ballistics-fuzz v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/fuzz)` → `Finished 'recovery_decode' [unoptimized + debuginfo] target(s) in 0.20s` |

PO-003 source-lint forbidden-pattern gates (from proof-obligations.planned.jsonl),
each executed post-fix and each returning zero matches (exit 1 → inverted by `!`
in the gate chain ⇒ true → forbidden-pattern gate passes):

| Gate                                          | rg exit | Gate (inverted) |
|-----------------------------------------------|---------|-----------------|
| `! rg -n 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |
| `! rg -n 'matches!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |
| `! rg -n 'let _summary' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |
| `! rg -n '\bdbg!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |
| `! rg -n '\.unwrap\(\)' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |
| `! rg -n '\.expect\(' fuzz/src/journal_target/readback.rs` | 1 (no match) | PASS |

(The pre-existing `data.first().copied().unwrap_or(0)` at readback.rs:185 contains
the substring `.copied().unwrap_or(0)` and therefore would not match the bare
`\.unwrap\(\)` pattern — it is `unwrap_or`, not `unwrap`. The gate is satisfied.)

## Benchmark / Profiler Evidence for Performance Claims

No performance claim. This bead is a P1 test correctness fix; no hot-path, no
latency/throughput target, no allocator budget, no profile budget. Performance
layer is `BLOCK_GLOBAL` precedent (no claim made, no evidence required, no
blocker recorded).

## Second-Ring Evidence

Not applicable. The patch does not claim zero-cost abstraction, vectorization,
bounds-check removal, or release-provenance changes. The patch is a single-step
test-assertion rewrite.

## Skipped / Residual Risks

- `moon ci` was not re-run end-to-end on this isolated workspace. Per
  delivery-scope.jsonl row 27, moon-ci is `required=false` for this bead
  ("Canonical gate deferred to landing per bead workflow; targeted gates above
  are sufficient for state 2-7."). Landing will run `moon ci` against the merged
  change.
- Pre-existing clippy findings exist in other fuzz crate files
  (`src/expression_target.rs:257`, `src/workflow_target/budget.rs:142`,
  `src/workflow_target/collect.rs:87`, `src/workflow_target/node_slots.rs:100`,
  `src/ipc_target.rs:47`). These are `BLOCK_GLOBAL` pre-existing lint debt in
  the fuzz crate, wholly outside `readback.rs` and outside this bead's delivery
  scope. None were introduced or modified by this patch — clippy reports zero
  findings against `fuzz/src/journal_target/readback.rs`.
- The fuzz harness still gates on byte-length parity to choose between
  populated events and empty events; this binary branching is unchanged. Future
  work (epic vb-82snf, "Fuzz Test: recovery corruption assertions and mutation
  strength") will broaden payload diversity.

## Artifact Hashes

```
8fa31a41261158087bb73d169ebbe061804233795e422de0cbbe41ae70e3eef0  fuzz/src/journal_target/readback.rs (post-fix)
8fdd279d385f9c8c7e409b4cf621ec0ab5176e45f099813877a934ae3005a347  .beads/vb-vzo9b/evidence/00-baseline-summarize_recovery_events.txt
5ea21918827c495e23d12043f4de98cf76061c6bcd1955d5dda94ead7a1cd4da  .beads/vb-vzo9b/evidence/00-baseline-recover_runtime_frame_seed_from_events.txt
abe58872e6d771a5d6787f8eab9fbc9ef4bd362f8a5ad6c4337b8fb61c5222c8  .beads/vb-vzo9b/evidence/00-baseline-build-recovery_decode.txt
596b9cf65e3611653f154cb9b2e80c02c34afa954ec7cb419b22a7d6765a5607  .beads/vb-vzo9b/evidence/01-postfix-summarize_recovery_events.txt
01328a23f021a1c4c949ce19a578b72849d0e0330d7ba95d84a38291baa49893  .beads/vb-vzo9b/evidence/01-postfix-recover_runtime_frame_seed_from_events.txt
943caed5e54a41a17ad9ea47a82e859d4ce9861f9960e2c646abe6154b697435  .beads/vb-vzo9b/evidence/01-postfix-build-recovery_decode.txt
```

## Files Read Before Editing

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode activation bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/fuzz/src/journal_target/readback.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/fuzz/src/journal_target/errors.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/crates/vb_storage/src/recovery/types.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/crates/vb_storage/src/recovery/mod.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/crates/vb_storage/src/recovery/replay/summary/apply.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/crates/vb_storage/src/events.rs`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/proof-plan-review.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/proof-obligations.planned.jsonl`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/delivery-scope.jsonl`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/baseline-report.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/STATE.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/routing-ledger.jsonl`
