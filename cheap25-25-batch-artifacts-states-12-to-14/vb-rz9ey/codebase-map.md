# Codebase Map — vb-rz9ey

- bead_id: vb-rz9ey
- title: Fix vb_compile test compilation: WorkflowSourceParts private (P0)
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
- source_checkout: /home/lewis/src/velvet-ballistics
- upstream_main: 2c8ea33c9
- jj_workspace: cheap25-vb-rz9ey
- explored_at: 2026-07-01T15:25:00Z
- explorer: explore-agent (State 2, go-skill)

## Bead Restatement

`vb_compile` integration tests under `crates/vb_compile/tests/` fail to compile because:

1. The struct `WorkflowSourceParts` is `pub(crate)` in production builds
   (gated by `#[cfg(not(any(test, feature = "test-util")))]`).
2. The associated constructor `WorkflowSource::new(parts)` is also `pub(crate)`
   under the same gate.
3. Integration tests are external test binaries; they cannot see `pub(crate)`
   items.
4. The `test-util` Cargo feature is declared in `crates/vb_compile/Cargo.toml`
   but never enabled by any consumer (it is `default = []`).

Direct rustc evidence: `cargo build -p vb_compile --tests --message-format=human`
produces **38 errors** (mix of `E0432 unresolved import` and `E0624 private
associated function`) across 9 integration-test source files. The compiler's
own help note pinpoints the gate at `crates/vb_compile/src/lib.rs:241`:

```
241 | #[cfg(any(test, feature = "test-util"))]
    |          ----------------------------- the item is gated here
242 | pub use yaml_ast::types::WorkflowSourceParts;
```

The minimum-surface fix is to enable the existing `test-util` feature for the
test build via a self-referencing dev-dependency:

```toml
[dev-dependencies]
vb_compile = { path = ".", features = ["test-util"] }
```

This is a textbook Cargo pattern (`specifying-dependencies.html#self-references`).
Production builds are unaffected because `test-util` is not in `default`.

## Files Mapped

### Production source (visibility gate)

- `crates/vb_compile/src/yaml_ast/types/workflow.rs`
  - `WorkflowSource` (`pub struct`, lines 7–27): fields are all `pub(crate)`
    with accessor methods `version()`, `name()`, `trigger()`, `inputs()`,
    `vars()`, `secrets()`, `steps()`, `result()`, `examples()` (lines 60–102).
  - `impl WorkflowSource::new` has TWO definitions gated by `cfg`:
    - lines 32–35: `#[cfg(not(any(test, feature = "test-util")))] pub(crate) fn new`
    - lines 40–43: `#[cfg(any(test, feature = "test-util"))] pub fn new`
    Both delegate to a private `fn from_parts(parts: WorkflowSourceParts) -> Self`
    at line 45.
  - `WorkflowSourceParts` has TWO definitions gated by the same `cfg`:
    - lines 107–127: `pub(crate) struct` with `pub(crate)` fields (production)
    - lines 129–149: `pub struct` with `pub` fields (test-util / cfg(test))
    Field shape is identical; only the visibility attribute differs. This is
    the standard Rust workaround for cfg-gated visibility.

- `crates/vb_compile/src/yaml_ast/types.rs` (21 lines)
  - Line 15: `pub use workflow::WorkflowSource;`
  - Line 17–18: `#[cfg(any(test, feature = "test-util"))] pub use workflow::WorkflowSourceParts;`
  - Line 20–21: `#[cfg(not(any(test, feature = "test-util")))] pub(crate) use workflow::WorkflowSourceParts;`

- `crates/vb_compile/src/yaml_ast/mod.rs` (37 lines)
  - Lines 25–29: explicit `pub use types::{...}` list (no glob, to avoid
    accidentally re-exporting `pub(crate)` items in production).
  - Lines 33–34: `#[cfg(any(test, feature = "test-util"))] pub use types::WorkflowSourceParts;`
  - Line 23 comment: "explicit list — no glob, so that pub(crate)-restricted
    items like WorkflowSourceParts are not accidentally made public in
    production builds".

