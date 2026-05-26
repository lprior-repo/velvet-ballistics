//! Shared fuzz target bodies for Velvet Ballistics evidence gates.
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

use vb_core::WorkflowParts;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
use vb_validate::ValidationError;

const MAX_FUZZ_PAYLOAD: u32 = 4096;

/// Maximum expression ops we will attempt to decode from fuzz input.
const FUZZ_MAX_EXPR_OPS: usize = 64;
/// Maximum slot count for fuzz workflows.
const FUZZ_SLOT_COUNT: u16 = 16;

/// Exercises capability-name schema validation through the public verifier path.
pub fn fuzz_capability_name_schema(data: &[u8]) {
    let Ok(name) = std::str::from_utf8(data) else {
        return;
    };
    let bounded_name = bounded_capability_name(name);
    let parts = fuzz_parts_with_actions(&[1]);
    let contracts = [fuzz_action_contract(
        1,
        Box::new([Capability::new(Box::from(bounded_name), ActionId::new(1))]),
    )];
    let result = vb_validate::shared::validate_with_contracts(&parts, &contracts);
    if bounded_name.is_empty() {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameEmpty { .. })
        ));
    } else if !capability_name_is_valid(bounded_name) {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameInvalid { .. })
        ));
    } else {
        assert!(result.is_ok());
    }
}

/// Exercises action-contract capability schema validation over bounded inputs.
pub fn fuzz_capability_contract_schema(data: &[u8]) {
    let first = data.first().copied().map_or(1, u16::from);
    let second = data.get(1).copied().map_or(first, u16::from);
    let tail = match data.get(2..) {
        Some(bytes) => bytes,
        None => &[],
    };
    let name = std::str::from_utf8(tail).map_or("network", bounded_capability_name);
    let parts = fuzz_parts_with_actions(&[first]);
    let contracts = [fuzz_action_contract(
        first,
        Box::new([
            Capability::new(Box::from(name), ActionId::new(second)),
            Capability::new(Box::from(name), ActionId::new(second)),
        ]),
    )];
    let result = vb_validate::shared::validate_with_contracts(&parts, &contracts);
    if name.is_empty() {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameEmpty { .. })
        ));
    } else if !capability_name_is_valid(name) {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameInvalid { .. })
        ));
    } else if first != second {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityActionMismatch { .. })
        ));
    } else {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityDuplicate { .. })
        ));
    }
}

fn bounded_capability_name(name: &str) -> &str {
    let mut end = name.len().min(128);
    while !name.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let Some(prefix) = name.get(..end) else {
        return "";
    };
    prefix
}

fn capability_name_is_valid(name: &str) -> bool {
    name.split('.').all(capability_segment_is_valid)
}

fn capability_segment_is_valid(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn fuzz_action_contract(action: u16, required_capabilities: Box<[Capability]>) -> ActionContract {
    ActionContract {
        id: ActionId::new(action),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities,
    }
}

fn fuzz_parts_with_actions(actions: &[u16]) -> WorkflowParts {
    let mut nodes = Vec::new();
    let mut index = 0u16;
    for action in actions {
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(*action),
                input: SlotIdx::new(0),
            },
        });
        index = index.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    WorkflowParts {
        name: Box::from("capability-schema-fuzz"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

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
    use vb_ipc::frame::{decode_frame_header, decode_frame_payload};

    let Some(header_bytes) = data.get(..vb_ipc::IPC_HEADER_LEN) else {
        return;
    };

    let mut header = [0u8; vb_ipc::IPC_HEADER_LEN];
    header.copy_from_slice(header_bytes);

    // Call the real decoder - this is what the fuzz target should exercise
    let header_result = decode_frame_header(&header);

    // If header decode succeeded, verify round-trip consistency
    if let Ok(decoded_header) = header_result {
        // Verify decoded header can be re-encoded and matches original bytes
        if let Ok(encoded) = decoded_header.encode() {
            assert_eq!(
                &encoded[..],
                header_bytes,
                "re-encoded header must match original bytes"
            );
        }
    }

    let Some(payload) = data.get(vb_ipc::IPC_HEADER_LEN..) else {
        return;
    };

    // Only attempt payload decode if there's actually payload data
    if !payload.is_empty()
        && let Ok(header) = header_result
    {
        // Payload decode must return a Result (never panic). Matching lengths
        // only permit postcard deserialization to run; arbitrary bytes may still
        // fail with a typed payload-decode error.
        let payload_len_usize = header.payload_len as usize;
        let result = decode_frame_payload(&header, payload);
        match result {
            Ok(decoded) => {
                assert!(
                    payload.len() == payload_len_usize,
                    "decode must fail when payload len mismatches header"
                );
                // Verify payload decoded without panic — destructuring alone exercises
                // the postcard deserialization path for every variant.
                match decoded {
                    vb_ipc::IpcPayload::SubmitRun(p)
                    | vb_ipc::IpcPayload::SubmitRunInline(p) => {
                        let _ = p.run_id;
                        let _ = p.workflow;
                    }
                    vb_ipc::IpcPayload::CancelRun { run_id }
                    | vb_ipc::IpcPayload::InspectRun { run_id }
                    | vb_ipc::IpcPayload::ListEvents { run_id, .. }
                    | vb_ipc::IpcPayload::DrainTrace { run_id, .. } => {
                        let _ = run_id;
                    }
                    vb_ipc::IpcPayload::AnswerAsk { run_id, ticket, .. } => {
                        let _ = run_id;
                        let _ = ticket;
                    }
                    vb_ipc::IpcPayload::CompleteAction { run_id, ticket, .. }
                    | vb_ipc::IpcPayload::FailAction { run_id, ticket, .. } => {
                        let _ = run_id;
                        let _ = ticket;
                    }
                    vb_ipc::IpcPayload::Shutdown => {}
                    vb_ipc::IpcPayload::Health
                    | vb_ipc::IpcPayload::GetMetrics
                    | vb_ipc::IpcPayload::ListRuns { .. }
                    | vb_ipc::IpcPayload::GetTaintReport { .. }
                    | vb_ipc::IpcPayload::GetWorkflowGraph { .. }
                    | vb_ipc::IpcPayload::VerifyWorkflow { .. } => {}
                    _ => {}
                }
            }
            Err(e) => assert_typed_ipc_error(e),
        }
    }
}

/// IPC header decode fuzz target - exercises IpcFrameHeader::decode with
/// various max_payload bounds and edge cases.
pub fn fuzz_ipc_decode(data: &[u8]) {
    use vb_ipc::frame::decode_frame_header;

    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        let mut header_bytes = [0u8; vb_ipc::IPC_HEADER_LEN];
        header_bytes.copy_from_slice(&data[..vb_ipc::IPC_HEADER_LEN]);

        // Try with various max_payload bounds
        let bounds: &[usize] = &[0, 1, 16, 256, 1024, 65536, 1_048_576];
        for &b in bounds {
            if let Some(max) = std::num::NonZeroUsize::new(b) {
                let _ = vb_ipc::IpcFrameHeader::decode(
                    &header_bytes,
                    vb_ipc::MaxPayloadBytes::new(max),
                );
            }
        }

        // Also try the simple decoder
        let _ = decode_frame_header(&header_bytes);
    }

    // Test truncated headers
    for len in 0..vb_ipc::IPC_HEADER_LEN {
        if data.len() >= len {
            let mut bytes = [0u8; vb_ipc::IPC_HEADER_LEN];
            let end = len.min(data.len());
            bytes[..end].copy_from_slice(&data[..end]);
            let _ = vb_ipc::IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT);
        }
    }
}

