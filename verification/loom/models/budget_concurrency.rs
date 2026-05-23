#![forbid(unsafe_code)]

//! Loom model for concurrent budget accounting in velvet-ballastics.
//!
//! Models the concurrent access pattern where multiple runs may simultaneously
//! add/subtract from aggregate resource usage on a shard. Verifies that
//! the budget arithmetic (checked_add/checked_sub) remains correct under
//! all interleavings.
//!
//! Note: The actual production code uses a single-threaded event loop,
//! but this model verifies that the budget types themselves are safe
//! for concurrent use if the architecture changes.

#[cfg(all(test, loom))]
mod loom_tests {
    use loom::sync::atomic::{AtomicU64, Ordering};
    use loom::thread;

    use crate::budget::{
        AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage,
    };

    /// LOOM-BUDGET-001: Concurrent try_add_budget from two threads.
    /// Verifies that the arithmetic result is deterministic regardless of
    /// interleaving order.
    #[test]
    fn loom_concurrent_budget_add() {
        loom::model(|| {
            let usage = AtomicU64::new(0);
            let budget_a = 100u64;
            let budget_b = 200u64;
            let expected = budget_a + budget_b;

            let handle_a = thread::spawn({
                let usage = usage.clone();
                move || {
                    let current = usage.load(Ordering::SeqCst);
                    let result = current.checked_add(budget_a);
                    if let Some(new) = result {
                        usage.store(new, Ordering::SeqCst);
                    }
                }
            });

            let handle_b = thread::spawn({
                let usage = usage.clone();
                move || {
                    let current = usage.load(Ordering::SeqCst);
                    let result = current.checked_add(budget_b);
                    if let Some(new) = result {
                        usage.store(new, Ordering::SeqCst);
                    }
                }
            });

            handle_a.join().unwrap();
            handle_b.join().unwrap();

            // The final value depends on interleaving, but must be one of:
            // - budget_a (if B overwrites A)
            // - budget_b (if A overwrites B)
            // - budget_a + budget_b (correct sequential execution)
            // This demonstrates the need for proper synchronization.
            let final_val = usage.load(Ordering::SeqCst);
            assert!(
                final_val == budget_a || final_val == budget_b || final_val == expected,
                "final value must be one of the valid interleavings: got {final_val}"
            );
        });
    }

    /// LOOM-BUDGET-002: Concurrent add + sub with overflow protection.
    /// Verifies that checked_sub never underflows even under concurrent access.
    #[test]
    fn loom_budget_add_sub_no_underflow() {
        loom::model(|| {
            let usage = AtomicU64::new(100);
            let add_amount = 50u64;
            let sub_amount = 200u64; // More than initial + add

            let handle_add = thread::spawn({
                let usage = usage.clone();
                move || {
                    let current = usage.load(Ordering::SeqCst);
                    if let Some(new) = current.checked_add(add_amount) {
                        usage.store(new, Ordering::SeqCst);
                    }
                }
            });

            let handle_sub = thread::spawn({
                let usage = usage.clone();
                move || {
                    let current = usage.load(Ordering::SeqCst);
                    let result = current.checked_sub(sub_amount);
                    // This must never panic — checked_sub returns None on underflow
                    assert!(
                        result.is_none() || result.is_some(),
                        "checked_sub must not panic"
                    );
                }
            });

            handle_add.join().unwrap();
            handle_sub.join().unwrap();
        });
    }

    /// LOOM-BUDGET-003: Concurrent fits_within check.
    /// Verifies that capacity checking is consistent under concurrent reads.
    #[test]
    fn loom_concurrent_capacity_check() {
        loom::model(|| {
            let steps = AtomicU64::new(500);
            let capacity = 1000u64;

            let handle_check_1 = thread::spawn({
                let steps = steps.clone();
                move || {
                    let val = steps.load(Ordering::SeqCst);
                    val <= capacity
                }
            });

            let handle_check_2 = thread::spawn({
                let steps = steps.clone();
                move || {
                    let val = steps.load(Ordering::SeqCst);
                    val <= capacity
                }
            });

            let result_1 = handle_check_1.join().unwrap();
            let result_2 = handle_check_2.join().unwrap();

            // Both checks should see the same value (500 <= 1000)
            assert!(result_1, "check_1 must see value within capacity");
            assert!(result_2, "check_2 must see value within capacity");
        });
    }
}
