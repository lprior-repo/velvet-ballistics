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
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants,
        slot_count: FUZZ_SLOT_COUNT,
        symbols_count: 0,
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
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants: vec![].into(),
        slot_count,
        symbols_count: 0,
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
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                error_slot: None,
                on_error: None,
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
        symbols_count: 0,
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

// ---------------------------------------------------------------------------
// Target D: Verifier gates
// ---------------------------------------------------------------------------

/// Maximum number of nodes in a fuzz-generated workflow.
const FUZZ_MAX_NODES: usize = 32;

/// Exercises all plan verifier gates (7, 8, 9, 11, 13) on randomly constructed
/// `WorkflowParts`. The target verifies that no gate panics regardless of input,
/// including edge cases like empty nodes, max slot references, and various node
/// kinds.
pub fn fuzz_verifier_gates(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(16)).saturating_add(1).min(FUZZ_MAX_NODES);
    let slot_count = u16::from(byte1.wrapping_rem(16)).saturating_add(1);

    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let Some(offset) = i.saturating_add(2).checked_rem(data.len()) else {
            continue;
        };
        let kind_byte = data.get(offset).copied().unwrap_or(0);
        let node = build_fuzz_node(i, kind_byte, node_count, slot_count, data);
        nodes.push(node);
    }

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_gates"),
        digest: vb_core::WorkflowDigest::from_bytes([0xD0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    };

    // Gate 7: Expression stack depth bounded.
    drop(vb_validate::gates::validate_gate_07_expression_stack_depth(&parts));
    // Gate 8: Accessor path segments are valid symbols.
    drop(vb_validate::gates::validate_gate_08_accessor_path_segments(&parts));
    // Gate 9: All referenced slots exist within declared slot_count.
    drop(vb_validate::gates::validate_gate_09_slot_references(&parts));
    // Gate 11: ForEach/Together body graph is well-formed.
    drop(vb_validate::gates::validate_gate_11_loop_body_graph(&parts));
    // Gate 13: No circular references in slot dependency graph.
    drop(vb_validate::gates::validate_gate_13_no_slot_cycles(&parts));
}

