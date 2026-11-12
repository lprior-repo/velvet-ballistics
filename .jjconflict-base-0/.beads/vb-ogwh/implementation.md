bead_id: vb-ogwh
phase: 10
updated_at: 2026-05-17T22:27:00Z

# Implementation

Changed production/test file:
- `crates/vb_runtime/src/runtime.rs`

Production change:
- `ShardDirective::Shutdown` now enqueues `ShardCommand::Shutdown` before `drain_for_shutdown()`.

Test change:
- Added exact runtime tests for Continue, Suspend, Migrate error cases, and Shutdown drain/dead result.

Global CI repairs from prior workspace were dropped because vb-ib8i already landed them on main.
