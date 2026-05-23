//! BDD parity comparison types for generated-vs-IR behavioral equivalence.
//!
//! This module provides the types and functions needed to compare observed runs
//! from the IR interpreter and the generated Rust runtime, supporting the BDD
//! test suite in `vb_0sps_generated_ir_parity_bdd`.
//!
//! ## Design
//!
//! `ObservedRun` captures the complete terminal state of a workflow execution,
//! including the final status, result value, journal event sequence, and all
//! slot values and taints at termination.
//!
//! `ParityError` enumerates every mismatch category that can occur between two
//! runs, enabling precise BDD scenario reporting.
//!
//! `compare_observed_runs` performs a field-by-field comparison and returns the
//! first detected mismatch, making it suitable for both positive parity tests
//! and mutation checkpoints.

use vb_core::{
    action::ActionJournalEvent,
    ids::{RunId, SlotIdx, StepIdx},
    value::{SlotValue, Taint},
};

/// Marker type for a run that reached a terminal finished state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedRun {
    /// Run identifier.
    pub run_id: RunId,
    /// Final program counter at termination.
    pub pc: StepIdx,
    /// Number of executed transitions.
    pub executed: u64,
    /// Final result value produced by the finish node.
    pub result: SlotValue,
    /// Taint of the result value.
    pub result_taint: Taint,
}

/// Marker type for a run that blocked on a suspension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedRun {
    /// Run identifier.
    pub run_id: RunId,
    /// Program counter at the blocking step.
    pub pc: StepIdx,
    /// Number of executed transitions before blocking.
    pub executed: u64,
    /// Step index that blocked.
    pub blocked_step: StepIdx,
    /// Kind of suspension (action, ask, wait_until, etc.).
    pub block_kind: BlockKind,
}

/// Kind of suspension that caused a run to block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockKind {
    /// Blocked on a Do action.
    Action,
    /// Blocked on an Ask prompt.
    Ask,
    /// Blocked on a WaitUntil timer.
    WaitUntil,
    /// Blocked on a WaitEvent.
    WaitEvent,
}

/// Marker type for a run that terminated with a typed error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRun {
    /// Run identifier.
    pub run_id: RunId,
    /// Program counter at error detection.
    pub pc: StepIdx,
    /// Number of executed transitions before error.
    pub executed: u64,
    /// Step index where the error originated.
    pub error_step: StepIdx,
    /// Error class for parity comparison.
    pub error_class: ErrorClass,
}

/// Classifies errors for parity comparison.
///
/// These map to the BDD error scenario variants:
/// - `MissingSlot` → B-007 / Missing slot error
/// - `DivByZero` → B-007 / Divide by zero
/// - `TypeMismatch` → B-007 / Type mismatch
/// - `BudgetExhausted` → B-007 / Budget exhaustion
/// - `Other` → Catch-all for unspecified errors
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorClass {
    /// Slot index was out of bounds.
    MissingSlot,
    /// Division by zero was attempted.
    DivByZero,
    /// Type mismatch in expression evaluation.
    TypeMismatch,
    /// Step budget was exhausted.
    BudgetExhausted,
    /// Some other error class.
    Other,
}

/// Terminal status of a workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalStatus {
    /// Workflow reached a finish node and produced a result.
    Finished(FinishedRun),
    /// Workflow blocked on a suspension primitive.
    Blocked(BlockedRun),
    /// Workflow terminated with a typed error.
    Error(ErrorRun),
}

/// Complete observed state of a single workflow run.
///
/// This type captures all observable dimensions needed for BDD parity comparison:
/// - Terminal status (finished, blocked, or error)
/// - Journal event sequence in emission order
/// - Final slot values and taints for every written slot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRun {
    /// Terminal status of the run.
    pub status: TerminalStatus,
    /// Journal events recorded during the run, in emission order.
    pub journal: Vec<ActionJournalEvent>,
    /// Slot values at terminal state, indexed by slot.
    pub slots: Vec<(SlotIdx, SlotValue)>,
    /// Taint markers at terminal state, indexed by slot.
    pub taints: Vec<(SlotIdx, Taint)>,
    /// Total number of journal events.
    pub journal_len: usize,
    /// Whether this run used the generated Rust path.
    pub is_generated: bool,
}

