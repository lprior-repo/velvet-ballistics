use crate::shard::types::custody_stage::{MigrationCustody, TransferProgress};

struct QueueMigrationState {
    fenced: bool,
    fully_staged: bool,
    original_len: usize,
    planned_source_len: Option<usize>,
    rollback: Option<MigrationRollback>,
    stage: MigrationCustody<QueuedShardCommand>,
    planned_pending: IndexMap<RunId, u32>,
    planned_depth: IndexMap<RunId, u32>,
    planned_submissions: IndexMap<RunId, u32>,
}

impl QueueMigrationState {
    fn try_new(capacity: usize) -> RuntimeResult<Self> {
        let stage = MigrationCustody::try_new(capacity, MAX_COMMAND_QUEUE_CAPACITY)
            .map_err(|_| migration_invariant("migration_staging_allocation"))?;
        Ok(Self {
            fenced: false,
            fully_staged: false,
            original_len: 0,
            planned_source_len: None,
            rollback: None,
            stage,
            planned_pending: migration_index_buffer(capacity)?,
            planned_depth: migration_index_buffer(capacity)?,
            planned_submissions: migration_index_buffer(capacity)?,
        })
    }
}

fn migration_index_buffer(capacity: usize) -> RuntimeResult<IndexMap<RunId, u32>> {
    let mut entries = IndexMap::new();
    entries
        .try_reserve(capacity)
        .map_err(|_| migration_invariant("migration_index_allocation"))?;
    Ok(entries)
}

fn migration_invariant(operation: &'static str) -> crate::RuntimeError {
    crate::RuntimeError::UnsupportedOperation { operation }
}

impl ShardCommandQueue {
    pub(crate) fn is_migration_fenced(&self) -> bool {
        self.migration.fenced
    }

    pub(crate) fn require_unfenced(&self) -> RuntimeResult<()> {
        if self.migration.fenced {
            return Err(migration_invariant("migration_queue_fenced"));
        }
        Ok(())
    }

    pub(crate) fn fence_migration(&mut self) -> RuntimeResult<()> {
        self.require_unfenced()?;
        if self.migration.stage.len() != 0
            || self.migration.rollback.is_some()
            || self.migration.planned_source_len.is_some()
        {
            return Err(migration_invariant("migration_staging_not_empty"));
        }
        self.migration.fully_staged = false;
        self.migration.original_len = self.inner.len();
        self.migration.fenced = true;
        Ok(())
    }

    pub(crate) fn stage_migration_source(&mut self) -> RuntimeResult<()> {
        self.fence_migration()?;
        for _ in 0..self.migration.original_len {
            let result = self.migration.stage.fill_one(|| self.inner.pop());
            if result != TransferProgress::Accepted {
                return Err(migration_invariant("migration_staging_invariant"));
            }
        }
        self.migration.fully_staged = true;
        Ok(())
    }

    pub(crate) fn restore_migration_source(&mut self) -> RuntimeResult<()> {
        if !self.migration.fenced || !self.migration.fully_staged {
            return Err(migration_invariant(
                "migration_restore_without_complete_staging",
            ));
        }
        for _ in 0..self.capacity {
            match self
                .migration
                .stage
                .transfer_front(|entry| self.inner.push(entry).err())
            {
                TransferProgress::Accepted => {}
                TransferProgress::Empty => break,
                TransferProgress::Refused => {
                    return Err(migration_invariant("migration_restore_refused"));
                }
            }
        }
        self.finish_migration_restoration()
    }

    fn finish_migration_restoration(&mut self) -> RuntimeResult<()> {
        if self.migration.stage.len() != 0 || self.inner.len() != self.migration.original_len {
            return Err(migration_invariant("migration_restore_length_mismatch"));
        }
        self.migration.fenced = false;
        Ok(())
    }

    pub(crate) fn migration_entries(&self) -> impl Iterator<Item = &QueuedShardCommand> {
        self.migration.stage.entries()
    }
}

include!("migration/queue_plan.rs");
include!("migration/queue_commit.rs");
include!("migration/queue_rollback.rs");

include!("migration/queue_index_validation.rs");
