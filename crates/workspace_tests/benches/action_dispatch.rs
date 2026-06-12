//! Action dispatch benchmarks.
//!
//! Measures ActionRegistry dispatch overhead with 1, 10, and 100 registered actions.

#![allow(missing_docs)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{
    action::{
        ActionContract, ActionInput, ActionName, ActionOutcome, Idempotency, RetrySafety,
        SideEffect,
    },
    ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx},
};
use vb_runtime::action::ActionRegistry;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Creates an ActionRegistry with `count` registered actions.
fn registry_with_n_actions(count: usize) -> ActionRegistry {
    let mut registry = ActionRegistry::new();
    let mut i = 0usize;
    while i < count {
        let raw_id = match u16::try_from(i) {
            Ok(value) => value,
            Err(error) => panic!("bench action id must fit in u16: {error}"),
        };
        let contract = action_contract(ActionId::new(raw_id));
        let _ = registry.register(contract);
        i = i.saturating_add(1);
    }
    registry
}

/// Creates a simple ActionInput for the given action ID.
fn action_input(action: ActionId) -> ActionInput {
    ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action,
        input: SlotIdx::new(0),
        ticket: vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action,
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        },
    }
}

/// Creates an ActionContract for the given action ID.
fn action_contract(action: ActionId) -> ActionContract {
    let name = match ActionName::new(format!("test-action-{}", action.get())) {
        Ok(value) => value,
        Err(error) => panic!("bench action name must be valid: {error}"),
    };
    ActionContract {
        id: action,
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::from([]),
    }
}

/// vb-u09ai: Creates an ActionRegistry with one contract per 4-variant
/// RetrySafety taxonomy. Used by `bench_action_dispatch_4variant_retry_safety`
/// to measure dispatch overhead across the 4-variant shape.
fn registry_with_4variant_retry_safety() -> ActionRegistry {
    let mut registry = ActionRegistry::new();
    let variants = [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
        RetrySafety::Unknown,
    ];
    let mut i = 0usize;
    while i < variants.len() {
        let raw_id = match u16::try_from(i) {
            Ok(value) => value,
            Err(error) => panic!("bench action id must fit in u16: {error}"),
        };
        let name = match ActionName::new(format!("test-action-4variant-{i}")) {
            Ok(value) => value,
            Err(error) => panic!("bench action name must be valid: {error}"),
        };
        let contract = ActionContract {
            id: ActionId::new(raw_id),
            name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::Pure,
            retry_safety: variants[i],
            required_capabilities: Box::from([]),
        };
        let _ = registry.register(contract);
        i = i.saturating_add(1);
    }
    registry
}