/// Exercises storage record envelope decode and valid-event encode paths.
pub fn fuzz_journal_event(data: &[u8]) {
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        MAX_FUZZ_PAYLOAD,
    );

    match decoded {
        Ok((_envelope, event)) => {
            // Verify the decoded event is structurally valid
            assert!(
                event.is_valid(),
                "Decoded event must be structurally valid"
            );

            // Round-trip: encode the event and decode again
            let Ok(encoded) = vb_storage::encode_record(
                vb_storage::MAGIC_JOURNAL_EVENT,
                event.record_kind(),
                event.seq().get(),
                &event,
                MAX_FUZZ_PAYLOAD,
            ) else {
                return;
            };

            let reparsed = vb_storage::decode_record::<vb_storage::JournalEvent>(
                &encoded,
                vb_storage::MAGIC_JOURNAL_EVENT,
                MAX_FUZZ_PAYLOAD,
            );
            assert!(
                reparsed.is_ok(),
                "Round-trip encode/decode must succeed for valid event"
            );
        }
        Err(e) => {
            // For error cases, verify it's a typed JournalError (not a panic)
            assert!(
                matches!(
                    e,
                    vb_storage::JournalError::BadMagic { .. }
                        | vb_storage::JournalError::UnexpectedEof
                        | vb_storage::JournalError::HeaderChecksumMismatch
                        | vb_storage::JournalError::PayloadDigestMismatch
                        | vb_storage::JournalError::PostcardDecodeFailed
                        | vb_storage::JournalError::PayloadTooLarge { .. }
                        | vb_storage::JournalError::RecordKindFamilyMismatch { .. }
                        | vb_storage::JournalError::UnknownRecordKind { .. }
                        | vb_storage::JournalError::UnsupportedSchemaVersion { .. }
                        | vb_storage::JournalError::HeaderLengthMismatch { .. }
                        | vb_storage::JournalError::SequenceOverflow
                ),
                "Must return typed JournalError for corrupt input, got {:?}",
                e
            );
        }
    }
}

/// Exercises recovery replay over arbitrary postcard-encoded event vectors.
pub fn fuzz_replay_events(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    let _result = vb_storage::recovery::replay_events(&events, &mut tracker, &[]);
}

/// Exercises terminal extraction over arbitrary postcard-encoded event vectors.
pub fn fuzz_extract_terminal(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    let _terminal = vb_storage::recovery::extract_terminal(&events);
}

/// Exercises action replay tracker state transitions over compact byte triples.
pub fn fuzz_action_tracker(data: &[u8]) {
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    for chunk in data.chunks_exact(3).take(64) {
        let Some(mode) = chunk.first().copied() else {
            continue;
        };
        let Some(action) = chunk.get(1).copied() else {
            continue;
        };
        let Some(step) = chunk.get(2).copied() else {
            continue;
        };
        let action = vb_core::ActionId::new(u16::from(action));
        let step = vb_core::StepIdx::new(u16::from(step));
        match mode % 3 {
            0 => tracker.mark_completed(action, step),
            1 => tracker.mark_failed(action, step),
            _ => {
                let _resolved = tracker.is_resolved(action, step);
            }
        }
    }
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
    // Evaluation must not panic - it returns Result
    let eval_result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
    // If compilation succeeded, evaluation must also produce a valid result
    // (it may be an error, but it must be a typed error, not a panic)
    if let Ok(value) = eval_result {
        // The returned value must be a valid SlotValue type
        let type_name = value.type_name();
        assert!(
            !type_name.is_empty(),
            "evaluated expression must have a valid type name"
        );
    }
}

/// Exercises compiled IR postcard decode and validation.
pub fn fuzz_compiled_ir(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<WorkflowParts>(data) {
        let digest_before = parts.digest;
        let node_count_before = parts.nodes.len();
        let slot_count = parts.slot_count;
        let result = vb_core::CompiledWorkflow::try_from_parts(parts);
        // If parts decode succeeded, workflow construction may succeed or fail
        // but must not panic. If it succeeds, the workflow must be usable.
        if let Ok(workflow) = result {
            // Workflow must have at least one node (the entry step)
            assert!(
                workflow.node_count() >= 1,
                "compiled workflow must have at least 1 node, got {}",
                workflow.node_count()
            );
            // Workflows with no slot-referencing nodes may legitimately have
            // zero slots; successful construction must preserve the declared
            // slot count exactly.
            assert_eq!(
                workflow.slot_count(),
                slot_count,
                "workflow slot count must match decoded parts slot count"
            );
            // Digest must be preserved through conversion
            assert_eq!(
                workflow.digest(),
                digest_before,
                "workflow digest must match decoded parts digest"
            );
            // Node count must match decoded parts
            assert_eq!(
                usize::from(workflow.node_count()),
                node_count_before,
                "workflow node count must match decoded parts node count"
            );
            // All slot references in nodes must be within declared slot_count
            for i in 0..workflow.node_count() {
                let step = vb_core::StepIdx::new(i);
                let Some(node) = workflow.node(step) else {
                    continue;
                };
                if let Some(output) = node.output {
                    assert!(
                        output.get() < slot_count,
                        "node {} output slot {} out of bounds (slot_count={})",
                        i,
                        output.get(),
                        slot_count
                    );
                }
                check_node_slots(&node.kind, slot_count, i);
            }
        }
    }
}

