# Type Contracts — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 3 (rust-contract)
- companion: `domain-model.md`, `contract.md`

This bead is a Cargo-manifest metadata change. The "type contract" is the Rust visibility/configuration contract that is already encoded in source at `crates/vb_compile/src/yaml_ast/types/workflow.rs` plus a single Cargo TOML invariant. No new types are introduced; no existing types are renamed or reshaped.

## TC-1 `WorkflowSourceParts` Visibility Contract

**Symbol**: `vb_compile::WorkflowSourceParts`
**Source of truth**: `crates/vb_compile/src/yaml_ast/types/workflow.rs:107` (production) and `:129` (test-util)

| Build context | Required visibility | Required accessor visibility |
|---------------|---------------------|------------------------------|
| `#[cfg(not(any(test, feature = "test-util")))]` | `pub(crate) struct` with `pub(crate)` fields | n/a (constructor also `pub(crate)`) |
| `#[cfg(any(test, feature = "test-util"))]` | `pub struct` with `pub` fields | n/a |

**Invariants**:

- TC-1.a — Exactly two `WorkflowSourceParts` struct declarations exist, gated by mutually exclusive `cfg` arms. Both MUST declare the SAME nine fields in the SAME order with the SAME types.
- TC-1.b — The `pub`/`pub(crate)` toggle is the ONLY difference between the two declarations.
- TC-1.c — `WorkflowSource::new(parts: WorkflowSourceParts) -> Self` has two cfg-gated arms (`:33` and `:41`), each delegating to the same private `fn from_parts`. Both MUST be field-by-field copies of the parameter bag. No logic divergence.
- TC-1.d — A test build (`cfg(test)` or `feature = "test-util"`) MUST be the only context in which external `pub` visibility is granted.

**Enforcement lane**: `cargo build -p vb_compile --tests` must compile (positive); `cargo build -p vb_cli` and `cargo build -p workspace_tests` must compile (negative — they must NOT activate `test-util`).

## TC-2 Cargo `test-util` Feature Contract

**Source of truth**: `crates/vb_compile/Cargo.toml:21-23`

```
[features]
default = []
test-util = []
```

**Invariants**:

- TC-2.a — `default` MUST NOT list `test-util`. Production consumers (i.e. the `cargo build` of any crate that depends on `vb_compile` without features) MUST NOT activate this feature.
- TC-2.b — `test-util` MUST be declared as an empty-feature flag (no implicit dependencies). Activation must be opt-in.
- TC-2.c — `test-util` activation is intentionally surfaced only inside `vb_compile`'s own test build. No other crate in the workspace may activate `vb_compile`'s `test-util` feature in its `[dependencies]`.

## TC-3 Self-Referencing Dev-Dependency Contract

**Source of truth**: `crates/vb_compile/Cargo.toml:[dev-dependencies]`

**Required entry (after the fix)**:

```toml
[dev-dependencies]
proptest.workspace = true
vb_compile = { path = ".", features = ["test-util"] }   # NEW
```

**Invariants**:

- TC-3.a — The self-referencing entry MUST live in `[dev-dependencies]`, NOT `[dependencies]`. Placement in `[dependencies]` would activate `test-util` for production builds of `vb_compile` itself.
- TC-3.b — `path = "."` is required (per `specifying-dependencies.html#self-references`); relative paths of the form `"."`, `"./"`, and the directory path are equivalent in modern Cargo, but `"."` is the canonical form.
- TC-3.c — `features = ["test-util"]` MUST activate exactly one feature — the existing `test-util`. Adding any other feature would alter the public-API surface in unintended ways.
- TC-3.d — There MUST NOT be any duplicated self-reference (one is sufficient). Duplication would silently bloat the build graph.

## TC-4 Downstream API Surface Preservation Contract

| Consumer | `vb_compile` dep declaration | Expected visibility of `WorkflowSourceParts` in consumer's build |
|----------|------------------------------|-------------------------------------------------------------------|
| `vb_cli` | `vb_compile = { path = "../vb_compile" }` (no features) | `pub(crate)` — NOT in public API |
| `workspace_tests` | `vb_compile = { path = "../vb_compile" }` (no features) | `pub(crate)` — NOT in public API |
| `vb_compile` (self, production binary only) | n/a | `pub(crate)` |
| `vb_compile` (self, test build) | via TC-3 | `pub` |

**Invariants**:

- TC-4.a — Neither `vb_cli` nor `workspace_tests` invokes `WorkflowSourceParts` directly in their source (verified at `codebase-map.md` "Downstream crates" section). No compile-error regression in either crate is acceptable after the fix.
- TC-4.b — `cargo doc -p vb_compile --no-deps` MUST NOT list `WorkflowSourceParts` among the publicly documented items (since `#[doc(hidden)]` is applied to both cfg-gated arms and the feature that would expose them is off).

## Anti-Patterns (Forbidden)

| Anti-pattern | Why forbidden |
|--------------|---------------|
| `pub struct WorkflowSourceParts { ... }` without a `#[cfg]` | Leaks to production API surface. |
| `default = ["test-util"]` | Activates `test-util` for every consumer. |
| Removing the `#[cfg]` gates from `WorkflowSource::new` | Breaks production visibility, breaks downstream compilation. |
| A second self-referencing entry that activates a different feature | Silent feature creep; not the scope of this bead. |
| Editing `common/mod.rs` or any `tests/*.rs` to avoid the visibility issue | Forbidden — the visibility gate is the *correct* architecture; only the feature activation is missing. |

## Compile-Time vs. Runtime

This bead makes NO runtime contract changes. Type contracts above are enforced entirely by `rustc` (visibility errors `E0432`, `E0624`) and by Cargo (feature resolution + lockfile).
