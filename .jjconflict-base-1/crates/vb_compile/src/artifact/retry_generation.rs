//! Version-one planning attachment; never admission or execution authority.
//!
//! Wire: `01 ff VBRTRY`, generation u16 LE (1), nonzero count u16 LE,
//! exact checked payload digest (32 bytes), checked contracts digest (32 bytes),
//! then count records of owner/action/inclusive-total u16 LE (six bytes).
//! The 76-byte header and exact length are checked before allocation. Owners
//! are sorted and unique. Totals include the initial attempt and range 1..=10;
//! this new planning cap does not reinterpret legacy RetryCheck. Context digests
//! reuse VerifiedCompilation's exact payload/ordered-contract byte bindings.
//! Publicly constructible policy data is neither authenticated nor authorized.
//! Only a linear forward checked workflow is supported; no runtime conversion
//! or YAML activation exists. The prefix is invalid legacy WorkflowParts UTF-8.

use super::VerifiedCompilation;
use std::num::NonZeroU16;
use vb_core::workflow::ErrorRouting;
use vb_core::{ActionId, CompiledNodeKind, StepIdx};

const MAGIC: &[u8; 8] = b"\x01\xffVBRTRY";
const GENERATION: u16 = 1;
const HEADER_BYTES: usize = 76;
const RECORD_BYTES: usize = 6;

/// Refusal of malformed, unsupported, or incorrectly bound planning data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetryPlanningError {
    /// Missing or unknown namespace; no legacy fallback exists.
    #[error("invalid retry planning magic")]
    Magic,
    /// Unsupported generation.
    #[error("unsupported retry planning generation {0}")]
    Generation(u16),
    /// Truncated, trailing, or overflowing encoded length.
    #[error("invalid retry planning length")]
    Length,
    /// Empty or context-exceeding policy count.
    #[error("invalid retry planning count")]
    Count,
    /// Payload or ordered-contract binding differs from checked context.
    #[error("retry planning context mismatch")]
    Binding,
    /// Workflow is outside the supported forward linear profile.
    #[error("unsupported retry planning workflow profile")]
    Profile,
    /// Owners are duplicated or not strictly sorted.
    #[error("retry planning owners are not canonical")]
    Owners,
    /// Owner is absent, is not Do, or names a different action.
    #[error("retry planning owner/action mismatch at {0:?}")]
    Owner(StepIdx),
    /// Inclusive total is outside the new planning range 1..=10.
    #[error("unsupported retry planning total {0}")]
    TotalAttempts(u16),
    /// Bounded allocation could not be reserved.
    #[error("retry planning allocation failed")]
    Allocation,
}

/// Immutable per-Do planning policy; carries no retry execution authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicyRecord {
    owner: StepIdx,
    action: ActionId,
    total_attempts: NonZeroU16,
}
impl RetryPolicyRecord {
    /// Construct a policy whose total includes the initial attempt (1..=10).
    pub fn new(owner: StepIdx, action: ActionId, total: u16) -> Result<Self, RetryPlanningError> {
        let total_attempts = NonZeroU16::new(total)
            .filter(|total| total.get() <= 10)
            .ok_or(RetryPlanningError::TotalAttempts(total))?;
        Ok(Self {
            owner,
            action,
            total_attempts,
        })
    }
    /// Numeric owning Do step.
    #[must_use]
    pub const fn owner(&self) -> StepIdx {
        self.owner
    }
    /// Exact action declared by the owning Do.
    #[must_use]
    pub const fn action(&self) -> ActionId {
        self.action
    }
    /// Inclusive total, including the initial attempt.
    #[must_use]
    pub const fn total_attempts(&self) -> u16 {
        self.total_attempts.get()
    }
}

/// Validated immutable planning view tied to independently checked compilation.
#[derive(Debug)]
pub struct RetryPlanningAttachment<'a> {
    compilation: &'a VerifiedCompilation,
    policies: Vec<RetryPolicyRecord>,
}
impl<'a> RetryPlanningAttachment<'a> {
    /// Independently checked context; this view adds no execution authority.
    #[must_use]
    pub const fn compilation(&self) -> &'a VerifiedCompilation {
        self.compilation
    }
    /// Canonical per-Do policies.
    #[must_use]
    pub fn policies(&self) -> &[RetryPolicyRecord] {
        &self.policies
    }
}

/// Encode canonical planning data against independently checked context.
pub fn encode_retry_planning(
    compilation: &VerifiedCompilation,
    policies: &[RetryPolicyRecord],
) -> Result<Vec<u8>, RetryPlanningError> {
    validate_count(compilation, policies.len())?;
    validate_profile(compilation)?;
    let mut previous = None;
    for policy in policies {
        validate_policy(compilation, policy, &mut previous)?;
    }
    let count = u16::try_from(policies.len()).map_err(|_| RetryPlanningError::Count)?;
    let length = attachment_length(policies.len())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| RetryPlanningError::Allocation)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&GENERATION.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&compilation.payload_digest().as_bytes());
    bytes.extend_from_slice(&compilation.action_contract_digest().as_bytes());
    for policy in policies {
        bytes.extend_from_slice(&policy.owner.get().to_le_bytes());
        bytes.extend_from_slice(&policy.action.get().to_le_bytes());
        bytes.extend_from_slice(&policy.total_attempts.get().to_le_bytes());
    }
    Ok(bytes)
}

