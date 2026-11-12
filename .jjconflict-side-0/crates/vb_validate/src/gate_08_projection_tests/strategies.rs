use crate::test_helpers::make_parts;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::source_metadata::{
    CompiledSecretRequirement, SourceMetadataError, WorkflowSourceMetadata,
};
use vb_core::workflow::{
    AccessorProgram, CompiledInputConstraint, CompiledInputField, CompiledInputSlot,
    CompiledNodeKind, ExprOp, ExprProgram, InputDefaultValue, InputSlotKind, PathSegment,
    ResourceContract, SecretResultPolicy, WorkflowParts, node,
};

fn kind() -> impl Strategy<Value = InputSlotKind> {
    prop::sample::select(vec![
        InputSlotKind::Null,
        InputSlotKind::Bool,
        InputSlotKind::I64,
        InputSlotKind::F64,
        InputSlotKind::Symbol,
        InputSlotKind::List,
        InputSlotKind::Object,
        InputSlotKind::Blob,
        InputSlotKind::Any,
    ])
}

fn cold_default() -> impl Strategy<Value = InputDefaultValue> {
    prop_oneof![
        Just(InputDefaultValue::Null),
        any::<bool>().prop_map(InputDefaultValue::Bool),
        any::<i64>().prop_map(InputDefaultValue::I64),
        ".{0,12}".prop_map(|s| InputDefaultValue::Text(s.into_boxed_str())),
    ]
    .prop_recursive(2, 12, 2, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..=2)
                .prop_map(|v| InputDefaultValue::List(v.into_boxed_slice())),
            prop::collection::vec(("[a-z]{0,8}", inner), 0..=2).prop_map(|v| {
                InputDefaultValue::Object(
                    v.into_iter()
                        .map(|(k, v)| (k.into_boxed_str(), v))
                        .collect(),
                )
            }),
        ]
    })
}

fn schema() -> impl Strategy<Value = CompiledInputConstraint> {
    let leaf = (
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            prop::option::of(any::<i64>()),
        ),
        (
            prop::option::of(any::<i64>()),
            prop::option::of(any::<i64>()),
            prop::option::of(any::<i64>()),
            prop::option::of(any::<i64>()),
        ),
        (
            prop::option::of(kind()),
            prop::option::of(any::<bool>()),
            prop::option::of(cold_default()),
        ),
    )
        .prop_map(
            |(
                (optional, nullable, secret, default),
                (min, max, min_length, max_length),
                (element, allow_extra, default_cold),
            )| CompiledInputConstraint {
                presence: optional.into(),
                nullability: nullable.into(),
                secretness: secret.into(),
                default_value: default.map(SlotValue::I64),
                min,
                max,
                min_length,
                max_length,
                element,
                extra: allow_extra.into(),
                default_cold,
                fields: None,
            },
        )
        .boxed();
    leaf.clone().prop_recursive(2, 12, 2, move |inner| {
        (
            leaf.clone(),
            prop::collection::vec(("[a-z]{0,8}", kind(), inner), 0..=2),
        )
            .prop_map(|(mut parent, fields)| {
                parent.fields = Some(
                    fields
                        .into_iter()
                        .map(|(name, kind, constraint)| CompiledInputField {
                            name: name.into_boxed_str(),
                            kind,
                            constraint,
                        })
                        .collect(),
                );
                parent
            })
    })
}

fn resources() -> impl Strategy<Value = ResourceContract> {
    (
        any::<[u16; 7]>(),
        any::<[u32; 6]>(),
        any::<[u64; 3]>(),
        any::<u8>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                [
                    max_steps,
                    max_slots,
                    max_constants,
                    max_accessors,
                    max_expressions,
                    max_retry_attempts,
                    max_fanout,
                ],
                [
                    max_input_bytes,
                    max_output_bytes,
                    max_ipc_payload_bytes,
                    max_collect_items,
                    max_queue_depth,
                    max_journal_batch_bytes,
                ],
                [
                    max_step_budget_per_tick,
                    max_transitions_per_tick,
                    max_blob_bytes,
                ],
                max_expr_stack,
                allow,
            )| ResourceContract {
                max_steps,
                max_slots,
                max_constants,
                max_accessors,
                max_expressions,
                max_retry_attempts,
                max_fanout,
                max_input_bytes,
                max_output_bytes,
                max_ipc_payload_bytes,
                max_collect_items,
                max_queue_depth,
                max_journal_batch_bytes,
                max_step_budget_per_tick,
                max_transitions_per_tick,
                max_blob_bytes,
                max_expr_stack,
                allows_secret_results: if allow {
                    SecretResultPolicy::Allow
                } else {
                    SecretResultPolicy::Deny
                },
            },
        )
}

