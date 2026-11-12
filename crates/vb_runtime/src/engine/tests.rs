#![forbid(unsafe_code)]

//! Engine tests moved from engine.rs for line count compliance.
#![allow(unused_imports)]

use vb_core::action::ActionName;
use vb_core::action::Idempotency;
use vb_core::action::RetrySafety;
use vb_core::action::SideEffect;
use vb_core::engine::EngineSignal;
use vb_core::errors::EngineError;
use vb_core::value::SlotValue;

use crate::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    compute_idempotency_key, drive_deterministic_full, execute_do, execute_do_without_contract,
    execute_error_handler, execute_retry_check, resolve_contract, resume_action_outcome,
};
use vb_core::action::ActionFailure;
use vb_core::action::ActionFailureCode;
use vb_core::action::ActionOutcome;
use vb_core::action::ActionTicket;
use vb_core::action::RetryPolicy as VbRetryPolicy;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledNodeKind;
use vb_core::workflow::CompiledWorkflow;

include!("tests/tests_retry.rs");
include!("tests/tests_idempotency_signal.rs");
include!("tests/tests_engine_error_budget.rs");
include!("tests/tests_execute.rs");
include!("tests/tests_resolve_drive.rs");
include!("tests/tests_blackhat.rs");
include!("tests/tests_proptest_blackhat_review.rs");
