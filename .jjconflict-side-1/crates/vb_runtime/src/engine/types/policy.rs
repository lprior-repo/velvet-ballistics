/// Retry policy for action invocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts before giving up.
    pub max_attempts: u16,
    /// Base delay between attempts in milliseconds.
    pub base_delay_ms: u64,
    /// Whether to use exponential backoff.
    pub exponential_backoff: bool,
}

impl RetryPolicy {
    /// Policy that never retries.
    pub const NEVER: Self = Self {
        max_attempts: 1,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    /// Policy with up to 3 attempts and no backoff.
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: false,
    };
}

/// Extended engine signal returned by the action-aware execution loop.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeSignal {
    /// Deterministic execution can continue.
    Continue,
    /// Run finished with a result value.
    Finished(SlotValue),
    /// Step budget was exhausted before completion.
    StepBudgetExhausted,
    /// Run is awaiting action completion with the given ticket.
    AwaitingAction(ActionTicket),
    /// Run is awaiting a wait condition.
    AwaitingWait,
    /// Run is awaiting external input (ask).
    AwaitingAsk,
    /// Core engine produced a signal variant not handled by this runtime.
    ///
    /// `EngineSignal` is `#[non_exhaustive]` so a future vb_core revision
    /// can add variants without breaking this crate's compile. The
    /// wildcard arm of `runtime_from_core` catches those variants and
    /// surfaces them as this typed marker instead of silently mapping
    /// to `Continue` (which would advance the drive loop and commit the
    /// step state to `Succeeded` — master §45 violation). Callers MUST
    /// handle this variant explicitly; the drive loop aborts with
    /// `RuntimeEngineError::UnknownEngineSignal`.
    UnknownEngineSignal {
        /// `Debug` rendering of the offending core signal for diagnostics.
        signal_debug: String,
    },
}
