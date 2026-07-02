# Black-Hat Review — vb-f7k6 State 12 Retry

STATUS: APPROVED

## Startup Doctrine Cited

- `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md` lines 12-16 require contract/bead parity first and immediate rejection on parity failure; lines 23-33 require typed illegal-state prevention and panic-vector scrutiny.
- `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md` lines 12-16 and 23-33 contain the same rules and are authoritative on conflict.

## Findings

No blocking defects found in the repaired State 12 scope.

## Repair Verification

1. `Runtime::timer_fired(run)` now fails closed instead of deriving/fabricating timer authority from `RunId` only: `crates/vb_runtime/src/runtime.rs:351-355` returns `Err(RuntimeError::InvalidTimerFire)` after shard existence check.
2. Typed captured authority is the only runtime success path: `Runtime::capture_timer_entry` and `Runtime::timer_entry_fired(TimerEntry)` carry `generation`, `deadline`, and `kind` through `crates/vb_runtime/src/runtime.rs:357-372`.
3. Shard validation checks the full authority tuple before removing the run or timer: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:64-76`; mutation starts only after the exact match.
4. Timer wheel generation overflow is explicit non-wrapping error: `crates/vb_runtime/src/shard/timer_wheel.rs:76-83` returns `TimerWheelError::GenerationExhausted`; `crates/vb_runtime/src/shard/transitions.rs:158-164` maps pending-timer exhaustion to `RuntimeError::InvalidTimerFire` without wrap.
5. Tests bind the repaired behavior: public run-only fail-closed at `crates/vb_runtime/src/shard/tests/chunk_029.rs:57-82`, stale replacement rejection at `chunk_029.rs:2-55`, overflow fail-closed at `chunk_029.rs:225-261`, and wheel overflow preservation at `timer_wheel.rs:319-338`.
6. Formal evidence is approved and current: `.beads/vb-f7k6/formal-verification-report.md:53-65` records TLA, Loom, runtime timer tests, cargo check, and `moon ci` all PASS; `.beads/vb-f7k6/verification-ledger.jsonl:1-13` accounts for every obligation.
7. Regression gate is clean: I reran `/usr/bin/env cargo test -p vb_runtime timer` (`77` unit timer-filtered + `1` integration timer-filtered passed) and `/usr/bin/env moon ci` (`Tasks: 23 completed`, `Time: 34s 23ms`). This also confirms the prior panic-lint regression is not present in the canonical lint gate.

## Phase Verdicts

- Phase 1 Contract & Bead Parity: PASS. Public run-only delivery is fail-closed; captured authority path carries the required freshness tuple; overflow is explicit error.
- Phase 2 Farley Rigor: PASS. The tests hit the public runtime path and direct shard authority path; canonical CI passes.
- Phase 3 Holzman Rust: PASS. Timer freshness is represented in types and checked before state mutation.
- Phase 4 DDD/Simplicity: PASS. No production panic/unwrap/unchecked arithmetic added in the repaired timer path; generation uses `checked_add`.
- Phase 5 Bitter Truth: PASS. Evidence no longer overclaims the old forged-authority behavior.

## Brutal Verdict

APPROVED. The two rejected defects are fixed: run-only timer delivery is closed, captured authority is explicit, generation exhaustion no longer wraps, and the formal/machine evidence is green.