/// Parity mismatch variants detected during comparison.
///
/// Each variant carries the minimum context needed to identify and report
/// the specific behavioral divergence in BDD scenario output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParityError {
    /// Terminal status variant or fields differ.
    TerminalMismatch {
        /// IR terminal status summary.
        ir_status: String,
        /// Generated terminal status summary.
        gen_status: String,
        /// Detailed field-level differences.
        detail: String,
    },
    /// Journal event sequence differs in length or content.
    JournalMismatch {
        /// Event index where the first difference occurs.
        first_diff_index: usize,
        /// IR event at the differing index, if any.
        ir_event: Option<String>,
        /// Generated event at the differing index, if any.
        gen_event: Option<String>,
        /// Human-readable mismatch description.
        detail: String,
    },
    /// Slot value differs at a written slot.
    SlotValueMismatch {
        /// Slot index with differing value.
        slot: SlotIdx,
        /// IR slot value.
        ir_value: SlotValue,
        /// Generated slot value.
        gen_value: SlotValue,
    },
    /// Slot value collections differ in length.
    SlotCountMismatch {
        /// Number of IR slot values.
        ir_len: usize,
        /// Number of generated-runtime slot values.
        gen_len: usize,
    },
    /// Taint marker differs at a written slot.
    TaintMismatch {
        /// Slot index with differing taint.
        slot: SlotIdx,
        /// IR taint value.
        ir_taint: Taint,
        /// Generated taint value.
        gen_taint: Taint,
    },
    /// Suspension boundary metadata differs.
    SuspensionMismatch {
        /// Step index of the blocked node.
        step: StepIdx,
        /// Suspension kind reported by IR.
        ir_kind: String,
        /// Suspension kind reported by generated.
        gen_kind: String,
        /// Detailed suspension field differences.
        detail: String,
    },
    /// Resume input or output differs after resumption.
    ResumeMismatch {
        /// Step index of the resumed node.
        step: StepIdx,
        /// IR resume result summary.
        ir_result: String,
        /// Generated resume result summary.
        gen_result: String,
        /// Detailed resume differences.
        detail: String,
    },
    /// Workflow contains an unsupported feature and cannot be compared.
    UnsupportedMismatch {
        /// Description of the unsupported feature.
        feature: String,
    },
}

impl ParityError {
    /// Returns a short label suitable for BDD scenario naming.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::TerminalMismatch { .. } => "terminal_mismatch",
            Self::JournalMismatch { .. } => "journal_mismatch",
            Self::SlotValueMismatch { .. } => "slot_value_mismatch",
            Self::SlotCountMismatch { .. } => "slot_count_mismatch",
            Self::TaintMismatch { .. } => "taint_mismatch",
            Self::SuspensionMismatch { .. } => "suspension_mismatch",
            Self::ResumeMismatch { .. } => "resume_mismatch",
            Self::UnsupportedMismatch { .. } => "unsupported_mismatch",
        }
    }
}

/// Compares two observed runs for behavioral parity.
///
/// Returns `Ok(())` when the IR interpreter run and generated runtime run
/// produce identical observable behavior across all dimensions:
/// - Terminal status (finished, blocked, error)
/// - Journal event sequence (order, kind, fields)
/// - Slot values at every written slot
/// - Taint markers at every written slot
///
/// Returns `Err(ParityError)` describing the first detected mismatch,
/// using the comparison order: terminal status → journal → taints.
///
/// # Parity Dimensions
///
/// 1. **Terminal parity (B-001, B-013)**: Status variant, result value,
///    result taint, final PC, executed step count all match.
/// 2. **Journal parity (B-002)**: Event count and each event's kind and
///    fields match in order.
/// 3. **Taint parity (B-003)**: Every written slot has identical taint
///    in both runs.
/// 4. **Suspension parity (B-004, B-005)**: Block kind and metadata
///    match; PC does not advance past the blocked step.
/// 5. **Resume parity (B-006)**: Identical resume input produces identical
///    output slot write, completion event, and PC advance.
/// 6. **Unsupported (B-008–B-010)**: Unsupported workflows are classified
///    as unsupported, not as parity failures.
pub fn compare_observed_runs(ir: &ObservedRun, gen_run: &ObservedRun) -> Result<(), ParityError> {
    compare_terminal_status(&ir.status, &gen_run.status)?;
    compare_journals(&ir.journal, &gen_run.journal)?;
    compare_slots(&ir.slots, &gen_run.slots)?;
    compare_taints(&ir.taints, &gen_run.taints)?;
    Ok(())
}

