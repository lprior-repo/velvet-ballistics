//! Resource contract emission for code generation.

use std::fmt::Write;
use crate::{CodegenResult, fmt_err};
use vb_core::ResourceContract;

pub(crate) fn emit_resource_contract(out: &mut String, contract: ResourceContract) -> CodegenResult<()> {
    writeln!(out, "// --- Resource contract ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_STEPS: u16 = {};",
        contract.max_steps
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_SLOTS: u16 = {};",
        contract.max_slots
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_CONSTANTS: u16 = {};",
        contract.max_constants
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_ACCESSORS: u16 = {};",
        contract.max_accessors
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_EXPRESSIONS: u16 = {};",
        contract.max_expressions
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_EXPR_STACK: u8 = {};",
        contract.max_expr_stack
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_INPUT_BYTES: u32 = {};",
        contract.max_input_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_OUTPUT_BYTES: u32 = {};",
        contract.max_output_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_STEP_BUDGET_PER_TICK: u64 = {};",
        contract.max_step_budget_per_tick
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_BLOB_BYTES: u64 = {};",
        contract.max_blob_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_IPC_PAYLOAD_BYTES: u32 = {};",
        contract.max_ipc_payload_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_RETRY_ATTEMPTS: u16 = {};",
        contract.max_retry_attempts
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_FANOUT: u16 = {};",
        contract.max_fanout
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_COLLECT_ITEMS: u32 = {};",
        contract.max_collect_items
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_QUEUE_DEPTH: u32 = {};",
        contract.max_queue_depth
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_JOURNAL_BATCH_BYTES: u32 = {};",
        contract.max_journal_batch_bytes
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

// Emit a trybuild compile-fail test fixture for the generated code.
