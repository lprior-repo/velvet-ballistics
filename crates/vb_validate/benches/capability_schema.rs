#![forbid(unsafe_code)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts, node};

fn parts() -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("capability-schema-bench"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(
                StepIdx::new(0),
                CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
        name: ActionName::new("test-action").unwrap(),
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

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;measurement=criterion-timing;external-counters=separate-evidence";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

fn bench_capability_schema_validation(c: &mut Criterion) {
    let parts = parts();
    let contracts = [contract()];
    let fixture_bytes = parts.nodes.len().saturating_add(contracts.len());
    let id = metadata(
        "capability schema validation",
        fixture_bytes,
        "fixture=workflow_2_nodes_1_contract;surface=capability_schema_validation",
    );
    c.bench_function(&id, |b| {
        b.iter(|| {
            black_box(vb_validate::shared::validate_with_contracts(
                black_box(&parts),
                black_box(&contracts),
            ))
        })
    });
}

criterion_group!(benches, bench_capability_schema_validation);
criterion_main!(benches);
