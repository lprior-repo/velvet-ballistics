//! Expression compilation and evaluation fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

const FUZZ_MAX_EXPR_OPS: usize = 64;
const FUZZ_SLOT_COUNT: u16 = 16;

pub fn fuzz_expression(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(tokens) = vb_compile::lexer::lex_expr(text) else {
        return;
    };
    let Ok(ast) = vb_compile::parser::parse_expr(&tokens) else {
        return;
    };
    let mut constants = Vec::new();
    let Ok(program) = vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants) else {
        return;
    };
    let eval_result = vb_compile::eval::eval_expr_program(&program, &[], &constants);
    if let Ok(value) = eval_result {
        let type_name = value.type_name();
        assert!(
            !type_name.is_empty(),
            "evaluated expression must have a valid type name"
        );
    }
}

pub fn fuzz_expr_bytecode(data: &[u8]) {
    let Ok(ops): Result<Box<[vb_core::ExprOp]>, _> = postcard::from_bytes(data) else {
        return;
    };

    if ops.len() > FUZZ_MAX_EXPR_OPS {
        return;
    }

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

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(vb_core::WorkflowParts {
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
        step_names: Box::new([]),
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

    let result = vb_core::engine::eval_expr_with_store(
        &workflow,
        &run,
        &mut store,
        vb_core::ExprIdx::new(0),
    );

    match result {
        Ok((slot_val, _taint)) => {
            let type_name = slot_val.type_name();
            assert!(
                !type_name.is_empty(),
                "evaluated expression must have a known type_name"
            );
            assert!(
                !matches!(slot_val, vb_core::SlotValue::Null),
                "eval_expr_with_store returned Ok(Null) — evaluator produced no useful result"
            );
        }
        Err(_engine_error) => {}
    }
}

pub fn fuzz_taint_propagation(data: &[u8]) {
    if data.len() < 2 {
        return;
    }

    let slot_count_byte = data.first().copied().unwrap_or(0);
    let slot_count = u16::from(slot_count_byte.wrapping_rem(16)).saturating_add(1);
    let slot_count_usize = usize::from(slot_count);

    let max_ops = slot_count_usize.min(FUZZ_MAX_EXPR_OPS);
    let mut ops: Vec<vb_core::ExprOp> = Vec::new();
    for i in 0..max_ops {
        ops.push(vb_core::ExprOp::LoadSlot(vb_core::SlotIdx::new(
            u16::try_from(i).unwrap_or(0),
        )));
    }

    let Ok(expr) = vb_core::ExprProgram::try_from_ops(ops.into_boxed_slice()) else {
        return;
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(vb_core::WorkflowParts {
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
        step_names: Box::new([]),
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

    const TAINT_LEVELS: [vb_core::Taint; 3] = [
        vb_core::Taint::Clean,
        vb_core::Taint::Secret,
        vb_core::Taint::DerivedFromSecret,
    ];
    const TAINT_LEVELS_LEN: usize = TAINT_LEVELS.len();
    let mut max_input_taint = vb_core::Taint::Clean;
    let data_len = data.len();
    for i in 0..slot_count_usize {
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
    let result = vb_core::engine::eval_expr_with_store(
        &workflow,
        &run,
        &mut store,
        vb_core::ExprIdx::new(0),
    );

    if let Ok((_value, output_taint)) = result {
        assert!(
            taint_discriminant(output_taint) >= taint_discriminant(max_input_taint),
            "taint invariant violated: output {output_taint:?} < max input {max_input_taint:?}"
        );

        if max_input_taint == vb_core::Taint::Clean {
            assert!(
                output_taint == vb_core::Taint::Clean,
                "clean inputs produced tainted output: {output_taint:?}"
            );
        }
    }
}

fn taint_discriminant(taint: vb_core::Taint) -> u8 {
    match taint {
        vb_core::Taint::Clean => 0,
        vb_core::Taint::Secret => 1,
        vb_core::Taint::DerivedFromSecret => 2,
        _ => 3,
    }
}

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
        let mut i: u16 = 0;
        let mut eval_count: u32 = 0;
        loop {
            let expr_idx = vb_core::ExprIdx::new(i);
            if workflow.expression(expr_idx).is_none() {
                break;
            }
            match vb_core::engine::eval_expr_with_store(&workflow, &run, &mut store, expr_idx) {
                Ok((slot_val, _taint)) => {
                    eval_count += 1;
                    assert!(
                        !matches!(slot_val, vb_core::SlotValue::Null),
                        "eval_expr_with_store returned Ok(Null) — evaluator produced no useful result"
                    );
                }
                Err(_) => {}
            }
            i = i.saturating_add(1);
            if i == 0 {
                break;
            }
        }
        if workflow.expression(vb_core::ExprIdx::new(0)).is_some() {
            assert!(
                eval_count > 0,
                "workflow has expressions but eval_count = 0 — evaluator may not be running"
            );
        }
    }
}