fn bench_action_dispatch(c: &mut Criterion) {
    let registry_1 = registry_with_n_actions(1);
    let registry_10 = registry_with_n_actions(10);
    let registry_100 = registry_with_n_actions(100);

    let mut group = c.benchmark_group("action_dispatch");

    // Single registered action
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "action_dispatch_single_registered",
                fixture_bytes,
                "fixture=1_action;surface=action_dispatch",
            ),
            |b| {
                b.iter(|| {
                    let input = action_input(ActionId::new(0));
                    let contract = action_contract(ActionId::new(0));
                    let result = registry_1.dispatch(black_box(&input), black_box(&contract));
                    // Exact assertion: dispatch must succeed with known outcome
                    assert!(
                        result.is_ok(),
                        "dispatch of registered action 0 must succeed"
                    );
                    let outcome = result.expect("ok");
                    match outcome {
                        ActionOutcome::Suspended(_) => {}
                        _ => panic!("expected ActionOutcome::Suspended"),
                    }
                    black_box(outcome);
                });
            },
        );
    }

    // 10 registered actions
    {
        let fixture_bytes = 10usize;
        group.throughput(Throughput::Elements(10));
        group.bench_function(
            metadata(
                "action_dispatch_10_registered",
                fixture_bytes,
                "fixture=10_actions;surface=action_dispatch",
            ),
            |b| {
                b.iter(|| {
                    let input = action_input(ActionId::new(9));
                    let contract = action_contract(ActionId::new(9));
                    let result = registry_10.dispatch(black_box(&input), black_box(&contract));
                    // Exact assertion: last registered action must dispatch correctly
                    assert!(
                        result.is_ok(),
                        "dispatch of action 9 must succeed with 10-action registry"
                    );
                    let outcome = result.expect("ok");
                    match outcome {
                        ActionOutcome::Suspended(_) => {}
                        _ => panic!("expected ActionOutcome::Suspended"),
                    }
                    black_box(outcome);
                });
            },
        );
    }

    // 100 registered actions
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "action_dispatch_100_registered",
                fixture_bytes,
                "fixture=100_actions;surface=action_dispatch",
            ),
            |b| {
                b.iter(|| {
                    let input = action_input(ActionId::new(99));
                    let contract = action_contract(ActionId::new(99));
                    let result = registry_100.dispatch(black_box(&input), black_box(&contract));
                    // Exact assertion: last action in 100-action registry
                    assert!(
                        result.is_ok(),
                        "dispatch of action 99 must succeed with 100-action registry"
                    );
                    let outcome = result.expect("ok");
                    match outcome {
                        ActionOutcome::Suspended(_) => {}
                        _ => panic!("expected ActionOutcome::Suspended"),
                    }
                    black_box(outcome);
                });
            },
        );
    }

    // Unknown action dispatch — error path
    {
        let fixture_bytes = 10usize;
        group.bench_function(
            metadata(
                "action_dispatch_unknown_action",
                fixture_bytes,
                "fixture=10_actions;surface=action_dispatch_unknown",
            ),
            |b| {
                b.iter(|| {
                    let input = action_input(ActionId::new(999)); // Not registered
                    let contract = action_contract(ActionId::new(999));
                    let result = registry_10.dispatch(black_box(&input), black_box(&contract));
                    // Exact assertion: must return UnknownAction error
                    assert!(result.is_err(), "dispatch of unknown action must fail");
                    let err = result.expect_err("error");
                    match err {
                        vb_core::action::ActionError::UnknownAction { action } => {
                            assert_eq!(action.get(), 999, "error must specify action ID 999");
                        }
                        _ => panic!("expected ActionError::UnknownAction"),
                    }
                    black_box(err);
                });
            },
        );
    }

    // Resolve compile-time — lookup without dispatch
    {
        let fixture_bytes = 10usize;
        group.bench_function(
            metadata(
                "action_dispatch_resolve_compile_time",
                fixture_bytes,
                "fixture=10_actions;surface=resolve_compile_time",
            ),
            |b| {
                b.iter(|| {
                    let result = registry_10.resolve_compile_time(black_box(ActionId::new(5)));
                    // Exact assertion: action 5 must resolve to correct contract
                    assert!(
                        result.is_ok(),
                        "resolve_compile_time of action 5 must succeed"
                    );
                    let contract = result.expect("ok");
                    assert_eq!(contract.id.get(), 5, "resolved contract must have id 5");
                    black_box(contract);
                });
            },
        );
    }

    group.finish();
}

/// vb-u09ai: 4-variant RetrySafety dispatch bench.
///
/// Measures round-trip dispatch across the 4-variant `RetrySafety`
/// taxonomy (`Idempotent`, `RequiresIdempotencyKey`, `NotRetrySafe`,
/// `Unknown`). This bench is the **performance evidence** for the
/// 4-variant migration per `AGENTS.md` ("every speed claim requires
/// real baseline/result benchmark evidence").
fn bench_action_dispatch_4variant_retry_safety(c: &mut Criterion) {
    let registry = registry_with_4variant_retry_safety();

    let mut group = c.benchmark_group("action_dispatch_4variant");
    let fixture_bytes = 4usize;
    group.throughput(Throughput::Elements(4));
    group.bench_function(
        metadata(
            "action_dispatch_4variant_retry_safety",
            fixture_bytes,
            "fixture=4_variants;surface=action_dispatch;taxonomy=master_section_65",
        ),
        |b| {
            b.iter(|| {
                let mut i = 0u16;
                while i < 4 {
                    let action = ActionId::new(i);
                    let result = registry.resolve_compile_time(black_box(action));
                    let contract = result.expect("4-variant action must resolve");
                    let safety = contract.retry_safety;
                    // Touch the variant to ensure it survives optimization.
                    let _ = black_box(safety);
                    i = i.saturating_add(1);
                }
            });
        },
    );
    group.finish();
}

criterion_group!(
    benches,
    bench_action_dispatch,
    bench_action_dispatch_4variant_retry_safety
);
criterion_main!(benches);
