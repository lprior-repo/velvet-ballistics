//! Shared minimal fuzz target bodies for Velvet Ballastics.

use bytes::Bytes;
use std::num::NonZeroUsize;

const MAX_FUZZ_PAYLOAD: u32 = 4096;
const FUZZ_MAGIC: u32 = 0x5654_465A;

/// Exercises the YAML profile/parser on arbitrary UTF-8 input.
pub fn fuzz_workflow_parse(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let _profile = vb_yaml::validate_yaml_profile(text);
        let _parsed = vb_yaml::parse_workflow_source(text);
    }
}

/// Exercises the current compile API on arbitrary bytes.
pub fn fuzz_workflow_compile(data: &[u8]) {
    let _compiled = vb_compile::compile_workflow(data);
}

/// Exercises postcard SlotValue decoding and re-encoding when input is valid.
pub fn fuzz_slot_value_roundtrip(data: &[u8]) {
    if let Ok(value) = postcard::from_bytes::<vb_core::value::SlotValue>(data) {
        let _encoded = postcard::to_allocvec(&value);
    }
}

/// Exercises IPC header/frame decoding and typed payload decoding.
pub fn fuzz_binary_ipc_frame(data: &[u8]) {
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
pub fn fuzz_journal_record(data: &[u8]) {
    let _decoded: Result<(vb_storage::RecordEnvelope, vb_storage::JournalEvent), _> =
        vb_storage::decode_record(data, FUZZ_MAGIC, MAX_FUZZ_PAYLOAD);

    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x5A; 32]),
    };
    let _encoded = vb_storage::encode_record(
        FUZZ_MAGIC,
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