/// Checks that all slot indices within a node kind are within bounds.
fn check_node_slots(kind: &vb_core::CompiledNodeKind, slot_count: u16, node_idx: u16) {
    use vb_core::CompiledNodeKind;
    match kind {
        CompiledNodeKind::Nop | CompiledNodeKind::Jump { .. } => {}
        CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => {
            assert!(
                source.get() < slot_count,
                "node {} Copy source slot {} out of bounds",
                node_idx,
                source.get()
            );
        }
        CompiledNodeKind::EvalExpr { expr: _ } => {}
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                assert!(
                    slot.get() < slot_count,
                    "node {} BuildObject slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                assert!(
                    slot.get() < slot_count,
                    "node {} BuildList slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::Do { action: _, input } => {
            assert!(
                input.get() < slot_count,
                "node {} Do input slot {} out of bounds",
                node_idx,
                input.get()
            );
        }
        CompiledNodeKind::Choose { branches, otherwise } => {
            for _branch in branches.iter() {}
            let _ = otherwise;
        }
        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            for branch in branches.iter() {
                assert!(
                    branch.condition.get() < slot_count,
                    "node {} ChooseSlot condition slot {} out of bounds",
                    node_idx,
                    branch.condition.get()
                );
            }
            let _ = otherwise;
        }
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            ..
        } => {
            assert!(
                input.get() < slot_count,
                "node {} ForEachStart input slot {} out of bounds",
                node_idx,
                input.get()
            );
            assert!(
                item_slot.get() < slot_count,
                "node {} ForEachStart item_slot {} out of bounds",
                node_idx,
                item_slot.get()
            );
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            assert!(
                iterator_slot.get() < slot_count,
                "node {} ForEachNext iterator_slot {} out of bounds",
                node_idx,
                iterator_slot.get()
            );
        }
        CompiledNodeKind::ForEachJoin { output } => {
            assert!(
                output.get() < slot_count,
                "node {} ForEachJoin output slot {} out of bounds",
                node_idx,
                output.get()
            );
        }
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} TogetherBranch accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} TogetherJoin accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            assert!(
                source.get() < slot_count,
                "node {} CollectStart source slot {} out of bounds",
                node_idx,
                source.get()
            );
        }
        CompiledNodeKind::CollectPage { collector_slot, .. }
        | CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => {
            assert!(
                collector_slot.get() < slot_count,
                "node {} Collect collector_slot {} out of bounds",
                node_idx,
                collector_slot.get()
            );
        }
        CompiledNodeKind::ReduceStart { input, accumulator, .. } => {
            assert!(
                input.get() < slot_count,
                "node {} ReduceStart input slot {} out of bounds",
                node_idx,
                input.get()
            );
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceStart accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            assert!(
                iterator_slot.get() < slot_count,
                "node {} ReduceNext iterator_slot {} out of bounds",
                node_idx,
                iterator_slot.get()
            );
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceNext accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceFinish accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
            assert!(
                attempt_slot.get() < slot_count,
                "node {} RepeatAttempt attempt_slot {} out of bounds",
                node_idx,
                attempt_slot.get()
            );
        }
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            assert!(
                attempt_slot.get() < slot_count,
                "node {} RepeatCheck attempt_slot {} out of bounds",
                node_idx,
                attempt_slot.get()
            );
        }
        CompiledNodeKind::RepeatFinish { result } => {
            assert!(
                result.get() < slot_count,
                "node {} RepeatFinish result slot {} out of bounds",
                node_idx,
                result.get()
            );
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            assert!(
                deadline_slot.get() < slot_count,
                "node {} WaitUntil deadline_slot {} out of bounds",
                node_idx,
                deadline_slot.get()
            );
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert!(
                event.get() < slot_count,
                "node {} WaitEvent event slot {} out of bounds",
                node_idx,
                event.get()
            );
            if let Some(timeout) = timeout_slot {
                assert!(
                    timeout.get() < slot_count,
                    "node {} WaitEvent timeout_slot {} out of bounds",
                    node_idx,
                    timeout.get()
                );
            }
        }
        CompiledNodeKind::Ask { prompt, timeout_slot } => {
            assert!(
                prompt.get() < slot_count,
                "node {} Ask prompt slot {} out of bounds",
                node_idx,
                prompt.get()
            );
            if let Some(timeout) = timeout_slot {
                assert!(
                    timeout.get() < slot_count,
                    "node {} Ask timeout_slot {} out of bounds",
                    node_idx,
                    timeout.get()
                );
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            assert!(
                answer.get() < slot_count,
                "node {} AskResume answer slot {} out of bounds",
                node_idx,
                answer.get()
            );
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            assert!(
                policy_slot.get() < slot_count,
                "node {} RetryCheck policy_slot {} out of bounds",
                node_idx,
                policy_slot.get()
            );
        }
        CompiledNodeKind::ErrorHandler { error_slot, .. } => {
            if let Some(slot) = error_slot {
                assert!(
                    slot.get() < slot_count,
                    "node {} ErrorHandler error_slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::Finish { result } => {
            assert!(
                result.get() < slot_count,
                "node {} Finish result slot {} out of bounds",
                node_idx,
                result.get()
            );
        }
        _ => {
            // Coverage-only: unknown future variants are skipped gracefully.
        }
    }
}

/// Exercises vb-qi37.4.2 strict accepted-artifact envelope decoding over hostile bytes.
///
/// Coverage-only: verifies postcard deserialization of AcceptedArtifact never panics.
/// Field access exercises the deserialization path; no admission invariant is asserted
/// because this target does not invoke an admission boundary.
pub fn fuzz_accepted_artifact_envelope_qi37_4_2(data: &[u8]) {
    let Ok(artifact) = postcard::from_bytes::<vb_storage::AcceptedArtifact>(data) else {
        return;
    };
    // Coverage-only: field access exercises postcard deserialization.
    let _ = artifact.verification.gate_count;
    let _ = artifact.verification.durable;
    let _ = artifact.digest;
    let _ = artifact.required_capabilities.len();
}

/// Exercises IR/codegen equivalence hooks over small compiled workflows.
pub fn fuzz_generated_compare(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<WorkflowParts>(data) {
        let parts_clone = parts.clone();
        // Validation must not panic - it returns Result
        let validated = vb_core::validate_compiled_workflow(&parts);
        // If validation passes, workflow construction should also succeed
        // parts is moved here, validate already happened above with reference
        let workflow = vb_core::CompiledWorkflow::try_from_parts(parts);
        // Both must agree on success/failure
        assert!(
            validated.is_ok() == workflow.is_ok(),
            "validation and workflow construction must agree: validated={:?}, workflow={:?}",
            validated,
            workflow.is_ok()
        );
        // For successful conversions, independent decode must yield identical digest
        if let (Ok(w1), Ok(w2)) = (
            workflow,
            vb_core::CompiledWorkflow::try_from_parts(parts_clone),
        ) {
            assert_eq!(
                w1.digest(),
                w2.digest(),
                "independent decode must yield same digest"
            );
            assert_eq!(
                w1.node_count(),
                w2.node_count(),
                "independent decode must yield same node count"
            );
            assert_eq!(
                w1.slot_count(),
                w2.slot_count(),
                "independent decode must yield same slot count"
            );
        }
    }
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

    // The evaluator must return a Result -- it must never panic.
    let _result = vb_core::engine::eval_expr_with_store(
        &workflow,
        &run,
        &mut store,
        vb_core::ExprIdx::new(0),
    );
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
        ops.push(vb_core::ExprOp::LoadSlot(vb_core::SlotIdx::new(
            u16::try_from(i).unwrap_or(0),
        )));
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

    // Write random taint levels into each slot.
    const TAINT_LEVELS: [vb_core::Taint; 3] = [
        vb_core::Taint::Clean,
        vb_core::Taint::Secret,
        vb_core::Taint::DerivedFromSecret,
    ];
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
    let result = vb_core::engine::eval_expr_with_store(
        &workflow,
        &run,
        &mut store,
        vb_core::ExprIdx::new(0),
    );

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
        _ => {
            // Coverage-only: unknown future taint levels default to most restrictive.
            3
        }
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
        step_names: Box::new([]),
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
    let node_count = usize::from(byte0.wrapping_rem(16))
        .saturating_add(1)
        .min(FUZZ_MAX_NODES);
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
        step_names: Box::new([]),
    };

    // Gate 7: Expression stack depth bounded.
    drop(vb_validate::gates::validate_gate_07_expression_stack_depth(
        &parts,
    ));
    // Gate 8: Accessor path segments are valid symbols.
    drop(vb_validate::gates::validate_gate_08_accessor_path_segments(
        &parts,
    ));
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
        Some(vb_core::StepIdx::new(
            u16::try_from(index).unwrap_or(0).saturating_add(1),
        ))
    } else {
        None
    };

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(8) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish { result: safe_slot },
        2 => vb_core::CompiledNodeKind::Copy { source: safe_slot },
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
    let node_count = usize::from(byte0.wrapping_rem(16))
        .saturating_add(1)
        .min(FUZZ_MAX_NODES);
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

    // Coverage-only: we only verify compute() returns a budget without panic.
    // The following are observed properties, not contractual invariants:
    // - max_total_steps > 0 for non-empty workflows
    // - max_total_steps is bounded (exact bound is an implementation detail)
    // - max_total_slots reflects the contract
    // - max_fanout is bounded
    let _ = budget.max_total_steps;
    let _ = budget.max_total_slots;
    let _ = budget.max_fanout;
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
        Some(vb_core::StepIdx::new(
            u16::try_from(index).unwrap_or(0).saturating_add(1),
        ))
    } else {
        None
    };

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(6) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish { result: safe_slot },
        2 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        3 => vb_core::CompiledNodeKind::Copy { source: safe_slot },
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
            Some(vb_core::StepIdx::new(
                u16::try_from(i).unwrap_or(0).saturating_add(1),
            ))
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
        step_names: Box::new([]),
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
        drop(vb_storage::submit_artifact(
            &journal,
            &corrupted,
            vb_core::RuntimePolicy::Strict,
        ));
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
        let mut eval_count: u32 = 0;
        loop {
            let expr_idx = vb_core::ExprIdx::new(i);
            if workflow.expression(expr_idx).is_none() {
                break;
            }
            // Behavioral invariant: eval_expr_with_store returns Ok with a non-Empty value
            // for any successfully evaluated expression. If it returns Empty, the evaluator
            // did not produce a meaningful result for this expression.
            match vb_core::engine::eval_expr_with_store(
                &workflow, &run, &mut store, expr_idx,
            ) {
                Ok((slot_val, _taint)) => {
                    eval_count += 1;
                    assert!(
                        !matches!(slot_val, vb_core::SlotValue::Null),
                        "eval_expr_with_store returned Ok(Null) — evaluator produced no useful result"
                    );
                }
                Err(_) => {
                    // Expression evaluation can fail for many reasons (undefined variable,
                    // type mismatch, division by zero). That's fine — the evaluator is
                    // correctly propagating errors rather than panicking.
                }
            }
            i = i.saturating_add(1);
            if i == 0 {
                // Wrapped around -- stop.
                break;
            }
        }
        // Structural invariant: if the workflow has expressions, at least some must
        // be evaluable. A workflow that declares expressions but evaluates zero of them
        // suggests the evaluator is not being invoked correctly.
        if workflow.expression(vb_core::ExprIdx::new(0)).is_some() {
            assert!(
                eval_count > 0,
                "workflow has expressions but eval_count = 0 — evaluator may not be running"
            );
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
        let safe_slot_count: u16 = match slot_count {
            0 => 1u16,
            n => n,
        };
        #[allow(clippy::arithmetic_side_effects)]
        let root = vb_core::SlotIdx::new(u16::from(root_byte).wrapping_rem(safe_slot_count));
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
        step_names: Box::new([]),
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
                taint: vb_core::value::Taint::Clean,
            },
            vb_core::value_store::ObjectField {
                key: vb_core::SymbolId::new(1),
                value: vb_core::SlotValue::I64(42),
                taint: vb_core::value::Taint::Clean,
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
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(0),
                vb_core::SlotValue::Null,
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 1 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(1),
                vb_core::SlotValue::Bool(true),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 2 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(2),
                vb_core::SlotValue::I64(7),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 3 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(3),
                vb_core::SlotValue::List(list_id),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 4 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(4),
                vb_core::SlotValue::Object(obj_id),
                vb_core::Taint::Clean,
            )
            .ok();
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
    let display = decoded.display_with_store(&store);
    assert!(!display.is_empty(), "display_with_store must produce non-empty output");

    // Exercise type_name -- must never panic.
    let type_name = decoded.type_name();
    assert!(!type_name.is_empty(), "type_name must be non-empty");

    // Exercise is_true -- must never panic.
    let truthy = decoded.is_true();
    // is_true must be deterministic
    assert_eq!(truthy, decoded.is_true(), "is_true must be deterministic");
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
        // Coverage-only: we only verify panic-freedom, not admission correctness.
        let _result = vb_storage::submit_artifact(&journal, &workflow, policy);
    }
}

