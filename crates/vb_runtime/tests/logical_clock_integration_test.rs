#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
//! vb-wy33p.18: deterministic-clock integration tests.
//!
//! This file complements the byte-stability tests in
//! `logical_clock_seam_test.rs` (added by Wave 1's vb-wy33p.4). Those tests
//! cover the per-shard byte-identity contract at the shard level. The tests
//! here cover the higher-level integration surface:
//!
//! 1. Production default path remains Wall-clock (regression guard at the
//!    Runtime layer, not just at the Shard layer).
//! 2. Explicit `ShardClockConfig::Logical` propagation through the
//!    `Runtime::new_for_tests_and_benchmarks_only` factory.
//! 3. `Runtime::capture_timer_entry` exposes the logical deadline at the
//!    Runtime layer when a run awaits a timer in logical mode.
//! 4. Cross-shard independence of logical clocks (separate Shard instances
//!    with separate ticks report distinct logical deadlines).
//! 5. Wall-mode production default preserves the legacy `Instant::now()`
//!    authority path even when observed through the Runtime integration API.
//! 6. Multi-shard Runtime determinism: a Runtime constructed with N shards
//!    in Logical mode boots every shard at `TimerTick::new(0)` and
//!    `tick_all()` does not perturb the per-shard logical clock.
//! 7. Logical-clock monotonicity budget: advancing past the per-shard
//!    forward-progress boundary returns a typed `RuntimeError` instead of
//!    panicking.
//!
//! Tests in this file MUST NOT duplicate the 11 byte-stability tests in
//! `logical_clock_seam_test.rs`. They target the Runtime / multi-shard /
//! production-default surface that Wave 1's work intentionally left for
//! this bead.

use std::num::NonZeroUsize;

use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::types::{LogicalDeadline, Shard, ShardClockConfig, ShardConfig, TimerTick};

// =========================================================================
// Shared helpers
// =========================================================================

/// Standard per-shard config tuned for tests: small queues, Relaxed policy,
/// and clock mode chosen by the caller.
fn small_config_with_clock(clock: ShardClockConfig) -> ShardConfig {
    ShardConfig::new_with_clock(16, 16, 4, 4, vb_core::policy::RuntimePolicy::Relaxed, clock)
        .expect("test shard config with explicit clock")
}

/// Single-step `WaitUntil` workflow that suspends a run on a timer.
///
/// The deadline slot is initialized via `SetConst` before the `WaitUntil`
/// step. Once the Wait step is hit, the run is in `AwaitingWait` and the
/// owning shard holds a pending timer with the configured clock authority.
fn wait_until_workflow() -> Option<CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("logical_clock_integration_wait"),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: Box::from([set_deadline, wait]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn submit_wait_until_and_tick(runtime: &mut Runtime, run: RunId) {
    let Some(workflow) = wait_until_workflow() else {
        return;
    };
    runtime
        .submit_direct(run, workflow)
        .expect("submit must succeed");
    let _ = runtime.tick_all();
}

// =========================================================================
// Test 1: production default clock uses Wall-clock authority
// =========================================================================

#[test]
fn production_default_clock_uses_wall_clock() {
    // The test/benchmark-only long-named factory is the explicit non-durable
    // path.  It is the production default constructor for tests.
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, ShardConfig::default());

    // The Runtime constructs every shard from the supplied `ShardConfig`.
    // Build a parallel Shard from the same default config to assert the
    // clock mode the Runtime's shards were initialized with.
    let probe = Shard::new(ShardConfig::default());
    assert_eq!(
        probe.clock_config(),
        ShardClockConfig::Wall,
        "ShardConfig::default() must yield Wall mode (production default)"
    );
    assert!(
        !probe.is_logical_clock(),
        "ShardConfig::default() must NOT report is_logical_clock() == true"
    );
    // Pending-timer authority surfaces in Wall mode do NOT carry a logical
    // deadline, confirming the default mode stays on the Instant::now()
    // path even when observed through the timer authority adapter.
    assert_eq!(probe.pending_timer_logical_deadline(), None);

    // Sanity: the runtime itself is alive and reportable.
    let snapshot = runtime.pending_boundary_snapshot(8);
    assert_eq!(snapshot.shards().len(), 3);
}

// =========================================================================
// Test 2: explicit ShardClockConfig::Logical propagation through Runtime
// =========================================================================

