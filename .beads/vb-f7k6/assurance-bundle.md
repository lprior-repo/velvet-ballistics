# Assurance Bundle — vb-f7k6 State 13

STATUS: APPROVED

## Scope

- Bead: `vb-f7k6` / Add TLA+ Timer Wheel Model.
- Workdir verified by direct command: `/home/lewis/src/go-skill-vb-f7k6`.
- This bundle maps timer freshness requirements to existing TLA+, Loom, runtime test, formal-verifier, machine-gate, and black-hat evidence.

## Timer Freshness Requirement Mapping

| Requirement | Evidence | Direct State 13 verification |
|---|---|---|
| Bounded deadline arithmetic must not wrap; overflow/generation exhaustion fails closed. | `.beads/vb-f7k6/tla-report.md` TLA-TW-001; runtime tests `replacement_generation_overflow_fails_closed`, `shard_pending_timer_generation_overflow_fails_closed_without_wrap`; `timer_wheel.rs` uses `checked_add`. | TLC rerun exit 0; `/usr/bin/env cargo test -p vb_runtime timer` exit 0; source read confirmed `checked_add` and `GenerationExhausted`. |
| One active timer per run, replacement removes old index entry. | TLA-TW-002; runtime timer wheel tests; black-hat repair item 3/4/5. | TLC rerun exit 0; runtime timer tests exit 0; source read confirmed dual index and replacement path. |
| Cancel removes timer from all indexes and stale cancel fire cannot resurrect. | TLA-TW-003/TLA-TW-005; Loom `timer_fired_cancel_ordering`; tests `runtime_timer_fired_returns_invalid_timer_fire_when_cancelled_timer_event_arrives`. | TLC rerun exit 0; Loom rerun exit 0; cargo timer tests exit 0. |
| `fire_expired(now)` emits only due entries and destructively removes fired entries. | TLA-TW-004; timer wheel unit tests `fire_expired_returns_only_past_deadlines`, `fire_expired_drains_all_expired`. | TLC rerun exit 0; cargo timer tests exit 0. |
| Timer delivery must carry freshness authority `(generation, deadline, kind)`; run-only delivery fails closed. | AUTH-TW-001; tests `runtime_run_only_timer_fired_fails_closed_without_consuming_live_timer`, `timer_fired_command_exposes_generation_deadline_and_kind_authority_metadata`; black-hat repair items 1/2. | Runtime source read lines 351-371: run-only returns `InvalidTimerFire`; typed path enqueues generation/deadline/kind. Cargo timer tests exit 0. |
| Stale replacement/wrong generation/wrong deadline/wrong kind delivery is rejected before mutation. | TLA-TW-005; Loom `timer_fired_replace_ordering`; runtime tests named wrong generation/deadline/kind and stale replacement. | Source read `handle_timer` lines 71-76 checks `matches_authority` before mutation; cargo timer tests exit 0; Loom exit 0. |
| Terminal/cancelled/shutdown runs cannot be resurrected by timer fire. | TLA-TW-006; Loom `timer_fired_terminal_ordering`; tests `runtime_timer_fired_returns_invalid_timer_fire_when_terminal_timer_event_arrives`. | TLC exit 0; Loom exit 0; cargo timer tests exit 0. |
| Whole-workspace regression gate remains clean after timer freshness repair. | `.beads/vb-f7k6/machine-gate-report.md`; black-hat review reran timer tests and `moon ci`. | State 13 reran `/usr/bin/env moon ci`, exit 0, `Tasks: 23 completed`. |

## Artifact Status Checks

Direct command:

```text
pwd -P && test -s .beads/vb-f7k6/contract.md && test -s .beads/vb-f7k6/tla-report.md && test -s .beads/vb-f7k6/loom-report.md && test -s .beads/vb-f7k6/test-report.md && test -s .beads/vb-f7k6/formal-verification-report.md && test -s .beads/vb-f7k6/black-hat-review.md && python - <<'PY'
from pathlib import Path
files = [
'.beads/vb-f7k6/tla-report.md',
'.beads/vb-f7k6/loom-report.md',
'.beads/vb-f7k6/test-report.md',
'.beads/vb-f7k6/formal-verification-report.md',
'.beads/vb-f7k6/black-hat-review.md',
'.beads/vb-f7k6/machine-gate-report.md',
]
for name in files:
    text = Path(name).read_text()
    status = next((line for line in text.splitlines() if line.startswith('STATUS:')), 'STATUS: MISSING')
    print(f'{name}: {status}')
PY
```

Observed output:

```text
/home/lewis/src/go-skill-vb-f7k6
.beads/vb-f7k6/tla-report.md: STATUS: PASS
.beads/vb-f7k6/loom-report.md: STATUS: PASS
.beads/vb-f7k6/test-report.md: STATUS: PASS
.beads/vb-f7k6/formal-verification-report.md: STATUS: APPROVED
.beads/vb-f7k6/black-hat-review.md: STATUS: APPROVED
.beads/vb-f7k6/machine-gate-report.md: STATUS: APPROVED
```

Exit: 0.

## JSONL Ledger Integrity

