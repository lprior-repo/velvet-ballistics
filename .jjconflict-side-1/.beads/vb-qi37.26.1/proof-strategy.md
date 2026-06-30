# Proof Strategy: vb-qi37.26.1

## Bead Summary
- **ID:** vb-qi37.26.1
- **Title:** fix: vb_ipc typed handler compile errors blocking workspace-tests
- **Type:** Compile-fix prerequisite bead
- **Fix commit:** 0ebc5270
- **Risk tags:** compile, master-gap, prerequisite
- **Required verifier mode:** verify-standard

## Risk Classification

| Risk | Classification | Rationale |
|---|---|---|
| Temporal/state-machine | Absent | No state transitions, queues, retries, or distributed protocol modified |
| Rust-local invariant | Absent | No new pure/core functions; type consistency is enforced by rustc |
| Bounded state | Absent | No parser, codec, or finite-state machine changes |
| Refinement/type-state | Absent | Enum types already existed; fix only uses correct variants |
| Concurrency | Absent | No threads, atomics, channels, or async changes |
| Unsafe/UB | Absent | `#![forbid(unsafe_code)]` present; no unsafe introduced |
| Untrusted input | Absent | No parser/protocol boundary changes |
| Dependency/supply-chain | Absent | No dependency files changed |
| Performance | Absent | No hot paths modified; no performance claims |
| Release-critical | No | Explicitly marked non-release-critical in delivery-scope |

## Verifier Lane Strategy

### Active Lanes (verify-standard)

| Lane | Obligations | Rationale |
|---|---|---|
| **static-scan** | COMP-001, COMP-002, COMP-003, SAFE-001, SAFE-002, ORPH-001, TYPE-001 | Compilation + clippy + grep safety/orphan/type checks are the canonical gates for a compile fix. |

### Waived Lanes

| Lane | Waiver ID | Rationale |
|---|---|---|
| **Kani** | WAIV-KANI-001 | No bounded state machine, parser, codec, or arithmetic/index bounds risk. The fix is purely replacing String literals with enum variants; rustc provides equivalent assurance. |
| **Verus** | WAIV-VERUS-001 | No new pure Rust-core logic. The type checker enforces enum/struct field compatibility. |
| **TLA+** | WAIV-TLA-001 | No temporal workflow, protocol, scheduler, or lifecycle behavior modified. |
| **Flux** | WAIV-FLUX-001 | No refinement-type or numeric predicate changes. Enum variants are already strongly typed. |
| **Loom** | WAIV-LOOM-001 | No concurrency changes (threads, atomics, channels, locks). |
| **Miri** | WAIV-MIRI-001 | `#![forbid(unsafe_code)]` at top of file. No unsafe, FFI, raw pointers, or interior mutability introduced. |
| **proptest** | WAIV-PROP-001 | No broad input space or serialization boundary changes. The fix is type-correctness, not input handling. |
| **fuzz** | WAIV-FUZZ-001 | No untrusted input boundary or parser/protocol frame changes. |

## Discovery Evidence

```
$ cargo check -p vb_ipc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.98s

$ cargo check -p velvet-ballistics-workspace-tests --tests
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.08s

$ cargo clippy -p vb_ipc -- -D warnings
    No issues found

$ grep -n 'unsafe' crates/vb_ipc/src/server/handlers.rs
    Line 1: #![forbid(unsafe_code)]

$ ls crates/vb_ipc/src/server/handlers/mod.rs 2>/dev/null
    No such file or directory

$ /usr/bin/rg -n 'EdgeType::|PassFail::|GateKind::|NodeKind::|TaintPathStatus::' crates/vb_ipc/src/server/handlers.rs | wc -l
    227
```

## Pre-existing Safety Context

`handlers.rs` contains **pre-existing** `unwrap`, `expect`, `assert!`, and `?` usage from code that predates this bead (e.g., `to_allocvec().expect("encode payload")`, `try_from().unwrap_or()`). The contract obligation C3 / INV-003 requires that the **fix** (commit 0ebc5270) does not **introduce** new instances. Since the fix only replaced String literals with enum variants, no new panicking APIs were added. The grep-based scan validates this by inspection of the diff region.

## Execution Order

1. **COMP-001** (cargo check vb_ipc) - Gate 0: crate compiles
2. **COMP-003** (cargo clippy vb_ipc) - Gate 1: source lint zero tolerance
3. **COMP-002** (cargo check workspace-tests) - Gate 2: cross-crate compilation
4. **SAFE-002** (grep unsafe) - Gate 3a: no unsafe regression
5. **SAFE-001** (grep panicking APIs) - Gate 3b: no new panicking APIs introduced
6. **ORPH-001** (ls + cargo check) - Gate 4: orphaned files remain excluded
7. **TYPE-001** (grep enum variants) - Gate 5: typed enum consistency

## Rollback / Rerun Guidance

- If any COMP-* gate fails, rerun from COMP-001 after fixing the compile error.
- If SAFE-* gate finds new matches, inspect the git diff for commit 0ebc5270 to confirm they are pre-existing.
- If ORPH-001 fails (mod.rs exists), remove it and rerun from ORPH-001.
- All obligations map to `owner_state=11` (formal execution). No obligations are deferred.

## Artifact Targets

| Obligation | Artifact | Command |
|---|---|---|
| COMP-001 | compilation-report.md | `cargo check -p vb_ipc` |
| COMP-002 | compilation-report.md | `cargo check -p velvet-ballistics-workspace-tests --tests` |
| COMP-003 | compilation-report.md | `cargo clippy -p vb_ipc -- -D warnings` |
| SAFE-001 | safety-scan-report.md | `grep -n 'unwrap\|expect\|panic!\|todo!\|unimplemented!' crates/vb_ipc/src/server/handlers.rs` |
| SAFE-002 | safety-scan-report.md | `grep -n 'unsafe' crates/vb_ipc/src/server/handlers.rs` |
| ORPH-001 | compilation-report.md | `ls crates/vb_ipc/src/server/handlers/mod.rs 2>/dev/null; cargo check -p vb_ipc` |
| TYPE-001 | type-consistency-report.md | `grep -n 'EdgeType::\|PassFail::\|GateKind::\|NodeKind::\|TaintPathStatus::' crates/vb_ipc/src/server/handlers.rs` |
