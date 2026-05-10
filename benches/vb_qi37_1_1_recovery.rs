use criterion::{Criterion, criterion_group, criterion_main};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

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
        },
        JournalEvent::StepSucceeded {
            run: RunId::new(90),
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: SlotIdx::ZERO,
        },
    ];

    c.bench_function("vb_qi37_1_1_no_output_recovery", |b| {
        b.iter(|| vb_storage::recovery::recover_runtime_frame_seed_from_events(&events))
    });
}

criterion_group!(benches, bench_no_output_recovery);
criterion_main!(benches);
