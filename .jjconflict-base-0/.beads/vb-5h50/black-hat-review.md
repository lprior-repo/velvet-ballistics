bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-11-black-hat
updated_at: 2026-05-09T00:00:00Z

# Black-Hat Review

## PHASE 1: Contract Parity

### Bead Requirements vs Implementation
| Requirement | Status | Evidence |
|---|---|---|
| Delete old events only after durable snapshot | ✅ | `latest_durable_snapshot_seq` + `NoDurableSnapshot` error |
| Retain terminal runs per retention policy | ✅ | `check_retention_policy` + `RetentionPolicyBlocks` error |
| Trim is idempotent | ✅ | `<` comparison + `skip_noop_runs` |
| No acknowledged state recoverable only from deleted events | ✅ | Replay starts from snapshot; `replay_equivalence_after_trim` proves it |

### Contract Spec vs Code
- All preconditions enforced: snapshot existence checked before trim ✅
- All postconditions tested: remaining events verified, headers preserved ✅
- All invariants tested: idempotency, retention, boundary safety ✅

## PHASE 2: Farley Engineering Rigor

### Function Lengths
| Function | Lines | Status |
|---|---|---|
| `latest_durable_snapshot_seq` | ~30 | ✅ ≤50 |
| `trim_events_for_run` | ~45 | ✅ ≤50 |
| `trim_all_eligible_runs` | ~20 | ✅ ≤50 |
| `has_terminal_event` | ~20 | ✅ ≤50 |
| `check_retention_policy` | ~35 | ✅ ≤50 |

### Parameter Counts
All public functions have ≤3 parameters ✅

### Pure Logic / I/O Separation
- `trim_events_for_run`: Imperative shell (orchestrates snapshot check, retention, batch delete) ✅
- `has_terminal_event`: I/O boundary (scans keyspace) — acceptable as private helper ✅
- `check_retention_policy`: I/O boundary (reads headers, scans events) — acceptable as private helper ✅

## PHASE 3: NASA-Level Functional Rust

### The Big 6
1. **Illegal states unrepresentable**: `TrimStatus` enum (Trimmed/NoOp) prevents invalid states ✅
2. **Parse, Don't Validate**: Events are decoded into `JournalEvent` enum at storage boundary ✅
3. **Types as Documentation**: `TrimPolicy`, `TrimError`, `TrimmedRunResult` are domain-specific ✅
4. **Workflows explicit**: `trim_events_for_run` → snapshot check → retention check → batch delete ✅
5. **Newtypes**: `EventSeq`, `RunId` are newtypes from `vb_core` ✅
6. **No panic paths**: Zero `unwrap`, `expect`, `panic`, `todo`, `unimplemented` in production code ✅

## PHASE 4: Ruthless Simplicity & DDD

### CUPID Assessment
- **Composable**: `trim_events_for_run` + `trim_all_eligible_runs` compose naturally ✅
- **Unix philosophy**: Each function does one thing ✅
- **Predictable**: Same inputs → same outputs (deterministic) ✅
- **Idiomatic**: Uses `Result`, `?`, pattern matching, iterators ✅
- **Domain-based**: Names are from the domain (trim, snapshot, retention) ✅

### Panic Vector Scan
```bash
grep -n "unwrap\|expect\|panic\|todo!\|unimplemented!" crates/vb_storage/src/trimming.rs
```
Result: 0 matches in production code. (Only in `#[cfg(test)]` module, which is allowed.)

## PHASE 5: The Bitter Truth

### Cleverness Check
- No macros invented for this feature ✅
- No generic abstractions with single use ✅
- No trait proliferation ✅
- Code reads like a straightforward procedure ✅

### YAGNI Check
- No "future-proofing" abstractions ✅
- Retention policy is a simple struct with two fields ✅
- No plugin architecture for trim strategies ✅

### Junior Developer Test
Would a junior understand this code after 5 minutes? **Yes.** The flow is:
1. Find snapshot
2. Check retention
3. Delete old events
4. Return result

## Findings

### LETHAL: 0
### MAJOR: 0
### MINOR: 1
- `check_retention_policy` calls `has_terminal_event` which scans ALL events for the run. For runs with many events, this is O(N). A potential optimization would be to check only the last event or use an index. However, trimming is a background operation and correctness is prioritized over performance in this bead.

## Decision

STATUS: APPROVED

The implementation is clean, correct, and follows all engineering constraints. Minor performance note is not a blocker for this feature bead.
