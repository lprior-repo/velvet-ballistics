use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;measurement=criterion-timing;external-counters=separate-evidence";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

fn bench_no_output_recovery(c: &mut Criterion) {
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(90),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([90; 32]),
        },
        JournalEvent::StepStarted {
            run: RunId::new(90),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: RunId::new(90),
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: SlotIdx::ZERO,
        },
    ];

    let id = metadata(
        "vb_qi37_1_1_no_output_recovery",
        events.len(),
        "fixture=events_3;surface=no_output_recovery",
    );
    c.bench_function(&id, |b| {
        b.iter(|| {
            black_box(
                vb_storage::recovery::recover_runtime_frame_seed_from_events(black_box(&events)),
            )
        })
    });
}

criterion_group!(benches, bench_no_output_recovery);
criterion_main!(benches);