/// Compares terminal status between two runs.
fn compare_terminal_status(
    ir: &TerminalStatus,
    gen_run: &TerminalStatus,
) -> Result<(), ParityError> {
    match (ir, gen_run) {
        (TerminalStatus::Finished(ir_f), TerminalStatus::Finished(gen_f)) => {
            if ir_f.result != gen_f.result {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Finished {{ result: {:?} }}", ir_f.result),
                    gen_status: format!("Finished {{ result: {:?} }}", gen_f.result),
                    detail: format!(
                        "result mismatch: ir={:?}, gen_run={:?}",
                        ir_f.result, gen_f.result
                    ),
                });
            }
            if ir_f.result_taint != gen_f.result_taint {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Finished {{ taint: {:?} }}", ir_f.result_taint),
                    gen_status: format!("Finished {{ taint: {:?} }}", gen_f.result_taint),
                    detail: format!(
                        "result taint mismatch: ir={:?}, gen_run={:?}",
                        ir_f.result_taint, gen_f.result_taint
                    ),
                });
            }
            if ir_f.pc != gen_f.pc {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Finished {{ pc: {:?} }}", ir_f.pc),
                    gen_status: format!("Finished {{ pc: {:?} }}", gen_f.pc),
                    detail: format!("pc mismatch: ir={:?}, gen_run={:?}", ir_f.pc, gen_f.pc),
                });
            }
            if ir_f.executed != gen_f.executed {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Finished {{ executed: {} }}", ir_f.executed),
                    gen_status: format!("Finished {{ executed: {} }}", gen_f.executed),
                    detail: format!(
                        "executed count mismatch: ir={}, gen_run={}",
                        ir_f.executed, gen_f.executed
                    ),
                });
            }
            Ok(())
        }
        (TerminalStatus::Blocked(ir_b), TerminalStatus::Blocked(gen_b)) => {
            if ir_b.blocked_step != gen_b.blocked_step {
                return Err(ParityError::SuspensionMismatch {
                    step: ir_b.blocked_step,
                    ir_kind: format!("{:?}", ir_b.block_kind),
                    gen_kind: format!("{:?}", gen_b.block_kind),
                    detail: format!(
                        "blocked step mismatch: ir={:?}, gen_run={:?}",
                        ir_b.blocked_step, gen_b.blocked_step
                    ),
                });
            }
            if ir_b.block_kind != gen_b.block_kind {
                return Err(ParityError::SuspensionMismatch {
                    step: ir_b.blocked_step,
                    ir_kind: format!("{:?}", ir_b.block_kind),
                    gen_kind: format!("{:?}", gen_b.block_kind),
                    detail: format!(
                        "block kind mismatch: ir={:?}, gen_run={:?}",
                        ir_b.block_kind, gen_b.block_kind
                    ),
                });
            }
            if ir_b.pc != gen_b.pc {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Blocked {{ pc: {:?} }}", ir_b.pc),
                    gen_status: format!("Blocked {{ pc: {:?} }}", gen_b.pc),
                    detail: format!(
                        "blocked pc mismatch: ir={:?}, gen_run={:?}",
                        ir_b.pc, gen_b.pc
                    ),
                });
            }
            Ok(())
        }
        (TerminalStatus::Error(ir_e), TerminalStatus::Error(gen_e)) => {
            if ir_e.error_class != gen_e.error_class {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Error {{ class: {:?} }}", ir_e.error_class),
                    gen_status: format!("Error {{ class: {:?} }}", gen_e.error_class),
                    detail: format!(
                        "error class mismatch: ir={:?}, gen_run={:?}",
                        ir_e.error_class, gen_e.error_class
                    ),
                });
            }
            if ir_e.error_step != gen_e.error_step {
                return Err(ParityError::TerminalMismatch {
                    ir_status: format!("Error {{ step: {:?} }}", ir_e.error_step),
                    gen_status: format!("Error {{ step: {:?} }}", ir_e.error_step),
                    detail: format!(
                        "error step mismatch: ir={:?}, gen_run={:?}",
                        ir_e.error_step, gen_e.error_step
                    ),
                });
            }
            Ok(())
        }
        (ir_status, gen_status) => {
            let ir_label = status_label(ir_status);
            let gen_label = status_label(gen_status);
            let detail = format!(
                "status variant mismatch: ir={}, gen_run={}",
                ir_label, gen_label
            );
            Err(ParityError::TerminalMismatch {
                ir_status: ir_label,
                gen_status: gen_label,
                detail,
            })
        }
    }
}