Direct command:

```text
python - <<'PY'
import json
from pathlib import Path
for name in ['.beads/vb-f7k6/proof-obligations.jsonl','.beads/vb-f7k6/proof-obligations.planned.jsonl','.beads/vb-f7k6/verification-ledger.jsonl','.beads/vb-f7k6/traceability-matrix.jsonl']:
    count=0
    for line in Path(name).read_text().splitlines():
        if line.strip():
            json.loads(line)
            count += 1
    print(f'{name}: parsed {count} JSONL records')
PY
printf 'EXIT:%s\n' "$?"
```

Observed output:

```text
.beads/vb-f7k6/proof-obligations.jsonl: parsed 12 JSONL records
.beads/vb-f7k6/proof-obligations.planned.jsonl: parsed 11 JSONL records
.beads/vb-f7k6/verification-ledger.jsonl: parsed 13 JSONL records
.beads/vb-f7k6/traceability-matrix.jsonl: parsed 32 JSONL records
EXIT:0
```

## TLA+ Evidence

Direct command:

```text
tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla
printf 'EXIT:%s\n' "$?"
```

Observed output excerpt:

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 67 and seed 570487069140941904 with 1 worker on 32 cores with 30688MB heap and 64MB offheap memory [pid: 2349975] (Linux 7.0.3-arch1-2 amd64, Oracle Corporation 26.0.1 x86_64, MSBDiskFPSet, DiskStateQueue).
Parsing file /home/lewis/src/go-skill-vb-f7k6/verification/tla/TimerWheel.tla
Checking temporal properties for the complete state space with 315211 total distinct states at (2026-05-18 16:38:42)
Finished checking temporal properties in 00s at 2026-05-18 16:38:43
Model checking completed. No error has been found.
4209522 states generated, 315211 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 16.
Finished in 13s at (2026-05-18 16:38:43)
EXIT:0
```

## Loom Evidence

Direct command:

```text
/usr/bin/env cargo xtask loom --model timer_fired_cancel
printf 'EXIT:%s\n' "$?"
```

Observed output excerpt:

```text
Running loom model: timer_fired_cancel
Model path: crates/vb_runtime/src/models/loom/timer_fired_cancel.rs
Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel
running 3 tests
test models::loom::timer_fired_cancel::timer_fired_terminal_ordering ... ok
test models::loom::timer_fired_cancel::timer_fired_cancel_ordering ... ok
test models::loom::timer_fired_cancel::timer_fired_replace_ordering ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1454 filtered out; finished in 0.01s
PASS: Loom model 'timer_fired_cancel' completed successfully
EXIT:0
```

Warnings were unused/dead-code warnings in unrelated loom model modules, not this required model's failure.

## Runtime Test Evidence

Direct command:

```text
/usr/bin/env cargo test -p vb_runtime timer
printf 'EXIT:%s\n' "$?"
```

Observed output excerpt:

```text
running 77 tests
test shard::tests::runtime_run_only_timer_fired_fails_closed_without_consuming_live_timer ... ok
test shard::tests::runtime_timer_fired_rejects_wrong_deadline_authority ... ok
test shard::tests::runtime_timer_fired_returns_invalid_timer_fire_when_old_replaced_timer_event_arrives ... ok
test shard::tests::runtime_timer_fired_returns_invalid_timer_fire_when_terminal_timer_event_arrives ... ok
test shard::tests::runtime_timer_fired_returns_invalid_timer_fire_when_cancelled_timer_event_arrives ... ok
test shard::tests::runtime_timer_fired_rejects_wrong_generation_authority ... ok
test shard::tests::runtime_timer_fired_rejects_wrong_kind_authority ... ok
test shard::tests::shard_pending_timer_generation_overflow_fails_closed_without_wrap ... ok
test shard::tests::timer_fired_command_exposes_generation_deadline_and_kind_authority_metadata ... ok
test shard::tests::timer_wheel_fired_entry_carries_freshness_metadata_for_runtime_validation ... ok
test shard::timer_wheel::tests::replacement_generation_overflow_fails_closed ... ok
test result: ok. 77 passed; 0 failed; 0 ignored; 0 measured; 1369 filtered out; finished in 0.00s
running 1 test
test timer_fired_persists_before_ack ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
EXIT:0
```

## Canonical CI Evidence

Direct command:

```text
/usr/bin/env moon ci
printf 'EXIT:%s\n' "$?"
```

Observed output excerpt:

```text
Resolved targets: 23
                       velvet-ballastics:test |      Summary [   9.323s] 10894 tests run: 10894 passed, 44 skipped
              velvet-ballastics:mutants-smoke | 1 mutant tested: 1 caught
                       velvet-ballastics:miri | test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 935 filtered out; finished in 2.30s
Tasks: 23 completed
 Time: 29s 900ms

EXIT:0
```

## Decision

All required State 13 evidence exists, parses, and was independently sampled/rerun in this active execution context. The timer freshness contract is covered by executable TLA+, Loom, runtime tests, formal-verifier ledger/report, canonical CI, and black-hat approval.

STATUS: APPROVED
