# State 11 Workspace and Shutdown Verus Route Report — vb-8mdp.12

schema_version: state11-workspace-and-verus-route-report/v1  
bead_id: vb-8mdp.12  
state: 11  
sublane: workspace-verification-and-shutdown-verus-route  
delegate: holzman-rust  
attempt: 6

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Required inputs read

- `.beads/vb-8mdp.12/STATE.md`
- `.beads/vb-8mdp.12/implementation.md`
- `.beads/vb-8mdp.12/state11-shutdown-verus-exec-spec-binding-report.md`
- `.beads/vb-8mdp.12/proof-writer-report.md`
- `.beads/vb-8mdp.12/proof-evidence.md`
- `verification/verus/vb_8mdp_12_source_bound.rs`
- `crates/vb_runtime/src/shutdown_contract.rs`
- `crates/vb_runtime/src/runtime.rs`
- `crates/vb_ipc/src/server/dispatch.rs`
- `crates/vb_storage/src/queue/writer.rs`

## Workspace identity verdict

PASS for required Git worktree identity; JJ is not usable for this checkout.

- `pwd -P`: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.12`
- Required isolated path: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.12`
- Source checkout avoided: not `/home/lewis/src/velvet-ballistics`
- `git branch --show-current`: `review/vb-8mdp.12`
- `git rev-parse --show-toplevel`: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.12`
- `git rev-parse --short main`: `6a431705c`
- `jj root`: returned `/home/lewis`, but subsequent `jj workspace list` / `jj log -r @ --no-graph --limit 1` failed with `Failed to read commit backend type` and missing `/home/lewis/.jj/repo/store/type`. I treat JJ context as unavailable/broken, not authoritative for this Git worktree.

Raw log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-workspace.log`.

## Decision

Route **B**: terminal blocker / elevation report. I did not add production hooks.

Reason: the current production surface already has the minimal safe source-binding hooks that can honestly be added without changing the Verus architecture:

- Runtime pure shutdown contract: `DrainSnapshot::new`, `apply_intake`, `close_intake`, `drain_one`, `rejects_new_work`, `closed_intake_rejects_snapshot`, `closed_gate_drain_progresses_or_finalizes`.
- IPC/shard classifications: `classify_ipc_command_id`, `classify_shard_command_kind`, `ipc_command_rejected_after_shutdown`, `shard_command_rejected_after_shutdown`, `allows_post_shutdown_observation`.
- Runtime/action/timer hooks: `runtime_submit_rejected_after_shutdown`, `action_delivery_rejected_after_shutdown`, `timer_delivery_rejected_after_shutdown`.
- Storage hooks compiled through production source: `storage_enqueue_rejected_after_shutdown`, `storage_empty_drain_is_finalized`, wired into `JournalWriterQueue::enqueue` and `drain_all`.

Adding another boolean/const hook would only expand source-presence evidence. It would not close `PO-vb-8mdp.12-VERUS-001..005`, because the blocker is not lack of a small helper; it is lack of Verus-compatible production exec/spec binding for async/runtime/IPC/storage shells and their data structures.

## Fresh Verus result

Command:

```text
verus --crate-type=lib verification/verus/vb_8mdp_12_source_bound.rs
```

Result: PASS/PARTIAL. Raw log `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-verus.log` reports:

```text
verification results:: 1 verified, 0 errors
```

Interpretation: this proves the direct artifact is syntactically/Verus-checkable and contains a nonempty proof function. It does not prove semantic shutdown correctness for `Runtime`, `IpcServer` dispatch/polling, `JournalWriterQueue`/Fjall durability, timer delivery, or action delivery.

## Terminal blocker details

The source-bound artifact imports production Rust modules and const-asserts pure hooks, but it still cannot express or discharge full shutdown invariants over the production shells because:

1. `Runtime`, `Shard`, IPC server dispatch/polling, and storage queue methods are ordinary Rust, not Verus `exec fn` with `requires`/`ensures`.
2. The production shells rely on standard and async-adjacent structures Verus cannot reason about without modeled wrappers or trusted extern specs: `Mutex`, `Vec`, `VecDeque`, shard queues, resolver trait objects, storage journal adapters, and Fjall-backed durability behavior.
3. The storage section of `verification/verus/vb_8mdp_12_source_bound.rs` compiles `writer.rs` only through local crate-shell stubs. That is useful source binding for hooks, but it is a trusted boundary, not semantic proof of the real storage crate or Fjall effects.
4. The proof body remains `proof_source_bound_artifact_is_nonempty` with `ensures true`; this is intentionally honest non-vacuity/source-presence evidence, not a claim of semantic closure.
5. The planned obligations reject standalone/duplicate or hook-only closure. A stronger proof must bind actual production entry points with real contracts or formally approved trusted wrappers.