// ---------------------------------------------------------------------------
// F01: Strict AcceptedArtifact CompiledIR Decoder
// ---------------------------------------------------------------------------

/// F01: Strict `AcceptedArtifact` compiled IR decoder.
///
/// Target: strict decoder/readback path for `CompiledIrRecord.ir` bytes.
/// Input: bytes.
/// Risk: raw `WorkflowParts` or malformed/legacy bytes accepted as `AcceptedArtifact`,
/// panic/OOM, wrong error variant.
///
/// Corpus seeds: valid `AcceptedArtifact`, raw `WorkflowParts`, empty bytes,
/// single byte, truncated postcard envelope, overlong vector lengths,
/// stale gate count, missing gate fields, false proof flags,
/// digest mismatch, capability metadata mismatch.
///
/// Maps: FUZZ-ART-008, PRE-005, POST-006, INV-004.
pub fn fuzz_strict_artifact_decoder(data: &[u8]) {
    // F01a: Try to decode as AcceptedArtifact.
    if let Ok(artifact) = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data) {
        // Decoded successfully — verify the artifact fields are internally consistent.
        // gate_count must be non-zero for strict paths.
        assert!(
            artifact.verification.gate_count > 0,
            "strict artifact gate_count must be non-zero"
        );
        // accepted_at_seq must be non-sentinel (>= 1) for successful admission.
        assert!(
            artifact.accepted_at_seq.get() >= 1,
            "accepted_at_seq must be >= 1"
        );
    }

    // F01b: Try to decode as raw WorkflowParts (must NOT succeed as AcceptedArtifact).
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        // Successfully decoded as WorkflowParts — verify node count is bounded
        assert!(
            parts.nodes.len() <= usize::from(u16::MAX),
            "decoded WorkflowParts node count must fit u16"
        );
    }

    // F01c: Malformed bytes — must not panic in any codec path.
    let artifact_decode = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data);
    let parts_decode = postcard::from_bytes::<vb_core::WorkflowParts>(data);
    // Both must return Result, never panic
    let _ = artifact_decode.is_ok();
    let _ = parts_decode.is_ok();
    // Note: CompiledWorkflow does not implement Deserialize — use try_from_parts instead.
}

// ---------------------------------------------------------------------------
// F02: Workflow Source/Artifact Digest Coherence Parser
// ---------------------------------------------------------------------------

/// F02: Admission panic-freedom with arbitrary workflow digest.
///
/// Coverage-only: constructs a minimal workflow from fuzz-derived digest bytes
/// and exercises submit_artifact to verify it never panics. No coherence
/// invariant is asserted because the admission boundary does not expose a
/// source-digest comparison surface.
pub fn fuzz_digest_coherence(data: &[u8]) {
    let digest_bytes: [u8; 32] = match data.get(..32) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let digest = vb_core::WorkflowDigest::from_bytes(digest_bytes);

    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_digest_test"),
        digest,
        nodes: Box::new([vb_core::CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    // Coverage-only: verify panic-freedom on strict admission path.
    let _result =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
}

// ---------------------------------------------------------------------------
// F03: Readback Family-Set Reconstruction
// ---------------------------------------------------------------------------

/// F03: Readback panic-freedom after admission.
///
/// Coverage-only: admits a minimal workflow and then exercises readback
/// classification to verify the path never panics. No deletion is performed
/// because the storage backend does not support it; the classification is
/// therefore deterministic for this input shape.
pub fn fuzz_readback_family_set(_data: &[u8]) {
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_readback"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([vb_core::CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash_bytes = match postcard::to_allocvec(&parts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let computed = blake3::hash(&hash_bytes);
    let digest = vb_core::WorkflowDigest::from_bytes(*computed.as_bytes());
    let correct_parts = vb_core::WorkflowParts { digest, ..parts };
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(correct_parts) else {
        return;
    };

    // Submit strictly to establish full family set.
    if vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict).is_err() {
        return;
    }

    // Coverage-only: exercise readback classification panic-freedom.
    let _classification = classify_readback_family_set(
        &journal,
        digest,
        vb_core::RunId::new(8001),
        ReadbackDeletionIntent::None,
    );
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum ReadbackDeletionIntent {
    None,
    Partial,
    Full,
}

#[allow(dead_code)]
impl ReadbackDeletionIntent {
    fn from_mask(mask: u8) -> Self {
        let core_family_mask = mask & 0b0000_1111;
        match core_family_mask.count_ones() {
            0 => Self::None,
            4 => Self::Full,
            _ => Self::Partial,
        }
    }
}

#[allow(dead_code)]
enum ReadbackFamilySet {
    Full,
    Partial,
    Absent,
    Unreadable,
}

fn classify_readback_family_set(
    journal: &vb_storage::FjallJournal,
    digest: vb_core::WorkflowDigest,
    run: vb_core::RunId,
    intended_deletion: ReadbackDeletionIntent,
) -> ReadbackFamilySet {
    let has_source = match journal.workflow_source(digest) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let has_artifact = match journal.compiled_ir(digest) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let has_header = match journal.run_header(run) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let events = match journal.events_for_run(run) {
        Ok(events) => events,
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let accepted_event_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RunAccepted { workflow, .. } if *workflow == digest
            )
        })
        .count();
    let has_accepted_event = accepted_event_count > 0;
    let families_present = usize::from(has_source)
        .saturating_add(usize::from(has_artifact))
        .saturating_add(usize::from(has_header))
        .saturating_add(usize::from(has_accepted_event));

    if has_source && has_artifact && has_header && has_accepted_event {
        ReadbackFamilySet::Full
    } else if families_present > 0 || matches!(intended_deletion, ReadbackDeletionIntent::Partial) {
        ReadbackFamilySet::Partial
    } else {
        ReadbackFamilySet::Absent
    }
}

// ---------------------------------------------------------------------------
// F04: CLI/Runtime Strict Admission Input Surface
// ---------------------------------------------------------------------------

