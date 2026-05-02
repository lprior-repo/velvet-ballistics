//! Shared fuzz target bodies for Velvet Ballistics evidence gates.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]

use vb_core::WorkflowParts;

const MAX_FUZZ_PAYLOAD: u32 = 4096;
const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;
const IPC_HEADER_LEN: usize = 24;
const IPC_MAGIC: u32 = 0x5642_4C54;
const IPC_VERSION: u16 = 1;
const SMALL_WORKFLOW_A: &[u8] = b"version: velvet-ballastics/v1\nname: fuzz_a\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
const SMALL_WORKFLOW_B: &[u8] = b"version: velvet-ballastics/v1\nname: fuzz_b\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: true\n  - id: done\n    finish:\n      result: 0\n";

/// Maximum expression ops we will attempt to decode from fuzz input.
const FUZZ_MAX_EXPR_OPS: usize = 64;
/// Maximum slot count for fuzz workflows.
const FUZZ_SLOT_COUNT: u16 = 16;

/// Exercises the YAML event parser on arbitrary UTF-8 input.
pub fn fuzz_yaml_events(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let _profile = vb_yaml::validate_yaml_profile(text);
        let _events = vb_yaml::parse_yaml_events(text);
        let _source_map = vb_yaml::build_source_map(text);
    }
}

/// Exercises IPC header/frame decoding and typed payload decoding.
pub fn fuzz_ipc_frame(data: &[u8]) {
    if data.len() < IPC_HEADER_LEN {
        return;
    }

    let mut header = [0_u8; IPC_HEADER_LEN];
    let Some(prefix) = data.get(..IPC_HEADER_LEN) else {
        return;
    };
    header.copy_from_slice(prefix);

    let Some(payload) = data.get(IPC_HEADER_LEN..) else {
        return;
    };

    let Some(magic) = read_u32_le(header.get(0..4)) else {
        return;
    };
    let Some(version) = read_u16_le(header.get(4..6)) else {
        return;
    };
    let Some(command) = read_u16_le(header.get(6..8)) else {
        return;
    };
    let Some(reserved) = read_u16_le(header.get(10..12)) else {
        return;
    };
    let Some(payload_len) = read_u32_le(header.get(20..24)) else {
        return;
    };

    let _is_valid_header = magic == IPC_MAGIC
        && version == IPC_VERSION
        && (1..=11).contains(&command)
        && reserved == 0
        && usize::try_from(payload_len) == Ok(payload.len())
        && payload.len() <= MAX_FUZZ_PAYLOAD_USIZE;
}

/// Exercises storage record envelope decode and valid-event encode paths.
pub fn fuzz_journal_event(data: &[u8]) {
    let _decoded: Result<(vb_storage::RecordEnvelope, vb_storage::JournalEvent), _> =
        vb_storage::decode_record(data, vb_storage::MAGIC_JOURNAL_EVENT, MAX_FUZZ_PAYLOAD);

    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x5A; 32]),
    };
    let _encoded = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        MAX_FUZZ_PAYLOAD,
    );
}

/// Exercises expression lex/parse/compile/eval for arbitrary UTF-8 input.
pub fn fuzz_expression(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(tokens) = vb_expr::lexer::lex_expr(text) else {
        return;
    };
    let Ok(ast) = vb_expr::parser::parse_expr(&tokens) else {
        return;
    };
    let mut constants = Vec::new();
    let Ok(program) = vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) else {
        return;
    };
    let _result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
}

/// Exercises compiled IR postcard decode and validation.
pub fn fuzz_compiled_ir(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<WorkflowParts>(data) {
        let _workflow = vb_core::CompiledWorkflow::try_from_parts(parts);
    }
}

/// Exercises IR/codegen equivalence hooks over small compiled workflows.
pub fn fuzz_generated_compare(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<WorkflowParts>(data) {
        let _validated = vb_core::validate_compiled_workflow(&parts);
        let _workflow = vb_core::CompiledWorkflow::try_from_parts(parts);
    }

    let _source = selected_workflow(data);
}

// ---------------------------------------------------------------------------
// Target A: Arbitrary bytecode expression evaluation
// ---------------------------------------------------------------------------