/// Strict bounded decoder; no fallback, authentication, or authority conversion.
pub fn decode_retry_planning<'a>(
    compilation: &'a VerifiedCompilation,
    bytes: &[u8],
) -> Result<RetryPlanningAttachment<'a>, RetryPlanningError> {
    if bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err(RetryPlanningError::Magic);
    }
    let generation = read_u16(bytes, 8)?;
    if generation != GENERATION {
        return Err(RetryPlanningError::Generation(generation));
    }
    let count = usize::from(read_u16(bytes, 10)?);
    validate_count(compilation, count)?;
    if bytes.len() != attachment_length(count)? {
        return Err(RetryPlanningError::Length);
    }
    if bytes.get(12..44) != Some(compilation.payload_digest().as_bytes().as_slice())
        || bytes.get(44..76) != Some(compilation.action_contract_digest().as_bytes().as_slice())
    {
        return Err(RetryPlanningError::Binding);
    }
    validate_profile(compilation)?;
    let mut policies = Vec::new();
    policies
        .try_reserve_exact(count)
        .map_err(|_| RetryPlanningError::Allocation)?;
    let records = bytes
        .get(HEADER_BYTES..)
        .ok_or(RetryPlanningError::Length)?;
    let mut previous = None;
    for record in records.chunks_exact(RECORD_BYTES) {
        let policy = RetryPolicyRecord::new(
            StepIdx::new(read_u16(record, 0)?),
            ActionId::new(read_u16(record, 2)?),
            read_u16(record, 4)?,
        )?;
        validate_policy(compilation, &policy, &mut previous)?;
        policies.push(policy);
    }
    Ok(RetryPlanningAttachment {
        compilation,
        policies,
    })
}

fn attachment_length(count: usize) -> Result<usize, RetryPlanningError> {
    count
        .checked_mul(RECORD_BYTES)
        .and_then(|n| n.checked_add(HEADER_BYTES))
        .ok_or(RetryPlanningError::Length)
}
fn read_u16(bytes: &[u8], start: usize) -> Result<u16, RetryPlanningError> {
    let end = start.checked_add(2).ok_or(RetryPlanningError::Length)?;
    let pair = bytes.get(start..end).ok_or(RetryPlanningError::Length)?;
    let pair = <[u8; 2]>::try_from(pair).map_err(|_| RetryPlanningError::Length)?;
    Ok(u16::from_le_bytes(pair))
}
fn validate_count(
    compilation: &VerifiedCompilation,
    count: usize,
) -> Result<(), RetryPlanningError> {
    if count == 0
        || count > usize::from(u16::MAX)
        || count > usize::from(compilation.workflow().node_count())
    {
        return Err(RetryPlanningError::Count);
    }
    Ok(())
}
fn validate_profile(compilation: &VerifiedCompilation) -> Result<(), RetryPlanningError> {
    let workflow = compilation.workflow();
    if workflow.entry() != StepIdx::new(0) {
        return Err(RetryPlanningError::Profile);
    }
    for raw in 0..workflow.node_count() {
        let node = workflow
            .node(StepIdx::new(raw))
            .ok_or(RetryPlanningError::Profile)?;
        if node.id != StepIdx::new(raw) || node.routing.error != ErrorRouting::None {
            return Err(RetryPlanningError::Profile);
        }
        let next = raw.checked_add(1).ok_or(RetryPlanningError::Profile)?;
        match &node.kind {
            CompiledNodeKind::Finish { .. }
                if next == workflow.node_count() && node.routing.next.step().is_none() => {}
            CompiledNodeKind::Nop
            | CompiledNodeKind::SetConst { .. }
            | CompiledNodeKind::Copy { .. }
            | CompiledNodeKind::EvalExpr { .. }
            | CompiledNodeKind::BuildObject { .. }
            | CompiledNodeKind::BuildList { .. }
            | CompiledNodeKind::Do { .. }
                if next < workflow.node_count()
                    && node.routing.next.step() == Some(StepIdx::new(next)) => {}
            _ => return Err(RetryPlanningError::Profile),
        }
    }
    Ok(())
}
fn validate_policy(
    compilation: &VerifiedCompilation,
    policy: &RetryPolicyRecord,
    previous: &mut Option<StepIdx>,
) -> Result<(), RetryPlanningError> {
    if previous.is_some_and(|owner| owner >= policy.owner) {
        return Err(RetryPlanningError::Owners);
    }
    let node = compilation
        .workflow()
        .node(policy.owner)
        .ok_or(RetryPlanningError::Owner(policy.owner))?;
    if !matches!(node.kind, CompiledNodeKind::Do { action, .. } if action == policy.action) {
        return Err(RetryPlanningError::Owner(policy.owner));
    }
    *previous = Some(policy.owner);
    Ok(())
}

#[cfg(test)]
#[path = "retry_generation_tests.rs"]
mod tests;
