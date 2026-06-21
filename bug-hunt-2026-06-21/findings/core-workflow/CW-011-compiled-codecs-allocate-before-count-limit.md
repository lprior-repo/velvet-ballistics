# CW-011: Compiled slug/query codecs allocate decoded arrays before enforcing count limits

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_core/src/workflow/compiled_slug/codec.rs:26-31`, `crates/vb_core/src/workflow/compiled_query/mod.rs:54-59`
- **Confidence**: confirmed

## Description

The compiled slug and query byte decoders deserialize into boxed collections before validating `MAX_*_PER_WORKFLOW`. Oversized payloads can force postcard to allocate and populate large arrays before the post-decode validator rejects the count.

## Evidence

```rust
// compiled_slug/codec.rs:26
pub fn from_bytes_compiled_slugs(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedSlugs, SlugParseError> {
    let compiled: CompiledSlugs = postcard::from_bytes(bytes).map_err(SlugParseError::Decode)?;
    validate_compiled_slugs(compiled, max_yield_budget)
}
```

```rust
// compiled_query/mod.rs:54
pub fn from_bytes_compiled_queries(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    let compiled: CompiledQueries = postcard::from_bytes(bytes).map_err(QueryParseError::Decode)?;
    validate_compiled_queries(compiled, max_yield_budget)
}
```

The count checks are post-decode:

```rust
// compiled_slug/validation.rs:17
pub fn validate_compiled_slug_count(count: usize) -> Result<(), SlugParseError> {
    if count > MAX_SLUGS_PER_WORKFLOW { ... }
```

```rust
// compiled_query/validation.rs:40
pub fn validate_compiled_query_count(count: usize) -> Result<(), QueryParseError> {
    if count > MAX_QUERIES_PER_WORKFLOW { ... }
```

## Adversarial Check

The validators are correct after a `CompiledSlugs` or `CompiledQueries` value exists, but they do not protect the decode boundary itself. `max_yield_budget` also cannot bound allocation because it is not consulted until after deserialization. Any upstream byte-size cap would be an external assumption; these public decode functions do not enforce one.

## Suggested Fix

Use a bounded deserialization path that checks collection length before allocation, or add a conservative byte-length preflight derived from `MAX_*_PER_WORKFLOW` and the minimum postcard encoding size. If postcard cannot enforce a sequence-length cap directly, decode through a custom visitor/container that rejects lengths above the hard limit before allocating.
