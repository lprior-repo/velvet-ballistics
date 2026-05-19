#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_core::value::SlotValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelRuntimeError {
    InvalidActionCompletion,
    RunNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreMode {
    Missing,
    AlwaysPresent,
    StorageBackedAccepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShardKernelState {
    active_run: Option<RunId>,
    runtime_policy: RuntimePolicy,
    store_mode: StoreMode,
    queue_depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AskKernelFrame {
    prompt: SlotValue,
    timeout: SlotValue,
    executed: u64,
}

impl StoreMode {
    fn selected(selector: u8) -> Self {
        match selector % 3 {
            0 => Self::Missing,
            1 => Self::AlwaysPresent,
            _ => Self::StorageBackedAccepted,
        }
    }
}

impl AskKernelFrame {
    fn new(prompt: SlotValue, timeout: SlotValue) -> Self {
        Self {
            prompt,
            timeout,
            executed: 0,
        }
    }

    fn ask(&mut self) -> Result<(), KernelAskError> {
        if matches!(self.prompt, SlotValue::Bool(_)) {
            return Err(KernelAskError::TypeMismatchPrompt);
        }
        if !matches!(self.timeout, SlotValue::I64(_) | SlotValue::F64(_)) {
            return Err(KernelAskError::TypeMismatchTimeout);
        }
        self.executed = self.executed.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelAskError {
    TypeMismatchPrompt,
    TypeMismatchTimeout,
}

impl ShardKernelState {
    fn explicit(policy: RuntimePolicy, store_mode: StoreMode) -> Self {
        Self {
            active_run: None,
            runtime_policy: policy,
            store_mode,
            queue_depth: 0,
        }
    }

    fn runtime_constructed(policy: RuntimePolicy) -> Self {
        let store_mode = match policy {
            RuntimePolicy::Relaxed => StoreMode::AlwaysPresent,
            RuntimePolicy::Strict | RuntimePolicy::Journaled => StoreMode::Missing,
            _ => StoreMode::Missing,
        };
        Self::explicit(policy, store_mode)
    }

    fn with_active_run(policy: RuntimePolicy, run: RunId) -> Self {
        Self {
            active_run: Some(run),
            runtime_policy: policy,
            store_mode: StoreMode::AlwaysPresent,
            queue_depth: 0,
        }
    }

    fn action_failed_lower(&self, ticket_run: RunId) -> Result<(), KernelRuntimeError> {
        if self.active_run == Some(ticket_run) {
            Err(KernelRuntimeError::InvalidActionCompletion)
        } else {
            Err(KernelRuntimeError::RunNotFound)
        }
    }

    fn runtime_action_failed(&self, ticket_run: RunId) -> Result<(), KernelRuntimeError> {
        self.action_failed_lower(ticket_run)
            .map_err(Self::runtime_action_failure_error)
    }

    fn runtime_action_failure_error(error: KernelRuntimeError) -> KernelRuntimeError {
        match error {
            KernelRuntimeError::RunNotFound => KernelRuntimeError::InvalidActionCompletion,
            other => other,
        }
    }
}

fn policy(selector: u8) -> RuntimePolicy {
    match selector % 3 {
        0 => RuntimePolicy::Relaxed,
        1 => RuntimePolicy::Strict,
        _ => RuntimePolicy::Journaled,
    }
}

fn prompt_value(selector: u8) -> SlotValue {
    match selector % 4 {
        0 => SlotValue::Bool(true),
        1 => SlotValue::I64(7),
        2 => SlotValue::Null,
        _ => SlotValue::Bool(false),
    }
}

#[kani::proof]
fn vt2f_shard_lower_semantics() {
    let selector: u8 = kani::any();
    let run = RunId::new(10 + u64::from(selector % 5));
    let other = RunId::new(80 + u64::from(selector % 7));
    let selected_policy = policy(selector);
    let selected_store = StoreMode::selected(selector);

    kani::cover!(
        selected_policy == RuntimePolicy::Relaxed,
        "relaxed policy covered"
    );
    kani::cover!(
        selected_policy == RuntimePolicy::Strict,
        "strict policy covered"
    );
    kani::cover!(
        selected_policy == RuntimePolicy::Journaled,
        "journaled policy covered"
    );
    kani::cover!(
        selected_store == StoreMode::Missing,
        "missing store covered"
    );
    kani::cover!(
        selected_store == StoreMode::AlwaysPresent,
        "always-present store covered"
    );
    kani::cover!(
        selected_store == StoreMode::StorageBackedAccepted,
        "storage-backed accepted store covered"
    );

    let absent_lower = ShardKernelState::explicit(RuntimePolicy::Relaxed, StoreMode::AlwaysPresent);
    assert!(matches!(
        absent_lower.action_failed_lower(run),
        Err(KernelRuntimeError::RunNotFound)
    ));

    let active_lower = ShardKernelState::with_active_run(RuntimePolicy::Relaxed, run);
    let ticket_run = if selector & 1 == 0 { run } else { other };
    let facade_result = active_lower.runtime_action_failed(ticket_run);
    assert!(matches!(
        facade_result,
        Err(KernelRuntimeError::InvalidActionCompletion)
    ));

    let explicit = ShardKernelState::explicit(selected_policy, selected_store);
    assert_eq!(explicit.runtime_policy, selected_policy);
    assert_eq!(explicit.store_mode, selected_store);
    assert_eq!(explicit.queue_depth, 0);

    let runtime_constructed = ShardKernelState::runtime_constructed(selected_policy);
    assert_eq!(runtime_constructed.runtime_policy, selected_policy);
    assert_eq!(runtime_constructed.queue_depth, 0);
    if selected_policy == RuntimePolicy::Relaxed {
        assert_eq!(runtime_constructed.store_mode, StoreMode::AlwaysPresent);
    } else {
        assert_eq!(runtime_constructed.store_mode, StoreMode::Missing);
    }

    let mut frame = AskKernelFrame::new(
        prompt_value(selector),
        SlotValue::I64(30 + i64::from(selector % 3)),
    );
    let executed_before = frame.executed;
    let ask_result = frame.ask();
    if matches!(prompt_value(selector), SlotValue::Bool(_)) {
        kani::cover!(true, "bool prompt rejection path covered");
        assert!(ask_result.is_err());
        assert_eq!(frame.executed, executed_before);
    } else {
        kani::cover!(true, "non-bool prompt resume path covered");
        assert!(ask_result.is_ok());
        assert_eq!(frame.executed, executed_before.saturating_add(1));
    }
}
