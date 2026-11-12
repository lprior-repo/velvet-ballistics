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
/// Provides a bounded, non-blocking command queue with domain-specific terminology
/// (`enqueue`, `pop`, `is_full`, `remaining_capacity`) and proper error taxonomy
/// (`RuntimeError::QueueFull`). This wrapper establishes the `ShardCommand` queue
/// as a first-class domain boundary rather than a raw field.
pub struct ShardCommandQueue {
    inner: ArrayQueue<ShardCommand>,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
    /// Pending command count per run, keyed by the run derived from each
    /// command. The shard keeps one queue; this map is accounting only.
    /// Entries are removed when their count reaches zero, so the map is
    /// bounded by the number of distinct runs with pending commands.
    pending: std::sync::Mutex<IndexMap<RunId, u32>>,
    /// Admitted per-run queue depth recorded from a pending submit, keyed
    /// by run. A submit's workflow budget is the cap for commands that
    /// arrive before the run has an attached admission in run state.
    /// Entries are removed when the run's pending count reaches zero, so
    /// the map is bounded by the number of distinct runs with pending
    /// commands.
    admitted_depth: std::sync::Mutex<IndexMap<RunId, u32>>,
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
            capacity,
            pending: std::sync::Mutex::new(IndexMap::new()),
            admitted_depth: std::sync::Mutex::new(IndexMap::new()),
        })
    }

    /// Creates a command queue from an already-accepted shard configuration.
    ///
    /// `Shard::new` has historically been infallible and accepted `ShardConfig`
    /// by value. The validated constructor for externally supplied capacity is
    /// `ShardConfig::new`; this helper preserves `Shard::new`'s existing shape
    /// while placing the raw queue construction behind the domain wrapper.
    pub(crate) fn from_config(config: ShardConfig) -> Self {
        Self {
            inner: ArrayQueue::new(config.command_queue_capacity),
            capacity: config.command_queue_capacity,
            pending: std::sync::Mutex::new(IndexMap::new()),
            admitted_depth: std::sync::Mutex::new(IndexMap::new()),
        }
    }

    fn lock_or_recover(
        pending: &std::sync::Mutex<IndexMap<RunId, u32>>,
    ) -> std::sync::MutexGuard<'_, IndexMap<RunId, u32>> {
        match pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Enqueues a command. Returns `Ok(())` if the command was enqueued, or
    /// `Err(RuntimeError::QueueFull)` if the queue is at capacity.
    ///
    /// This operation is non-blocking and never allocates on failure.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.enqueue_with_limit(cmd, None)
    }

    /// Enqueues a command, enforcing a per-run pending cap when one is given.
    ///
    /// The cap check and the count increment are held under one lock with
    /// the push, so concurrent enqueues cannot jointly exceed the cap. A
    /// rejected command is never pushed and never counted; a command that
    /// fails on shard-wide capacity is likewise never counted. When no cap
    /// is given, a cap recorded from a pending submit for the same run
    /// applies; a submit records its workflow budget on a successful push.
    ///
    /// # Errors
    /// Returns `RuntimeError::PerRunQueueDepthExceeded` when the command's
    /// run already has `limit` commands pending, or
    /// `RuntimeError::QueueFull` when the queue is at capacity.
    pub fn enqueue_with_limit(&self, cmd: ShardCommand, limit: Option<u32>) -> RuntimeResult<()> {
        let run = cmd.run_id();
        let submit_depth = match &cmd {
            ShardCommand::Submit { run, workflow, .. }
            | ShardCommand::SubmitPrePersisted { run, workflow, .. }
            | ShardCommand::SubmitWithInputs { run, workflow, .. }
            | ShardCommand::SubmitWithContracts { run, workflow, .. }
            | ShardCommand::SubmitWithInputsAndContracts { run, workflow, .. } => {
                vb_core::budget::AggregateResourceBudget::from_workflow(workflow)
                    .ok()
                    .map(|budget| (*run, budget.max_queue_depth))
            }
            _ => None,
        };
        let mut pending = Self::lock_or_recover(&self.pending);
        let mut admitted = Self::lock_or_recover(&self.admitted_depth);
        let recorded = run.and_then(|run| admitted.get(&run).copied());
        let effective = limit
            .or(recorded)
            .or_else(|| submit_depth.map(|(_, depth)| depth));
        let current = run.and_then(|run| pending.get(&run).copied()).unwrap_or(0);
        if let (Some(run), Some(limit)) = (run, effective)
            && current >= limit
        {
            return Err(crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: current,
                limit,
            });
        }
        let next = match run {
            Some(run) => Some((
                run,
                current
                    .checked_add(1)
                    .ok_or(crate::RuntimeError::PerRunQueueDepthExceeded {
                        run,
                        pending: current,
                        limit: current,
                    })?,
            )),
            None => None,
        };
        match self.inner.push(cmd) {
            Ok(()) => {
                if let Some((run, next)) = next {
                    let _previous = pending.insert(run, next);
                }
                if let Some((run, depth)) = submit_depth {
                    let _previous = admitted.insert(run, depth);
                }
                Ok(())
            }
            Err(_) => Err(crate::RuntimeError::QueueFull),
        }
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
    /// A dequeued command decrements its run's pending count.
    pub fn pop(&self) -> Option<ShardCommand> {
        let cmd = self.inner.pop()?;
        if let Some(run) = cmd.run_id() {
            let mut pending = Self::lock_or_recover(&self.pending);
            match pending.get(&run).copied() {
                Some(1) => {
                    let _removed = pending.swap_remove(&run);
                    let mut admitted = Self::lock_or_recover(&self.admitted_depth);
                    let _removed = admitted.swap_remove(&run);
                }
                Some(count) => {
                    if let Some(next) = count.checked_sub(1) {
                        let _previous = pending.insert(run, next);
                    }
                }
                None => {}
            }
        }
        Some(cmd)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the queue contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the capacity of this queue (set at construction).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of remaining free slots in the queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.inner.len())
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity
    }

    /// Returns the compile-time bounded capacity limit (65536).
    ///
    /// This is the maximum capacity any `ShardCommandQueue` can be configured with.
    #[must_use]
    pub const fn bounded_capacity() -> usize {
        MAX_COMMAND_QUEUE_CAPACITY
    }
}