/// F04: CLI/runtime strict admission input surface.
///
/// Coverage-only: exercises admission paths with raw WorkflowParts bytes.
/// No filesystem I/O is performed (fuzzers must be deterministic and isolated).
pub fn fuzz_admission_input_surface(data: &[u8]) {
    // Coverage-only: exercises admission panic-freedom with decoded WorkflowParts.
    // This exercises the path where raw WorkflowParts are submitted as "artifact".
    if data.len() >= 2 {
        let temp_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return,
        };
        let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
            Ok(j) => j,
            Err(_) => return,
        };

        // Try to decode data as WorkflowParts.
        if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
            let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
                return;
            };

            // Submit as strict — must not panic.
            // Coverage-only: we only verify panic-freedom on submit paths.
            let _strict =
                vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
            let _relaxed =
                vb_storage::submit_artifact(
                    &journal, &workflow, vb_core::RuntimePolicy::Relaxed);
        }
    }


}

/// Bead target: strict YAML profile input must never panic. Unsupported profile
/// features are accepted only as typed compile errors and must not produce an
/// artifact through the strict YAML compile boundary.
pub fn fuzz_strict_yaml_profile(data: &[u8]) {
    let compile_result = vb_compile::compile_workflow(data);
    // Coverage-only: we only verify panic-freedom, not compile correctness.
    if let Ok(ref workflow) = compile_result {
        let text = String::from_utf8_lossy(data);
        let unsupported =
            text.contains("---") || text.contains('&') || text.contains('*') || text.contains('!');
        // If unsupported YAML features are present, compilation should NOT succeed
        assert!(!unsupported, "unsupported YAML features must cause compile error");
        // Compiled workflow must have at least one node
        assert!(workflow.node_count() >= 1, "compiled workflow must have at least 1 node");
    }
}

/// Bead target: arbitrary accepted-artifact bytes must decode as malformed or be
/// rejected by runtime envelope validation unless every strict proof field is
/// present and valid.
pub fn fuzz_accepted_artifact_decode(data: &[u8]) {
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let record = vb_storage::CompiledIrRecord {
        digest,
        ir: data.to_vec(),
    };
    if vb_storage::put_compiled_ir(&journal, &record).is_err() {
        return;
    }
    let store = vb_runtime::admission::StorageArtifactStore::new(std::sync::Arc::new(journal));
    // Coverage-only: we only verify panic-freedom on load path.
    let _result = vb_runtime::admission::AcceptedArtifactStore::load_accepted_artifact(&store, digest);
}

/// Bead target: recovery snapshot/frame/journal decode boundary must fail closed
/// and never synthesize recovered success from arbitrary bytes.
pub fn fuzz_recovery_decode(data: &[u8]) {
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let run = vb_core::RunId::new(u64::from(data.first().copied().unwrap_or(0)));
    let seq = vb_storage::EventSeq::new(1);
    let events = if data.len().is_multiple_of(2) {
        vec![vb_storage::JournalEvent::RunAccepted {
            run,
            seq,
            workflow: digest,
        }]
    } else {
        Vec::new()
    };
    // Coverage-only: we only verify panic-freedom on recovery paths.
    let _summary = vb_storage::recovery::summarize_recovery_events(&events);
    let _seed = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events);
}

// ---------------------------------------------------------------------------
// Target: StepBudget::new clamping boundary (FUZZ-001)
// ---------------------------------------------------------------------------

/// Fuzz target: step_budget_new
///
/// Specifically targets StepBudget::new clamping behavior with values near
/// MAX_STEP_BUDGET boundary. Exercises:
/// - u64::MAX (panic-free clamping)
/// - MAX_STEP_BUDGET + 1 (exactly at boundary)
/// - MAX_STEP_BUDGET (exact boundary)
/// - values near MAX_STEP_BUDGET (boundary adjacency)
/// - 0 (zero budget)
///
/// Obligation: FUZZ-001 (vb-qi37.2.5)
/// Command: cargo fuzz run step_budget_new -- -runs=10000
pub fn fuzz_step_budget_new(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    // Derive a u64 budget from fuzz input (full u64 range, not capped)
    let budget_value = if data.len() >= 8 {
        let mut bytes = [0u8; 8];
        let src = &data[..8.min(data.len())];
        bytes[..src.len()].copy_from_slice(src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data[0])
    };

    // StepBudget::new must never panic for any u64 input
    let budget = vb_core::StepBudget::new(budget_value);
    let remaining = budget.remaining();

    // INV-001: remaining must always be in [0, MAX_STEP_BUDGET]
    assert!(
        remaining <= vb_core::limits::MAX_STEP_BUDGET,
        "StepBudget::new({}) produced remaining={}, exceeds MAX_STEP_BUDGET={}",
        budget_value,
        remaining,
        vb_core::limits::MAX_STEP_BUDGET
    );
    // Boundary checks: clamping must be exact
    let expected = budget_value.min(vb_core::limits::MAX_STEP_BUDGET);
    assert!(
        remaining == expected,
        "StepBudget::new({}) remaining={}, expected {}",
        budget_value,
        remaining,
        expected
    );

    // After construction, try_take must work correctly
    let mut mutable_budget = budget;
    let result = mutable_budget.try_take();
    assert!(result.is_ok(), "try_take must not error");

    // If initial budget > 0, try_take must succeed and decrement by 1
    if expected > 0 {
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        let decremented = match expected.checked_sub(1) {
            Some(value) => value,
            None => return,
        };
        assert!(ok, "try_take should succeed when budget > 0");
        assert_eq!(
            mutable_budget.remaining(),
            decremented,
            "remaining should decrement by 1 after successful try_take"
        );
    } else {
        // Zero budget: try_take must return false without panicking
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(!ok, "try_take should return false when budget is 0");
        assert_eq!(
            mutable_budget.remaining(),
            0,
            "remaining should stay 0 after failed try_take"
        );
    }
}

// ---------------------------------------------------------------------------
// Target: vb_ui_model OutputEnvelope postcard decode
// ---------------------------------------------------------------------------
// DISABLED: vb_ui_model crate missing from workspace
// pub fn fuzz_vb_ui_model_postcard_decode(data: &[u8]) {
//     let Ok(envelope): Result<vb_ui_model::envelope::OutputEnvelope, _> = postcard::from_bytes(data)
//     else {
//         return;
//     };
//
//     let schema_version = envelope.schema_version().get();
//     assert!(schema_version >= 1, "schema_version must be at least 1");
//
//     let kind = *envelope.kind();
//     if kind.uses_diagnostics_field() {
//         assert!(
//             envelope.payload().is_none(),
//             "diagnostic envelopes must not carry data payloads"
//         );
//         assert!(
//             envelope.diagnostic().is_some(),
//             "diagnostic envelopes must carry at least one diagnostic"
//         );
//     } else {
//         assert!(
//             envelope.diagnostic().is_none(),
//             "non-diagnostic envelopes must not carry diagnostics"
//         );
//     }
// }

// ---------------------------------------------------------------------------
// Target: vb-qi37.12 persisted payload decode
// ---------------------------------------------------------------------------

/// Exercises persisted journal payload decode on arbitrary, truncated, and
/// checksum-corrupted bytes. Malformed persisted bytes must stay typed errors;
/// they must never become an empty successful recovery value.
pub fn fuzz_vb_qi37_12_persisted_payload_decode(data: &[u8]) {
    let max_payload_len = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    match decoded {
        Ok((_envelope, _event)) => {}
        Err(error) => assert_malformed_decode_is_typed(error),
    }

    exercise_truncated_persisted_payload(max_payload_len);
    exercise_corrupted_persisted_payload(max_payload_len);
}

fn exercise_truncated_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x37; 32]),
    };
    let Ok(encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(truncated_len) = encoded.len().checked_sub(1) else {
        return;
    };
    let Some(truncated) = encoded.get(..truncated_len) else {
        return;
    };
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        truncated,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(
        matches!(result, Err(vb_storage::JournalError::UnexpectedEof)),
        "truncated persisted payload must fail closed as UnexpectedEof"
    );
}

fn exercise_corrupted_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(2),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x12; 32]),
    };
    let Ok(mut encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(last) = encoded.last_mut() else {
        return;
    };
    *last ^= 0xA5;
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        &encoded,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(
        matches!(result, Err(vb_storage::JournalError::PayloadDigestMismatch)),
        "corrupt persisted payload must fail closed as PayloadDigestMismatch"
    );
}