- `crates/vb_compile/src/lib.rs`
  - Lines 186–187 (comment): "WorkflowSourceParts which is pub(crate) in
    production and only re-exported as pub when test-util feature is active".
  - Lines 188–199: Kani harnesses for vb-xi2f.33 (digest_ask_*) gated by
    `#[cfg(all(kani, any(test, feature = "test-util")))]`. These harnesses
    `use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};`
    — see "Open Questions" Q1 below; this `use` path is broken independent of
    this bead.
  - Lines 241–242: root-level re-export
    `#[cfg(any(test, feature = "test-util"))] pub use yaml_ast::types::WorkflowSourceParts;`
    — this is the exact line rustc points to in the help note.
  - Lines 243–247: root-level re-export of `yaml_ast::types::{...WorkflowSource}`
    (always pub; not gated).

### Cargo manifest (target of fix)

- `crates/vb_compile/Cargo.toml` (60 lines)
  - Lines 7–16 `[dependencies]`: includes `vb_core = { path = "../vb_core", features = ["test-util"] }`
    (vb_core test-util is correctly activated here).
  - Lines 18–19 `[dev-dependencies]`: only `proptest.workspace = true`.
    **This is where the fix must add `vb_compile = { path = ".", features = ["test-util"] }`**
    as a self-referencing dev-dependency so the integration tests get the
    test-util-gated public visibility.
  - Lines 21–23 `[features]`: `default = []`, `test-util = []`. The `test-util`
    feature flag itself is declared but unused — it has no
    `required-features` and is not in `default`.

### Tests with compile failures (verified by `cargo build -p vb_compile --tests`)

Counts after `cargo build -p vb_compile --tests --message-format=human`:

| File | E0432 imports | E0624 `new` calls | Lines (calls) |
|------|---------------|-------------------|---------------|
| `crates/vb_compile/tests/common/mod.rs` | 1 (line 12) | 9 | 20, 61, 88, 114, 140, 181, 196, 211, 226 |
| `crates/vb_compile/tests/digest_structural_fields.rs` | 4 (233, 297, 359, 438) | 7 | 260, 271, 324, 335, 386, 397, 439 |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` | 1 (line 29) | 4 | 113, 338, 391, 402 |
| `crates/vb_compile/tests/digest_set_finish_regression.rs` | 1 (line 185) | 1 | 187 |
| `crates/vb_compile/tests/digest_ask_explicit_arm.rs` | 1 (line 194) | 1 | 195 |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | 1 (line 18) | 1 | 62 |
| `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | 1 (line 18) | 1 | 34 |
| `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | 1 (line 18) | 1 | 34 |
| `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | 1 (line 18) | 1 | 49 |
| **TOTAL** | **12 E0432** | **26 E0624** | **38 errors** (matches `cargo build` summary) |

Note: `common/mod.rs` is not itself a `[[test]]` target; it is compiled as a
sibling module by every test that does `mod common;` (eight tests do so:
digest_structural_fields, digest_set_finish_regression, digest_duplicate_parity,
digest_ask_timeout_sensitivity, digest_ask_explicit_arm, digest_ask_determinism,
digest_ask_prompt_sensitivity, digest_ask_empty_prompt).

### Tests NOT affected by this bead (use only public `WorkflowSource` via parser)

- `tests/digest_compilation_pipeline.rs` (4.8K): uses `vb_compile::WorkflowSource`
  via `parse_workflow_source(&yaml)` — compiles fine.
- `tests/v1_primitive_lowering.rs` (91.4K): uses `vb_compile::WorkflowSource`
  via `vb_compile::parse_workflow_source(...)` — compiles fine.
