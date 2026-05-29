#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, SeqNo, StepIdx, WorkflowId};
use vb_storage::{JournalError, codec::decode_record_header, keys, types::EventSeq};

fn header_with_kind(kind: u16) -> [u8; vb_storage::constants::RECORD_HEADER_BYTES] {
    let mut header = [0_u8; vb_storage::constants::RECORD_HEADER_BYTES];
    header[0..4].copy_from_slice(&vb_storage::constants::MAGIC_JOURNAL_EVENT.to_le_bytes());
    header[4..6].copy_from_slice(&vb_storage::constants::CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&kind.to_le_bytes());
    header[8..12].copy_from_slice(&vb_storage::constants::RECORD_HEADER_LEN.to_le_bytes());
    header[12..16].copy_from_slice(&0_u32.to_le_bytes());
    header[16..24].copy_from_slice(&0_u64.to_le_bytes());
    header
}

fn unknown_kind(kind: u16) -> bool {
    !matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50)
}

proptest! {
    #[test]
    fn generated_typed_partitioned_ids_preserve_bytes(
        run in any::<u64>(),
        seq in any::<u64>(),
        workflow in any::<u32>(),
        action in any::<u16>(),
        step in any::<u16>(),
        kind in any::<u16>(),
    ) {
        let header = keys::run_header_key(RunId::new(run))?;
        prop_assert_eq!(header[0], vb_storage::constants::PREFIX_RUN_HEADER);
        prop_assert_eq!(&header[1..9], &run.to_be_bytes());

        let event = keys::run_event_key(RunId::new(run), EventSeq::new(seq))?;
        prop_assert_eq!(event[0], vb_storage::constants::PREFIX_RUN_EVENT);
        prop_assert_eq!(&event[1..9], &run.to_be_bytes());
        prop_assert_eq!(&event[9..17], &seq.to_be_bytes());

        let workflow_key = keys::index_workflow_key(WorkflowId::new(workflow), RunId::new(run))?;
        prop_assert_eq!(workflow_key[0], vb_storage::constants::PREFIX_INDEX_WORKFLOW);
        prop_assert_eq!(&workflow_key[1..5], &workflow.to_be_bytes());
        prop_assert_eq!(&workflow_key[5..13], &run.to_be_bytes());

        let action_key = keys::index_action_key(ActionId::new(action), RunId::new(run), StepIdx::new(step))?;
        prop_assert_eq!(action_key[0], vb_storage::constants::PREFIX_INDEX_ACTION);
        prop_assert_eq!(&action_key[1..3], &action.to_be_bytes());
        prop_assert_eq!(&action_key[3..11], &run.to_be_bytes());
        prop_assert_eq!(&action_key[11..13], &step.to_be_bytes());

        if seq == u64::MAX {
            prop_assert!(SeqNo::new(seq).checked_add(1).is_none());
        } else {
            prop_assert_eq!(SeqNo::new(seq).checked_add(1).map(SeqNo::get), Some(seq + 1));
        }

        if unknown_kind(kind) {
            let decoded = decode_record_header(
                &header_with_kind(kind),
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            match decoded {
                Err(JournalError::UnknownRecordKind { kind: found }) => prop_assert_eq!(found, kind),
                other => prop_assert!(false, "expected UnknownRecordKind, got {other:?}"),
            }
        }
    }
}

#[test]
fn explicit_edges_and_stable_record_kinds_hold() -> Result<(), JournalError> {
    for run in [0, 1, 0x0102_0304_0506_0708, u64::MAX - 1, u64::MAX] {
        let header = keys::run_header_key(RunId::new(run))?;
        assert_eq!(&header[1..9], &run.to_be_bytes());
    }
    for workflow in [0, 1, 0x0102_0304, u32::MAX - 1, u32::MAX] {
        let key = keys::index_workflow_key(WorkflowId::new(workflow), RunId::new(7))?;
        assert_eq!(&key[1..5], &workflow.to_be_bytes());
    }
    for value in [0, 1, 0x0102, u16::MAX - 1, u16::MAX] {
        let key = keys::index_action_key(ActionId::new(value), RunId::new(7), StepIdx::new(value))?;
        assert_eq!(&key[1..3], &value.to_be_bytes());
        assert_eq!(&key[11..13], &value.to_be_bytes());
    }
    assert_eq!(vb_storage::records::RecordKind::WorkflowSource.id(), 1);
    assert_eq!(vb_storage::records::RecordKind::CompiledIr.id(), 2);
    assert_eq!(vb_storage::records::RecordKind::RunHeader.id(), 3);
    assert_eq!(vb_storage::records::RecordKind::RunAccepted.id(), 10);
    assert_eq!(vb_storage::records::RecordKind::RunAnswered.id(), 27);
    assert_eq!(vb_storage::records::RecordKind::Snapshot.id(), 30);
    assert_eq!(vb_storage::records::RecordKind::Blob.id(), 40);
    assert_eq!(vb_storage::records::RecordKind::IndexUpdate.id(), 50);
    Ok(())
}
