use super::super::{ContinuationAuthority, payload::VERSION};
use super::{TransportFailure, operation::Operation};
use vb_core::value_store::transport::{CheckpointShape, CheckpointWireCursor as Cursor};
use vb_core::value_store::transport::{CheckpointTransportError as Error, preflight_checkpoint};

/// Unforgeable witness borrowing precisely the input and independent authority.
pub(super) struct Witness<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) authority: &'a ContinuationAuthority,
}

pub(super) fn inspect<'a>(
    bytes: &'a [u8],
    authority: &'a ContinuationAuthority,
    op: &mut Operation,
) -> Result<Witness<'a>, TransportFailure> {
    let mut cursor = Cursor::new(bytes);
    let version = cursor.unsigned(16)?;
    if version != u64::from(VERSION) {
        return Err(TransportFailure::UnsupportedVersion(version));
    }
    header(&mut cursor, authority, op)?;
    frame(&mut cursor, authority, op)?;
    let resource = authority.workflow.resource_contract();
    let shape = CheckpointShape {
        arena_entries: usize::from(resource.max_slots),
        blob_bytes: usize::try_from(resource.max_blob_bytes)
            .map_err(|_| Error::Overflow)?
            .min(vb_core::limits::MAX_BLOB_BYTES_PER_VALUE),
    };
    preflight_checkpoint(&mut cursor, &mut op.budget, shape)?;
    attempts(&mut cursor, authority, op)?;
    cursor.matching(op.known(&authority.admission)?)?;
    // Both supported terminal cuts require genuinely empty pagination AND lineage.
    if cursor.length(0, "collect_entries")? != 0 || cursor.length(0, "collect_lineages")? != 0 {
        return Err(Error::Malformed("collect_tables").into());
    }
    cursor.matching(op.known(&authority.contracts)?)?;
    integrity(bytes, &mut cursor)?;
    Ok(Witness { bytes, authority })
}

fn frame(
    cursor: &mut Cursor<'_>,
    authority: &ContinuationAuthority,
    op: &mut Operation,
) -> Result<(), TransportFailure> {
    cursor.matching(op.known(&authority.expected.run)?)?;
    cursor.unsigned(16)?;
    cursor.unsigned(64)?;
    cursor.unsigned(16)?;
    if cursor.unsigned(16)? != 0 {
        return Err(Error::Bound("parallel_in_flight").into());
    }
    let nodes = usize::from(authority.workflow.node_count());
    let slots = usize::from(authority.workflow.slot_count());
    states(cursor, nodes, op)?;
    exact_count(cursor, slots, "frame_slots")?;
    op.budget.reserve(slots, 256)?;
    op.budget.work(slots)?;
    for _ in 0..slots {
        match cursor.take(1)?.first().copied() {
            Some(0) => {}
            Some(1) => cursor.value()?,
            _ => return Err(Error::Malformed("slot_option").into()),
        }
    }
    exact_count(cursor, slots, "frame_taints")?;
    for _ in 0..slots {
        cursor.taint()?;
    }
    Ok(())
}

fn attempts(
    cursor: &mut Cursor<'_>,
    authority: &ContinuationAuthority,
    op: &mut Operation,
) -> Result<(), TransportFailure> {
    let nodes = usize::from(authority.workflow.node_count());
    exact_count(cursor, nodes, "action_attempts")?;
    op.budget.reserve(nodes, 8)?;
    for _ in 0..nodes {
        cursor.unsigned(16)?;
    }
    Ok(())
}

fn exact_count(cursor: &mut Cursor<'_>, count: usize, field: &'static str) -> Result<(), Error> {
    if cursor.length(count, field)? != count {
        return Err(Error::Bound(field));
    }
    Ok(())
}

fn integrity(bytes: &[u8], cursor: &mut Cursor<'_>) -> Result<(), TransportFailure> {
    let body_len = bytes
        .len()
        .checked_sub(32)
        .ok_or(Error::Malformed("digest"))?;
    let digest = cursor.take(32)?;
    if !cursor.remaining().is_empty() {
        return Err(Error::Malformed("trailing_bytes").into());
    }
    let body = bytes.get(..body_len).ok_or(Error::Malformed("body"))?;
    let expected = blake3::derive_key("velvet-ballistics/run-continuation/v1", body);
    if digest != expected {
        return Err(TransportFailure::Integrity);
    }
    Ok(())
}

fn header(
    cursor: &mut Cursor<'_>,
    authority: &ContinuationAuthority,
    op: &mut Operation,
) -> Result<(), TransportFailure> {
    cursor.matching(op.known(&authority.expected)?)?;
    cursor.matching(&authority.workflow.digest().as_bytes())?;
    cursor.matching(&authority.input_digest.as_bytes())?;
    if cursor.unsigned(32)? > 1 {
        return Err(Error::Malformed("boundary_cut").into());
    }
    for _ in 0..3 {
        cursor.unsigned(16)?;
    }
    Ok(())
}

fn states(
    cursor: &mut Cursor<'_>,
    nodes: usize,
    op: &mut Operation,
) -> Result<(), TransportFailure> {
    exact_count(cursor, nodes, "frame_states")?;
    op.budget.reserve(nodes, 32)?;
    op.budget.work(nodes)?;
    for _ in 0..nodes {
        if !matches!(cursor.unsigned(32)?, 0 | 2 | 4) {
            return Err(Error::Bound("step_state").into());
        }
    }
    Ok(())
}