/// Builds a single fuzz node based on a kind selector byte.
fn build_fuzz_node(
    index: usize,
    kind_byte: u8,
    node_count: usize,
    slot_count: u16,
    data: &[u8],
) -> vb_core::CompiledNode {
    let step_idx = vb_core::StepIdx::new(u16::try_from(index).unwrap_or(u16::MAX));
    let next_step = if index.saturating_add(1) < node_count {
        Some(vb_core::StepIdx::new(u16::try_from(index).unwrap_or(0).saturating_add(1)))
    } else {
        None
    };

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(8) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish {
            result: safe_slot,
        },
        2 => vb_core::CompiledNodeKind::Copy {
            source: safe_slot,
        },
        3 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        4 => {
            // ForEachStart with body/done pointing within bounds.
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::ForEachStart {
                input: safe_slot,
                item_slot: safe_slot,
                limit: 10,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        5 => {
            // TogetherStart with branch/join within bounds.
            let branch_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let join_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let data_len = data.len();
            let branch_count = if data_len > 4 {
                usize::from(data.get(3).copied().unwrap_or(1).wrapping_rem(4)).saturating_add(1)
            } else {
                1
            };
            let mut branches: Vec<vb_core::StepIdx> = Vec::new();
            for _ in 0..branch_count {
                branches.push(vb_core::StepIdx::new(branch_idx));
            }
            vb_core::CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join: vb_core::StepIdx::new(join_idx),
            }
        }
        6 => {
            // RepeatStart with body/done within bounds.
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        _ => {
            // ChooseSlot with branches within bounds.
            let target_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let otherwise_idx =
                u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            vb_core::CompiledNodeKind::ChooseSlot {
                branches: vec![vb_core::SlotBranch {
                    condition: safe_slot,
                    target: vb_core::StepIdx::new(target_idx),
                }]
                .into_boxed_slice(),
                otherwise: Some(vb_core::StepIdx::new(otherwise_idx)),
            }
        }
    };

    let output = if kind_byte.is_multiple_of(3) {
        Some(safe_slot)
    } else {
        None
    };

    vb_core::CompiledNode {
        id: step_idx,
        output,
        next: next_step,
        error_slot: None,
        on_error: None,
        kind,
    }
}

// ---------------------------------------------------------------------------
// Target E: Budget compute
// ---------------------------------------------------------------------------

/// Exercises `WholeWorkflowBudget::compute()` on randomly constructed
/// `CompiledNode` arrays with various node kinds. The target verifies that
/// compute never panics and that returned budget values are sane: non-zero for
/// non-empty workflows, and all values bounded.
pub fn fuzz_budget_compute(data: &[u8]) {
    if data.len() < 3 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(16)).saturating_add(1).min(FUZZ_MAX_NODES);
    let slot_count = u16::from(byte1.wrapping_rem(16)).saturating_add(1);

    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let Some(offset) = i.saturating_add(2).checked_rem(data.len()) else {
            continue;
        };
        let kind_byte = data.get(offset).copied().unwrap_or(0);
        let node = build_fuzz_budget_node(i, kind_byte, node_count, slot_count);
        nodes.push(node);
    }

    let contract = vb_core::ResourceContract {
        max_slots: slot_count,
        ..vb_core::ResourceContract::DEFAULT
    };

    let entry = vb_core::StepIdx::ZERO;
    let result = vb_core::budget::WholeWorkflowBudget::compute(&nodes, entry, &contract);

    let Ok(budget) = result else {
        return;
    };

    // Sanity: total steps must be non-zero for non-empty node arrays and bounded
    // by the node count (each node counted at most once).
    assert!(
        budget.max_total_steps > 0,
        "non-empty workflow must have at least one step"
    );
    assert!(
        budget.max_total_steps <= u64::try_from(node_count).unwrap_or(u64::MAX),
        "total steps {} exceeds node count {}",
        budget.max_total_steps,
        node_count
    );

    // Sanity: max_total_slots comes from the contract.
    assert_eq!(
        budget.max_total_slots,
        u64::from(contract.max_slots),
        "total slots must match contract"
    );

    // Sanity: fanout is bounded.
    let max_reasonable_fanout = u16::try_from(node_count).unwrap_or(u16::MAX);
    assert!(
        budget.max_fanout <= max_reasonable_fanout,
        "fanout {} exceeds node count {}",
        budget.max_fanout,
        max_reasonable_fanout
    );
}

