use super::OwnedInputError;
use crate::{JournalError, RecordEnvelope, RecordKind, codec::EnforceKindParity};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{CompiledInputSlot, ResourceContract};
use vb_core::{RunId, SlotIdx, SlotValue, Taint, WorkflowDigest};

pub(super) const VERSION: u16 = 1;
const DOMAIN: &str = "velvet-ballistics/owned-run-inputs/v1";

/// Exact digest preimage; the digest itself is deliberately absent.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Body {
    pub(super) version: u16,
    pub(super) run: RunId,
    pub(super) artifact: WorkflowDigest,
    pub(super) workflow: WorkflowDigest,
    pub(super) source: WorkflowDigest,
    pub(super) slots: u16,
    pub(super) inputs: Box<[CompiledInputSlot]>,
    pub(super) resources: ResourceContract,
    pub(super) roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    pub(super) checkpoint: ValueStoreCheckpointV1,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Wire {
    pub(super) body: Body,
    pub(super) digest: WorkflowDigest,
}

impl Body {
    pub(super) fn digest(&self) -> Result<WorkflowDigest, JournalError> {
        let bytes = postcard::to_allocvec(self).map_err(JournalError::PostcardEncodeFailed)?;
        Ok(WorkflowDigest::from_bytes(blake3::derive_key(
            DOMAIN, &bytes,
        )))
    }
}

impl Wire {
    pub(super) fn validate_integrity(&self) -> Result<(), JournalError> {
        if self.body.version != VERSION {
            return Err(OwnedInputError::UnsupportedVersion {
                version: self.body.version,
            }
            .into());
        }
        if self.body.digest()? != self.digest {
            return Err(OwnedInputError::DigestMismatch.into());
        }
        Ok(())
    }
}

impl EnforceKindParity for Wire {
    fn decode_payload<'a>(
        envelope: &RecordEnvelope,
        payload: &'a [u8],
    ) -> Result<(Self, &'a [u8]), JournalError> {
        if envelope.record_kind != RecordKind::RunOwnedInputs.id() {
            return Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind: envelope.record_kind,
                payload_kind: RecordKind::RunOwnedInputs.id(),
            });
        }
        let (version, _): (u16, _) =
            postcard::take_from_bytes(payload).map_err(JournalError::PostcardDecodeFailed)?;
        if version != VERSION {
            return Err(OwnedInputError::UnsupportedVersion { version }.into());
        }
        postcard::take_from_bytes(payload).map_err(JournalError::PostcardDecodeFailed)
    }

    fn enforce_kind_parity(envelope: &RecordEnvelope, value: &Self) -> Result<(), JournalError> {
        if envelope.sequence != value.body.run.get() {
            return Err(OwnedInputError::IdentityMismatch.into());
        }
        value.validate_integrity()
    }
}
