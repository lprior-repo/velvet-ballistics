/// Maximum bounded command queue capacity per shard.
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;

/// Returns true when a command queue capacity is inside the supported domain.
#[must_use]
pub const fn is_valid_command_queue_capacity(capacity: usize) -> bool {
    capacity > 0 && capacity <= MAX_COMMAND_QUEUE_CAPACITY
}

// ============================================================================
// ShardCommandQueue — domain wrapper around ArrayQueue<ShardCommand>
// ============================================================================

/// Domain-named wrapper around `crossbeam_queue::ArrayQueue<ShardCommand>`.
///
/// Provides a bounded command queue with domain-specific terminology
/// (`enqueue`, `pop`, `is_full`, `remaining_capacity`) and proper error taxonomy
/// (`RuntimeError::QueueFull`). This wrapper establishes the `ShardCommand` queue
/// as a first-class domain boundary rather than a raw field.
pub struct ShardCommandQueue {
    inner: ArrayQueue<QueuedShardCommand>,
    migration: QueueMigrationState,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
    /// Pending command count per run, keyed by the run derived from each
    /// command. The shard keeps one queue; this map is accounting only.
    /// Entries are removed when their count reaches zero, so the map is
    /// bounded by the number of distinct runs with pending commands.
    pending: std::sync::Mutex<IndexMap<RunId, u32>>,
    /// Tightest known per-run queue depth retained from pending commands,
    /// keyed by run. It applies until that run's pending count reaches zero.
    /// Entries are removed when the run's pending count reaches zero, so
    /// the map is bounded by the number of distinct runs with pending
    /// commands.
    admitted_depth: std::sync::Mutex<IndexMap<RunId, u32>>,
    /// Pending submit count per run; only the first pending submit carries a
    /// quota reservation, preserving duplicate-run failure at dequeue time.
    pending_submissions: std::sync::Mutex<IndexMap<RunId, u32>>,
}

impl ShardCommandQueue {
    /// Creates a new `ShardCommandQueue` with the given capacity.
    ///
    /// # Errors
    /// Returns `RuntimeError::CommandQueueCapacityExceeded` if `capacity` is 0
    /// or exceeds `MAX_COMMAND_QUEUE_CAPACITY`.
    pub fn new(capacity: usize) -> RuntimeResult<Self> {
        if !is_valid_command_queue_capacity(capacity) {
            return Err(crate::RuntimeError::CommandQueueCapacityExceeded {
                capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        Ok(Self {
            inner: ArrayQueue::new(capacity),
            migration: QueueMigrationState::try_new(capacity)?,
            capacity,
            pending: std::sync::Mutex::new(migration_index_buffer(capacity)?),
            admitted_depth: std::sync::Mutex::new(migration_index_buffer(capacity)?),
            pending_submissions: std::sync::Mutex::new(migration_index_buffer(capacity)?),
        })
    }

    /// Creates a command queue after the owning shard constructor validates capacity.
    ///
    /// This helper is private to the shard construction path; callers validate the
    /// configured capacity before reaching this allocation.
    pub(crate) fn from_config(config: ShardConfig) -> RuntimeResult<Self> {
        Self::new(config.command_queue_capacity)
    }

    fn lock_or_recover<T>(
        pending: &std::sync::Mutex<IndexMap<RunId, T>>,
    ) -> std::sync::MutexGuard<'_, IndexMap<RunId, T>> {
        match pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Enqueues a command. Returns `Ok(())` if the command was enqueued, or
    /// `Err(RuntimeError::QueueFull)` if the queue is at capacity.
    ///
    /// Index accounting is mutex-serialized; this operation may wait for a lock.
    /// A full queue returns an error without waiting for a free slot.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.enqueue_with_limit(cmd, None)
    }

    /// Enqueues a command, enforcing a per-run pending cap when one is given.
    ///
    /// The cap check and the count increment are held under one lock with
    /// the push, so concurrent enqueues cannot jointly exceed the cap. A
    /// rejected command is never pushed and never counted; a command that
    /// fails on shard-wide capacity is likewise never counted. When no cap
    /// is given, the recorded bound for the same pending run applies; every
    /// successful push records the tightest supplied bound.
    ///
    /// # Errors
    /// Returns `RuntimeError::PerRunQueueDepthExceeded` when the command's
    /// run already has `limit` commands pending, or
    /// `RuntimeError::QueueFull` when the queue is at capacity.
    pub fn enqueue_with_limit(&self, cmd: ShardCommand, limit: Option<u32>) -> RuntimeResult<()> {
        self.enqueue_with_reservation(cmd, limit, None, None)
    }

    /// Returns the number of commands currently pending for a run.
    #[must_use]
    pub fn pending_for(&self, run: RunId) -> u32 {
        Self::lock_or_recover(&self.pending)
            .get(&run)
            .copied()
            .unwrap_or(0)
    }

    /// Dequeues the frontmost command, if any.
    ///
    /// Returns `Some(cmd)` in FIFO order, or `None` if the queue is empty.
    /// A dequeued command decrements its run's pending count under the same
    /// index locks used by enqueue. This operation may wait for those locks.
    pub fn pop(&self) -> Option<ShardCommand> {
        self.pop_queued().map(|queued| queued.command)
    }

    pub(crate) fn pop_queued(&self) -> Option<QueuedShardCommand> {
        self.require_unfenced().ok()?;
        self.pop_unfenced_queued()
    }

    fn pop_unfenced_queued(&self) -> Option<QueuedShardCommand> {
        self.pop_unfenced_queued_observed(|| {})
    }

    fn pop_unfenced_queued_observed(
        &self,
        after_raw_pop: impl FnOnce(),
    ) -> Option<QueuedShardCommand> {
        // Match enqueue lock order before freeing a raw slot. Otherwise a
        // refill can insert a new run while the removed run is still indexed.
        let mut indexes = self.lock_indexes();
        let queued = self.inner.pop()?;
        after_raw_pop();
        indexes.remove_command(&queued.command);
        Some(queued)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.migration.fenced {
            self.migration
                .original_len
                .max(self.inner.len().saturating_add(self.migration.stage.len()))
        } else {
            self.inner.len()
        }
    }

    /// Returns `true` if the queue contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of this queue (set at construction).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of remaining free slots in the queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity
    }

    /// Returns the compile-time bounded capacity limit (65536).
    ///
    /// This is the maximum capacity any `ShardCommandQueue` can be configured with.
    #[must_use]
    pub const fn bounded_capacity() -> usize {
        MAX_COMMAND_QUEUE_CAPACITY
    }
}

fn pending_submit_run(command: &ShardCommand) -> Option<RunId> {
    match command {
        ShardCommand::Submit { run, .. }
        | ShardCommand::SubmitWithLifetimeCap { run, .. }
        | ShardCommand::SubmitTenantScoped { run, .. }
        | ShardCommand::SubmitTenantWithInputsAndContracts { run, .. }
        | ShardCommand::SubmitPrePersisted { run, .. }
        | ShardCommand::SubmitWithInputs { run, .. }
        | ShardCommand::SubmitWithContracts { run, .. }
        | ShardCommand::SubmitWithInputsAndContracts { run, .. } => Some(*run),
        ShardCommand::Recover { command }
            if matches!(
                &command.admission,
                RecoveredAdmissionKind::Historical(admission) if admission.budget().is_some()
            ) =>
        {
            Some(command.run)
        }
        _ => None,
    }
}

#[cfg(test)]
include!("migration/pop_critical_section_tests.rs");

#[cfg(test)]
include!("migration/pop_submission_index_tests.rs");
