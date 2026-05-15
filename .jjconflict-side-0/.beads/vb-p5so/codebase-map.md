bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 2
updated_at: 2026-05-09T00:00:00Z

## Codebase Map: vb_runtime Shard Timer/Shutdown Machinery

### Target Files
1. `crates/vb_runtime/src/shard/impl_.rs` (788 lines)
   - `Shard::new_with_journal_and_artifact_store` (line 29-49): constructs Shard with `pending_timers: IndexMap::new()`
   - `Shard::tick()` (line 118-162): main command dispatch loop. On `ShardCommand::Shutdown`, sets `self.shutting_down = true` and returns `false`.
   - `Shard::drain_for_shutdown()` (line 331-341): loops `tick()` until shutdown or capacity limit. **BUG**: never clears `pending_timers`.
   - `Shard::pending_timer_count()` (line 101-103): accessor for `pending_timers.len()`.

2. `crates/vb_runtime/src/shard/lifecycle.rs` (2040 lines)
   - `Shard::handle_timer()` (line 354-374): processes `TimerFired` command, removes timer via `pending_timers.swap_remove(&run)`.
   - `Shard::handle_cancel()` (line 376-388): cancels run, clears timer via `pending_timers.swap_remove(&run)`.
   - `Shard::handle_ask_answer()` (line 304-352): clears ask timer when answer arrives.
   - Timer registration happens in transitions.rs (line 80): `self.pending_timers.insert(run, PendingTimer { step, kind })`.

3. `crates/vb_runtime/src/shard/types.rs` (257 lines)
   - `Shard` struct (line 183): `pub(crate) pending_timers: IndexMap<RunId, PendingTimer>`
   - `PendingTimer` struct (line 28-31): `{ step: StepIdx, kind: PendingTimerKind }`
   - `PendingTimerKind` enum (line 22-25): `Wait | Ask`

4. `crates/vb_runtime/src/shard/timer_wheel.rs` (271 lines)
   - External `TimerWheel` struct for deadline tracking. Shard's `pending_timers` is the in-shard mirror.
   - The timer wheel lives outside the shard; shard only holds `pending_timers` as a fast lookup.

5. `crates/vb_runtime/src/shard/tests.rs` (6789 lines)
   - Existing drain_for_shutdown tests: lines 595-636 (impl_.rs module tests), lines 4185+, 4376+, 6129+, 6150+, 6159+, 6298+
   - Existing pending_timer tests throughout.

### Bug Analysis
`drain_for_shutdown()` processes commands until `Shutdown` is seen, then returns `Ok(())`. It never iterates `self.pending_timers` to clear them. Suspended runs with pending wait/ask timers are left dangling in the `IndexMap`. This violates the zero-leak graceful shutdown contract.

### Fix Strategy
In `drain_for_shutdown()`, after the tick loop completes, drain `self.pending_timers` completely (e.g., `self.pending_timers.clear()` or iterate and remove all). The runs themselves may or may not still be in `self.runs` depending on whether they were cancelled; the key is that `pending_timers` must be empty after shutdown drain.

### Existing Test Patterns
- Tests use `TestShardBuilder` pattern
- `shard.pending_timers.len()` is used to verify timer state
- `drain_for_shutdown()` returns `Ok(())` on successful shutdown
- Tests at lines 4378+, 6129+ cover shutdown behavior