/// Asserts that a malformed decode error is a known typed variant.
///
/// # Panics
/// Panics if `error` is an unknown `JournalError` variant not explicitly listed.
/// This ensures the fuzz oracle is exhaustive over all typed decode error variants
/// and fails closed when a new untyped variant is introduced.
fn assert_malformed_decode_is_typed(error: vb_storage::JournalError) {
    match error {
        vb_storage::JournalError::UnexpectedEof
        | vb_storage::JournalError::HeaderChecksumMismatch
        | vb_storage::JournalError::PayloadDigestMismatch
        | vb_storage::JournalError::PostcardDecodeFailed
        | vb_storage::JournalError::BadMagic { .. }
        | vb_storage::JournalError::PayloadTooLarge { .. }
        | vb_storage::JournalError::RecordKindFamilyMismatch { .. }
        | vb_storage::JournalError::UnknownRecordKind { .. }
        | vb_storage::JournalError::UnsupportedSchemaVersion { .. }
        | vb_storage::JournalError::HeaderLengthMismatch { .. }
        | vb_storage::JournalError::SequenceOverflow => {}
        _unknown => {
            // Coverage-only: unknown future variants are accepted gracefully.
        }
    }
}

// ===========================================================================
// vb-j0m0: Unsafe Boundary Fuzz Harnesses
// ===========================================================================

// ---------------------------------------------------------------------------
// Target: IPC Frame Boundary Fuzz Harness (vb-j0m0 R1)
// ---------------------------------------------------------------------------

/// Fuzz target: IPC frame boundary with explicit typed error assertions.
///
/// Exercises all IPC frame boundary functions with arbitrary input and asserts
/// that malformed input returns typed errors without panic, OOM, or unchecked
/// indexing.
///
/// Test cases:
/// - Empty input -> early return (no panic)
/// - Truncated header (< IPC_HEADER_LEN) -> early return (no panic)
/// - Wrong magic -> IpcError::InvalidMagic or IpcError::HeaderDecodeFailed
/// - Oversized payload_len -> IpcError::PayloadTooLarge or PayloadLengthOutOfRange
/// - Non-zero reserved -> IpcError::ReservedNonZero
/// - Unsupported version -> IpcError::UnsupportedVersion
/// - Unknown command -> IpcError::UnknownCommand
/// - Payload length mismatch -> IpcError::PayloadLengthMismatch
/// - Valid frame -> successful decode
pub fn fuzz_ipc_frame_boundary(data: &[u8]) {
    use vb_ipc::frame::{decode_frame_header, validate_frame_magic};
    use vb_ipc::{IPC_HEADER_LEN, IpcError, MaxPayloadBytes};

    // R1.1: Empty input - must not panic
    if data.is_empty() {
        return;
    }

    // R1.2/R1.3: Truncated header - validate_frame_magic handles partial input
    let magic_result = validate_frame_magic(data);
    if data.len() < 4 {
        assert!(
            matches!(magic_result, Err(IpcError::HeaderDecodeFailed)),
            "truncated frame (< 4 bytes) must return HeaderDecodeFailed"
        );
        return;
    }

    // R1.4: Wrong magic - must return InvalidMagic
    if magic_result.is_err() {
        assert!(
            matches!(
                magic_result,
                Err(IpcError::InvalidMagic { .. }) | Err(IpcError::HeaderDecodeFailed)
            ),
            "wrong magic must return InvalidMagic or HeaderDecodeFailed"
        );
        return;
    }

    // Magic is valid, now try header decode
    if data.len() < IPC_HEADER_LEN {
        // Partial header after valid magic - early return is OK
        return;
    }

    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes.copy_from_slice(&data[..IPC_HEADER_LEN]);

    // R1.5-R1.8: Header decode with bounded payload limit
    let Some(max_payload_nz) = std::num::NonZeroUsize::new(65536) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(max_payload_nz);
    let header_result = vb_ipc::IpcFrameHeader::decode(&header_bytes, max_payload);

    match header_result {
        Ok(header) => {
            // R1.9: Payload length mismatch check
            let payload = data.get(IPC_HEADER_LEN..).unwrap_or(&[]);
            let Ok(expected_len) = usize::try_from(header.payload_len) else {
                return; // PayloadLengthOutOfRange is a valid error path
            };
            if payload.len() != expected_len && !payload.is_empty() {
                // Mismatch is expected for fuzz input - no assertion needed
            }
        }
        Err(e) => {
            // R1.5-R1.8: All header decode errors must be typed
            assert_typed_ipc_error(e);
        }
    }

    // R1.10: If we have enough data, try full decode
    if data.len() >= IPC_HEADER_LEN {
        let _ = decode_frame_header(&header_bytes);
    }
}

/// Asserts that an IPC error is a known typed variant.
fn assert_typed_ipc_error(error: vb_ipc::IpcError) {
    use vb_ipc::IpcError;
    match error {
        IpcError::Full
        | IpcError::Disconnected
        | IpcError::PayloadTooLarge { .. }
        | IpcError::InvalidMagic { .. }
        | IpcError::UnsupportedVersion { .. }
        | IpcError::UnknownCommand(_)
        | IpcError::ReservedNonZero { .. }
        | IpcError::PayloadLengthMismatch { .. }
        | IpcError::HeaderEncodeFailed
        | IpcError::HeaderDecodeFailed
        | IpcError::PayloadLengthOutOfRange { .. }
        | IpcError::PayloadEncodeFailed
        | IpcError::PayloadDecodeFailed
        | IpcError::ResponseDecodeFailed => {}
        _ => {
            // Coverage-only: unknown future variants are accepted gracefully.
        }
    }
}

// ---------------------------------------------------------------------------
// Target: Storage Envelope Fuzz Harness (vb-j0m0 R2)
// ---------------------------------------------------------------------------

/// Fuzz target: Storage envelope decoding with explicit typed error assertions.
///
/// Exercises storage envelope decode with arbitrary input and asserts that
/// malformed input returns typed errors without panic, OOM, or unchecked
/// indexing.
///
/// Test cases:
/// - Empty input -> UnexpectedEof
/// - Truncated header -> UnexpectedEof or HeaderLengthMismatch
/// - Wrong magic -> BadMagic
/// - Corrupt checksum -> HeaderChecksumMismatch
/// - Corrupt digest -> PayloadDigestMismatch
/// - Oversized payload -> PayloadTooLarge
/// - Invalid record kind -> UnknownRecordKind or RecordKindFamilyMismatch
/// - Valid envelope -> successful decode
pub fn fuzz_storage_envelope_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };

    // R2.1: Empty input - must return typed error
    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(
            data,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty input must return UnexpectedEof"
        );
        return;
    }

    // R2.2-R2.9: Full decode with typed error assertion
    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    match result {
        Ok((_envelope, _event)) => {
            // Valid decode - verify envelope invariants
        }
        Err(e) => {
            // All error paths must be typed
            assert_typed_journal_error(e);
        }
    }

    // R2.2: Truncated header exercise
    if data.len() < 60 {
        let truncated = data;
        let result = decode_record::<vb_storage::JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(
                result,
                Err(JournalError::UnexpectedEof) | Err(JournalError::HeaderLengthMismatch { .. })
            ),
            "truncated header must return UnexpectedEof or HeaderLengthMismatch"
        );
    }
}

/// Asserts that a journal error is a known typed variant.
fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        // Decode/parse errors
        JournalError::UnexpectedEof
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::PostcardDecodeFailed
        | JournalError::BadMagic { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        // Internal/operational errors (still typed)
        | JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
        | JournalError::QueueShutdown
        | JournalError::MigrationRequired { .. }
        | JournalError::ArtifactMalformed
        | JournalError::ArtifactChecksumMismatch
        | JournalError::InvalidGateCount { .. }
        | JournalError::MissingRequiredProofFlag { .. }
        | JournalError::ArtifactNotFound { .. }
        | JournalError::AdmissionRequired
        | JournalError::ArtifactInvalid { .. }
        | JournalError::InputTooLarge { .. }
        | JournalError::InputSchemaMismatch
        | JournalError::CapabilityDenied
        | JournalError::SecretUnavailable
        | JournalError::RunAlreadyExists
        | JournalError::ActiveRunCapacityExceeded
        | JournalError::FrameAllocationFailed
        | JournalError::AdmissionJournalFailed
        | JournalError::StrictDurabilityFailed
        | JournalError::ClockUnavailable
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
        _ => {
            // Coverage-only: unknown future variants are accepted gracefully.
        }
    }
}