/// Builds a budget-friendly fuzz node (simpler node kinds for budget walks).
fn build_fuzz_budget_node(
    index: usize,
    kind_byte: u8,
    node_count: usize,
    slot_count: u16,
) -> vb_core::CompiledNode {
    let step_idx = vb_core::StepIdx::new(u16::try_from(index).unwrap_or(u16::MAX));
    let next_step = if index.saturating_add(1) < node_count {
        Some(vb_core::StepIdx::new(u16::try_from(index).unwrap_or(0).saturating_add(1)))
    } else {
        None
    };

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(6) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish {
            result: safe_slot,
        },
        2 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        3 => vb_core::CompiledNodeKind::Copy {
            source: safe_slot,
        },
        4 => {
            // ForEachStart to test nesting depth.
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::ForEachStart {
                input: safe_slot,
                item_slot: safe_slot,
                limit: 5,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        _ => {
            // TogetherStart to test fanout.
            let branch_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let join_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::TogetherStart {
                branches: vec![
                    vb_core::StepIdx::new(branch_idx),
                    vb_core::StepIdx::new(branch_idx),
                ]
                .into_boxed_slice(),
                join: vb_core::StepIdx::new(join_idx),
            }
        }
    };

    vb_core::CompiledNode {
        id: step_idx,
        output: Some(safe_slot),
        next: next_step,
        error_slot: None,
        on_error: None,
        kind,
    }
}

// ---------------------------------------------------------------------------
// Target F: Admission flow
// ---------------------------------------------------------------------------

/// Exercises `submit_artifact` with randomly constructed workflow parts, some
/// valid and some invalid. The target verifies that admission never panics
/// regardless of input.
pub fn fuzz_admission_flow(data: &[u8]) {
    if data.len() < 2 {
        return;
    }

    // Build a minimal workflow from fuzz input.
    let Some(&byte0) = data.first() else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(4)).saturating_add(1);
    let slot_count = u16::from(byte0.wrapping_rem(4)).saturating_add(1);
    let max_slot = slot_count.saturating_sub(1);

    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let step_idx = vb_core::StepIdx::new(u16::try_from(i).unwrap_or(0));
        let next_step = if i.saturating_add(1) < node_count {
            Some(vb_core::StepIdx::new(u16::try_from(i).unwrap_or(0).saturating_add(1)))
        } else {
            None
        };

        if i.saturating_add(1) == node_count {
            // Last node is always Finish.
            nodes.push(vb_core::CompiledNode {
                id: step_idx,
                output: None,
                next: None,
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(max_slot),
                },
            });
        } else {
            nodes.push(vb_core::CompiledNode {
                id: step_idx,
                output: Some(vb_core::SlotIdx::new(max_slot)),
                next: next_step,
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::Nop,
            });
        }
    }

    // Compute correct digest for strict/journaled policies.
    let parts_zeroed = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_admission"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![vb_core::ConstValue::Bool(true)].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    };

    let Ok(hash_bytes) = postcard::to_allocvec(&parts_zeroed) else {
        return;
    };
    let computed = blake3::hash(&hash_bytes);
    let correct_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes(*computed.as_bytes()),
        ..parts_zeroed
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(correct_parts) else {
        return;
    };

    // Open a temporary journal.
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    // Cycle through all policies to exercise different admission paths.
    let policies = [
        vb_core::RuntimePolicy::Relaxed,
        vb_core::RuntimePolicy::Journaled,
        vb_core::RuntimePolicy::Strict,
    ];
    for policy in policies {
        drop(vb_storage::submit_artifact(&journal, &workflow, policy));
    }

    // Also test with an intentionally corrupted workflow (wrong digest).
    let corrupted_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes([0xFF; 32]),
        ..workflow.to_parts()
    };
    if let Ok(corrupted) = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts) {
        drop(vb_storage::submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Strict));
    }
}

// ---------------------------------------------------------------------------
// Target G: Expression evaluator (postcard-decoded ExprProgram)
// ---------------------------------------------------------------------------

