# vb-vt2f State 5 Proof Evidence

## Scope

- Bead: `vb-vt2f`
- State: 5
- Sublane: `owner-authorized-proof-evidence-sync-after-contract-approval`
- Attempt: owner-authorized evidence sync
- Workdir for every command: `/home/lewis/src/bd-vb-vt2f-bdd`

## Supersession Notice

Evidence in this file covers two distinct proof attempts:

1. **Original attempt 7 concrete Kani** (lines 52-118): Targeted full concrete Runtime/Shard. Timed out after 300s. **Superseded**.
2. **Owner-authorized projection kernels** (lines 137-241): Targeted bounded projection kernels. **PASS — current evidence**.

The original concrete Kani timeouts are retained for audit trail only and do not represent current proof status.

## TLA-VT2F-LIFECYCLE-001

Status: `PASS_FROM_ATTEMPT_6`.

Previous raw evidence remains applicable: `tlc -config verification/tla/Vt2fRuntimeLifecycle.cfg verification/tla/Vt2fRuntimeLifecycle.tla` completed with no errors over 1302 distinct states, checking `EventuallyTerminalOrSuspendedOrTypedErrorWithinBounds` and `NoDeadlockWithoutHeartbeatMask` after heartbeat masking was removed from the lifecycle model.

## TLA-VT2F-STRICT-ADMISSION-001

Command: `tlc -metadir states/vt2f-strict-admission-attempt7 -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla`

