//! Fuzz target: collect_page_pagination
//!
//! Specifically targets `collect_page` pagination behavior with:
//! - Arbitrary list source values
//! - Page size boundaries (0, 1, max, overflow)
//! - Cursor positions across page boundaries
//! - Non-list collector types (must error)
//! - Empty list (edge case)
//!
//! Verifies:
//! - `collect_page` returns Result, never panics
//! - Output page count is consistent with list length and page size
//! - Each page's item count ≤ page_size
//! - Non-list inputs return typed error variants
//!
//! Obligation: C.25 (vb-qi37.2.5)
//! Command: cargo fuzz run collect_page_pagination -- -runs=50000

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_collect_page_pagination)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
