# Round 4 Agent A10 — Bench Duplication + Test Density (CRITICAL)

**Reviewer:** black-hat-reviewer · **Composite severity: 85/100 · SHIP-BLOCKER**

## Per-File-Pair Verdict (12 bench pairs)

| # | Pair | Orig LoC | Migrated LoC | Diff | Compile? | Verdict |
|---|------|---------:|-------------:|-----:|:--------:|---------|
| 1 | action_dispatch | 242 | 255 | 87 | **NO (fatal syntax error)** | **CATASTROPHIC** |
| 2 | action_queuing | 262 | 257 | 126 | yes | diverged |
| 3 | array_queue | 241 | 281 | 121 | yes | diverged |
| 4 | cold_start | 262 | 274 | 62 | yes | diverged |
| 5 | collect_page | 292 | 340 | 118 | yes | diverged |
| 6 | ir_traversal | 418 | 415 | 21 | yes | diverged |
| 7 | memory_footprint | 260 | 260 | 41 | yes | diverged |
| 8 | pagination_cost | 300 | 325 | 121 | yes | diverged |
| 9 | rtrb | 255 | 278 | 152 | yes | diverged |
| 10 | snapshot_restore | 281 | 271 | 218 | yes | diverged |
| 11 | snapshot_save | 251 | 250 | 140 | yes | diverged |
| 12 | timer_wheel_tick | 336 | 320 | 121 | yes | diverged |

**The user's count of "11" is off by one — there are 12 pairs, not 11.** This is the first sign of a casual audit; the actual number is *worse* than reported.

**SHA-256 verdict: 0 of 12 are byte-identical. 12 of 12 have diverged.**

## Drift Mechanism

```bash
$ rg 'compile_error!|include_bytes!|include_str!' crates/workspace_tests/benches/
# (no output)
```

**There is ZERO static enforcement of equivalence between the two files.**

## Cargo Registration Asymmetry

`crates/workspace_tests/Cargo.toml` contains **15 `[[bench]]` entries**. **For all 12 `*_root_migrated.rs` files, there is NO `[[bench]]` entry.**

`cargo check --benches`, `cargo build --benches`, `cargo bench`, `cargo clippy --benches`, Miri-on-benches, and every other Rust toolchain gate operates on **only the 15 active files.** The 12 migrated files are **completely invisible to the compiler**.

## THE SMOKING GUN: Fatal Syntax Error in `action_dispatch_root_migrated.rs`

```rust
// File: crates/workspace_tests/benches/action_dispatch_root_migrated.rs
// Lines 7-13 (verbatim):
7: use criterion::{criterion_group, criterion_main, Criterion, Throughput};
8: use std::hint::black_box;
9: use vb_core::{
10: use vb_core::action::ActionName;        // ← BUG: a complete `use` stmt inside a `use {` block
11:     action::{ActionContract, ActionInput, ActionOutcome, Idempotency, SideEffect, RetrySafety},
12:     ids::{ActionId, RunId, SeqNo, StepIdx, SlotIdx},
13: };
```

`rustc` output (verbatim):
```
error: expected identifier, found keyword `use`
  --> crates/workspace_tests/benches/action_dispatch_root_migrated.rs:10:1
   |
10 | use vb_core::action::ActionName;
   | ^^^ expected identifier, found keyword
```

**`cargo build --benches` reports `Finished dev profile ... in 8.75s` for this exact file. The "Finished" message is a lie.** The file has been on disk with this bug since its creation in commit `838248499` ("chore: architectural drift repair", 2026-05-24) — and it has **never** been compiled, type-checked, or run in the 2+ weeks since.

## Semantic Divergence: Not Just Formatting

### `action_dispatch.rs:114` vs `action_dispatch_root_migrated.rs:118` — **Different expected state**

| File | Line | Code |
|------|-----:|------|
| `action_dispatch.rs` (orig) | 114 | `_ => panic!("expected ActionOutcome::Suspended")` |
| `action_dispatch_root_migrated.rs` (orphan) | 118 | `_ => panic!("expected ActionOutcome::Ready")` |

These are **two different enum variants** of `ActionOutcome`. The orig asserts the action is suspended (pending external completion); the orphan asserts it is ready (immediately deliverable). **These are mutually exclusive states.**

### `action_dispatch.rs:35` vs `action_dispatch_root_migrated.rs:32` — **Panic vs silent fallback**

| File | Line | Code |
|------|-----:|------|
| `action_dispatch.rs` (orig) | 35 | `panic!("bench action id must fit in u16: {error}")` |
| `action_dispatch_root_migrated.rs` (orphan) | 32 | `u16::try_from(i).unwrap_or(0)` — silent fallback to 0 |

A bench with `unwrap_or(0)` masks the legitimate failure mode. The bench is no longer measuring "1 action", "10 actions", "100 actions" — it's measuring 1, 10, and 100 where every id silently collides on 0 if `usize > u16::MAX`.

## Real Benchmark Evidence: The Emperor Has No Clothes

The master line 1796 says: *"Compileable Criterion scaffold benchmarks are placeholders only; no-op scaffolds such as `black_box(())` prove the harness builds, not that the implementation is faster, lower allocation, lower latency, or production ready."*

**Real measurement evidence exists for EXACTLY 3 bench targets:**
- `bench_engine_step_once_save_const_single_transition`
- `engine_run_until_blocked_budget_10_small_workflow`
- `ipc_frame_decode`

**Real measurement evidence is MISSING for ALL 12 of the duplicated bench files** (action_dispatch, action_queuing, array_queue, cold_start, collect_page, ir_traversal, memory_footprint, pagination_cost, rtrb, snapshot_restore, snapshot_save, timer_wheel_tick). They compile, they exist, they have `criterion_main!` at the bottom, and **they have never produced a number, ever.**

Per master line 1796, these are exactly the "compileable Criterion scaffold benchmarks" the master **explicitly rejects as performance evidence.**

## Top 3 Worst Findings

1. **The build system green-lights a fatal syntax error in production-adjacent code.** `action_dispatch_root_migrated.rs:10` will not compile under any rustc version. `cargo check --benches` does not catch it. The CI green badge is reporting health for code that is structurally **invisible to the compiler.**

2. **The 12 files are not duplicates — they are semantic fork-twins that measure different things.** `action_dispatch.rs` expects `ActionOutcome::Suspended`; the orphan expects `ActionOutcome::Ready`. The orig uses unique action names; the orphan uses the same name for 100 actions. The orig panics on `u16::overflow`; the orphan silently substitutes 0. **These are different benchmarks.**

3. **The 3.99x "test density gap" is phantom data and the 5x "master requirement" is not in the master.** The claim is technically false; no defensible LoC calculation reproduces 3.99x. The duplication hazard in `*_root_migrated.rs` is real, severe, and structurally invisible — the test-density discussion is a red herring.

## Verdict: SHIP-BLOCKER

**Mandatory fixes:**

1. **Delete the 12 `*_root_migrated.rs` files immediately.** `git rm crates/workspace_tests/benches/*_root_migrated.rs`
2. **Add a `compile_error!` fingerprint check in `velvet_ballistics.rs`**
3. **Audit `[[bench]]` registrations vs `benches/*.rs` files** in a new `scripts/check-bench-registration.sh`
4. **Open a P0 bead to re-derive the test-density claim with a reproducible LoC script**
5. **Re-run `moon ci` and demand the diffstat shows the orphan deletion**
