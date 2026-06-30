# Proof Strategy — vb-f7k6 Timer Wheel — Attempt 3 Repair

## State / Scope

- Bead: `vb-f7k6`.
- Current state: State 4 ledger repair only.
- Next state: State 5 proof repair.
- Status target: `READY_FOR_PROOF_REPAIR`.
- Isolated workspace: `/home/lewis/src/go-skill-vb-f7k6`.
- This attempt edits only `.beads/vb-f7k6/` planning/state artifacts. No production code, proof model, tests, harnesses, dependencies, or CI config were edited.

## Rejection Inputs Read

- `.beads/vb-f7k6/contract-verification-review.md`: `STATUS: REJECTED`.
- `.beads/vb-f7k6/proof-repair-guide.md`: `STATUS: REJECTED` guidance for authority mismatch, TLA coverage, Loom, and runtime parity.
- Required planning inputs read: `delivery-scope.jsonl`, `contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `codebase-map.md`, `proof-obligations.planned.jsonl`, and `STATE.md`.

## Attempt 3 Repairs

1. **TLA schema repair**
   - Added explicit `state_constraints` to every canonical TLA row `TLA-TW-001` through `TLA-TW-006` in `proof-obligations.jsonl`.
   - Mirrored the constraints in planned TLA rows `PO-001` through `PO-006` for reviewer clarity.

2. **Verus waiver status repair**
   - `VERUS-TW-001`, `VERUS-TW-002`, and planned `PO-009` now use `status:"planned"`.
   - Waiver semantics remain in `required:false`, `mode:"waived;schema-status-planned"`, and the `waiver` object.

3. **Runtime parity evidence path repair**
   - Runtime parity remains `.beads/vb-f7k6/test-report.md`.
   - Because `STATE.md` says State 5 already ran `/usr/bin/env cargo test -p vb_runtime timer`, State 5 must persist the command, exit code, and relevant output to that file before proof review can accept runtime parity.

4. **Production/proof authority binding repair**
   - Chosen route: **Option A**. Do not fake RunId-only authority.
   - Current TLA and Loom stale-fire proofs are marked `target-design-pre-implementation` until production carries or derives a timer freshness metadata/token equivalent to `(run, generation, deadline, kind)` and validates it before mutation.
   - Added required State 10 obligation `AUTH-TW-001` / `PO-011` for the production authority binding and post-change runtime parity evidence.

## Risk Classification

| Risk class | Applies | Lane |
|---|---:|---|
| Temporal/state-machine | yes | TLA+/TLC required |
| Bounded state/arithmetic | yes | TLA+/TLC plus runtime parity required |
| Rust-local invariant | yes | runtime parity required; Verus waived with planned status |
| Refinement/type-state | yes | TLA+/TLC + State 10 authority binding |
| Concurrency/stale fire | yes | Loom required, target design until State 10 |
| Unsafe/UB | no | Miri not applicable for this bead scope |
| Untrusted input | no | fuzz not applicable |
| Dependency/supply-chain | no | `changed_dependencies=[]` |
| Performance | no | benchmark not applicable |
| Release-critical | yes | required rows must pass or block before landing |

## State 5 Requirements

- Rerun/record `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla` and show checked properties/coverage.
- Rerun/record `cargo xtask loom --model timer_fired_cancel` and state it is target-design evidence until State 10.
- Write `.beads/vb-f7k6/test-report.md` for `/usr/bin/env cargo test -p vb_runtime timer` if already run, including command, exit code, and relevant output.

## State 10 Requirement

- Repair production authority mismatch by carrying or deriving freshness metadata/token for `TimerFired` delivery and validating it before mutation.
- Runtime parity must include stale fired event after replacement: `InvalidTimerFire`, no mutation, no resurrection.

No row claims proof success. This is a repaired proof plan only.
