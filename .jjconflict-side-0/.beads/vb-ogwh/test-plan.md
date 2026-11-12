bead_id: vb-ogwh
phase: 7
updated_at: 2026-05-17T22:27:00Z

# Test Plan

BDD scenarios:
- Given queued work on a selected shard, when `Continue` ticks, then only selected shard work advances.
- Given queued work and `Suspend`, when ticked, then queued work is preserved until `Continue`.
- Given invalid migration targets, when `Migrate` ticks, then exact typed errors are returned.
- Given queued work and `Shutdown`, when ticked, then shutdown drains and returns `Ok(false)`.
