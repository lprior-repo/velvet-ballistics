# Boundary Map — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 3 (rust-contract)
- scope: Cargo-manifest metadata-only; no production-code changes.

## Boundary Diagram

```
                                        ┌─────────────────────────────────────────┐
                                        │  vb_compile (production source)         │
                                        │                                         │
                          ┌──────────── │  - WorkflowSource:    pub               │
                          │             │  - WorkflowSourceParts:                 │
                          │             │     pub(crate) under                   │
                          │             │       cfg(not(any(test, "test-util")))  │
                          │             │     pub under cfg(any(test,"test-util"))│
                          │             │  - WorkflowSource::new: cfg-gated       │
                          │             └─────────────────────────────────────────┘
                          │                                   │
              ┌───────────┴──────────┐                        │
              │                      │                        │
              ▼                      ▼                        ▼
   ┌────────────────────┐  ┌────────────────────────┐  ┌─────────────────────────┐
   │ Production Build   │  │ Test Build (crate-     │  │ Test Build (external    │
   │ Boundary           │  │ internal, via #[...])  │  │ integration tests)      │
   │                    │  │                        │  │                         │
   │ - vb_cli           │  │ - #[cfg(test)] modules │  │ - crates/vb_compile/    │
   │ - workspace_tests  │  │   in src/              │  │     tests/*.rs          │
   │ - (any consumer    │  │ - src/kani_digest_*.rs │  │   (38 baseline errors)  │
   │   without         │  │   (gated by cfg(kani)) │  │                         │
   │   test-util)      │  │                        │  │ Visibility required:    │
   │                    │  │ Visibility required:   │  │   WorkflowSourceParts   │
   │ Visibility         │  │   WorkflowSourceParts  │  │   = pub                 │
   │ required:          │  │   = pub (any test)     │  │   WorkflowSource::new   │
   │   WorkflowSource-  │  │   via cfg(test)        │  │   = pub                 │
   │   Parts = pub(crate)│ │                        │  │                         │
   │                    │  │ Trigger: cfg(test) is  │  │ Trigger: dev-dep        │
   │ Trigger: no        │  │ always on inside the   │  │ activates feature       │
   │ feature, no        │  │ crate root for unit    │  │ `test-util` for this    │
   │ cfg(test) on       │  │ tests, OR kani gate    │  │ build only              │
   │ this build         │  │ plus test-util for     │  │                         │
   │                    │  │ Kani-runner builds     │  │                         │
   └────────────────────┘  └────────────────────────┘  └─────────────────────────┘
```

## Boundary 1 — Production-Build Boundary (External Consumers)

**Inside the boundary**:

- `Cargo.lock` for `vb_compile` is unchanged in the *production-binary* closure graphs. The self-reference lives only in `vb_compile`'s own *test* closure.
- `vb_cli` Cargo.toml: `vb_compile = { path = "../vb_compile" }` (no features).
- `workspace_tests` Cargo.toml: `vb_compile = { path = "../vb_compile" }` (no features).

**Invariants**:

- B-1.a: Neither `vb_cli` nor `workspace_tests` explicitly activates `test-util` on `vb_compile`.
- B-1.b: Neither crate may begin activating `test-util` because it would silently leak `WorkflowSourceParts` into their public-API surface.
- B-1.c: `cargo build -p vb_cli --message-format=human` MUST exit 0 both before and after the fix.
- B-1.d: `cargo build -p workspace_tests --message-format=human` MUST exit 0 both before and after the fix.

**Validation commands** (negative checks; mandatory post-fix):

```bash
cargo build -p vb_cli --message-format=human
cargo build -p workspace_tests --message-format=human
```

## Boundary 2 — Crate-Internal Test Boundary

**Inside the boundary**:

- `#[cfg(test)]` modules compiled into `vb_compile` itself (e.g. `src/tests/foreach_digest_tests.rs`).
- Kani harnesses at `src/kani_digest_*.rs` (gated by `#[cfg(all(kani, any(test, feature = "test-util")))]`).

**Invariants**:

