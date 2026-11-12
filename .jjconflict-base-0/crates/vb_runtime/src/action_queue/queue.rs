impl BoundedActionCompletionQueue {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        Ok(Self {
            inner: std::sync::Mutex::new(Inner {
                items: VecDeque::with_capacity(capacity.get()),
            }),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with the given capacity, skipping capacity
    /// validation.
    ///
    /// Use this constructor ONLY when the caller has verified that
    /// `1 <= capacity <= MAX_ACTION_COMPLETION_QUEUE_CAPACITY`. Unlike
    /// [`BoundedActionCompletionQueue::new`], this constructor does NOT
    /// validate the input and is purely infallible (no Result, no panic).
    /// The naming convention `new_infallible` makes the trust boundary
    /// explicit at the call site.
    ///
    /// For runtime/derived capacity where the value is not statically known,
    /// use the fallible [`BoundedActionCompletionQueue::new`] and propagate
    /// the error.
    pub fn new_infallible(capacity: usize) -> Self {
        let capacity = ActionQueueCapacity::new_unchecked(capacity);
        Self {
            inner: std::sync::Mutex::new(Inner {
                items: VecDeque::with_capacity(capacity.get()),
            }),
            capacity,
            backpressure_tx: None,
        }
    }

    /// Creates a new bounded queue with backpressure notification channel,
    /// skipping capacity validation.
    ///
    /// Use this constructor ONLY when the caller has verified that
    /// `1 <= capacity <= MAX_ACTION_COMPLETION_QUEUE_CAPACITY`. Unlike
    /// [`BoundedActionCompletionQueue::with_backpressure`], this constructor
    /// does NOT validate the input and is purely infallible (no Result, no
    /// panic). The naming convention `with_backpressure_infallible` makes
    /// the trust boundary explicit at the call site.
    ///
    /// Returns the queue together with the receiving end of the backpressure
    /// notification channel. The sender is owned by the queue.
    ///
    /// For runtime/derived capacity where the value is not statically known,
    /// use the fallible [`BoundedActionCompletionQueue::with_backpressure`]
    /// and propagate the error.
    pub fn with_backpressure_infallible(capacity: usize) -> (Self, Receiver<BackpressureWarning>) {
        let capacity = ActionQueueCapacity::new_unchecked(capacity);
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity.get());
        (
            Self {
                inner: std::sync::Mutex::new(Inner {
                    items: VecDeque::with_capacity(capacity.get()),
                }),
                capacity,
                backpressure_tx: Some(tx),
            },
            rx,
        )
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, Receiver<BackpressureWarning>), ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity.get());
        Ok((
            Self {
                inner: std::sync::Mutex::new(Inner {
                    items: VecDeque::with_capacity(capacity.get()),
                }),
                capacity,
                backpressure_tx: Some(tx),
            },
            rx,
        ))
    }

    /// Attempts to enqueue an action ticket.
    ///
    /// Returns `Ok(())` if the item was enqueued.
    /// Returns `Err(ActionQueueError::QueueFull)` if the queue is at capacity.
    ///
    /// Emits a backpressure warning if the queue reaches or exceeds 80% capacity
    /// after the enqueue.
    pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError> {
        self.enqueue_with_backpressure_warning_outcome(ticket).0
    }

    /// Attempts to enqueue an action ticket, surfacing the backpressure
    /// warning send outcome to the caller.
    ///
    /// The first element is exactly the [`Self::enqueue`] result. The second
    /// reports whether the backpressure warning was delivered, not sent
    /// (below threshold, no channel configured, or enqueue failed), or
    /// dropped because the channel was full or disconnected.
    pub fn enqueue_with_backpressure_warning_outcome(
        &self,
        ticket: ActionTicket,
    ) -> (Result<(), ActionQueueError>, BackpressureWarningOutcome) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.items.len() >= self.capacity.get() {
            return (
                Err(ActionQueueError::QueueFull {
                    capacity: self.capacity,
                }),
                BackpressureWarningOutcome::NotSent,
            );
        }

        inner.items.push_back(ticket);

        let depth = inner.items.len();
        let threshold = backpressure_threshold(self.capacity);
        let outcome = if depth >= threshold {
            match self.backpressure_tx {
                Some(ref tx) => match tx.try_send(BackpressureWarning {
                    depth,
                    capacity: self.capacity.get(),
                }) {
                    Ok(()) => BackpressureWarningOutcome::Delivered,
                    Err(TrySendError::Full(_)) => BackpressureWarningOutcome::ChannelFull,
                    Err(TrySendError::Disconnected(_)) => {
                        BackpressureWarningOutcome::ChannelDisconnected
                    }
                },
                None => BackpressureWarningOutcome::NotSent,
            }
        } else {
            BackpressureWarningOutcome::NotSent
        };

        (Ok(()), outcome)
    }

    /// Dequeues an action ticket in FIFO order.
    ///
    /// Returns `Some(ticket)` if an item was available.
    /// Returns `None` if the queue is empty.
    #[must_use]
    pub fn dequeue(&self) -> Option<ActionTicket> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.items.pop_front()
    }

    /// Returns the number of items currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        match self.inner.lock() {
            Ok(guard) => guard.items.len(),
            Err(poisoned) => poisoned.into_inner().items.len(),
        }
    }

    /// Returns `true` if the queue contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity.get()
    }

    /// Returns the number of slots available for new items.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.get().saturating_sub(self.len())
    }

    /// Returns the fixed capacity of this queue.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }
}

fn parse_capacity(capacity: usize) -> Result<ActionQueueCapacity, ActionQueueError> {
    if capacity == 0 {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: InvalidActionQueueCapacity::Zero,
        });
    }

    if capacity > MAX_ACTION_COMPLETION_QUEUE_CAPACITY {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: InvalidActionQueueCapacity::AboveMaximum {
                maximum: MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
            },
        });
    }

    Ok(ActionQueueCapacity(capacity))
}

/// Returns the backpressure threshold for a given capacity.
///
/// The threshold is the smallest integer `>= capacity * 8 / 10` (ceiling of 80%),
/// with a minimum of 1 so even a capacity-1 queue still emits a warning at depth 1.
/// Uses checked arithmetic; if `capacity * 8` would overflow `usize`, the function
/// falls back to `capacity` itself (which is already > the would-be threshold
/// in any realistic capacity that could overflow).
fn backpressure_threshold(capacity: ActionQueueCapacity) -> usize {
    let raw = capacity.get();
    // INVARIANT: ceiling division `ceil(raw * 8 / 10) = (raw * 8 + 9) / 10` is
    // computed via checked arithmetic so that capacity values near `usize::MAX / 8`
    // cannot wrap. On overflow we return `raw` (the original capacity) which is
    // always >= any valid threshold for inputs the queue can hold.
    let threshold = usize::checked_mul(raw, 8)
        .and_then(|scaled| usize::checked_add(scaled, 9))
        .and_then(|numerator| usize::checked_div(numerator, 10));
    match threshold {
        Some(value) => value.max(1),
        None => raw,
    }
}
