#![forbid(unsafe_code)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

fn parts() -> WorkflowParts {
    WorkflowParts {
        name: Box::from("capability-schema-bench"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
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

fn contract() -> ActionContract {
    ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new(
            Box::from("network.github"),
            ActionId::new(1),
        )]),
    }
}

fn bench_capability_schema_validation(c: &mut Criterion) {
    let parts = parts();
    let contracts = [contract()];
    c.bench_function("capability schema validation", |b| {
        b.iter(|| {
            vb_validate::shared::validate_with_contracts(black_box(&parts), black_box(&contracts))
        })
    });
}

criterion_group!(benches, bench_capability_schema_validation);
criterion_main!(benches);