- B-2.a: `cfg(test)` is always satisfied within the crate root during `cargo test`. `pub(crate)` items are visible by definition to in-crate code.
- B-2.b: Kani harnesses additionally gate on `feature = "test-util"` so that they see `pub` visibility (mirroring integration tests). The Kani-runner builds `vb_compile` with `test-util` enabled; this bead does not affect that path.
- B-2.c: Latent pre-existing defect at `src/lib.rs:188-199`: Kani harnesses `use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};` — `crate::ast` does not re-export `WorkflowSource`. This is OUT OF SCOPE for vb-rz9ey (not affected by `cargo build --tests`).

## Boundary 3 — External Integration-Test Boundary (The Bead's Target)

**Inside the boundary**:

- `crates/vb_compile/tests/*.rs` (38 baseline errors before fix).
- `crates/vb_compile/tests/common/mod.rs` (helper module shared by 8 integration-test targets via `mod common;`).

**Invariants**:

- B-3.a: External integration tests CANNOT reach `pub(crate)` items — they are compiled as external crates against the `vb_compile` rlib's `pub` surface only.
- B-3.b: The visibility gate is `#[cfg(any(test, feature = "test-util"))]` at `src/lib.rs:241`. Activating `test-util` for this build re-exports `WorkflowSourceParts` as `pub` in the rlib's metadata.
- B-3.c: After the fix, every test file in `crates/vb_compile/tests/` (including `common/mod.rs`) MUST compile.
- B-3.d: The fix MUST NOT be implemented by editing any `tests/*.rs` file. The visibility gate is the correct architecture; only the feature activation is missing.

## Boundary 4 — Feature-Resolution Boundary (Cargo)

**Inside the boundary**:

- Cargo's feature unification algorithm: features activated by a `[dev-dependencies]` entry of one binary propagate to that binary's dependency closure ONLY.
- `vb_compile`'s `[dev-dependencies]` entry is consumed solely by `cargo test` for `vb_compile`'s own test binaries. It does NOT influence any other crate's build.

**Invariants**:

- B-4.a: Cargo's documented self-reference rule (`specifying-dependencies.html#self-references`) MUST be respected: the self-reference must use `path = "."` (or equivalent) and live in `[dev-dependencies]`.
- B-4.b: `Cargo.lock` MUST contain exactly one new line that lists `vb_compile` in `vb_compile`'s own test-binary dependency closure. The lockfile diff is the single external artifact proof of the fix.
- B-4.c: `[features].default` MUST remain `[]` — a single accidental change here would propagate `test-util` to every consumer.

## Boundary 5 — Source-Code Boundary (Do Not Cross)

This bead's diff MUST be confined to:

- `crates/vb_compile/Cargo.toml` — add one self-reference line (and probably a leading-section comment) in `[dev-dependencies]`.
- `Cargo.lock` — regenerated by Cargo.

**Files explicitly OFF-LIMITS** in this bead:

| File | Reason |
|------|--------|
| `crates/vb_compile/src/yaml_ast/types/workflow.rs` | Visibility logic is already correct; no change. |
| `crates/vb_compile/src/yaml_ast/types.rs` | Re-exports already correct. |
| `crates/vb_compile/src/yaml_ast/mod.rs` | Module-level re-exports already correct. |
| `crates/vb_compile/src/lib.rs` | Root re-exports already correct. |
| `crates/vb_compile/tests/**/*.rs` | Tests already correctly exercise the public surface; do not edit. |
| `crates/vb_compile/Cargo.toml [features]` | Feature declaration already correct; do not move into `default`. |
| `Cargo.toml` (workspace root) | Workspace member list unchanged. |

## Adversarial Boundary Inputs

The only "hostile input" boundary in this bead is the rustc compiler itself: cargo invokes rustc with the cfg-mask resolved by Cargo. If Cargo and rustc ever disagreed about which `cfg(any(...))` arm is active, the visibility toggle would yield the wrong visibility. We rely on the upstream invariant that feature resolution is the source of truth for the `feature = "test-util"` flag.
