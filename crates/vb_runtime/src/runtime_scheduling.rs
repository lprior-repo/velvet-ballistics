#![forbid(unsafe_code)]

use crate::runtime::Runtime;
use crate::shard::{ShardCommand, ShardDirective};
use crate::{RuntimeError, RuntimeResult};

impl Runtime {
    /// Processes one command on each shard. Returns false if any shard is shutting down.
    pub fn tick_all(&mut self) -> RuntimeResult<bool> {
        self.shards
            .iter_mut()
            .try_fold(true, |alive, shard| shard.tick().map(|next| alive && next))
    }

    /// Processes one tick on a specific shard with the given directive.
    pub fn tick_shard(
        &mut self,
        shard_index: u32,
        directive: ShardDirective,
    ) -> RuntimeResult<bool> {
        let source_idx = self.validate_shard_index(shard_index)?;
        match directive {
            ShardDirective::Continue => self.tick_existing_shard(source_idx, shard_index),
            ShardDirective::Suspend => Ok(true),
            ShardDirective::Migrate { target } => self.migrate_shard(source_idx, target),
            ShardDirective::Shutdown => self.shutdown_existing_shard(source_idx, shard_index),
            ShardDirective::Cancel => unsupported_tick_directive("tick_shard_cancel"),
            ShardDirective::Barrier => unsupported_tick_directive("tick_shard_barrier"),
        }
    }

    /// Shuts down all shards gracefully.
    pub fn shutdown_graceful(&mut self) -> RuntimeResult<()> {
        self.shards
            .iter_mut()
            .try_for_each(|shard| shard.drain_pending_and_shutdown())?;
        self.journal.drain_for_shutdown()?;
        Ok(())
    }

    fn validate_shard_index(&self, shard_index: u32) -> RuntimeResult<usize> {
        let index = usize::try_from(shard_index)
            .map_err(|_| RuntimeError::ShardNotFound { shard: shard_index })?;
        self.shards
            .get(index)
            .map(|_| index)
            .ok_or(RuntimeError::ShardNotFound { shard: shard_index })
    }

    fn tick_existing_shard(&mut self, index: usize, shard_id: u32) -> RuntimeResult<bool> {
        self.shards
            .get_mut(index)
            .ok_or(RuntimeError::ShardNotFound { shard: shard_id })?
            .tick()
    }

    fn shutdown_existing_shard(&mut self, index: usize, shard_id: u32) -> RuntimeResult<bool> {
        self.shards
            .get_mut(index)
            .ok_or(RuntimeError::ShardNotFound { shard: shard_id })?
            .drain_pending_and_shutdown()?;
        Ok(false)
    }

    fn migrate_shard(&mut self, source_idx: usize, target: u32) -> RuntimeResult<bool> {
        let source = source_index_to_u32(source_idx, target)?;
        let target_idx = self.validate_migration_target(source, target)?;
        let commands = self.drain_source_commands(source_idx, source)?;
        self.enqueue_migrated_commands(target_idx, target, commands)?;
        self.source_shard_alive(source_idx, source)
    }

    fn validate_migration_target(&self, source: u32, target: u32) -> RuntimeResult<usize> {
        if target == source {
            return Err(RuntimeError::MigrateSelf);
        }
        self.validate_shard_index(target)
    }

    fn drain_source_commands(
        &mut self,
        source_idx: usize,
        source: u32,
    ) -> RuntimeResult<Vec<ShardCommand>> {
        let shard = self
            .shards
            .get_mut(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source })?;
        Ok(std::iter::from_fn(|| shard.command_queue.pop()).collect())
    }

    fn enqueue_migrated_commands(
        &mut self,
        target_idx: usize,
        target: u32,
        commands: Vec<ShardCommand>,
    ) -> RuntimeResult<()> {
        let target_shard = self
            .shards
            .get_mut(target_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: target })?;
        commands
            .into_iter()
            .try_for_each(|command| target_shard.enqueue(command))
    }

    fn source_shard_alive(&self, source_idx: usize, source: u32) -> RuntimeResult<bool> {
        let shard = self
            .shards
            .get(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source })?;
        Ok(shard.active_run_count() > 0 || !shard.command_queue.is_empty())
    }
}

fn source_index_to_u32(source_idx: usize, target: u32) -> RuntimeResult<u32> {
    u32::try_from(source_idx).map_err(|_| RuntimeError::ShardNotFound { shard: target })
}

fn unsupported_tick_directive(operation: &'static str) -> RuntimeResult<bool> {
    Err(RuntimeError::UnsupportedOperation { operation })
}
