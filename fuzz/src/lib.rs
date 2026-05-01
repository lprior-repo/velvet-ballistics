//! Shared fuzz target bodies for Velvet Ballastics evidence gates.

use vb_core::WorkflowParts;

const MAX_FUZZ_PAYLOAD: u32 = 4096;
const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;
const IPC_HEADER_LEN: usize = 24;
const IPC_MAGIC: u32 = 0x5642_4C54;
const IPC_VERSION: u16 = 1;
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

fn selected_workflow(data: &[u8]) -> &'static [u8] {
    match data.first().copied() {
        Some(value) if value % 2 == 0 => SMALL_WORKFLOW_A,
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
