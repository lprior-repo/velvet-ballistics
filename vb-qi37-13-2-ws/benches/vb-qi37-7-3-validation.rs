// vb-qi37.7.3 Benchmarks — Symbol Bounds & Resource Contract Validation
// Criterion benchmarks for validate_gate_08 and validate_resource_contract hot paths.

use criterion::{Criterion, Throughput};
use vb_core::engine::validate_resource_contract;
use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, WorkflowParts,
};
use vb_validate::gates::validate_gate_08_accessor_path_segments;

// ---------------------------------------------------------------------------
// Helper: minimal WorkflowParts factory
// ---------------------------------------------------------------------------

fn make_parts(
    slot_count: u16,
    symbols_count: u32,
    accessors: Vec<AccessorProgram>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("bench"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: accessors.into_boxed_slice(),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn accessor(root: u16, path: Vec<PathSegment>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path: path.into_boxed_slice(),
    }
}

// ---------------------------------------------------------------------------
// BM-01: validate_gate_08 — empty accessors (baseline)
// ---------------------------------------------------------------------------

pub fn bench_gate_08_empty_accessors(c: &mut Criterion) {
    let parts = make_parts(1, 0, vec![]);
    let mut group = c.benchmark_group("validate_gate_08/empty_accessors");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_gate_08", |b| {
        b.iter(|| validate_gate_08_accessor_path_segments(&parts))
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// BM-02: validate_gate_08 — single valid field segment
// ---------------------------------------------------------------------------

pub fn bench_gate_08_single_field_segment(c: &mut Criterion) {
    let parts = make_parts(
        1,
        10,
        vec![accessor(0, vec![PathSegment::Field(SymbolId::new(5))])],
    );
    let mut group = c.benchmark_group("validate_gate_08/single_field_segment");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_gate_08", |b| {
        b.iter(|| validate_gate_08_accessor_path_segments(&parts))
    });
}

// ---------------------------------------------------------------------------
// BM-03: validate_gate_08 — multiple field segments (at boundary)
// ---------------------------------------------------------------------------

pub fn bench_gate_08_multiple_field_segments(c: &mut Criterion) {
    let parts = make_parts(
        1,
        10,
        vec![accessor(
            0,
            vec![
                PathSegment::Field(SymbolId::new(0)),
                PathSegment::Field(SymbolId::new(5)),
                PathSegment::Field(SymbolId::new(9)),
            ],
        )],
    );
    let mut group = c.benchmark_group("validate_gate_08/multiple_field_segments");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_gate_08", |b| {
        b.iter(|| validate_gate_08_accessor_path_segments(&parts))
    });
}

// ---------------------------------------------------------------------------
// BM-04: validate_gate_08 — many accessors (10 accessors)
// ---------------------------------------------------------------------------

pub fn bench_gate_08_many_accessors(c: &mut Criterion) {
    let accessors: Vec<_> = (0..10)
        .map(|i| {
            accessor(
                i as u16 % 5,
                vec![PathSegment::Field(SymbolId::new(i as u32 % 10))],
            )
        })
        .collect();
    let parts = make_parts(5, 10, accessors);
    let mut group = c.benchmark_group("validate_gate_08/many_accessors");
    group.throughput(Throughput::Elements(10));
    group.bench_function("validate_gate_08", |b| {
        b.iter(|| validate_gate_08_accessor_path_segments(&parts))
    });
}

// ---------------------------------------------------------------------------
// BM-05: validate_resource_contract — within bounds (DEFAULT contract)
// ---------------------------------------------------------------------------

pub fn bench_resource_contract_within_bounds(c: &mut Criterion) {
    let parts = make_parts(1, 0, vec![]);
    // Default contract is within all bounds
    let mut group = c.benchmark_group("validate_resource_contract/within_bounds");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_resource_contract", |b| {
        b.iter(|| validate_resource_contract(&parts))
    });
}

// ---------------------------------------------------------------------------
// BM-06: validate_resource_contract — max_steps at hard limit
// ---------------------------------------------------------------------------

pub fn bench_resource_contract_max_steps_at_limit(c: &mut Criterion) {
    let mut parts = make_parts(1, 0, vec![]);
    parts.resource_contract.max_steps = vb_core::limits::MAX_STEPS_PER_WORKFLOW as u16;
    let mut group = c.benchmark_group("validate_resource_contract/max_steps_at_limit");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_resource_contract", |b| {
        b.iter(|| validate_resource_contract(&parts))
    });
}

// ---------------------------------------------------------------------------
// BM-07: validate_resource_contract — all fields at limit
// ---------------------------------------------------------------------------

pub fn bench_resource_contract_all_at_limit(c: &mut Criterion) {
    let mut parts = make_parts(1, 0, vec![]);
    parts.resource_contract = ResourceContract {
        max_steps: vb_core::limits::MAX_STEPS_PER_WORKFLOW as u16,
        max_slots: vb_core::limits::MAX_SLOTS_PER_WORKFLOW as u16,
        max_constants: vb_core::limits::MAX_CONSTANTS as u16,
        max_accessors: vb_core::limits::MAX_ACCESSORS as u16,
        max_expressions: vb_core::limits::MAX_EXPRESSIONS as u16,
        max_expr_stack: vb_core::limits::MAX_EXPRESSION_STACK,
        ..ResourceContract::DEFAULT
    };
    let mut group = c.benchmark_group("validate_resource_contract/all_at_limit");
    group.throughput(Throughput::Elements(1));
    group.bench_function("validate_resource_contract", |b| {
        b.iter(|| validate_resource_contract(&parts))
    });
}