/// Returns a short string label for a terminal status.
fn status_label(status: &TerminalStatus) -> String {
    match status {
        TerminalStatus::Finished(_) => "Finished".into(),
        TerminalStatus::Blocked(b) => format!("Blocked({:?})", b.block_kind),
        TerminalStatus::Error(e) => format!("Error({:?})", e.error_class),
    }
}

/// Compares journal event sequences between two runs.
fn compare_journals(
    ir: &[ActionJournalEvent],
    gen_run: &[ActionJournalEvent],
) -> Result<(), ParityError> {
    if ir.len() != gen_run.len() {
        return Err(ParityError::JournalMismatch {
            first_diff_index: 0,
            ir_event: None,
            gen_event: None,
            detail: format!(
                "journal length mismatch: ir_len={}, gen_len={}",
                ir.len(),
                gen_run.len()
            ),
        });
    }
    for (i, (ir_e, gen_e)) in ir.iter().zip(gen_run.iter()).enumerate() {
        if ir_e != gen_e {
            return Err(ParityError::JournalMismatch {
                first_diff_index: i,
                ir_event: Some(format!("{:?}", ir_e)),
                gen_event: Some(format!("{:?}", gen_e)),
                detail: format!(
                    "journal event {} mismatch: ir={:?}, gen_run={:?}",
                    i, ir_e, gen_e
                ),
            });
        }
    }
    Ok(())
}

/// Compares slot values between two runs.
fn compare_slots(
    ir: &[(SlotIdx, SlotValue)],
    gen_run: &[(SlotIdx, SlotValue)],
) -> Result<(), ParityError> {
    if ir.len() != gen_run.len() {
        return Err(ParityError::SlotCountMismatch {
            ir_len: ir.len(),
            gen_len: gen_run.len(),
        });
    }
    for ((ir_slot, ir_value), (gen_slot, gen_value)) in ir.iter().zip(gen_run.iter()) {
        if ir_slot != gen_slot {
            return Err(ParityError::SlotValueMismatch {
                slot: *ir_slot,
                ir_value: *ir_value,
                gen_value: *gen_value,
            });
        }
        if ir_value != gen_value {
            return Err(ParityError::SlotValueMismatch {
                slot: *ir_slot,
                ir_value: *ir_value,
                gen_value: *gen_value,
            });
        }
    }
    Ok(())
}

/// Compares taint markers between two runs.
fn compare_taints(
    ir: &[(SlotIdx, Taint)],
    gen_run: &[(SlotIdx, Taint)],
) -> Result<(), ParityError> {
    if ir.len() != gen_run.len() {
        return Err(ParityError::JournalMismatch {
            first_diff_index: 0,
            ir_event: None,
            gen_event: None,
            detail: format!(
                "taint slot count mismatch: ir_len={}, gen_len={}",
                ir.len(),
                gen_run.len()
            ),
        });
    }
    for ((ir_slot, ir_taint), (gen_slot, gen_taint)) in ir.iter().zip(gen_run.iter()) {
        if ir_slot != gen_slot {
            return Err(ParityError::TaintMismatch {
                slot: *ir_slot,
                ir_taint: *ir_taint,
                gen_taint: *gen_taint,
            });
        }
        if ir_taint != gen_taint {
            return Err(ParityError::TaintMismatch {
                slot: *ir_slot,
                ir_taint: *ir_taint,
                gen_taint: *gen_taint,
            });
        }
    }
    Ok(())
}
