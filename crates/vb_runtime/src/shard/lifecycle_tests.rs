// Lifecycle module tests
//
// DO NOT EDIT - Generated from lifecycle_tests/ chunks
//
// RS-007: production code must keep Holzman safety lints enforced. This
// test module is the only place where the otherwise-deny lints are
// scoped off; the allow list below matches patterns that legitimate
// tests use (Result/Option unwrapping, panic-on-failure assertions,
// integer casts between same-size types, indexed/slice lookups into
// fixed test fixtures, and small offset arithmetic for step indices).
// Production code paths in `lifecycle.rs` and its `include!`d chunks
// must NOT inherit these allows.

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::indexing_slicing,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::let_underscore_must_use,
        clippy::needless_pass_by_value,
        clippy::needless_collect,
        clippy::single_match,
        clippy::needless_bool,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::redundant_closure,
        clippy::redundant_clone,
        clippy::useless_format,
        clippy::uninlined_format_args,
        clippy::unnecessary_wraps,
        clippy::useless_vec,
        clippy::or_fun_call,
        clippy::option_if_let_else,
        clippy::if_let_mutex,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_strip,
        clippy::unnecessary_map_or,
        clippy::get_first,
        clippy::iter_count,
        clippy::needless_range_loop,
        clippy::explicit_counter_loop,
        clippy::similar_names,
        clippy::shadow_unrelated,
        clippy::items_after_test_module,
        dead_code,
        unused_imports,
        unused_variables
    )]

    include!("lifecycle_tests/chunk_001.rs");
    include!("lifecycle_tests/chunk_002.rs");
    include!("lifecycle_tests/chunk_003.rs");
    include!("lifecycle_tests/chunk_004.rs");
    include!("lifecycle_tests/chunk_005.rs");
    include!("lifecycle_tests/chunk_006.rs");
    include!("lifecycle_tests/chunk_007.rs");
    include!("lifecycle_tests/chunk_008.rs");
}
