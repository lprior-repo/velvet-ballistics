use super::{ExecutorError, RegisterGrant};
use serde::{Deserialize, Serialize};
use vb_core::action_pack::{ActionPackDigest, CapabilityScopeSpec, InputDigest, ReceiptDigest};
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, StepIdx, WorkflowDigest};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Invocation {
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    action: ActionId,
    key: u128,
}

impl From<ActionTicket> for Invocation {
    fn from(ticket: ActionTicket) -> Self {
        Self {
            run: ticket.run,
            step: ticket.step,
            seq: ticket.seq,
            action: ticket.action,
            key: ticket.idempotency_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ReceiptBody {
    pub invocation: Invocation,
    pub artifact: WorkflowDigest,
    pub pack: ActionPackDigest,
    pub scope: CapabilityScopeSpec,
    pub input: InputDigest,
    pub value: i64,
    pub effect_ordinal: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ReceiptRecord {
    pub body: ReceiptBody,
    pub digest: ReceiptDigest,
}

/// Verified receipt returned only by a registered adapter, never deserialized publicly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterReceipt(pub(super) ReceiptRecord);

impl RegisterReceipt {
    /// Desired I64 value committed with this receipt.
    pub const fn value(&self) -> i64 {
        self.0.body.value
    }
    /// Durable ordinal of the effect, unchanged by duplicate calls/reconciliation.
    pub const fn effect_ordinal(&self) -> u64 {
        self.0.body.effect_ordinal
    }
    /// Digest of exact receipt body bytes.
    pub const fn digest(&self) -> ReceiptDigest {
        self.0.digest
    }
}

pub(super) fn hash(bytes: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(bytes).into())
}

pub(super) fn input_digest(value: i64) -> InputDigest {
    InputDigest::from_digest(hash(&value.to_le_bytes()))
}

pub(super) fn receipt_key(ticket: ActionTicket) -> Result<[u8; 32], ExecutorError> {
    let bytes = postcard::to_allocvec(&Invocation::from(ticket))?;
    Ok(blake3::hash(&bytes).into())
}

pub(super) fn record(
    grant: &RegisterGrant,
    value: i64,
    ordinal: u64,
) -> Result<ReceiptRecord, ExecutorError> {
    let body = ReceiptBody {
        invocation: grant.ticket.into(),
        artifact: grant.artifact,
        pack: grant.pack,
        scope: grant.scope,
        input: input_digest(value),
        value,
        effect_ordinal: ordinal,
    };
    let digest = ReceiptDigest::from_digest(hash(&postcard::to_allocvec(&body)?));
    Ok(ReceiptRecord { body, digest })
}

pub(super) fn validate_record(record: &ReceiptRecord) -> Result<(), ExecutorError> {
    if record.body.input != input_digest(record.body.value)
        || record.digest != ReceiptDigest::from_digest(hash(&postcard::to_allocvec(&record.body)?))
        || record.body.effect_ordinal == 0
    {
        return Err(ExecutorError::Corrupt);
    }
    Ok(())
}