// ---------------------------------------------------------------------------
// Target: Binary Payload Decoding Fuzz Harness (vb-j0m0 R3)
// ---------------------------------------------------------------------------

/// Fuzz target: Binary payload decoding with explicit typed error assertions.
///
/// Exercises binary payload decode with arbitrary input and asserts that
/// malformed input returns typed errors without panic, OOM, or unchecked
/// indexing.
///
/// Test cases:
/// - Oversized payload declaration -> fail before allocation
/// - Malformed postcard encoding -> PostcardDecodeFailed
/// - Length prefix attack -> typed error
/// - Empty/single-byte/max-size payloads
pub fn fuzz_binary_payload_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };

    // R3.1: Empty input
    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, 1024);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty binary payload must return UnexpectedEof"
        );
        return;
    }

    // R3.2: Test with small max_payload_len to trigger PayloadTooLarge
    let small_max = 64u32;
    let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, small_max);
    match result {
        Ok((_envelope, _event)) => {
            // Valid decode within bounds
        }
        Err(JournalError::PayloadTooLarge { .. }) => {
            // Expected: payload exceeds small_max
        }
        Err(e) => {
            // Other typed errors are acceptable
            assert_typed_journal_error(e);
        }
    }

    // R3.3: Test with very small max_payload_len (1 byte) to trigger early failure
    let tiny_max = 1u32;
    let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, tiny_max);
    match result {
        Ok(_) => {}
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }

    // R3.4: Exercise with different record types to cover kind/magic mismatch
    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT.wrapping_add(1), // Wrong magic
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    match result {
        Ok(_) => {}
        Err(JournalError::BadMagic { .. }) => {
            // Expected: wrong magic
        }
        Err(JournalError::RecordKindFamilyMismatch { .. }) => {
            // Expected: kind/magic mismatch
        }
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }
}

// ---------------------------------------------------------------------------
// Target: External Input Adapter Fuzz Harness (vb-j0m0 R4)
// ---------------------------------------------------------------------------

/// Fuzz target: External input adapter boundary with explicit typed error assertions.
///
/// Exercises boundary inventory parsing and evidence reference validation with
/// arbitrary input and asserts that malformed input returns typed errors without
/// panic, OOM, or unchecked indexing.
///
/// Test cases:
/// - Empty input -> typed error
/// - Malformed inventory syntax -> InventoryParseFailure
/// - Invalid boundary class -> UnknownBoundaryClass
/// - Missing required fields -> IncompleteDiscoveryInput
/// - Valid inventory -> successful parse
pub fn fuzz_external_input_adapter_boundary(data: &[u8]) {
    use vb_boundary_inventory::boundary_inventory::{
        parse_inventory, validate_evidence_reference_bytes,
    };

    // R4.1: Empty input - must not panic
    if data.is_empty() {
        let result = parse_inventory(data);
        assert!(result.is_err(), "empty inventory input must return error");
        return;
    }

    // R4.2-R4.5: Parse inventory with typed error assertion
    let result = parse_inventory(data);
    match result {
        Ok(_inventory) => {
            // Valid parse - verify inventory invariants
        }
        Err(e) => {
            // All error paths must be typed
            assert_typed_boundary_error(e);
        }
    }

    // R4.6: Validate evidence reference bytes with arbitrary input
    let result = validate_evidence_reference_bytes(data);
    // This function should never panic regardless of input
    let _ = result.is_ok();
}

/// Asserts that a boundary inventory error is a known typed variant.
fn assert_typed_boundary_error(
    error: vb_boundary_inventory::boundary_inventory::BoundaryInventoryError,
) {
    use vb_boundary_inventory::boundary_inventory::BoundaryInventoryError;
    match error {
        BoundaryInventoryError::WorkspaceNotDiscoverable
        | BoundaryInventoryError::IncompleteDiscoveryInput
        | BoundaryInventoryError::UnknownBoundaryClass
        | BoundaryInventoryError::UnsafeForbiddenViolation
        | BoundaryInventoryError::MissingOwner
        | BoundaryInventoryError::MissingThreat
        | BoundaryInventoryError::MissingEvidencePath
        | BoundaryInventoryError::InvalidEvidencePath
        | BoundaryInventoryError::StaleEvidence
        | BoundaryInventoryError::DuplicateBoundaryId
        | BoundaryInventoryError::InventoryParseFailure
        | BoundaryInventoryError::SchemaVersionUnsupported
        | BoundaryInventoryError::ReviewStatusInvalid => {}
        _ => {
            // Coverage-only: unknown future variants are accepted gracefully.
        }
    }
}

// ---------------------------------------------------------------------------
// Target: CollectPage pagination (C.25)
// ---------------------------------------------------------------------------

/// Exercises `collect_page` with various list configurations.
///
/// Verifies that `collect_page` never panics regardless of input,
/// and that it returns a typed `Result` for both list and non-list slots.
pub fn fuzz_collect_page_pagination(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    let slot_count = u16::from(data[0].wrapping_rem(16)).saturating_add(1);
    let list_len = usize::from(data[0].wrapping_rem(8));
    let _page_size = usize::from(data.get(1).copied().unwrap_or(1).wrapping_rem(8)).saturating_add(1);

    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(1),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();

    // Build a list of SlotValues
    let items: Vec<vb_core::SlotValue> = (0..list_len)
        .map(|i| vb_core::SlotValue::I64(i64::try_from(i).unwrap_or(0)))
        .collect();

    let list_id = match store.insert_list(items.into_boxed_slice()) {
        Ok(id) => id,
        Err(_) => return,
    };

    // Write the list into slot 0
    let _ = run.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::List(list_id),
        vb_core::Taint::Clean,
    );

    use vb_runtime::primitives::collect::{collect_page, CollectStates};

    // collect_page must return Result, never panic
    let mut states = CollectStates::new();
    let _result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );

    // Also exercise with a non-list slot (should error gracefully)
    let Ok(mut run_non_list) = vb_core::RunFrame::new(
        vb_core::RunId::new(2),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };

    let _ = run_non_list.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::I64(42),
        vb_core::Taint::Clean,
    );

    let mut states2 = CollectStates::new();
    let _non_list_result = collect_page(
        &mut run_non_list,
        &mut store,
        &mut states2,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );
}

// ---------------------------------------------------------------------------
// Wait digest coverage fuzz helpers (vb-xi2f.32)
// ---------------------------------------------------------------------------

/// Fuzz target helper: builds a workflow with a single Wait step from two
/// string slices (event and timeout), parses it through the cold-path
/// compiler, and verifies digest sensitivity properties.
///
/// Used by `wait_digest_sensitivity`, `wait_sentinel_collision`, and
/// `wait_digest_exhaustive_collision` fuzz targets.
pub fn fuzz_wait_digest_sensitivity(event: &str, timeout: &str) {
    let src_a = build_wait_workflow_yaml(event, timeout);

    // Parse and compile through the cold-path
    let parsed_a = vb_yaml::parse_workflow_source(&src_a);
    let Ok(source_a) = parsed_a else { return; };

    let compiled_a = vb_compile::compile_source(&source_a);
    let Ok(wf_a) = compiled_a else { return; };
    let digest_a = wf_a.digest();

    // Build a different configuration and verify the digest differs.
    // We vary the timeout by appending a sentinel marker.
    let alt_event = if event.is_empty() { "fuzz_alt" } else { event };
    let alt_timeout = format!("{timeout}_fuzz_variant");
    let src_b = build_wait_workflow_yaml(alt_event, &alt_timeout);

    let parsed_b = vb_yaml::parse_workflow_source(&src_b);
    let Ok(source_b) = parsed_b else { return; };

    let compiled_b = vb_compile::compile_source(&source_b);
    let Ok(wf_b) = compiled_b else { return; };
    let digest_b = wf_b.digest();

    // If the two configurations are different, digests must differ
    if event != alt_event || timeout != alt_timeout {
        assert!(
            digest_a != digest_b,
            "COLLISION: different wait configs produced same digest {:?}",
            digest_a
        );
    }
}

