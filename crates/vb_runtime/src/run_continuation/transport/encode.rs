use super::super::{ContinuationAuthority, RunContinuationV1};
use super::{BoundedContinuationPayload, ContinuationTransportError, ContinuationTransportLimits};
use super::{TransportFailure, operation::Operation, preflight, workflow};
use crate::shard::types::RunState;
use vb_core::value_store::transport::{CheckpointTransportError as Error, TransportUsage};

impl RunContinuationV1 {
    /// Encodes exact component content without first cloning the complete store.
    /// Outer journal tag/metadata/varints are not included in this component limit.
    pub fn capture_transport(
        state: &RunState,
        authority: &ContinuationAuthority,
        limits: ContinuationTransportLimits,
    ) -> Result<BoundedContinuationPayload, ContinuationTransportError> {
        let mut op = start(limits)?;
        super::capture::capture(state, authority, limits, &mut op)
            .map_err(|failure| report(failure, &op))
    }

    /// Checks all nested bounds, exact canonical authority bytes and raw integrity
    /// before allowing the unchanged ordinary strong decoder to allocate.
    pub fn from_transport_bytes(
        bytes: &[u8],
        authority: &ContinuationAuthority,
        limits: ContinuationTransportLimits,
    ) -> Result<(Self, TransportUsage), ContinuationTransportError> {
        if bytes.len() > limits.component {
            return Err(ContinuationTransportError {
                failure: Error::Bound("component_bytes").into(),
                usage: TransportUsage::default(),
                strong_decode_started: false,
            });
        }
        let mut op = start(limits)?;
        let result = decode(bytes, authority, &mut op).map_err(|failure| report(failure, &op))?;
        Ok((result, op.budget.usage()))
    }
}

pub(super) fn decode(
    bytes: &[u8],
    authority: &ContinuationAuthority,
    op: &mut Operation,
) -> Result<RunContinuationV1, TransportFailure> {
    workflow::charge(op, authority)?;
    op.budget.work(bytes.len())?;
    let witness = preflight::inspect(bytes, authority, op)?;
    op.decoded = true;
    let result = RunContinuationV1::from_bytes(witness.bytes, witness.authority)?;
    // Strong decoding preserves its existing semantics; this final bounded
    // reserialization additionally refuses any noncanonical postcard representation.
    let canonical = postcard::to_slice(&result.data, &mut op.scratch)?;
    if canonical != witness.bytes {
        return Err(Error::Malformed("noncanonical_wire").into());
    }
    Ok(result)
}

fn start(limits: ContinuationTransportLimits) -> Result<Operation, ContinuationTransportError> {
    Operation::new(limits).map_err(|failure| ContinuationTransportError {
        failure,
        usage: TransportUsage::default(),
        strong_decode_started: false,
    })
}

fn report(failure: TransportFailure, op: &Operation) -> ContinuationTransportError {
    ContinuationTransportError {
        failure,
        usage: op.budget.usage(),
        strong_decode_started: op.decoded,
    }
}
