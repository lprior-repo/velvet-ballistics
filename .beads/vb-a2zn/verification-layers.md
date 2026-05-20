# Verification Layers — vb-a2zn

## Boundary

- **Verus-owned kernel**: Pure Rust-local invariant that `events_for_run` and `events_for_run_from` never return `Ok([])` — the empty result path is replaced by `Err(NoEvents)`. This is a simple reachability invariant: after the prefix scan loop, `replay.is_empty()` → `Err`, not `Ok`.
- **Static-scan**: `JournalError::NoEvents` variant exists; enum is `#[non_exhaustive]`-compatible; `From<JournalError> for CliExitCode` maps `NoEvents` to `ValidationFailed`; no `Ok([])` in the events_for_run code path.
- **Test (BDD)**: All seven read-only CLI commands return exit code 2 for absent runs; exit code consistency invariant.
- **Kani**: No panic on arbitrary run IDs; bounded model checks that the empty-prefix path produces `Err(NoEvents)` not `Ok([])`.
- **Runtime shell**: Fjall journal I/O, WAL sync, snapshot encoding/decoding — excluded from formal proof.
- **External systems excluded from formal proof**: Fjall internal LSM-tree storage, OS filesystem sync.

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layers | Notes |
|---|---|---|---|
| PRE-001 (events_for_run preconditions) | static-scan | kani | RunId validity; no existence assumption |
| PRE-002 (events_for_run_from preconditions) | static-scan | kani | RunId validity; start_seq >= 0 |
| POST-001 (events_for_run returns Err(NoEvents)) | verus | kani | No Ok([]) path in function |
| POST-002 (events_for_run_from returns Err(NoEvents)) | verus | kani | Same invariant with start_seq filter |
| POST-003 (NoEvents variant exists) | static-scan | test | Enum variant present; field is RunId |
| POST-004 (NoEvents → ValidationFailed) | static-scan | test | From impl maps correctly |
| POST-005 (CLI exit code consistency) | test | — | BDD: all 7 commands return exit 2 |
| POST-006 (read_journal_events propagation) | static-scan | test | Err branch handles NoEvents |
| POST-007 (cmd_trace handles NoEvents) | test | — | BDD: trace returns exit 2 |
| POST-008 (cmd_diff handles NoEvents) | test | — | BDD: diff returns exit 2 |
| POST-009 (recover_full_journal dead code removal) | static-scan | — | Empty check becomes unreachable |
| POST-010 (cmd_events simplified) | static-scan | test | No empty-vec check needed |
| POST-011 (cmd_inspect simplified) | static-scan | test | No empty-vec check needed |
| POST-012 (cmd_retry/resume simplified) | static-scan | test | No empty-vec check needed |
| INV-001 (never returns Ok([])) | verus | kani | Core invariant — no Ok([]) path |
| INV-002 (NoEvents is only empty path) | verus | kani | Same as INV-001, different angle |
| INV-003 (CLI exit code consistency) | test | — | BDD: 7 commands, all exit 2 |
| INV-004 (ProcessLockHeld orthogonal) | static-scan | — | Lock check at open, NoEvents at events_for_run |
| INV-005 (NoEvents discriminant uniqueness) | static-scan | — | Non-exhaustive enum, automatic |
| INV-006 (NoEvents non-breaking) | static-scan | — | Adding fields is safe |
| INV-007 (NoEvents blanket From coverage) | static-scan | test | Match arm catches NoEvents explicitly |

---

## Verus Scope

### Rust Target: `crates/vb_storage/src/journal/replay.rs`

**Spec/Proof Functions**:
- `spec fn events_for_run_result(run, journal) -> Result<Vec<JournalEvent>, JournalError>` — models the function contract
- `proof fn no_empty_ok(result)` — `result.is_ok() → !result.unwrap().is_empty()`
- `proof fn no_events_on_empty_prefix(run, journal)` — if prefix scan yields zero items, returns `Err(NoEvents { run })`

**Invariants**:
- `events_for_run` never returns `Ok([])` — INV-001
- `events_for_run_from` never returns `Ok([])` — INV-002

**Trusted Boundary**:
- Fjall `snapshot.prefix` iteration — trusted external iterator
- `run_prefix_key(run)` — trusted key construction

**Shell Exclusions**:
- Fjall journal I/O (read events from prefix scan)
- Fjall WAL and durability

**Evidence Command**: `verus crates/vb_storage/src/journal/replay.rs`

---

## Static-Scan Scope

- **Clippy**: `JournalError` enum is `#[non_exhaustive]`; all variants have discriminants; `NoEvents` is reachable
- **grep**: No `Ok(replay)` or `Ok(events)` returns in `events_for_run` or `events_for_run_from` without preceding non-empty check removed
- **grep**: `From<JournalError> for CliExitCode` matches `NoEvents { .. }` → `ValidationFailed`
- **grep**: Empty-vec checks (`if events.is_empty()`) removed from `cmd_events`, `cmd_inspect`, `cmd_retry`, `cmd_resume`
- **grep**: `recover_full_journal` empty-vec check removed (dead code)

---

## Test (BDD) Scope

### Commands to test (all must return exit code 2 for absent runs):

| Command | Test Name | Run ID | Expected Exit |
|---|---|---|---|
| `inspect` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `events` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `replay` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `trace` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `retry` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `resume` | absent_run_returns_validation_failed | 999999999999 | 2 |
| `diff` | absent_run_a_returns_validation_failed | 999999999999 | 2 |
| `diff` | absent_run_b_returns_validation_failed | 999999999999 | 2 |

### Test database: fresh ephemeral Fjall instance with no runs

---

## Kani Scope

### Rust Target: `crates/vb_storage/src/journal/replay.rs`

**Harnesses**:
- `events_for_run_no_empty_ok_kani` — property: for all bounded `run` values, `events_for_run(run).is_ok() → events_for_run(run).unwrap().len() > 0`
- `events_for_run_from_no_empty_ok_kani` — same property with bounded `start_seq`

**State constraints**: bounded `run` (u64), bounded prefix scan depth (≤ 20 events per prefix)

**Evidence Command**: `cargo kani --harness events_for_run_no_empty_ok_kani --harness events_for_run_from_no_empty_ok_kani`

---

## Waivers

| Clause | Reason | Owner | Expiry |
|---|---|---|---|
| INV-004 (ProcessLockHeld orthogonality) | Lock check at open is separate code path; not modifiable by this bead | inherent | N/A — inherent property, not waiver |
| NoEvents in RecoveryError | RecoveryError::NoRecoveryData already covers this case at recovery layer | inherent | N/A |