- `tests/digest_repeat_unit.rs`, `tests/repeat_digest_integration.rs`,
  `tests/finish_digest_integration.rs`, `tests/finish_digest_structural.rs`,
  `tests/digest_ask_determinism.rs`, `tests/digest_ask_empty_prompt.rs`,
  `tests/digest_duplicate_parity.rs`, `tests/together_digest_sensitivity.rs`,
  `tests/digest_yaml_e2e.rs`, `tests/vb_a001_for_each_topology.rs`,
  `tests/vb_xi2f_*`, `tests/vb_core_yaml_e2e_chain_strict_yaml.rs`,
  `tests/integration_choose_body.rs`, `tests/idempotency_parity.rs`,
  `tests/vb_8mdp_7_collect_lowering_props.rs` — all use the public
  `WorkflowSource` (and never `WorkflowSourceParts` directly).

### Internal (crate-internal) tests unaffected by this bead

- `crates/vb_compile/src/tests/foreach_digest_tests.rs` (95.6K) and any
  `#[cfg(test)]` modules — these run inside the `vb_compile` crate so
  `pub(crate)` items are visible. They already work.

### Verification / contract surfaces (none affected)

- `contracts/`: no references to `WorkflowSourceParts`.
- `verification/`: no references to `WorkflowSourceParts` (no Verus spec, no
  Flux refinement, no Loom model, no TLA+ action mentions this struct).

### Downstream crates — verification of public-API non-leak

- `crates/vb_runtime/Cargo.toml` line 22: `vb_core = { path = "../vb_core", features = ["test-util"] }`
- `crates/vb_runtime/Cargo.toml`: does NOT depend on `vb_compile` directly
  (verified via grep — only `vb_core` test-util).
- `crates/vb_storage/Cargo.toml`: no `vb_compile` dep.
- `crates/vb_cli/Cargo.toml` line 8: `vb_compile = { path = "../vb_compile" }`
  (no features activated; the production build of vb_cli never sees
  `WorkflowSourceParts` as `pub`).
- `crates/workspace_tests/Cargo.toml` line 39: `vb_compile = { path = "../vb_compile" }`
  (no features activated; workspace_tests integration tests do NOT directly
  import `WorkflowSourceParts`, so they continue to compile under production
  visibility).

## Existing Tests Inventory (relevant)

| Path | Target | Uses `WorkflowSourceParts`? | Compiles today? |
|------|--------|---------------------------|-----------------|
| `crates/vb_compile/tests/common/mod.rs` | shared helpers (10 builders) | YES (10 sites) | NO |
| `crates/vb_compile/tests/digest_structural_fields.rs` | B15-B19 step/digest field sensitivity | YES (11 sites) | NO |
| `crates/vb_compile/tests/proptest_digest_foreach.rs` | proptest foreach parity | YES (5 sites) | NO |
| `crates/vb_compile/tests/digest_set_finish_regression.rs` | set+finish regression | YES (2 sites) | NO |
| `crates/vb_compile/tests/digest_ask_explicit_arm.rs` | ask explicit arm | YES (2 sites) | NO |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | proptest determinism | YES (2 sites) | NO |
| `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | ask timeout | YES (2 sites) | NO |
| `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | ask prompt | YES (2 sites) | NO |
| `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | ask ordering | YES (2 sites) | NO |
| `crates/vb_compile/tests/digest_ask_determinism.rs` | ask determinism (no direct `WorkflowSourceParts`) | NO | YES (uses `ask_source` from common) |
| `crates/vb_compile/tests/digest_ask_empty_prompt.rs` | ask empty prompt (uses common) | NO | NO (blocked by common) |
| `crates/vb_compile/tests/digest_ask_prompt_sensitivity.rs` | ask prompt sens. (uses common) | NO | NO (blocked by common) |
| `crates/vb_compile/tests/digest_ask_timeout_sensitivity.rs` | ask timeout sens. (uses common) | NO | NO (blocked by common) |
| `crates/vb_compile/tests/digest_duplicate_parity.rs` | duplicate parity (uses common) | NO | NO (blocked by common) |

After the fix: ALL rows above will compile and run.

### Internal Kani harnesses (gated by `#[cfg(all(kani, any(test, feature = "test-util")))]`)