Raw discovery log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-discovery.log`.

## Concrete route for Verus closure

Next required owning state: **State 4 replan / proof-planner + proof-plan-reviewer approval**. State 11 should not keep adding ad hoc hooks until a broader Verus architecture is approved.

Required replan content:

1. Define a Verus-compatible shutdown core crate/module boundary for pure transition semantics:
   - closed intake rejects work;
   - accepted work drains monotonically to zero;
   - finalized state is absorbing;
   - action/timer delivery after terminalization cannot resurrect work;
   - storage close rejects enqueue and drains accepted records before finalize.
2. Choose one honest binding strategy per production shell:
   - annotate production-compatible pure helpers as Verus `exec/spec` where feasible; or
   - introduce narrow Verus adapter functions that production code calls directly; or
   - declare explicit trusted extern specs only for standard library / Fjall / async-shell effects that Verus cannot model, with ledger entries and proof-review acceptance.
3. Replace `ensures true` with named lemmas tied to production-called functions, for example:
   - `Runtime::ensure_shard_accepts_work` / post-shutdown submit rejection;
   - `dispatch_command_with_resolver` post-boundary work-command rejection;
   - `JournalWriterQueue::enqueue`, `drain_all`, and `shutdown` close/drain/finalize behavior;
   - timer/action delivery rejection hooks used by runtime enqueue paths.
4. Add loop invariants and static bounds for drain/poll loops, or move those loops behind verified bounded adapters.
5. Update the trusted-base ledger for every stub/extern spec before State 5 proof writing.
6. Then send to State 5 proof-writer to implement Verus artifacts, followed by independent State 6 proof-reviewer sufficiency review.

If State 4 rejects the broader Verus architecture cost, the honest alternative is an approved waiver/replan that downgrades `PO-vb-8mdp.12-VERUS-001..005` from semantic closure to source-hook evidence plus Kani/Flux/TLA+/proptest/loom/fuzz defense-in-depth. That waiver must be approved outside this State 11 sublane.

## Commands and raw evidence

- Workspace identity command requested by femdation — PASS for Git, JJ unavailable/broken. Raw log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-workspace.log`.
- `verus --crate-type=lib verification/verus/vb_8mdp_12_source_bound.rs` — PASS/PARTIAL (`1 verified, 0 errors`). Raw log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-verus.log`.
- Shutdown Verus route discovery grep — PASS as discovery, shows hook-only artifact and production hook locations. Raw log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-discovery.log`.
- `moon ci` — FAIL/BLOCK_GLOBAL. Raw log: `.beads/vb-8mdp.12/evidence/state11-workspace-and-verus-route-moon-ci.log`. Observed failures include missing fuzz target files in `fuzz/Cargo.toml`, TLA jar path failure, Kani compile errors in unrelated package lanes, six `vb_ipc` tests failing under the long isolated path, and panic-surface finding `crates/vb_runtime/src/vb_8mdp_12_kani.rs:17: assert!(false);`.

## Files changed

- Added `.beads/vb-8mdp.12/state11-workspace-and-verus-route-report.md`.
- Added `.beads/vb-8mdp.12/transcript-state11-holzman-rust-workspace-and-verus-route.md`.
- Added raw logs under `.beads/vb-8mdp.12/evidence/`:
  - `state11-workspace-and-verus-route-workspace.log`
  - `state11-workspace-and-verus-route-verus.log`
  - `state11-workspace-and-verus-route-discovery.log`
  - `state11-workspace-and-verus-route-moon-ci.log`

No production Rust, no verifier source, and no `implementation.md` production-hook section were changed.

## Power-of-Ten / zero-panic impact

- No production code changed; no new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/casts/arithmetic, or production assert macros were introduced.
- The decision preserves bounded, typed failure semantics already exposed by the shutdown hooks rather than adding redundant code.

## Performance-layer decision

No performance claim made. No benchmark/profiler evidence required. No second-ring assembly/API/provenance claim made.

## Residual risks

- Verus semantic closure remains unclosed until State 4 approves a broader Verus architecture or waiver/replan.
- Full `moon ci` remains BLOCK_GLOBAL in this workspace and must not be represented as passing.
- JJ metadata appears broken at `/home/lewis`; Git worktree identity is valid and matches the required branch/path.
