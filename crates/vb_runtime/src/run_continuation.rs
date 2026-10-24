#![forbid(unsafe_code)]
//! Content-bound non-retry continuation at two quiescent Collect completion cuts.
//!
//! Callers must independently establish the committed receipt/prefix and attempt.
//! This component never authorizes public recovery or registered live effects.

mod transport;
pub use transport::{
    BoundedContinuationPayload, ContinuationTransportError, ContinuationTransportLimits,
    MAX_CONTINUATION_COMPONENT_BYTES, TransportFailure,
};

mod authority;
mod boundary;
mod error;
mod ownership;
mod payload;
mod validation;
mod wire;

pub use authority::{ContinuationAuthority, ContinuationExpected};
pub use error::RunContinuationError;
pub use payload::ContinuationCut;

use crate::primitives::collect::CollectStates;
use crate::shard::types::{RegisteredExecutionMode, RunState};
use payload::{Body, Wire};
use vb_core::{RunFrame, ValueStore, WorkflowDigest};

/// Immutable complete continuation; decoding alone cannot expose a RunState.
#[derive(Debug)]
pub struct RunContinuationV1 {
    data: Wire,
}

impl RunContinuationV1 {
    /// Captures actual state without changing its PC, step states or attempts.
    pub fn capture(
        state: &RunState,
        authority: &ContinuationAuthority,
    ) -> Result<Self, RunContinuationError> {
        if state.workflow != authority.workflow {
            return Err(RunContinuationError::WorkflowIdentity);
        }
        if state.run_attempt != authority.expected.run_attempt {
            return Err(RunContinuationError::AttemptIdentity);
        }
        let admission = state
            .admission
            .clone()
            .ok_or(RunContinuationError::Admission)?;
        let frame = state.frame.checkpoint_v1();
        validation::quiescent(&frame)?;
        let boundary = boundary::locate(&frame, authority)?;
        let body = Body::capture(state, authority, admission, frame, boundary);
        let digest = body.digest()?;
        let result = Self {
            data: Wire { body, digest },
        };
        result.validate_against(authority)?;
        Ok(result)
    }

    /// Validates the complete payload against independently accepted content authority.
    pub fn validate_against(
        &self,
        authority: &ContinuationAuthority,
    ) -> Result<(), RunContinuationError> {
        self.data.validate_integrity()?;
        validation::validate(&self.data.body, authority)
    }

    /// Restores exact mutable state and requires reconciliation of registered effects.
    pub fn restore(
        self,
        authority: ContinuationAuthority,
    ) -> Result<RunState, RunContinuationError> {
        self.validate_against(&authority)?;
        let body = self.data.body;
        let frame =
            RunFrame::from_checkpoint_v1(body.frame).map_err(RunContinuationError::Frame)?;
        let store = ValueStore::from_checkpoint_v1(body.value_store)?;
        let collect_states = CollectStates::from_checkpoint_v1(body.collect_states)
            .map_err(RunContinuationError::CollectCheckpoint)?;
        Ok(RunState {
            registered_execution: RegisteredExecutionMode::ReconcileOnly,
            frame,
            run_attempt: body.expected.run_attempt,
            workflow: authority.workflow,
            store,
            action_attempts: body.action_attempts,
            admission: Some(body.admission),
            collect_states,
            action_contracts: body.action_contracts,
        })
    }

    /// Exact continuation content digest, excluding the digest field itself.
    pub const fn digest(&self) -> WorkflowDigest {
        self.data.digest
    }

    /// The compiled quiescent boundary validated during capture or decoding.
    pub const fn cut(&self) -> ContinuationCut {
        self.data.body.boundary.cut
    }
}