Result: PASS. The cfg now includes `EverySubmitEventuallyAcceptedOrTypedRejectedWithinBounds`; the spec uses `WF_vars(AdmissionProgress)` over submit/reject/enqueue/tick progress rather than heartbeat-only stutter.

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 78 and seed 7804015045434797173 with 1 worker on 32 cores with 30688MB heap and 64MB offheap memory [pid: 1305170] (Linux 7.0.3-arch1-2 amd64, Oracle Corporation 26.0.1 x86_64, MSBDiskFPSet, DiskStateQueue).
Parsing file /home/lewis/src/bd-vb-vt2f-bdd/verification/tla/Vt2fStrictAdmission.tla
Semantic processing of module Vt2fStrictAdmission
Starting... (2026-05-18 04:49:58)
Implied-temporal checking--satisfiability problem has 1 branches.
Computing initial states...
Finished computing initial states: 36 distinct states generated at 2026-05-18 04:49:58.
Progress(6) at 2026-05-18 04:49:58: 2,892 states generated, 1,096 distinct states found, 0 states left on queue.
Checking temporal properties for the complete state space with 1096 total distinct states at (2026-05-18 04:49:58)
Finished checking temporal properties in 00s at 2026-05-18 04:49:58
Model checking completed. No error has been found.
2892 states generated, 1096 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 6.
Finished in 00s at (2026-05-18 04:49:58)
```

## Normal Build Sanity

Command: `RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo check -p vb_runtime`

Result: PASS.

```text
cargo build (1 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.56s
```

## KANI-VT2F-RUNTIME-FACADE-001 — Original Concrete Attempt (SUPERSEDED)

**⚠️ SUPERSEEDED — See Owner-Authorized Projection Kernel Evidence (lines 137-241) for current PASS evidence.**

Original concrete attempt targeting full Runtime facade. Timed out after 300s.

Planned command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics`

Result: `BLOCK_LOCAL/KANI_TIMEOUT_OR_TRACTABILITY`, not a proof pass. Full output path from shell tool: `/home/lewis/.local/share/opencode/tool-output/tool_e3a7ee3d6001j1Ahy8S0uZ6mSt`.

```text
...output truncated...
Full output saved to: /home/lewis/.local/share/opencode/tool-output/tool_e3a7ee3d6001j1Ahy8S0uZ6mSt
aborting path on assume(false) at file /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1/src/raw.rs line 2343 column 9 function hashbrown::raw::RawTableInner::bucket::<usize> thread 0
aborting path on assume(false) at file /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/quick_cache-0.6.21/src/linked_slab.rs line 87 column 5 function quick_cache::linked_slab::LinkedSlab::<quick_cache::shard::Entry<lsm_tree::cache::CacheKey, lsm_tree::cache::Item, std::sync::Arc<quick_cache::sync_placeholder::Placeholder<lsm_tree::cache::Item>>>>::get thread 0
Unwinding loop _RNvNtNtNtCsh1i4bqfV5wd_3std3sys6random5linux9getrandom.0 iteration 77 file crates/vb_runtime/src/lib.rs line 0 column 0 function std::sys::random::linux::getrandom thread 0
<shell_metadata>
shell tool terminated command after exceeding timeout 300000 ms. If this command is expected to take longer and is not waiting for interactive input, retry with a larger timeout value in milliseconds.
</shell_metadata>
```

Discoverability/codegen command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics --only-codegen`

Discoverability/codegen result: PASS. This does not prove the row.

```text
Kani Rust Verifier 0.67.0 (cargo plugin)
warning: Found the following unsupported constructs:
             - C string literal (1)
             - TerminatorKind::InlineAsm (1)
             - caller_location (1)
             - catch_unwind (2)
             - foreign function (43)
             - simd_reduce_all (1)
warning: Kani currently does not support concurrency. The following constructs will be treated as sequential operations.
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
```

## KANI-VT2F-SHARD-LOWER-001 — Original Concrete Attempt (SUPERSEDED)

**⚠️ SUPERSEEDED — See Owner-Authorized Projection Kernel Evidence (lines 137-241) for current PASS evidence.**

Original concrete attempt targeting full concrete Shard lower semantics. Timed out after 300s.

Planned command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics`

Result: `BLOCK_LOCAL/KANI_TIMEOUT_OR_TRACTABILITY`, not a proof pass. Full output path from shell tool: `/home/lewis/.local/share/opencode/tool-output/tool_e3a838de6002jWLugkCAkD4y75`.

```text
...output truncated...
Full output saved to: /home/lewis/.local/share/opencode/tool-output/tool_e3a838de6002jWLugkCAkD4y75
aborting path on assume(false) at file /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1/src/raw.rs line 2343 column 9 function hashbrown::raw::RawTableInner::bucket::<usize> thread 0
aborting path on assume(false) at file /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/quick_cache-0.6.21/src/linked_slab.rs line 87 column 5 function quick_cache::linked_slab::LinkedSlab::<quick_cache::shard::Entry<lsm_tree::descriptor_table::CacheKey, std::sync::Arc<std::fs::File>, std::sync::Arc<quick_cache::sync_placeholder::Placeholder<std::sync::Arc<std::fs::File>>>>>::get thread 0
Unwinding loop _RNvNtNtNtCsh1i4bqfV5wd_3std3sys6random5linux9getrandom.0 iteration 79 file crates/vb_runtime/src/lib.rs line 0 column 0 function std::sys::random::linux::getrandom thread 0
<shell_metadata>
shell tool terminated command after exceeding timeout 300000 ms. If this command is expected to take longer and is not waiting for interactive input, retry with a larger timeout value in milliseconds.
</shell_metadata>
```

Discoverability/codegen command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics --only-codegen`

Discoverability/codegen result: PASS. This does not prove the row.

```text
Kani Rust Verifier 0.67.0 (cargo plugin)
warning: Found the following unsupported constructs:
             - C string literal (1)
             - TerminatorKind::InlineAsm (1)
             - caller_location (1)
             - catch_unwind (2)
             - foreign function (43)
             - simd_reduce_all (1)
warning: Kani currently does not support concurrency. The following constructs will be treated as sequential operations.
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
```

## Files Changed In Attempt 7

- `.beads/vb-vt2f/proof-evidence.md`
- `.beads/vb-vt2f/proof-writer-report.md`
- `.beads/vb-vt2f/blocker-report.md`
- `crates/vb_runtime/src/idempotency.rs`
- `crates/vb_runtime/src/primitives/collect.rs`
- `crates/vb_runtime/src/shard/timer_wheel.rs`
- `verification/tla/Vt2fStrictAdmission.tla`
- `verification/tla/Vt2fStrictAdmission.cfg`

## Waiver Status

`WAIVER-VERUS-VT2F-002` is CANDIDATE_ONLY. All stated approval preconditions per proof-strategy.md are now satisfied:
- TLA-VT2F-LIFECYCLE-001: PASS ✓
- TLA-VT2F-STRICT-ADMISSION-001: PASS ✓
- KANI-VT2F-RUNTIME-FACADE-001 (projection kernel): PASS ✓
- KANI-VT2F-SHARD-LOWER-001 (projection kernel): PASS ✓
- PROJ-EQ-VT2F-001: APPROVED (contract-verification-review.md `STATUS: APPROVED`) ✓
- BDD/catalog nextest: PASS ✓
- moon ci: PASS ✓

Final approval rests with State 6 proof-reviewer.

## PROJ-EQ-VT2F-001 Status

`STATUS: APPROVED` via contract-verification-review.md. The projection-equivalence review maps `KernelRuntimeError`, `StoreMode`, `FacadeKernelState`, `ShardKernelState`, `TicketShape`, and `AskKernelFrame` to concrete behavior as manual trusted projections only. This is not executable proof of concrete refinement.

---

# Owner-Authorized Kani Tractable Proof Kernel Evidence

## Scope

- Sublane: `owner-authorized-unblock / kani-tractable-proof-kernel`
- Attempt: 1 under owner-authorized deeper proof architecture
- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`
- Architecture artifact: `.beads/vb-vt2f/proof-architecture-report.md`

## Tool Context

Command: `cargo kani --version && if command -v kani >/dev/null; then kani --version; fi && rustc --version --verbose && cargo --version && rustup show active-toolchain`

Result: PASS.

```text
cargo-kani 0.67.0
rustc 1.97.0-nightly (52b6e2c20 2026-04-27)
cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)
nightly-2026-04-28-x86_64-unknown-linux-gnu (overridden by '/home/lewis/src/bd-vb-vt2f-bdd/rust-toolchain.toml')
```

## Harness Inventory

Command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani list --format json`

Result: BLOCKED by workspace shape.

```text
Kani Rust Verifier 0.67.0 (cargo plugin)
error: No supported targets were found.
```

Fallback: exact harness commands below discover and run the named harnesses. Source scans found only `kani::cover!` in the two replacement harness files; no `kani::assume`, stubs, contracts, `bounded_any`, `Arbitrary`, or `unsafe` surfaces in those files.

## KANI-VT2F-RUNTIME-FACADE-001

Replacement command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics`

Result: PASS. Full output path from shell tool: `/home/lewis/.local/share/opencode/tool-output/tool_e3aee3d4c001snD6fEQ58yJghX`.

```text
SUMMARY:
 ** 0 of 500 failed
 ** 7 of 7 cover properties satisfied
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Harness-to-claim map:

- strict missing accepted artifact rejection before queue mutation;
- relaxed/accepted store enqueue behavior;
- matching/stale/wrong/absent action/ask ticket coverage;
- facade action failure maps to `InvalidActionCompletion` and preserves unrelated run snapshot;
- ask answer writes target answer value/taint only for target run and preserves unrelated run snapshot.

Trusted surface: `KernelRuntimeError`, `KernelInspectResponse`, `FacadeKernelState`, `StoreMode`, and `TicketShape` are proof-kernel projections of concrete public Runtime/shard behavior.

## KANI-VT2F-SHARD-LOWER-001

Replacement command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics`

Result: PASS. Full output path from shell tool: `/home/lewis/.local/share/opencode/tool-output/tool_e3aee3f09001FLJEIOwvxMIM5S`.

```text
SUMMARY:
 ** 0 of 122 failed
 ** 8 of 8 cover properties satisfied
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Harness-to-claim map:

- lower `ActionFailed` absent-run returns projected `RunNotFound`;
- public-boundary `RuntimeActionFailed` maps projected `RunNotFound` to `InvalidActionCompletion`;
- Relaxed/Strict/Journaled policy cross-product covered;
- Missing/AlwaysPresent/StorageBackedAccepted store modes covered;
- explicit shard store selection remains distinct from runtime no-store construction;
- bool prompt rejection leaves executed count unchanged; non-bool prompt increments exactly once.

Trusted surface: `KernelRuntimeError`, `ShardKernelState`, `StoreMode`, and `AskKernelFrame` are proof-kernel projections of concrete lower shard/admission/wait_ask behavior.

## Build Sanity

Command: `RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo check -p vb_runtime`

Result: PASS.

```text
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s
```

## Skipped Gates

- Focused vt2f nextest: skipped because only `#[cfg(kani)]` proof harness code and bead artifacts were changed in this sublane; no production behavior change.
- `moon ci`: skipped for same reason; no production behavior change after proof-kernel extraction.

## Final Kani Blocker Decision

`KANI-VT2F-RUNTIME-FACADE-001` and `KANI-VT2F-SHARD-LOWER-001` are cleared by owner-authorized replacement proof kernels. The remaining risk is projection equivalence, recorded in `.beads/vb-vt2f/proof-architecture-report.md`.

---

# LETHAL-001 Stale Ask Kani Projection Repair (Attempt 1)

## Scope

- Sublane: `black-hat-stale-ask-kani-projection-repair`
- Attempt: 1
- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`
- Defect: `defects.md:LETHAL-001` — `answer_ask` and `tick_after_answer` modeled stale ask tickets as successful; contract requires `RunNotFound`

## Tool Context

Command: `cargo kani --version && rustc --version`

Result: PASS.

```
cargo-kani 0.67.0
rustc 1.97.0-nightly (52b6e2c20 2026-04-27)
```

## Defect Fix

Three changes to `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`:

### `answer_ask` (lines 134-154)
Removed `TicketShape::Stale` from the success arm. Only `Matching` with `target_active && target_asking` returns `Ok`. `Stale` always returns `Err(KernelRuntimeError::RunNotFound)`.

### `tick_after_answer` (lines 157-171)
Removed `TicketShape::Stale` from the success arm. `Stale` always returns `Err(KernelRuntimeError::RunNotFound)`.

### Test assertion branching (lines 257-270)
Changed `if matches!(shape, TicketShape::Matching | TicketShape::Stale)` to `if matches!(shape, TicketShape::Matching)`. Now Stale correctly hits the else branch with `RunNotFound` assertions.

## KANI-VT2F-RUNTIME-FACADE-001 (LETHAL-001 Repair)

Command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics`

Result: PASS. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e3b5c68ff001X2hRbAWzZILtpN`

```
SUMMARY:
 ** 0 of 489 failed
 ** 7 of 7 cover properties satisfied
 VERIFICATION:- SUCCESSFUL
 Manual Harness Summary:
 Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

### Cover Points (all satisfied)
- missing accepted artifact store covered ✓
- accepted artifact store covered ✓
- strict policy covered ✓
- matching ticket covered ✓
- stale ticket covered ✓
- wrong-run ticket covered ✓
- absent-run ticket covered ✓

### Key Semantic Fix Verified
- `Matching` with active target → `Ok(())` in `answer_ask` and `tick_after_answer` ✓
- `Stale` → `Err(KernelRuntimeError::RunNotFound)` in both functions ✓
- `WrongRun`/`AbsentRun` → `Err(KernelRuntimeError::RunNotFound)` ✓
- Unrelated run snapshot preserved ✓

## Build Sanity

Command: `RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo check -p vb_runtime`

Result: PASS.

```
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

## LETHAL-001 Resolution

| Item | Status |
|---|---|
| `answer_ask` Stale → `RunNotFound` | FIXED ✓ |
| `tick_after_answer` Stale → `RunNotFound` | FIXED ✓ |
| Test assertions aligned to contract | FIXED ✓ |
| Kani harness passes | PASS ✓ |
| All 7 cover points satisfied | PASS ✓ |

LETHAL-001 is now repaired. The projection kernel correctly models stale ask semantics matching the public API oracle (`contract.md:64-65`, `vb_vt2f_direct_runtime_api_acceptance.rs:658-698`).
