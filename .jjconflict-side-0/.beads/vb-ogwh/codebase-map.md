bead_id: vb-ogwh
phase: 2
updated_at: 2026-05-17T22:26:00Z

# Codebase Map

Scoped local defect:
- `crates/vb_runtime/src/runtime.rs::Runtime::tick_shard` handled `ShardDirective::Shutdown` by calling `drain_for_shutdown()` without first enqueueing `ShardCommand::Shutdown`, so valid shutdown could return `ShutdownInProgress`.

Existing global CI repair work from old workspace is now on main via vb-ib8i and was not retained in this bead.
