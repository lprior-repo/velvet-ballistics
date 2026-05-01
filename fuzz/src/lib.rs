//! Shared fuzz target bodies for Velvet Ballastics evidence gates.

use bytes::Bytes;
use std::num::NonZeroUsize;
use vb_core::WorkflowParts;

const MAX_FUZZ_PAYLOAD: u32 = 4096;
const SMALL_WORKFLOW_A: &[u8] = b"version: velvet-ballastics/v1\nname: fuzz_a\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
const SMALL_WORKFLOW_B: &[u8] = b"version: velvet-ballastics/v1\nname: fuzz_b\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: true\n  - id: done\n    finish:\n      result: 0\n";

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
    if data.len() < vb_ipc::IPC_HEADER_LEN {
        return;
    }

    let mut header = [0_u8; vb_ipc::IPC_HEADER_LEN];
    let Some(prefix) = data.get(..vb_ipc::IPC_HEADER_LEN) else {
        return;
    };
    header.copy_from_slice(prefix);

    let payload = match data.get(vb_ipc::IPC_HEADER_LEN..) {
        Some(bytes) => Bytes::copy_from_slice(bytes),
        None => return,
    };
    let max_payload = vb_ipc::MaxPayloadBytes::new(NonZeroUsize::MIN);
    let Ok(frame) = vb_ipc::decode_frame(&header, payload, max_payload) else {
        return;
    };
    let _payload = vb_ipc::decode_payload(frame.payload());
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

    let source = selected_workflow(data);
    if let Ok(workflow) = vb_compile::compile_workflow(source) {
        let parts = workflow.to_parts();
        if let Ok(encoded) = postcard::to_allocvec(&parts) {
            let decoded = postcard::from_bytes::<WorkflowParts>(&encoded);
            if let Ok(decoded_parts) = decoded {
                let _validated = vb_core::CompiledWorkflow::try_from_parts(decoded_parts);
            }
        }
    }
}

/// Exercises IR/codegen equivalence hooks over small compiled workflows.
pub fn fuzz_generated_compare(data: &[u8]) {
    let source = match std::str::from_utf8(data) {
        Ok(text) if text.len() <= 4096 => text.as_bytes(),
        _ => selected_workflow(data),
    };
    let Ok(workflow) = vb_compile::compile_workflow(source) else {
        return;
    };
    let parts = workflow.to_parts();
    let _validated = vb_core::validate_compiled_workflow(&parts);
    let Ok(generated) = vb_codegen::emit_rust_workflow(&workflow) else {
        return;
    };
    let slot_marker = format!(
        "const WORKFLOW_SLOT_COUNT: usize = {};",
        workflow.slot_count()
    );
    let node_marker = format!(
        "const WORKFLOW_NODE_COUNT: u16 = {};",
        workflow.node_count()
    );
    let _slot_match = generated.contains(&slot_marker);
    let _node_match = generated.contains(&node_marker);
}

fn selected_workflow(data: &[u8]) -> &'static [u8] {
    match data.first().copied() {
        Some(value) if value % 2 == 0 => SMALL_WORKFLOW_A,
        _ => SMALL_WORKFLOW_B,
    }
}