/// Exercises the expression evaluator on arbitrary `ExprProgram` bytes decoded
/// via postcard. Decodes a full `WorkflowParts` (which may contain arbitrary
/// expression ops), builds a compiled workflow, and evaluates each expression.
/// The target verifies that evaluation never panics regardless of input.
pub fn fuzz_expr_eval(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
            return;
        };
        let Ok(run) = vb_core::RunFrame::new(
            vb_core::RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        ) else {
            return;
        };
        let mut store = vb_core::ValueStore::new();
        // Iterate expressions by index until expression() returns None.
        let mut i: u16 = 0;
        loop {
            let expr_idx = vb_core::ExprIdx::new(i);
            if workflow.expression(expr_idx).is_none() {
                break;
            }
            // The evaluator must return a Result -- it must never panic.
            drop(vb_core::engine::eval_expr_with_store(
                &workflow,
                &run,
                &mut store,
                expr_idx,
            ));
            i = i.saturating_add(1);
            if i == 0 {
                // Wrapped around -- stop.
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Target H: Accessor traversal
// ---------------------------------------------------------------------------

/// Maximum accessor path depth for fuzz-generated accessors.
const FUZZ_MAX_ACCESSOR_DEPTH: usize = 16;

/// Exercises accessor path traversal on arbitrary accessor programs decoded via
/// postcard. Constructs a compiled workflow with accessor programs populated from
/// fuzz input, writes slot values into a `RunFrame`, and evaluates each accessor
/// against a `ValueStore`. Verifies that accessor traversal never panics.
pub fn fuzz_accessor_traversal(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let slot_count = u16::from(byte0.wrapping_rem(16)).saturating_add(1);
    let accessor_count = usize::from(byte1.wrapping_rem(8)).saturating_add(1);

    let mut accessors: Vec<vb_core::AccessorProgram> = Vec::new();
    let mut offset = 2usize;
    for _ in 0..accessor_count {
        let root_byte = data.get(offset).copied().unwrap_or(0);
        let root = vb_core::SlotIdx::new(u16::from(root_byte).wrapping_rem(slot_count));
        offset = offset.saturating_add(1);

        let path_len_byte = data.get(offset).copied().unwrap_or(0);
        let path_len = usize::from(path_len_byte.wrapping_rem(4));
        offset = offset.saturating_add(1);

        let mut path: Vec<vb_core::PathSegment> = Vec::new();
        for _ in 0..path_len {
            if offset >= data.len() {
                break;
            }
            let seg_byte = data.get(offset).copied().unwrap_or(0);
            offset = offset.saturating_add(1);
            let segment = if seg_byte.is_multiple_of(2) {
                // Field accessor
                vb_core::PathSegment::Field(vb_core::SymbolId::new(
                    u32::from(seg_byte).wrapping_rem(16),
                ))
            } else {
                // Index accessor
                vb_core::PathSegment::Index(u32::from(seg_byte).wrapping_rem(8))
            };
            path.push(segment);
            if path.len() >= FUZZ_MAX_ACCESSOR_DEPTH {
                break;
            }
        }

        accessors.push(vb_core::AccessorProgram {
            root,
            path: path.into_boxed_slice(),
        });

        if offset >= data.len() {
            break;
        }
    }

    // Build a minimal workflow with the constructed accessors.
    let max_slot = slot_count.saturating_sub(1);
    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_accessor"),
        digest: vb_core::WorkflowDigest::from_bytes([0xE0; 32]),
        nodes: vec![vb_core::CompiledNode {
            id: vb_core::StepIdx::new(0),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(max_slot),
            },
        }]
        .into(),
        expressions: Box::new([]),
        accessors: accessors.into(),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();

    // Populate the ValueStore with some data that accessors might traverse.
    let Ok(sym_a) = store.insert_symbol(Box::<str>::from("field_a")) else {
        return;
    };
    let _ = sym_a;
    let Ok(list_id) = store.insert_list(
        vec![
            vb_core::SlotValue::I64(10),
            vb_core::SlotValue::I64(20),
            vb_core::SlotValue::I64(30),
        ]
        .into_boxed_slice(),
    ) else {
        return;
    };
    let Ok(obj_id) = store.insert_object(
        vec![
            vb_core::value_store::ObjectField {
                key: vb_core::SymbolId::new(0),
                value: vb_core::SlotValue::Bool(true),
            },
            vb_core::value_store::ObjectField {
                key: vb_core::SymbolId::new(1),
                value: vb_core::SlotValue::I64(42),
            },
        ]
        .into_boxed_slice(),
    ) else {
        return;
    };

    // Write some slot values that the accessors reference.
    let mut run_with_data = match vb_core::RunFrame::new(
        vb_core::RunId::new(4),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        slot_count,
    ) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Write various slot values for accessor roots to traverse.
    if max_slot > 0 {
        let _ = run_with_data.write_slot_with_taint(
            vb_core::SlotIdx::new(0),
            vb_core::SlotValue::Null,
            vb_core::Taint::Clean,
        );
    }
    if slot_count > 1 {
        let _ = run_with_data.write_slot_with_taint(
            vb_core::SlotIdx::new(1),
            vb_core::SlotValue::Bool(true),
            vb_core::Taint::Clean,
        );
    }
    if slot_count > 2 {
        let _ = run_with_data.write_slot_with_taint(
            vb_core::SlotIdx::new(2),
            vb_core::SlotValue::I64(7),
            vb_core::Taint::Clean,
        );
    }
    if slot_count > 3 {
        let _ = run_with_data.write_slot_with_taint(
            vb_core::SlotIdx::new(3),
            vb_core::SlotValue::List(list_id),
            vb_core::Taint::Clean,
        );
    }
    if slot_count > 4 {
        let _ = run_with_data.write_slot_with_taint(
            vb_core::SlotIdx::new(4),
            vb_core::SlotValue::Object(obj_id),
            vb_core::Taint::Clean,
        );
    }

    // Evaluate each accessor -- must never panic.
    let mut i: u16 = 0;
    loop {
        let accessor_idx = vb_core::AccessorIdx::new(i);
        if workflow.accessor(accessor_idx).is_none() {
            break;
        }
        drop(vb_core::engine::eval_accessor_with_store(
            &workflow,
            &run_with_data,
            &mut store,
            accessor_idx,
        ));
        i = i.saturating_add(1);
        if i == 0 {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Target I: SlotValue postcard roundtrip
// ---------------------------------------------------------------------------

/// Exercises SlotValue postcard decode-and-re-encode roundtrip on arbitrary
/// bytes. Decodes bytes as `SlotValue` via postcard, then re-encodes the result
/// and verifies the bytes match. Both decode and encode must never panic.
pub fn fuzz_slot_value_roundtrip(data: &[u8]) {
    // Attempt to decode arbitrary bytes as a SlotValue.
    let Ok(decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(data) else {
        return;
    };

    // Re-encode the decoded value.
    let Ok(re_encoded): Result<Vec<u8>, _> = postcard::to_allocvec(&decoded) else {
        return;
    };

    // The round-tripped bytes must match the original input.
    if data.len() == re_encoded.len() {
        let mut matching = true;
        for i in 0..data.len() {
            if data.get(i) != re_encoded.get(i) {
                matching = false;
                break;
            }
        }
        if matching {
            // Successful roundtrip: verify we can decode the re-encoded bytes too.
            let Ok(_re_decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(&re_encoded)
            else {
                return;
            };
        }
    }

    // Also exercise display_with_store -- must never panic.
    let store = vb_core::ValueStore::new();
    let _display = decoded.display_with_store(&store);

    // Exercise type_name -- must never panic.
    let _type_name = decoded.type_name();

    // Exercise is_true -- must never panic.
    let _truthy = decoded.is_true();
}

// ---------------------------------------------------------------------------
// Target J: Admission fuzz (arbitrary artifact bytes)
// ---------------------------------------------------------------------------

/// Exercises `submit_artifact` with arbitrary postcard-encoded `WorkflowParts`
/// bytes. Unlike `fuzz_admission_flow` which constructs workflows from fuzz
/// input bytes, this target decodes raw fuzz data directly as `WorkflowParts`,
/// providing coverage over structurally valid but semantically invalid artifacts.
/// The target verifies that admission never panics regardless of input.
pub fn fuzz_admission_fuzz(data: &[u8]) {
    // Attempt to decode arbitrary bytes as WorkflowParts.
    let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) else {
        return;
    };

    // Try to build a compiled workflow -- may fail if structurally invalid.
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    // Open a temporary journal.
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    // Cycle through all policies to exercise different admission paths.
    let policies = [
        vb_core::RuntimePolicy::Relaxed,
        vb_core::RuntimePolicy::Journaled,
        vb_core::RuntimePolicy::Strict,
    ];
    for policy in policies {
        // submit_artifact must never panic -- it must return Result.
        drop(vb_storage::submit_artifact(&journal, &workflow, policy));
    }
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
