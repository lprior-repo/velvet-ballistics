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
            kani::assert(key[0] == PREFIX_RUN_HEADER, "run_header prefix");
            kani::assert(key[1..9] == run_value.to_be_bytes(), "run_header run bytes");
        }
        Err(_) => , "run_header run bytes");
        }
        Err(_) => kani::assert(false, "run_header_key must succeed"),
    }
    match keys::run_event_key(run, seq) {
        Ok(key) => {
            kani::assert(key[0] == PREFIX_RUN_EVENT, "run_event prefix");
            kani::assert(key[1..9] == run_value.to_be_bytes(), "run_event run bytes");
            kani::assert(key[9..17] == seq_value.to_be_bytes(), "run_event seq bytes");
        }
        Err(_) => , "run_event seq bytes");
        }
        Err(_) => kani::assert(false, "run_event_key must succeed"),
    }
    match keys::index_workflow_key(workflow, run) {
        Ok(key) => {
            kani::assert(key[0] == PREFIX_INDEX_WORKFLOW, "index_workflow prefix");
            kani::assert(
                key[1..5] == workflow_value.to_be_bytes(),
                "index_workflow workflow bytes",
            );
            kani::assert(key[5..13] == run_value.to_be_bytes(),
                "index_workflow run bytes",
            );
        }
        Err(_) => ,
                "index_workflow run bytes",
            );
        }
        Err(_) => kani::assert(false, "index_workflow_key must succeed"),
    }
    match keys::index_action_key(action, run, step) {
        Ok(key) => {
            kani::assert(key[0] == PREFIX_INDEX_ACTION, "index_action prefix");
            kani::assert(
                key[1..3] == action_value.to_be_bytes(),
                "index_action action bytes",
            );
            kani::assert(key[3..11] == run_value.to_be_bytes(),
                "index_action run bytes",
            );
            kani::assert(key[11..13] == step_value.to_be_bytes(),
                "index_action step bytes",
            );
        }
        Err(_) => ,
                "index_action step bytes",
            );
        }
        Err(_) => kani::assert(false, "index_action_key must succeed"),
    }

    match SeqNo::new(seq_value).checked_add(1) {
        Some(next) => kani::assert(
            seq_value.checked_add(1) == Some(next.get()),
            "seq checked_add",
        ),
        None =>  == Some(next.get()),
            "seq checked_add",
        ),
        None => kani::assert(seq_value == u64::MAX, "seq overflow sentinel"),
    }
}

fn assert_record_kind_contract(input: SymbolicRecordKindInput) {
    let kind = u16::from(input.record_kind_raw);
    kani::assert(
        is_known_record_kind(kind) != unknown_record_kind(kind),
        "is_known != unknown",
    );
    if unknown_record_kind(kind) {
        kani::assert(unknown_record_kind_value(kind) == Some(kind),
            "unknown kind value",
        );
    }
    kani::assert(RecordKind::WorkflowSource.id() == 1, "WorkflowSource=1");
    kani::assert(RecordKind::CompiledIr.id() == 2, "CompiledIr=2");
    kani::assert(RecordKind::RunHeader.id() == 3, "RunHeader=3");
    kani::assert(RecordKind::RunAccepted.id() == 10, "RunAccepted=10");
    kani::assert(RecordKind::RunAnswered.id() == 27, "RunAnswered=27");
    kani::assert(RecordKind::Snapshot.id() == 30, "Snapshot=30");
    kani::assert(RecordKind::Blob.id() == 40, "Blob=40");
    kani::assert(RecordKind::IndexUpdate.id() == 50, "IndexUpdate=50");
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
    kani::assert(unknown_record_kind_value(kind) == Some(kind),
        "unknown kind returns Some(kind)",
    );
}