- `src/kani_digest_ask_empty_prompt.rs`, `kani_digest_ask_field_ordering.rs`,
  `kani_digest_ask_prompt_sensitivity.rs`, `kani_digest_ask_timeout_sensitivity.rs`,
  `kani_digest_ask_timeout_sentinel.rs`, `kani_digest_step_primitive_no_panic.rs`.

These `use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};`
but `crate::ast` only re-exports `WorkflowAst` (not `WorkflowSource`). See Q1.

## Dependencies / Build Graph

| Crate | Path dep on vb_compile? | Features activated |
|-------|-------------------------|---------------------|
| `vb_core` | n/a | n/a |
| `vb_runtime` | NO (uses only `vb_core`) | n/a |
| `vb_storage` | NO | n/a |
| `vb_validate` | NO | n/a |
| `vb_ipc` | NO | n/a |
| `vb_cli` | YES (`vb_compile = { path = "../vb_compile" }`) | none |
| `vb_queue_semantics` | not in workspace (per `Cargo.toml` members list) | n/a |
| `workspace_tests` | YES (dev-dep `vb_compile = { path = "../vb_compile" }`) | none |
| `vb_compile` (self) | via dev-dep (after fix) | `["test-util"]` (test build only) |

## Risk Tags (preview; full JSONL in `delivery-scope.jsonl`)

- `risk:build`: Cargo manifest change. Self-referencing dev-dep is canonical.
- `risk:public_api`: must verify `cargo build` of downstream crates
  (`vb_cli`, `workspace_tests`) still compiles WITHOUT `test-util` — i.e.
  `WorkflowSourceParts` must remain `pub(crate)` in non-test builds.
- `risk:lockfile`: `Cargo.lock` will gain one entry listing `vb_compile` in
  its own dependency closure. Expected and necessary.
- `risk:test_only`: change is dev-dep only; production binaries unaffected.

## Open Questions (downstream agents must address)

- **Q1 (out-of-scope flag)**: Kani harnesses at
  `crates/vb_compile/src/kani_digest_ask_*.rs` and `kani_digest_step_primitive_no_panic.rs`
  import `use crate::ast::{...WorkflowSource, WorkflowSourceParts};`. The
  `crate::ast` module (re-exporting `WorkflowAst` only — see `src/ast.rs` line 13)
  does **not** export `WorkflowSource`. This is a pre-existing latent defect
  that does NOT block vb-rz9ey (Kani harnesses are gated by `cfg(kani)` and
  are not part of `cargo build --tests`). Flag for a future bead; do not
  expand scope of vb-rz9ey.
- **Q2 (verification scope)**: No Verus/Flux/Kani/Loom obligations reference
  `WorkflowSourceParts`. The bead is build-only; no proof work needed.
- **Q3 (lockfile)**: A self-referencing dev-dep always adds one line to
  `Cargo.lock`. Confirm lockfile diff is one line and contains only the
  self-reference.
- **Q4 (downstream negative-check)**: Must run `cargo build -p vb_cli` and
  `cargo build -p workspace_tests` AFTER the fix to confirm `WorkflowSourceParts`
  is still `pub(crate)` in non-test builds (the help note from rustc confirms
  the gate is `#[cfg(any(test, feature = "test-util"))]` at lib.rs:241, but a
  belt-and-braces check is required).

## Recommended Downstream Owners

| Lane | Owner | Action |
|------|-------|--------|
| Contract | `rust-contract` (skip — pure build fix, no domain change) | none |
| Proof plan | `proof-planner` (skip — no proof obligations) | none |
| Proof write | `proof-writer` (skip — no proof obligations) | none |
| Test plan | `test-planner` (skip — existing test inventory already covers the surface) | none |
| Test write | `test-writer` (skip — all tests already exist; just need to compile) | none |
| Implementation | `holzman-rust` | edit `crates/vb_compile/Cargo.toml` `[dev-dependencies]` to add `vb_compile = { path = ".", features = ["test-util"] }` |
| Black-hat | `black-hat-reviewer` | verify Cargo.lock diff is one line; verify `cargo build -p vb_cli` and `cargo build -p workspace_tests` still succeed |
| Landing | `landing-skill` | standard jj land |
