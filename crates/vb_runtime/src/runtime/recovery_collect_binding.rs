use vb_core::CompiledNodeKind as K;
use vb_core::workflow::CompiledNode;

type CollectSnapshot = vb_storage::recovery::complete_scalar::OwnedCollectReplaySnapshot;
type CollectSnapshotIter<'a> = std::slice::Iter<'a, CollectSnapshot>;
type ReplayCollectState = vb_core::replay::step::ReplayCollectStateSnapshot;

struct CollectSnapshotScan<'a> {
    workflow: &'a CompiledWorkflow,
    snapshots: CollectSnapshotIter<'a>,
    active_step: Option<vb_core::StepIdx>,
}

impl<'a> CollectSnapshotScan<'a> {
    fn new(workflow: &'a CompiledWorkflow, snapshots: &'a [CollectSnapshot]) -> Self {
        Self {
            workflow,
            snapshots: snapshots.iter(),
            active_step: None,
        }
    }

    fn observe(&mut self, event: &vb_storage::JournalEvent) -> RuntimeResult<()> {
        match event {
            vb_storage::JournalEvent::StepStarted { step, .. } => self.active_step = Some(*step),
            vb_storage::JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                extra,
                ..
            } => self.observe_slot_write(*run, *seq, *slot, extra.as_deref())?,
            vb_storage::JournalEvent::StepSucceeded { .. }
            | vb_storage::JournalEvent::StepFailed { .. } => self.active_step = None,
            _ => {}
        }
        Ok(())
    }

    fn observe_slot_write(
        &mut self,
        run: vb_core::RunId,
        seq: vb_storage::EventSeq,
        slot: vb_core::SlotIdx,
        extra: Option<&[u8]>,
    ) -> RuntimeResult<()> {
        let Some(step) = self.active_step else {
            return Ok(());
        };
        let Some(node) = self.workflow.node(step) else {
            return Ok(());
        };
        if expected_collect_slot(node) != Some(slot) {
            return Ok(());
        }
        let snapshot = self
            .snapshots
            .next()
            .ok_or_else(|| collect_snapshot_error("replay snapshot missing"))?;
        validate_collect_snapshot_identity(snapshot, run, seq, step, slot)?;
        validate_collect_snapshot_extra(run, slot, extra, snapshot.state)
    }

    fn finish(&mut self) -> RuntimeResult<()> {
        if self.snapshots.next().is_some() {
            return Err(collect_snapshot_error("unexpected replay snapshot"));
        }
        Ok(())
    }
}

impl Runtime {
    fn validate_replayed_collect_snapshots(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        snapshots: &[CollectSnapshot],
    ) -> RuntimeResult<()> {
        let mut scan = CollectSnapshotScan::new(workflow, snapshots);
        for event in events {
            scan.observe(event)?;
        }
        scan.finish()
    }
}

fn expected_collect_slot(node: &CompiledNode) -> Option<vb_core::SlotIdx> {
    match node.kind {
        K::CollectStart { source, .. } => Some(node.output.slot().unwrap_or(source)),
        K::CollectNext { collector_slot, .. } => Some(collector_slot),
        _ => None,
    }
}

fn validate_collect_snapshot_identity(
    snapshot: &CollectSnapshot,
    run: vb_core::RunId,
    seq: vb_storage::EventSeq,
    step: vb_core::StepIdx,
    slot: vb_core::SlotIdx,
) -> RuntimeResult<()> {
    if snapshot.seq != seq || snapshot.run != run || snapshot.step != step || snapshot.slot != slot
    {
        return Err(collect_snapshot_error("replay snapshot identity mismatch"));
    }
    Ok(())
}

fn validate_collect_snapshot_extra(
    run: vb_core::RunId,
    slot: vb_core::SlotIdx,
    extra: Option<&[u8]>,
    expected: Option<ReplayCollectState>,
) -> RuntimeResult<()> {
    let actual = decode_collect_snapshot_extra(extra)?;
    if collect_snapshot_state_matches(expected, actual, run, slot) {
        return Ok(());
    }
    Err(collect_snapshot_error(
        "Collect frame extra differs from replay",
    ))
}

fn decode_collect_snapshot_extra(
    extra: Option<&[u8]>,
) -> RuntimeResult<
    Option<(
        crate::primitives::collect::CollectPaginationState,
        vb_core::Taint,
    )>,
> {
    let Some(bytes) = extra else {
        return Ok(None);
    };
    let decoded = vb_storage::decode_slot_written_extra(bytes)
        .map_err(|_| collect_snapshot_error("Collect frame extra envelope invalid"))?;
    let vb_storage::DecodedSlotWrittenExtra::Envelope(envelope) = decoded else {
        return Err(collect_snapshot_error(
            "Collect frame extra envelope invalid",
        ));
    };
    let Some(frame_extra) = envelope.frame_extra else {
        return Ok(None);
    };
    let state =
        postcard::from_bytes::<crate::primitives::collect::CollectPaginationState>(&frame_extra)
            .map_err(|_| collect_snapshot_error("Collect frame extra decode failed"))?;
    Ok(Some((state, envelope.taint)))
}

fn collect_snapshot_state_matches(
    expected: Option<ReplayCollectState>,
    actual: Option<(
        crate::primitives::collect::CollectPaginationState,
        vb_core::Taint,
    )>,
    run: vb_core::RunId,
    slot: vb_core::SlotIdx,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some((actual, taint))) => {
            collect_snapshot_fields_match(expected, actual, run, slot, taint)
        }
        _ => false,
    }
}

fn collect_snapshot_fields_match(
    expected: ReplayCollectState,
    actual: crate::primitives::collect::CollectPaginationState,
    run: vb_core::RunId,
    slot: vb_core::SlotIdx,
    taint: vb_core::Taint,
) -> bool {
    actual.run_id == run
        && actual.collector_slot == slot
        && actual.source == expected.source
        && actual.current_page == expected.current_page
        && actual.cursor == expected.cursor
        && actual.page_size == expected.page_size
        && actual.item_count == expected.item_count
        && actual.limit == expected.limit
        && taint == expected.taint
}

fn collect_snapshot_error(detail: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: detail.to_owned(),
    }
}
