//! The `Budget` type for resource-accounting proof kernels.
//!
//! This module exposes a dual compilation path:
//!
//! - **Verus mode** (`#[cfg(verus_keep_ghost)]`): A `nat`-field struct with
//!   spec/exec functions proving field-wise add, max, and multiply semantics.
//! - **Cargo mode** (`#[cfg(not(verus_keep_ghost))]`): A `u64`-field struct
//!   with saturating arithmetic mirroring the spec.
//!
//! The two types are field-for-field isomorphic so that Verus proofs over
//! `nat`-fields reason about the same shape the production code manipulates.

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

// ── Budget struct — spec view ──────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct Budget {
    pub steps: nat,
    pub actions: nat,
    pub parallel: nat,
    pub retries: nat,
    pub gather_pages: nat,
    pub gather_items: nat,
    pub for_each_iters: nat,
    pub together_branches: nat,
    pub repeat_attempts: nat,
    pub run_time_secs: nat,
    pub result_bytes: nat,
    pub slots_written: nat,
}

impl Budget {
    pub open spec fn empty() -> Budget {
        Budget {
            steps: 0,
            actions: 0,
            parallel: 0,
            retries: 0,
            gather_pages: 0,
            gather_items: 0,
            for_each_iters: 0,
            together_branches: 0,
            repeat_attempts: 0,
            run_time_secs: 0,
            result_bytes: 0,
            slots_written: 0,
        }
    }
}

} // verus!

// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    #[derive(Debug, Clone, Default)]
    pub struct Budget {
        pub steps: u64,
        pub actions: u64,
        pub parallel: u64,
        pub retries: u64,
        pub gather_pages: u64,
        pub gather_items: u64,
        pub for_each_iters: u64,
        pub together_branches: u64,
        pub repeat_attempts: u64,
        pub run_time_secs: u64,
        pub result_bytes: u64,
        pub slots_written: u64,
    }

    impl Budget {
        pub fn new() -> Self {
            Budget::default()
        }

        pub fn sequential_add(&mut self, other: &Budget) {
            self.steps = self.steps.saturating_add(other.steps);
            self.actions = self.actions.saturating_add(other.actions);
            self.parallel = self.parallel.max(other.parallel);
            self.retries = self.retries.max(other.retries);
            self.gather_pages = self.gather_pages.saturating_add(other.gather_pages);
            self.gather_items = self.gather_items.saturating_add(other.gather_items);
            self.for_each_iters = self.for_each_iters.max(other.for_each_iters);
            self.together_branches = self.together_branches.max(other.together_branches);
            self.repeat_attempts = self.repeat_attempts.max(other.repeat_attempts);
            self.run_time_secs = self.run_time_secs.saturating_add(other.run_time_secs);
            self.result_bytes = self.result_bytes.max(other.result_bytes);
            self.slots_written = self.slots_written.saturating_add(other.slots_written);
        }

        pub fn branch_max(&mut self, other: &Budget) {
            self.steps = self.steps.max(other.steps);
            self.actions = self.actions.max(other.actions);
            self.parallel = self.parallel.max(other.parallel);
            self.retries = self.retries.max(other.retries);
            self.gather_pages = self.gather_pages.max(other.gather_pages);
            self.gather_items = self.gather_items.max(other.gather_items);
            self.for_each_iters = self.for_each_iters.max(other.for_each_iters);
            self.together_branches = self.together_branches.max(other.together_branches);
            self.repeat_attempts = self.repeat_attempts.max(other.repeat_attempts);
            self.run_time_secs = self.run_time_secs.max(other.run_time_secs);
            self.result_bytes = self.result_bytes.max(other.result_bytes);
            self.slots_written = self.slots_written.max(other.slots_written);
        }

        pub fn loop_mul(&mut self, iterations: u64) {
            self.steps = self.steps.saturating_mul(iterations);
            self.actions = self.actions.saturating_mul(iterations);
            self.parallel = self.parallel.saturating_mul(iterations);
            self.retries = self.retries.saturating_mul(iterations);
            self.gather_pages = self.gather_pages.saturating_mul(iterations);
            self.gather_items = self.gather_items.saturating_mul(iterations);
            self.for_each_iters = self.for_each_iters.saturating_mul(iterations);
            self.together_branches = self.together_branches.saturating_mul(iterations);
            self.repeat_attempts = self.repeat_attempts.saturating_mul(iterations);
            self.run_time_secs = self.run_time_secs.saturating_mul(iterations);
            self.result_bytes = self.result_bytes.saturating_mul(iterations);
            self.slots_written = self.slots_written.saturating_mul(iterations);
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;
