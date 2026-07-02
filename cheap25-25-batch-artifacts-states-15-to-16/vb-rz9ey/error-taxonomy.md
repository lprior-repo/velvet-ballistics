# Error Taxonomy — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 3 (rust-contract)

This bead is Cargo-manifest metadata-only. There is no new production-code error path introduced. The taxonomy enumerates the rustc diagnostic codes that gate this bead's success, plus the domain error variants of the touched library surface (`vb_compile`) at the test-only layer.

## E-1 Domain Errors Already in Scope (Unchanged)

The semantic errors of `vb_compile` parsing/compilation are preserved exactly. This bead does not add or remove any of them. Listed here for completeness:

| Code | Source | Layer | Bead relevance |
|------|--------|-------|----------------|
| `CompileError` | `vb_compile::errors::CompileError` (`src/lib.rs:220`) | production | unaffected |
| `CompileErrors` | `vb_compile::errors::CompileErrors` | production | unaffected |
| `ValidationError` | re-exported from `vb_validate` (`src/lib.rs:235`) | production | unaffected |
| `YamlError` | re-exported from `yaml_error` (`src/lib.rs:238`) | production | unaffected |
| `WorkflowSourceParts` field accessors | `crates/vb_compile/src/yaml_ast/types/workflow.rs:60-102` (`version`, `name`, `trigger`, `inputs`, `vars`, `secrets`, `steps`, `result`, `examples`) | production | unaffected |

## E-2 rustc Diagnostics That Define This Bead's Success/Failure

These are not domain errors — they are the *external* diagnostics that gate whether the visibility invariant is satisfied.

### E-2.a `E0432` — Unresolved Import

**Description**: An integration test wrote `use vb_compile::WorkflowSourceParts;` (or similar) but the symbol is not reachable from the crate's public surface.

**Why it occurs in this bead**: `WorkflowSourceParts` is gated to `pub(crate)` in non-test, non-`test-util` builds. Integration tests are *external* test binaries; they see only the `pub` surface. Without the `test-util` feature activated, this import fails.

**Baseline count** (before fix): **12 occurrences** across 9 integration-test files (`codebase-map.md` Files Mapped → Tests section).

**After-fix required state**: 0 occurrences.

**Touched sites** (one per file path; some files have multiple unresolved imports):

| File | Lines |
|------|-------|
| `crates/vb_compile/tests/common/mod.rs` | 12 |
| `crates/vb_compile/tests/digest_structural_fields.rs` | 233, 297, 359, 438 |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` | 29 |
| `crates/vb_compile/tests/digest_set_finish_regression.rs` | 185 |
| `crates/vb_compile/tests/digest_ask_explicit_arm.rs` | 194 |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | 18 |
| `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | 18 |
| `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | 18 |
| `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | 18 |

### E-2.b `E0624` — Private Associated Function (Constructor)

**Description**: An integration test called `WorkflowSource::new(parts)` but the constructor is `pub(crate)` in production, invisible to external test binaries.

**Why it occurs in this bead**: Same gating root cause as `E0432`. The constructor has two cfg arms (`workflow.rs:33`, `:41`); without `test-util` activated, only the `pub(crate)` arm compiles.

**Baseline count** (before fix): **26 occurrences** across the same 9 files.

**After-fix required state**: 0 occurrences.

### E-2.c `help` Note from rustc (Informational, Not a Separate Error Code)

The compiler's help text pinpoints the gate at `crates/vb_compile/src/lib.rs:241`:

```
241 | #[cfg(any(test, feature = "test-util"))]
    |          ----------------------------- the item is gated here
242 | pub use yaml_ast::types::WorkflowSourceParts;
```

This is **not** an error; it is the diagnostic fingerprint that confirms the root cause. It is preserved in the after-fix state because the cfg gate itself is preserved.

## E-3 Domain Errors NOT in Scope

| Layer | Reason out-of-scope |
|-------|---------------------|
| AST semantic-validity errors | Owned by `vb_validate` (separate crate, separate bead history). |
| Compilation IR errors | Owned by `vb_compile::errors` module (`mod_compile_errors`); unaffected. |
| Runtime step errors | Out of scope of `vb_compile` entirely; owned by `vb_runtime`. |
| Storage / persistence errors | Out of scope; owned by `vb_storage`. |

## E-4 Forbidden Error Patterns

| Pattern | Why forbidden |
|---------|---------------|
| Wrapping `E0432`/`E0624` into a runtime error type | These are *compile-time* errors; propagating them past `cargo build` is a layering violation. |
| Silencing `dead_code` or `unused_import` warnings for `WorkflowSourceParts` in test build | The `#[cfg(test, ...)]` arms are intentional. Suppressing warnings on `pub(crate)` arms would mask accidental API drift. |
| Adding `#[allow(...)]` to `workflow.rs:32-43` or `:107-149` | These cfg arms must remain visible to lints; no allow attributes may be added. |

## E-5 Success Metric (Single Number)

The bead's primary success metric is the reduction of `E0432 + E0624` errors from **38 → 0** under `cargo build -p vb_compile --tests --message-format=human`.

Secondary metrics:

- `cargo build -p vb_cli --message-format=human` exits 0 (no new errors introduced).
- `cargo build -p workspace_tests --message-format=human` exits 0.
- `cargo doc -p vb_compile --no-deps 2>&1 | grep -c WorkflowSourceParts` returns **0** in production build (i.e. it remains `#[doc(hidden)]`).
- `Cargo.lock` diff = exactly **+1 line** (the self-reference).
