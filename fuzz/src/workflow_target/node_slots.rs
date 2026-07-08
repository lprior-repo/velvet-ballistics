//! Slot-reference checks for compiled-node fuzzing.

pub(super) fn check_node_slots(kind: &vb_core::CompiledNodeKind, slot_count: u16, node_idx: u16) {
    use vb_core::CompiledNodeKind;
    match kind {
        CompiledNodeKind::Nop | CompiledNodeKind::Jump { .. } => {}
        CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => assert_slot(*source, slot_count, node_idx),
        CompiledNodeKind::EvalExpr { expr: _ } => {}
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                assert_slot(*slot, slot_count, node_idx);
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                assert_slot(*slot, slot_count, node_idx);
            }
        }
        CompiledNodeKind::Do { action: _, input } => assert_slot(*input, slot_count, node_idx),
        CompiledNodeKind::Choose { otherwise, .. } => {
            let _ = otherwise;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                assert_slot(branch.condition, slot_count, node_idx);
            }
            let _ = otherwise;
        }
        CompiledNodeKind::ForEachStart {
            input, item_slot, ..
        } => {
            assert_slot(*input, slot_count, node_idx);
            assert_slot(*item_slot, slot_count, node_idx);
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            assert_slot(*iterator_slot, slot_count, node_idx);
        }
        CompiledNodeKind::ForEachJoin { output } => assert_slot(*output, slot_count, node_idx),
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. }
        | CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            assert_slot(*accumulator, slot_count, node_idx);
        }
        CompiledNodeKind::CollectStart { source, .. } => assert_slot(*source, slot_count, node_idx),
        CompiledNodeKind::CollectPage { collector_slot, .. }
        | CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => {
            assert_slot(*collector_slot, slot_count, node_idx);
        }
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => {
            assert_slot(*input, slot_count, node_idx);
            assert_slot(*accumulator, slot_count, node_idx);
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            assert_slot(*iterator_slot, slot_count, node_idx);
            assert_slot(*accumulator, slot_count, node_idx);
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert_slot(*accumulator, slot_count, node_idx);
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. }
        | CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            assert_slot(*attempt_slot, slot_count, node_idx);
        }
        CompiledNodeKind::RepeatFinish { result } => assert_slot(*result, slot_count, node_idx),
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            assert_slot(*deadline_slot, slot_count, node_idx);
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert_slot(*event, slot_count, node_idx);
            if let Some(timeout) = timeout_slot {
                assert_slot(*timeout, slot_count, node_idx);
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert_slot(*prompt, slot_count, node_idx);
            if let Some(timeout) = timeout_slot {
                assert_slot(*timeout, slot_count, node_idx);
            }
        }
        CompiledNodeKind::AskResume { answer } => assert_slot(*answer, slot_count, node_idx),
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            assert_slot(*policy_slot, slot_count, node_idx);
        }
        CompiledNodeKind::ErrorHandler {
            error_slot: Some(slot),
            ..
        } => assert_slot(*slot, slot_count, node_idx),
        CompiledNodeKind::ErrorHandler {
            error_slot: None, ..
        } => {}
        CompiledNodeKind::Finish { result } => assert_slot(*result, slot_count, node_idx),
        _ => {}
    }
}

fn assert_slot(slot: vb_core::SlotIdx, slot_count: u16, node_idx: u16) {
    assert!(
        slot.get() < slot_count,
        "node {} slot {} out of bounds",
        node_idx,
        slot.get()
    );
}