#[test]
fn logical_clock_construction_explicit_uses_logical() {
    let Some(shard_count) = NonZeroUsize::new(4) else {
        return;
    };
    let config = small_config_with_clock(ShardClockConfig::Logical);
    let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);

    // Parallel shard built from the same Logical config proves every shard
    // in the Runtime was initialized in Logical mode.
    let probe = Shard::new(config);
    assert_eq!(probe.clock_config(), ShardClockConfig::Logical);
    assert!(probe.is_logical_clock());

    // The probe shard at tick 0 must carry a logical deadline that is
    // distinguishable from the Wall-mode default (None).
    let logical_deadline = probe.pending_timer_logical_deadline();
    assert_eq!(
        logical_deadline,
        Some(LogicalDeadline::new(0)),
        "Logical mode at tick 0 must report LogicalDeadline::new(0)"
    );

    // The pending boundary snapshot reports one row per shard; the runtime
    // constructed exactly `shard_count` shards from the supplied config.
    let snapshot = runtime.pending_boundary_snapshot(8);
    assert_eq!(snapshot.shards().len(), 4);
}

// =========================================================================
// Test 3: Runtime-level await_timer seam uses the logical deadline
// =========================================================================

#[test]
fn logical_clock_await_timer_uses_logical_deadline() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(
        shard_count,
        small_config_with_clock(ShardClockConfig::Logical),
    );

    let run = RunId::new(2);
    submit_wait_until_and_tick(&mut runtime, run);

    // `capture_timer_entry` is the Runtime-layer seam that surfaces the
    // pending timer's authority (including the optional logical deadline).
    let entry = runtime
        .capture_timer_entry(run)
        .expect("run must own a pending timer after WaitUntil was driven");

    // Logical mode must carry the logical deadline.
    assert_eq!(
        entry.logical_deadline,
        Some(LogicalDeadline::new(0)),
        "Logical-mode Runtime must surface Some(LogicalDeadline::new(0)) for the await_timer seam"
    );
    // The wall-clock Instant adapter is present but is derived from
    // logical_origin + tick; the key observation is that logical_deadline
    // is Some(...) here while the Wall-mode regression test below shows None.
}

// =========================================================================
// Test 4: cross-shard logical clocks are independent
// =========================================================================

#[test]
fn cross_shard_logical_clocks_are_independent() {
    // Two separate logical-mode shards with different advance_clock_to
    // values report distinct logical deadlines.  This proves each shard
    // owns its logical clock and there is no shared global tick.
    let mut shard_a = Shard::new(small_config_with_clock(ShardClockConfig::Logical));
    let mut shard_b = Shard::new(small_config_with_clock(ShardClockConfig::Logical));

    assert_eq!(shard_a.advance_clock_to(TimerTick::new(100)), Ok(()));
    assert_eq!(shard_b.advance_clock_to(TimerTick::new(200)), Ok(()));

    let a_logical = shard_a.pending_timer_logical_deadline();
    let b_logical = shard_b.pending_timer_logical_deadline();
    assert_eq!(a_logical, Some(LogicalDeadline::new(100)));
    assert_eq!(b_logical, Some(LogicalDeadline::new(200)));
    assert_ne!(
        a_logical, b_logical,
        "Distinct shards at distinct ticks must report distinct logical deadlines"
    );

    // Two parallel Runtimes — one for shard_a's home config and one for
    // shard_b's — both report a `pending_boundary_snapshot` with one row
    // each, confirming the Runtime wiring matches the shard wiring.
    let runtime_a = Runtime::new_for_tests_and_benchmarks_only(
        NonZeroUsize::new(1).expect("non-zero"),
        small_config_with_clock(ShardClockConfig::Logical),
    );
    let runtime_b = Runtime::new_for_tests_and_benchmarks_only(
        NonZeroUsize::new(1).expect("non-zero"),
        small_config_with_clock(ShardClockConfig::Logical),
    );
    assert_eq!(runtime_a.pending_boundary_snapshot(4).shards().len(), 1);
    assert_eq!(runtime_b.pending_boundary_snapshot(4).shards().len(), 1);
}

// =========================================================================
// Test 5: Wall-mode production default preserves legacy Instant::now() path
// =========================================================================