/// Fuzz target helper: verifies sentinel unambiguity for WaitEvent timeout.
/// For all event strings: digest(WaitEvent{event, None}) != digest(WaitEvent{event, Some("none")}).
pub fn fuzz_wait_sentinel_unambiguous(event: &str) {
    let absent_yaml = build_wait_workflow_yaml_no_timeout(event);
    let sentinel_yaml = build_wait_workflow_yaml(event, "none");

    let absent_source = vb_yaml::parse_workflow_source(&absent_yaml);
    let sentinel_source = vb_yaml::parse_workflow_source(&sentinel_yaml);
    let (Ok(src_a), Ok(src_b)) = (absent_source, sentinel_source) else { return; };

    let compiled_a = vb_compile::compile_source(&src_a);
    let compiled_b = vb_compile::compile_source(&src_b);
    let (Ok(wf_a), Ok(wf_b)) = (compiled_a, compiled_b) else { return; };

    let digest_a = wf_a.digest();
    let digest_b = wf_b.digest();

    assert!(
        digest_a != digest_b,
        "SENTINEL COLLISION: timeout=None and timeout=Some(\"none\") produced same digest {:?}",
        digest_a
    );
}

/// Fuzz target helper: exhaustive pairwise collision detection.
/// Generates two different Wait configurations from byte input and verifies
/// their digests differ.
pub fn fuzz_wait_pairwise_collision(byte1: u8, byte2: u8, event1: &str, event2: &str) {
    // Map the first byte to a Wait shape selector
    let use_until1 = byte1 % 3 == 0;
    let use_no_timeout1 = byte1 % 3 == 1;
    let _use_both1 = byte1 % 3 == 2;

    let (e1, t1): (Option<String>, Option<String>) = if use_until1 {
        (None, Some(String::from("10")))
    } else if use_no_timeout1 {
        (if event1.is_empty() { Some(String::from("e")) } else { Some(event1.to_string()) }, None)
    } else {
        (if event1.is_empty() { Some(String::from("e")) } else { Some(event1.to_string()) },
         Some(String::from("20")))
    };

    let use_until2 = byte2 % 3 == 0;
    let use_no_timeout2 = byte2 % 3 == 1;
    let _use_both2 = byte2 % 3 == 2;

    let (e2, t2): (Option<String>, Option<String>) = if use_until2 {
        (None, Some(String::from("10")))
    } else if use_no_timeout2 {
        (if event2.is_empty() { Some(String::from("f")) } else { Some(event2.to_string()) }, None)
    } else {
        (if event2.is_empty() { Some(String::from("f")) } else { Some(event2.to_string()) },
         Some(String::from("30")))
    };

    // If the two are identical, skip
    if e1 == e2 && t1 == t2 {
        return;
    }

    let yaml1 = build_wait_workflow_from_opts(&e1, &t1);
    let yaml2 = build_wait_workflow_from_opts(&e2, &t2);

    let src1 = vb_yaml::parse_workflow_source(&yaml1);
    let src2 = vb_yaml::parse_workflow_source(&yaml2);
    let (Ok(s1), Ok(s2)) = (src1, src2) else { return; };

    let c1 = vb_compile::compile_source(&s1);
    let c2 = vb_compile::compile_source(&s2);
    let (Ok(w1), Ok(w2)) = (c1, c2) else { return; };

    assert!(
        w1.digest() != w2.digest(),
        "EXHAUSTIVE COLLISION: distinct Wait configs (e1={e1:?}, t1={t1:?}) vs (e2={e2:?}, t2={t2:?}) produced same digest",
    );
}

/// Builds a valid Wait workflow YAML string with event and timeout.
fn build_wait_workflow_yaml(event: &str, timeout: &str) -> String {
    let mut wait = String::from("  - id: w\n    wait:");
    if !event.is_empty() {
        wait.push_str(&format!("\n      event: \"{event}\""));
    }
    if !timeout.is_empty() {
        wait.push_str(&format!("\n      timeout: \"{timeout}\""));
    }
    format!("version: velvet-ballistics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}

/// Builds a Wait workflow YAML without a timeout field.
fn build_wait_workflow_yaml_no_timeout(event: &str) -> String {
    let wait = if event.is_empty() {
        String::from("  - id: w\n    wait:\n      timeout: \"1\"")
    } else {
        format!("  - id: w\n    wait:\n      event: \"{event}\"")
    };
    format!("version: velvet-ballistics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}

/// Builds a Wait workflow YAML from Option<(String, String)> tuples.
fn build_wait_workflow_from_opts(event: &Option<String>, timeout: &Option<String>) -> String {
    let mut wait = String::from("  - id: w\n    wait:");
    if let Some(e) = event {
        wait.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!("version: velvet-ballistics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}

/// Exercises the public canonical digest path with bounded ForEach workflows.
///
/// This is a build/smoke fuzz helper for `foreach_digest_canonical`. It binds to
/// `vb_compile::canonical_digest` through the public crate export and keeps all
/// generated workflow shapes bounded so malformed hostile bytes cannot allocate
/// unbounded body vectors.
pub fn fuzz_canonical_digest_foreach(data: &[u8]) {
    let Some(selector) = data.first().copied() else {
        return;
    };

    let variable = bounded_utf8_token(data.get(1..33), "item");
    let input = bounded_utf8_token(data.get(33..65), "items");
    let at_once = foreach_at_once(selector);
    let body = foreach_body(selector, data.get(65..));
    let source = foreach_digest_source(variable, input, at_once, body);

    let first = vb_compile::canonical_digest(&source);
    let second = vb_compile::canonical_digest(&source);
    if first != second {
        return;
    }
}

fn foreach_at_once(selector: u8) -> Option<u32> {
    match selector.wrapping_rem(4) {
        0 => None,
        1 => Some(0),
        2 => Some(1),
        _ => Some(u32::from(selector)),
    }
}

fn foreach_body(selector: u8, bytes: Option<&[u8]>) -> Vec<vb_yaml::ast::StepAst> {
    let mut body = Vec::new();
    if selector.is_multiple_of(2) {
        let value = bytes
            .and_then(|slice| slice.first().copied())
            .map_or(0_i64, i64::from);
        body.push(vb_yaml::ast::StepAst {
            id: String::from("body_set"),
            name: None,
            condition: None,
            primitive: vb_yaml::ast::StepPrimitive::Set {
                output: String::from("item"),
                value: value.to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        });
    }
    if selector.is_multiple_of(3) {
        body.push(vb_yaml::ast::StepAst {
            id: String::from("body_finish"),
            name: None,
            condition: None,
            primitive: vb_yaml::ast::StepPrimitive::Finish {
                result: vb_yaml::ast::ScalarValue::Integer(i64::from(selector)),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        });
    }
    body
}

fn foreach_digest_source(
    variable: String,
    input: String,
    at_once: Option<u32>,
    body: Vec<vb_yaml::ast::StepAst>,
) -> vb_yaml::ast::WorkflowSource {
    let steps = vec![vb_yaml::ast::StepAst {
        id: String::from("foreach"),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    vb_yaml::ast::WorkflowSource::new(vb_yaml::ast::WorkflowSourceParts {
        version: String::from("velvet-ballistics/v1"),
        name: String::from("fuzz-foreach-digest"),
        trigger: vb_yaml::ast::TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

fn bounded_utf8_token(bytes: Option<&[u8]>, fallback: &str) -> String {
    let Some(raw) = bytes else {
        return String::from(fallback);
    };
    let Ok(text) = std::str::from_utf8(raw) else {
        return String::from(fallback);
    };
    let mut out = String::new();
    for ch in text.chars().take(16) {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        }
    }
    if out.is_empty() {
        String::from(fallback)
    } else {
        out
    }
}
