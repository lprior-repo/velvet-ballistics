//! Budget Arithmetic Tests
//!
//! Tests for pure arithmetic functions: `budget_utilization_percent`,
//! `latency_within_budget`, `result_exceeds_threshold`, `baseline_within_budget`.
//!
//! # RED PHASE
//! These tests COMPILE but FAIL because the implementation contains intentional bugs.

use std::time::Duration;
use xtask::benchmark_metadata::{
    baseline_within_budget, budget_utilization_percent, latency_within_budget,
    result_exceeds_threshold,
};

// ============================================================================
// budget_utilization_percent Tests
// ============================================================================

#[test]
fn budget_utilization_100_percent() {
    // When elapsed == budget, utilization should be 10000 (100% in basis points)
    let utilization = budget_utilization_percent(Duration::from_micros(100000), 100000);
    assert_eq!(utilization, 10000);
}

#[test]
fn budget_utilization_50_percent() {
    let utilization = budget_utilization_percent(Duration::from_micros(50000), 100000);
    assert_eq!(utilization, 5000);
}

#[test]
fn budget_utilization_0_percent() {
    let utilization = budget_utilization_percent(Duration::from_micros(0), 100000);
    assert_eq!(utilization, 0);
}

#[test]
fn budget_utilization_over_100_percent() {
    // When elapsed > budget, utilization should exceed 10000
    let utilization = budget_utilization_percent(Duration::from_micros(150000), 100000);
    assert!(utilization > 10000);
    assert_eq!(utilization, 15000); // 150000/100000 * 10000 = 15000
}

#[test]
fn budget_utilization_zero_budget_returns_max() {
    let utilization = budget_utilization_percent(Duration::from_micros(50000), 0);
    assert_eq!(utilization, u128::MAX);
}

#[test]
fn budget_utilization_small_values() {
    // Test with small values to ensure no overflow
    let utilization = budget_utilization_percent(Duration::from_micros(1), 1000);
    assert_eq!(utilization, 10); // 1/1000 * 10000 = 10
}

// ============================================================================
// latency_within_budget Tests
// ============================================================================

#[test]
fn latency_within_budget_exactly_at_budget() {
    // At exactly budget boundary, should return true
    assert!(latency_within_budget(Duration::from_micros(100000), 100000));
}

#[test]
fn latency_within_budget_under_budget() {
    assert!(latency_within_budget(Duration::from_micros(50000), 100000));
}

#[test]
fn latency_within_budget_over_budget() {
    assert!(!latency_within_budget(
        Duration::from_micros(100001),
        100000
    ));
}

#[test]
fn latency_within_budget_zero_elapsed() {
    assert!(latency_within_budget(Duration::from_micros(0), 100000));
}

#[test]
fn latency_within_budget_zero_budget_always_false() {
    assert!(!latency_within_budget(Duration::from_micros(0), 0));
    assert!(!latency_within_budget(Duration::from_micros(1), 0));
    assert!(!latency_within_budget(Duration::from_micros(100000), 0));
}

// ============================================================================
// result_exceeds_threshold Tests
// ============================================================================

#[test]
fn result_exceeds_threshold_exactly_at_baseline() {
    // result == baseline should NOT exceed threshold
    assert!(!result_exceeds_threshold(
        Duration::from_micros(100000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn result_exceeds_threshold_just_under_threshold() {
    // result = baseline + threshold - 1 should NOT exceed
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta - 1);

    assert!(!result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_exactly_at_threshold() {
    // result = baseline + threshold should NOT exceed (boundary case)
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta);

    assert!(!result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_just_over_threshold() {
    // result = baseline + threshold + 1 should exceed
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta + 1);

    assert!(result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_double_the_baseline() {
    // result = 2 * baseline should definitely exceed any reasonable threshold
    assert!(result_exceeds_threshold(
        Duration::from_micros(200000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn result_exceeds_threshold_zero_threshold() {
    // With 0% threshold, any increase should be detected
    assert!(result_exceeds_threshold(
        Duration::from_micros(100001),
        Duration::from_micros(100000),
        0
    ));
}

#[test]
fn result_exceeds_threshold_result_less_than_baseline() {
    // result < baseline should never exceed threshold
    assert!(!result_exceeds_threshold(
        Duration::from_micros(90000),
        Duration::from_micros(100000),
        20
    ));
}

// ============================================================================
// baseline_within_budget Tests
// ============================================================================

#[test]
fn baseline_within_budget_exactly_at_budget() {
    assert!(baseline_within_budget(
        Duration::from_micros(100000),
        100000
    ));
}

#[test]
fn baseline_within_budget_under_budget() {
    assert!(baseline_within_budget(Duration::from_micros(50000), 100000));
}

#[test]
fn baseline_within_budget_over_budget() {
    assert!(!baseline_within_budget(
        Duration::from_micros(100001),
        100000
    ));
}

#[test]
fn baseline_within_budget_zero_baseline() {
    assert!(baseline_within_budget(Duration::from_micros(0), 100000));
}

#[test]
fn baseline_within_budget_zero_budget() {
    // With zero budget, nothing should be within budget (not even zero baseline)
    // Actually, zero baseline IS within zero budget (0 <= 0)
    assert!(baseline_within_budget(Duration::from_micros(0), 0));
    assert!(!baseline_within_budget(Duration::from_micros(1), 0));
}

// ============================================================================
// Consistency Tests
// ============================================================================

#[test]
fn budget_utilization_and_latency_consistency() {
    // If latency_within_budget returns true, budget_utilization should be <= 10000
    let elapsed = Duration::from_micros(50000);
    let budget_us = 100000u64;

    if latency_within_budget(elapsed, budget_us) {
        let utilization = budget_utilization_percent(elapsed, budget_us);
        assert!(
            utilization <= 10000,
            "utilization {} should be <= 10000 when within budget",
            utilization
        );
    }
}

#[test]
fn baseline_and_latency_consistency() {
    // baseline_within_budget and latency_within_budget should be consistent
    // for the same duration values
    let baseline = Duration::from_micros(80000);
    let budget_us = 100000u64;

    let baseline_within = baseline_within_budget(baseline, budget_us);
    let latency_within = latency_within_budget(baseline, budget_us);

    assert_eq!(
        baseline_within, latency_within,
        "baseline_within_budget and latency_within_budget should return same value for same inputs"
    );
}