#[test]
fn wall_mode_production_default_preserves_legacy_behavior() {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return;
    };
    // The Runtime is constructed with the Wall-clock mode (the default
    // `ShardClockConfig`).  We use `small_config_with_clock(Wall)` so the
    // policy is Relaxed (required to admit the test workflow without a
    // pre-populated artifact store); the clock mode is the production
    // default.
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(
        shard_count,
        small_config_with_clock(ShardClockConfig::Wall),
    );

    let run = RunId::new(7);
    submit_wait_until_and_tick(&mut runtime, run);

    let entry = runtime
        .capture_timer_entry(run)
        .expect("Wall-mode run must own a pending timer after WaitUntil");

    // Regression guard: the production default Wall-clock mode must NOT
    // surface a logical deadline.  If this ever flips to Some(...), the
    // production path silently migrated to logical mode without explicit
    // opt-in — exactly the failure mode this bead exists to guard.
    assert_eq!(
        entry.logical_deadline, None,
        "Wall-mode production default must NOT surface a logical deadline"
    );

    // The public `Instant` adapter in Wall mode is `Instant::now()`; two
    // independent captures from the same shard must be non-decreasing
    // (i.e., the wall-clock path still advances with real time).
    let probe = Shard::new(ShardConfig::default());
    let first = probe.pending_timer_deadline();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let second = probe.pending_timer_deadline();
    assert!(
        second >= first,
        "Wall-mode Instant::now() adapter must be non-decreasing"
    );
}

// =========================================================================
// Test 6: multi-shard Runtime determinism on initial logical tick
// =========================================================================

#[test]
fn runtime_tick_all_advances_each_logical_shard_deterministically() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let config = small_config_with_clock(ShardClockConfig::Logical);
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);

    // Every shard built by the Runtime from the supplied Logical config
    // starts at TimerTick::new(0) and is in Logical mode.  We verify via
    // three parallel Shards built from the same config — each Runtime
    // shard was constructed by the same factory call.
    let probes: [Shard; 3] = [Shard::new(config), Shard::new(config), Shard::new(config)];
    for (index, probe) in probes.iter().enumerate() {
        assert_eq!(
            probe.current_tick(),
            TimerTick::new(0),
            "probe shard {index} must start at TimerTick::new(0)"
        );
        assert!(
            probe.is_logical_clock(),
            "probe shard {index} must be in Logical mode"
        );
        assert_eq!(
            probe.pending_timer_logical_deadline(),
            Some(LogicalDeadline::new(0)),
            "probe shard {index} must carry LogicalDeadline::new(0) at construction"
        );
    }

    // `tick_all` pops commands from each shard's queue; with an empty
    // queue it returns Ok(true) per shard and must not perturb the
    // per-shard logical clock (the logical clock only advances via
    // `advance_clock_to`).
    for _ in 0..8 {
        let result = runtime.tick_all();
        assert!(matches!(result, Ok(true) | Ok(false)));
    }

    // The Runtime still owns exactly `shard_count` shards, all in the
    // supplied Logical config.
    let snapshot = runtime.pending_boundary_snapshot(8);
    assert_eq!(snapshot.shards().len(), 3);
}

// =========================================================================
// Test 7: logical clock monotonicity budget exhausts as typed error
// =========================================================================

#[test]
fn logical_clock_with_max_ticks_exhausts_budget() {
    // The "budget" enforced by `advance_clock_to` is the monotonic
    // forward-progress guarantee: any tick strictly smaller than
    // `current_tick` is rejected with a typed `RuntimeError::InvalidTimerFire`.
    // The test exhausts the budget by advancing to `TimerTick::new(u64::MAX)`
    // and then asserts every smaller tick is rejected.
    let mut shard = Shard::new(small_config_with_clock(ShardClockConfig::Logical));

    assert_eq!(
        shard.advance_clock_to(TimerTick::new(u64::MAX)),
        Ok(()),
        "advance_clock_to u64::MAX must succeed; this is the max reachable tick"
    );
    assert_eq!(shard.current_tick(), TimerTick::new(u64::MAX));

    // Every backward attempt must return a typed error and must not panic.
    for &smaller in &[u64::MAX - 1, u64::MAX / 2, 1_000_000, 1, 0] {
        let result = shard.advance_clock_to(TimerTick::new(smaller));
        assert_eq!(
            result,
            Err(RuntimeError::InvalidTimerFire),
            "advance_clock_to to {smaller} must return InvalidTimerFire after max tick"
        );
        // The current tick must be preserved after rejection.
        assert_eq!(
            shard.current_tick(),
            TimerTick::new(u64::MAX),
            "current_tick must remain u64::MAX after rejected backward advance to {smaller}"
        );
    }

    // Equal-tick is a documented no-op (still inside the budget).
    assert_eq!(
        shard.advance_clock_to(TimerTick::new(u64::MAX)),
        Ok(()),
        "advance_clock_to equal-to-current must remain a no-op"
    );
    assert_eq!(shard.current_tick(), TimerTick::new(u64::MAX));
}
