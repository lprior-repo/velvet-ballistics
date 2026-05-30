#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for typed partitioned storage IDs.

use crate::{
    codec::validation::{is_known_record_kind, unknown_record_kind_value},
    constants::{PREFIX_INDEX_ACTION, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER},
    keys,
    records::RecordKind,
    types::EventSeq,
};
use vb_core::{ActionId, RunId, SeqNo, StepIdx, WorkflowId};

#[derive(Clone, Copy, kani::Arbitrary)]
struct SymbolicKeyInputs {
    run_hi: u16,
    run_lo: u16,
    seq_hi: u16,
    seq_lo: u16,
    workflow_raw: u16,
    action_raw: u8,
    step_raw: u8,
}

#[derive(Clone, Copy, kani::Arbitrary)]
struct SymbolicRecordKindInput {
    record_kind_raw: u8,
}

fn unknown_record_kind(kind: u16) -> bool {
    !matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50)
}

fn run_raw(inputs: SymbolicKeyInputs) -> u64 {
    (u64::from(inputs.run_hi) << 16) | u64::from(inputs.run_lo)
}

fn seq_raw(inputs: SymbolicKeyInputs) -> u64 {
    (u64::from(inputs.seq_hi) << 16) | u64::from(inputs.seq_lo)
}

fn assert_key_contracts(inputs: SymbolicKeyInputs) {
    let run_value = run_raw(inputs);
    let seq_value = seq_raw(inputs);
    let workflow_value = u32::from(inputs.workflow_raw);
    let action_value = u16::from(inputs.action_raw);
    let step_value = u16::from(inputs.step_raw);

    let run = RunId::new(run_value);
    let seq = EventSeq::new(seq_value);
    let workflow = WorkflowId::new(workflow_value);
    let action = ActionId::new(action_value);
    let step = StepIdx::new(step_value);

    match keys::run_header_key(run) {
        Ok(key) => {
            assert!(key[0] == PREFIX_RUN_HEADER);
            assert!(key[1..9] == run_value.to_be_bytes());
        }
        Err(_) => assert!(false),
    }
    match keys::run_event_key(run, seq) {
        Ok(key) => {
            assert!(key[0] == PREFIX_RUN_EVENT);
            assert!(key[1..9] == run_value.to_be_bytes());
            assert!(key[9..17] == seq_value.to_be_bytes());
        }
        Err(_) => assert!(false),
    }
    match keys::index_workflow_key(workflow, run) {
        Ok(key) => {
            assert!(key[0] == PREFIX_INDEX_WORKFLOW);
            assert!(key[1..5] == workflow_value.to_be_bytes());
            assert!(key[5..13] == run_value.to_be_bytes());
        }
        Err(_) => assert!(false),
    }
    match keys::index_action_key(action, run, step) {
        Ok(key) => {
            assert!(key[0] == PREFIX_INDEX_ACTION);
            assert!(key[1..3] == action_value.to_be_bytes());
            assert!(key[3..11] == run_value.to_be_bytes());
            assert!(key[11..13] == step_value.to_be_bytes());
        }
        Err(_) => assert!(false),
    }

    match SeqNo::new(seq_value).checked_add(1) {
        Some(next) => assert!(seq_value.checked_add(1) == Some(next.get())),
        None => assert!(seq_value == u64::MAX),
    }
}

fn assert_record_kind_contract(input: SymbolicRecordKindInput) {
    let kind = u16::from(input.record_kind_raw);
    assert!(is_known_record_kind(kind) != unknown_record_kind(kind));
    if unknown_record_kind(kind) {
        assert!(unknown_record_kind_value(kind) == Some(kind));
    }
    assert!(RecordKind::WorkflowSource.id() == 1);
    assert!(RecordKind::CompiledIr.id() == 2);
    assert!(RecordKind::RunHeader.id() == 3);
    assert!(RecordKind::RunAccepted.id() == 10);
    assert!(RecordKind::RunAnswered.id() == 27);
    assert!(RecordKind::Snapshot.id() == 30);
    assert!(RecordKind::Blob.id() == 40);
    assert!(RecordKind::IndexUpdate.id() == 50);
}

#[kani::proof]
fn vb_eepg_typed_partitioned_ids() {
    let inputs: SymbolicKeyInputs = kani::any();
    assert_key_contracts(inputs);
}

#[kani::proof]
fn vb_eepg_record_kind_contracts() {
    let input: SymbolicRecordKindInput = kani::any();
    assert_record_kind_contract(input);
}

#[kani::proof]
fn vb_eepg_unknown_record_kind_error_contract() {
    let input: SymbolicRecordKindInput = kani::any();
    let kind = u16::from(input.record_kind_raw);
    kani::assume(unknown_record_kind(kind));
    assert!(unknown_record_kind_value(kind) == Some(kind));
}