pub(super) fn unrelated_parts() -> impl Strategy<Value = Result<WorkflowParts, SourceMetadataError>>
{
    (
        (
            ".{0,24}",
            any::<[u8; 32]>(),
            prop::option::of(any::<[u8; 32]>()),
            any::<u16>(),
        ),
        (
            prop::collection::vec((any::<u16>(), any::<u16>(), any::<bool>()), 0..=8),
            prop::collection::vec(
                (prop::collection::vec(any::<u16>(), 0..=16), any::<u8>()),
                0..=4,
            ),
            prop::collection::vec(any::<i64>(), 0..=4),
        ),
        (
            resources(),
            prop::collection::vec(".{0,12}", 0..=4),
            prop::collection::vec((any::<u16>(), kind(), schema()), 0..=4),
            prop::option::of(("[a-z]{1,8}", any::<bool>())),
        ),
    )
        .prop_map(
            |(
                (name, digest, identity, entry),
                (nodes, exprs, constants),
                (resources, names, inputs, metadata),
            )| {
                let nodes = nodes
                    .into_iter()
                    .map(|(id, source, copy)| {
                        node(
                            StepIdx::new(id),
                            if copy {
                                CompiledNodeKind::Copy {
                                    source: SlotIdx::new(source),
                                }
                            } else {
                                CompiledNodeKind::Nop
                            },
                            Some(SlotIdx::new(source)),
                            Some(StepIdx::new(id)),
                        )
                    })
                    .collect();
                let mut parts = make_parts(nodes, 0);
                parts.name = name.into_boxed_str();
                parts.digest = WorkflowDigest::from_bytes(digest);
                parts.semantic_source_identity = identity.map(WorkflowDigest::from_bytes);
                parts.entry = StepIdx::new(entry);
                parts.expressions = exprs
                    .into_iter()
                    .map(|(ops, max_stack)| ExprProgram {
                        ops: ops
                            .into_iter()
                            .map(|slot| ExprOp::LoadSlot(SlotIdx::new(slot)))
                            .collect(),
                        max_stack,
                    })
                    .collect();
                parts.constants = constants.into_iter().map(ConstValue::I64).collect();
                parts.resource_contract = resources;
                parts.step_names = names.into_iter().map(String::into_boxed_str).collect();
                parts.input_slots = inputs
                    .into_iter()
                    .map(|(slot, kind, constraint)| CompiledInputSlot {
                        slot: SlotIdx::new(slot),
                        kind,
                        constraint,
                    })
                    .collect();
                if let Some((symbol, secret)) = metadata {
                    let requirements: Box<[_]> = if secret {
                        Box::new([CompiledSecretRequirement::new(
                            SymbolId::new(0),
                            SymbolId::new(0),
                        )])
                    } else {
                        Box::new([])
                    };
                    parts.source_metadata = Some(Box::new(WorkflowSourceMetadata::try_new(
                        Box::new([symbol.into_boxed_str()]),
                        requirements,
                    )?));
                }
                Ok(parts)
            },
        )
}

pub(super) fn accessors() -> impl Strategy<Value = Vec<AccessorProgram>> {
    let segment = prop_oneof![
        any::<u32>().prop_map(|symbol| PathSegment::Field(SymbolId::new(symbol))),
        prop_oneof![Just(u32::MAX), any::<u32>()].prop_map(PathSegment::Index),
    ];
    prop::collection::vec(
        (any::<u16>(), prop::collection::vec(segment, 0..=18)),
        0..=6,
    )
    .prop_map(|values| {
        values
            .into_iter()
            .map(|(root, path)| AccessorProgram {
                root: SlotIdx::new(root),
                path: path.into_boxed_slice(),
            })
            .collect()
    })
}
