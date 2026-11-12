bead_id: vb-ogwh
phase: 3
updated_at: 2026-05-17T22:26:00Z

# Contract

POST-001: `Runtime::tick_shard(_, ShardDirective::Shutdown)` must enqueue shutdown work, drain the selected shard, and return `Ok(false)` for a valid shard.
INV-001: `Continue`, `Suspend`, `Migrate`, and `Shutdown` preserve exact typed `RuntimeResult<bool>` behavior and error variants.
ERR-001: Self-migration returns `RuntimeError::MigrateSelf`; invalid target returns `RuntimeError::ShardNotFound`.

Non-goals: global benchmark/source CI repair work already landed with vb-ib8i.