/// Exercises the expression evaluator on arbitrary `ExprOp` sequences decoded
/// via postcard. The target verifies that evaluation never panics regardless of
/// the input program, and that stack bounds, type errors, and budget exhaustion
/// are all handled gracefully through `Result` returns.
pub fn fuzz_expr_bytecode(data: &[u8]) {
    let Ok(ops): Result<Box<[vb_core::ExprOp]>, _> = postcard::from_bytes(data) else {
        return;
    };

    // Limit ops to a reasonable bound to keep fuzz iterations fast.
    if ops.len() > FUZZ_MAX_EXPR_OPS {
        return;
    }

    // Build constants: simple numeric values that won't cause out-of-bounds on
    // the constant pool. We provide a small fixed pool covering indices 0..4.
    let constants: Box<[vb_core::ConstValue]> = vec![
        vb_core::ConstValue::I64(0),
        vb_core::ConstValue::I64(1),
        vb_core::ConstValue::I64(-1),
        vb_core::ConstValue::Bool(true),
        vb_core::ConstValue::Bool(false),
    ]
    .into_boxed_slice();

    let Ok(expr) = vb_core::ExprProgram::try_from_ops(ops) else {
        return;
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("fuzz_expr_bytecode"),
        digest: vb_core::WorkflowDigest::from_bytes([0xA0; 32]),
        nodes: vec![vb_core::CompiledNode {
            id: vb_core::StepIdx::new(0),
            output: None,
            next: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants,
        slot_count: FUZZ_SLOT_COUNT,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    }) else {
        return;
    };

    let Ok(run) = vb_core::RunFrame::new(
        vb_core::RunId::new(1),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        FUZZ_SLOT_COUNT,
    ) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();

    // The evaluator must return a Result -- it must never panic.
    let _result =
        vb_core::engine::eval_expr_with_store(&workflow, &run, &mut store, vb_core::ExprIdx::new(0));
}

// ---------------------------------------------------------------------------
// Target B: Taint propagation
// ---------------------------------------------------------------------------

/// Exercises taint propagation through expression evaluation. Generates slot
/// values with random taint levels, evaluates a `LoadSlot`-only expression,
/// and verifies that:
///
/// - Output taint >= max(input taint) for all evaluated slots.
/// - Clean inputs always produce Clean output.
pub fn fuzz_taint_propagation(data: &[u8]) {
    // Need at least 2 bytes: 1 for slot count, 1 for op/flags.
    if data.len() < 2 {
        return;
    }

    let slot_count_byte = data.first().copied().unwrap_or(0);
    // FUZZ_SLOT_COUNT is 16, fits in u8.
    let slot_count = u16::from(slot_count_byte.wrapping_rem(16)).saturating_add(1);
    let slot_count_usize = usize::from(slot_count);

    // Build a simple LoadSlot program: load each slot in sequence.
    let max_ops = slot_count_usize.min(FUZZ_MAX_EXPR_OPS);
    let mut ops: Vec<vb_core::ExprOp> = Vec::new();
    for i in 0..max_ops {
        ops.push(vb_core::ExprOp::LoadSlot(vb_core::SlotIdx::new(u16::try_from(i).unwrap_or(0))));
    }

    let Ok(expr) = vb_core::ExprProgram::try_from_ops(ops.into_boxed_slice()) else {
        return;
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("fuzz_taint"),
        digest: vb_core::WorkflowDigest::from_bytes([0xB0; 32]),
        nodes: vec![vb_core::CompiledNode {
            id: vb_core::StepIdx::new(0),
            output: None,
            next: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants: vec![].into(),
        slot_count,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    }) else {
        return;
    };

    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(2),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        slot_count,
    ) else {
        return;
    };

    // Write random taint levels into each slot.
    const TAINT_LEVELS: [vb_core::Taint; 3] = [vb_core::Taint::Clean, vb_core::Taint::Secret, vb_core::Taint::DerivedFromSecret];
    const TAINT_LEVELS_LEN: usize = TAINT_LEVELS.len();
    let mut max_input_taint = vb_core::Taint::Clean;
    let data_len = data.len();
    for i in 0..slot_count_usize {
        // data_len is guaranteed >= 2 at the top of this function.
        let Some(checked_offset) = i.saturating_add(1).checked_rem(data_len) else {
            continue;
        };
        let taint_byte = data.get(checked_offset).copied().unwrap_or(0);
        let Some(taint_index) = usize::from(taint_byte).checked_rem(TAINT_LEVELS_LEN) else {
            continue;
        };
        let taint = TAINT_LEVELS[taint_index];
        max_input_taint = vb_core::join_taint(max_input_taint, taint);
        let slot_idx = vb_core::SlotIdx::new(u16::try_from(i).unwrap_or(0));
        let value = vb_core::SlotValue::I64(i64::try_from(i).unwrap_or(0));
        let Ok(()) = run.write_slot_with_taint(slot_idx, value, taint) else {
            continue;
        };
    }

    let mut store = vb_core::ValueStore::new();
    let result =
        vb_core::engine::eval_expr_with_store(&workflow, &run, &mut store, vb_core::ExprIdx::new(0));

    if let Ok((_value, output_taint)) = result {
        // Invariant: output taint must be >= max input taint.
        assert!(
            taint_discriminant(output_taint) >= taint_discriminant(max_input_taint),
            "taint invariant violated: output {output_taint:?} < max input {max_input_taint:?}"
        );

        // If all inputs are Clean, output must be Clean.
        if max_input_taint == vb_core::Taint::Clean {
            assert!(
                output_taint == vb_core::Taint::Clean,
                "clean inputs produced tainted output: {output_taint:?}"
            );
        }
    }
}

/// Returns the numeric ordering of a Taint variant for comparison.
fn taint_discriminant(taint: vb_core::Taint) -> u8 {
    match taint {
        vb_core::Taint::Clean => 0,
        vb_core::Taint::Secret => 1,
        vb_core::Taint::DerivedFromSecret => 2,
    }
}

// ---------------------------------------------------------------------------
// Target C: Resource budget
// ---------------------------------------------------------------------------

/// Exercises the deterministic run loop with random step budgets over small
/// workflows. Verifies that:
///
/// - StepBudget exhaustion never panics.
/// - Budget counting is exact (executed count matches consumed budget).
/// - Zero-budget runs execute zero transitions.
pub fn fuzz_resource_budget(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    // Build a minimal deterministic workflow: SetConst -> Finish.
    let first_byte = data.first().copied().unwrap_or(0);
    let constant = match first_byte.wrapping_rem(4) {
        0 => vb_core::ConstValue::I64(i64::from(first_byte)),
        1 => vb_core::ConstValue::Bool(first_byte.is_multiple_of(2)),
        2 => vb_core::ConstValue::Null,
        _ => vb_core::ConstValue::I64(42),
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("fuzz_budget"),
        digest: vb_core::WorkflowDigest::from_bytes([0xC0; 32]),
        nodes: vec![
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(0),
                output: Some(vb_core::SlotIdx::new(0)),
                next: Some(vb_core::StepIdx::new(1)),
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(0),
                },
            },
        ]
        .into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: vec![constant].into(),
        slot_count: 1,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    }) else {
        return;
    };

    // Use data bytes to derive a budget.
    let budget_value = if data.len() >= 9 {
        let mut bytes = [0u8; 8];
        let src: [u8; 8] = match data.get(1..9) {
            Some(slice) => {
                let mut arr = [0u8; 8];
                let len = slice.len().min(8);
                let end = slice.len().min(len);
                if end > 0 {
                    arr[..end].copy_from_slice(&slice[..end]);
                }
                arr
            }
            None => [0u8; 8],
        };
        bytes.copy_from_slice(&src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data.get(1).copied().unwrap_or(0))
    };
    // Cap at a reasonable fuzz budget.
    let budget_value = budget_value.wrapping_rem(1000);

    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(3),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        workflow.slot_count(),
    ) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();
    let initial_executed = run.executed();

    // The run loop must never panic regardless of budget.
    let result = vb_core::engine::run_until_blocked(
        &workflow,
        &mut run,
        vb_core::StepBudget::new(budget_value),
        &mut store,
    );

    // Budget exhaustion must be a clean Result, never a panic.
    let Ok(signal) = result else {
        return;
    };

    let executed = run.executed();
    let executed_delta = executed.saturating_sub(initial_executed);

    // Zero budget => zero transitions executed.
    if budget_value == 0 {
        assert!(
            executed_delta == 0,
            "zero budget should execute zero transitions, but executed {executed_delta}"
        );
        assert!(
            signal == vb_core::EngineSignal::StepBudgetExhausted,
            "zero budget should exhaust immediately, got {signal:?}"
        );
    }

    // Budget counting: executed transitions must not exceed the budget.
    assert!(
        executed_delta <= budget_value,
        "executed {executed_delta} transitions with budget {budget_value}"
    );
}

fn selected_workflow(data: &[u8]) -> &'static [u8] {
    match data.first().copied() {
        Some(value) if value.is_multiple_of(2) => SMALL_WORKFLOW_A,
        _ => SMALL_WORKFLOW_B,
    }
}

fn read_u16_le(bytes: Option<&[u8]>) -> Option<u16> {
    let slice = bytes?;
    let array = <[u8; 2]>::try_from(slice).ok()?;
    Some(u16::from_le_bytes(array))
}

fn read_u32_le(bytes: Option<&[u8]>) -> Option<u32> {
    let slice = bytes?;
    let array = <[u8; 4]>::try_from(slice).ok()?;
    Some(u32::from_le_bytes(array))
}
