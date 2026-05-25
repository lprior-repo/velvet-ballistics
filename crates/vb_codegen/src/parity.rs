#![forbid(unsafe_code)]

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

/// Minimal generated/IR journal token used by parity fixtures.
pub type JournalEvent = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Action,
    WaitUntil,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    MissingSlot,
    DivByZero,
    BudgetExhausted,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishedRun {
    pub run_id: RunId,
    pub pc: StepIdx,
    pub executed: u64,
    pub result: SlotValue,
    pub result_taint: Taint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockedRun {
    pub run_id: RunId,
    pub pc: StepIdx,
    pub executed: u64,
    pub blocked_step: StepIdx,
    pub block_kind: BlockKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorRun {
    pub run_id: RunId,
    pub pc: StepIdx,
    pub executed: u64,
    pub error_step: StepIdx,
    pub error_class: ErrorClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalStatus {
    Finished(FinishedRun),
    Blocked(BlockedRun),
    Error(ErrorRun),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRun {
    pub status: TerminalStatus,
    pub journal: Vec<JournalEvent>,
    pub slots: Vec<(SlotIdx, SlotValue)>,
    pub taints: Vec<(SlotIdx, Taint)>,
    pub journal_len: u64,
    pub is_generated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityError {
    TerminalMismatch { detail: &'static str },
    JournalMismatch { detail: &'static str },
    TaintMismatch { detail: &'static str },
    SuspensionMismatch { detail: &'static str },
    ResumeMismatch { detail: &'static str },
    UnsupportedMismatch { detail: &'static str },
    SlotValueMismatch { slot: SlotIdx, detail: &'static str },
}

pub fn compare_observed_runs(
    ir: &ObservedRun,
    generated: &ObservedRun,
) -> Result<(), ParityError> {
    compare_terminal(ir.status, generated.status)?;
    compare_journal(ir, generated)?;
    compare_slots(&ir.slots, &generated.slots)?;
    compare_taints(&ir.taints, &generated.taints)
}

fn compare_terminal(left: TerminalStatus, right: TerminalStatus) -> Result<(), ParityError> {
    match (left, right) {
        (TerminalStatus::Finished(a), TerminalStatus::Finished(b)) => compare_finished(a, b),
        (TerminalStatus::Blocked(a), TerminalStatus::Blocked(b)) => compare_blocked(a, b),
        (TerminalStatus::Error(a), TerminalStatus::Error(b)) => compare_error(a, b),
        _ => Err(ParityError::TerminalMismatch {
            detail: "terminal status mismatch",
        }),
    }
}

fn compare_finished(left: FinishedRun, right: FinishedRun) -> Result<(), ParityError> {
    if left.result != right.result {
        Err(ParityError::TerminalMismatch {
            detail: "result mismatch",
        })
    } else if left.result_taint != right.result_taint {
        Err(ParityError::TaintMismatch {
            detail: "terminal taint mismatch",
        })
    } else if left.pc != right.pc || left.executed != right.executed || left.run_id != right.run_id {
        Err(ParityError::TerminalMismatch {
            detail: "terminal metadata mismatch",
        })
    } else {
        Ok(())
    }
}

fn compare_blocked(left: BlockedRun, right: BlockedRun) -> Result<(), ParityError> {
    if left == right {
        Ok(())
    } else {
        Err(ParityError::SuspensionMismatch {
            detail: "suspension metadata mismatch",
        })
    }
}

fn compare_error(left: ErrorRun, right: ErrorRun) -> Result<(), ParityError> {
    if left == right {
        Ok(())
    } else {
        Err(ParityError::TerminalMismatch {
            detail: "error metadata mismatch",
        })
    }
}

fn compare_journal(left: &ObservedRun, right: &ObservedRun) -> Result<(), ParityError> {
    if left.journal_len != right.journal_len || left.journal != right.journal {
        Err(ParityError::JournalMismatch {
            detail: "journal mismatch",
        })
    } else {
        Ok(())
    }
}

fn compare_slots(
    left: &[(SlotIdx, SlotValue)],
    right: &[(SlotIdx, SlotValue)],
) -> Result<(), ParityError> {
    first_slot_mismatch(left, right).map_or(Ok(()), |slot| {
        Err(ParityError::SlotValueMismatch {
            slot,
            detail: "slot value mismatch",
        })
    })
}

fn first_slot_mismatch(
    left: &[(SlotIdx, SlotValue)],
    right: &[(SlotIdx, SlotValue)],
) -> Option<SlotIdx> {
    left.iter()
        .zip(right.iter())
        .find_map(|(a, b)| if a == b { None } else { Some(a.0) })
        .or_else(|| length_mismatch_slot(left, right))
}

fn length_mismatch_slot(
    left: &[(SlotIdx, SlotValue)],
    right: &[(SlotIdx, SlotValue)],
) -> Option<SlotIdx> {
    if left.len() == right.len() {
        None
    } else {
        left.first()
            .map(|(slot, _)| *slot)
            .or_else(|| right.first().map(|(slot, _)| *slot))
    }
}

fn compare_taints(
    left: &[(SlotIdx, Taint)],
    right: &[(SlotIdx, Taint)],
) -> Result<(), ParityError> {
    if left == right {
        Ok(())
    } else {
        Err(ParityError::TaintMismatch {
            detail: "slot taint mismatch",
        })
    }
}
