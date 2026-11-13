use vb_core::CompiledNodeKind as K;

impl Runtime {
    fn validate_replayed_collect_snapshots(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        snapshots: &[vb_storage::recovery::complete_scalar::OwnedCollectReplaySnapshot],
    ) -> RuntimeResult<()> {
        let mut snapshots = snapshots.iter();
        let mut active_step = None;
        for event in events {
            match event {
                vb_storage::JournalEvent::StepStarted { step, .. } => active_step = Some(*step),
                vb_storage::JournalEvent::SlotWrittenEvent {
                    run,
                    seq,
                    slot,
                    extra,
                    ..
                } => {
                    let Some(step) = active_step else {
                        continue;
                    };
                    let Some(node) = workflow.node(step) else {
                        continue;
                    };
                    let expected_slot = match node.kind {
                        K::CollectStart { source, .. } => {
                            Some(node.output.slot().unwrap_or(source))
                        }
                        K::CollectNext { collector_slot, .. } => Some(collector_slot),
                        _ => None,
                    };
                    if expected_slot != Some(*slot) {
                        continue;
                    }
                    let Some(snapshot) = snapshots.next() else {
                        return Err(collect_snapshot_error("replay snapshot missing"));
                    };
                    if snapshot.seq != *seq
                        || snapshot.run != *run
                        || snapshot.step != step
                        || snapshot.slot != *slot
                    {
                        return Err(collect_snapshot_error("replay snapshot identity mismatch"));
                    }
                    validate_collect_snapshot_extra(*run, *slot, extra.as_deref(), snapshot.state)?;
                }
                vb_storage::JournalEvent::StepSucceeded { .. }
                | vb_storage::JournalEvent::StepFailed { .. } => active_step = None,
                _ => {}
            }
        }
        if snapshots.next().is_some() {
            return Err(collect_snapshot_error("unexpected replay snapshot"));
        }
        Ok(())
    }
}

fn validate_collect_snapshot_extra(
    run: vb_core::RunId,
    slot: vb_core::SlotIdx,
    extra: Option<&[u8]>,
    expected: Option<vb_core::replay::step::ReplayCollectStateSnapshot>,
) -> RuntimeResult<()> {
    let actual = match extra {
        Some(bytes) => match vb_storage::decode_slot_written_extra(bytes) {
            Ok(vb_storage::DecodedSlotWrittenExtra::Envelope(envelope)) => {
                match envelope.frame_extra {
                    Some(frame_extra) => {
                        let state = postcard::from_bytes::<
                            crate::primitives::collect::CollectPaginationState,
                        >(&frame_extra)
                        .map_err(|_| collect_snapshot_error("Collect frame extra decode failed"))?;
                        Some((state, envelope.taint))
                    }
                    None => None,
                }
            }
            _ => {
                return Err(collect_snapshot_error(
                    "Collect frame extra envelope invalid",
                ));
            }
        },
        None => None,
    };
    match (expected, actual) {
        (None, None) => Ok(()),
        (Some(expected), Some((actual, taint)))
            if actual.run_id == run
                && actual.collector_slot == slot
                && actual.source == expected.source
                && actual.current_page == expected.current_page
                && actual.cursor == expected.cursor
                && actual.page_size == expected.page_size
                && actual.item_count == expected.item_count
                && actual.limit == expected.limit
                && taint == expected.taint =>
        {
            Ok(())
        }
        _ => Err(collect_snapshot_error(
            "Collect frame extra differs from replay",
        )),
    }
}

fn collect_snapshot_error(detail: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: detail.to_owned(),
    }
}
