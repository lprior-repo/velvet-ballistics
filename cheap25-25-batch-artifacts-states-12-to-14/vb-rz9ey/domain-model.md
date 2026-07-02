# Domain Model — vb-rz9ey

- bead_id: `vb-rz9ey`
- title: Fix `vb_compile` test compilation — `WorkflowSourceParts` private (P0)
- skill_state: 3 (rust-contract)
- scope_class: `cargo-manifest-metadata-only`
- upstream_main: `2c8ea33c9`

## Ubiquitous Language

| Term | Definition |
|------|------------|
| `WorkflowSource` | The top-level typed AST for one parsed workflow YAML document. `pub` in every build configuration. Constructor `WorkflowSource::new(parts)` is `pub(crate)` in production, `pub` under `cfg(any(test, feature = "test-util"))`. |
| `WorkflowSourceParts` | Bundle of fields used by `WorkflowSource::new` to construct a `WorkflowSource`. Field shape is identical across production and test-util configurations; only the visibility attribute differs. `pub(crate)` in production, `pub` under `cfg(any(test, feature = "test-util"))`. |
| `test-util` feature | A Cargo feature declared at `crates/vb_compile/Cargo.toml:23`. `default = []` so production crates that depend on `vb_compile` without explicit features never see `WorkflowSourceParts` as `pub`. |
| Self-referencing dev-dependency | Cargo's documented pattern (`specifying-dependencies.html#self-references`) where a `[dev-dependencies]` entry references the crate itself with `path = "."` to enable a feature ONLY for the test build. Activates `test-util` for integration tests while leaving production builds unaffected. |
| Integration test | External test binary compiled by `cargo test` against the crate's *public* API surface (i.e. `pub` items only). Cannot reach `pub(crate)` items. |
| Crate-internal test | A `#[cfg(test)]` module compiled inside the crate root; `pub(crate)` items are visible. Already works and is unaffected. |
| Production build | `cargo build -p <crate>` without `--tests` and without `--features test-util`. The visibility contract here is that `WorkflowSourceParts` MUST NOT be `pub`. |
| Test build | `cargo test -p vb_compile` (or `cargo build --tests`). The visibility contract here is that integration tests MUST see `WorkflowSourceParts` as `pub`. |

## Aggregate / Value-Object Inventory

This bead is build-only. The "aggregate" is the Cargo package `vb_compile` itself, and the value object is the `[dev-dependencies]` table.

### Aggregate: `vb_compile` Package

- Identity: `Cargo.toml::[package].name = "vb_compile"` (line 2).
- Members: production source in `src/`, integration tests in `tests/`, declared features in `[features]`.
- Policy: features listed in `[features].default` are activated for *all* consumers, including production binaries; the `test-util` feature is NOT in `default`, so consumers that do not opt in never expose `WorkflowSourceParts` as `pub`.

### Value Object: Self-Referencing Dev-Dependency Entry

```
vb_compile = { path = ".", features = ["test-util"] }
```

Invariants:

- Lives in `[dev-dependencies]`.
- `path = "."` so Cargo accepts the self-reference (per `specifying-dependencies.html#self-references`).
- `features = ["test-util"]` activates the existing `test-util` feature declared at `Cargo.toml:23`.
- Single source of activation: only this entry flips the visibility gate to `pub` for the test build.

## Forbidden / Illegal States

| Forbidden state | Why illegal |
|-----------------|-------------|
| `test-util` in `[features].default` | Would expose `WorkflowSourceParts` to every downstream consumer (`vb_cli`, `workspace_tests`), violating production-API surface. |
| `WorkflowSourceParts` declared `pub` unconditionally in `src/...` | Same API leak; would break the "hidden types behind `pub(crate)`" invariant that production builds rely on. |
| Visibility-gate `#[cfg]` replaced with a single unconditional `pub` | Same API leak. |
| Self-referencing entry in `[dependencies]` (instead of `[dev-dependencies]`) | Would activate `test-util` in production builds. |
| Two `WorkflowSourceParts` type declarations diverging in field shape | The two cfg-gated definitions at `workflow.rs:107–127` and `workflow.rs:129–149` MUST remain field-identical; only visibility differs. Drift would produce a type-mismatch between production and test-util builds. |
| `Cargo.lock` containing more than the self-reference change | Any other delta signals accidental dependency churn. |

## Invariants (Domain)

1. **Visibility by Configuration** — `WorkflowSourceParts` and `WorkflowSource::new` are `pub(crate)` precisely when neither `cfg(test)` nor `feature = "test-util"` is active, and `pub` otherwise. This invariant is encoded at `workflow.rs:32–43` and `:107–149`.
2. **`test-util` Feature Inertness in Default Profile** — `default = []` in `Cargo.toml` so the feature is OFF for all consumers that do not explicitly request it. Verified at `Cargo.toml:22`.
3. **Test-Build Activation via Dev-Dependency Only** — The only sanctioned path to flip `test-util` on for integration-test compilation is a `[dev-dependencies]` self-reference entry. Production-facing `[dependencies]` must never activate it.
4. **API Surface Preservation (Downstream)** — Downstream crates that depend on `vb_compile` without features (`vb_cli`, `workspace_tests`) see no new public symbols after this fix.
5. **Single Source of Truth** — The `Cargo.lock` entry for `vb_compile` inside `vb_compile`'s own closure is the only lockfile delta; production-binary closure graphs are otherwise unchanged.

## Out-of-Scope Domain Notes (Not Addressed by vb-rz9ey)

- **Q1 (flagged in `codebase-map.md`)** — Kani harnesses at `src/kani_digest_ask_*.rs` and `src/kani_digest_step_primitive_no_panic.rs` import `use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};`. `WorkflowSource` is not re-exported from `crate::ast` (only `WorkflowAst` is — `src/ast.rs:13`). This is a latent pre-existing defect independent of the test-util gate. Tracked as a follow-up bead, not vb-rz9ey.
- The `Cargo.toml` `[features]` block has no `required-features` on `test-util`. This is intentional and correct: a feature with no dependencies must be declared without `required-features`.

## Domain Decisions Closed by This Bead

- D1: `test-util` activation surface = a single line in `[dev-dependencies]` (self-reference).
- D2: Production API surface is preserved by leaving both `workflow.rs` cfg-gated declarations exactly as-is.
- D3: `Cargo.lock` is a generated artifact (expected +1 line, only the self-reference).
- D4: No type-shape divergence between the two cfg-gated `WorkflowSourceParts` declarations.
