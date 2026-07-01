# Workflow Model — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 3 (rust-contract)

vb-rz9ey is a Cargo-manifest metadata change. There is no runtime workflow to model. This document describes the *release-pipeline* workflow that is affected by this fix, plus a degenerate workflow describing the visibility toggle.

## W-1 Visibility Toggle Workflow (Degenerate, Build-Time Only)

This is a typestate machine over Cargo feature resolution + rustc cfg evaluation. There are exactly two states, no loops.

```
        ┌─────────────────────────┐
        │  visibility = pub(crate)│
        │  consumer = production  │
        └────────────┬────────────┘
                     │ feature = "test-util"
                     │ activated for this build
                     ▼
        ┌─────────────────────────┐
        │  visibility = pub       │
        │  consumer = test build  │
        └─────────────────────────┘
```

### Legal states

| State ID | Trigger preconditions | Visibility | Reachable from |
|----------|-----------------------|------------|----------------|
| `S_PROD`  | `cfg(not(any(test, feature = "test-util")))` | `pub(crate)` | always initially |
| `S_TEST`  | `cfg(any(test, feature = "test-util"))` | `pub` | only when feature flag is on |

### Transitions

| Transition | Guard | Outcome |
|------------|-------|---------|
| `S_PROD → S_TEST` | Either `cargo test` (`cfg(test)`) OR `cargo build --features test-util` (or its dev-dep-injected equivalent) | `WorkflowSourceParts` and `WorkflowSource::new` switch to `pub` |
| `S_TEST → S_PROD` | Build with `default-features` only and no `--tests` | Reverts to `pub(crate)` |

### Illegal states

- `S_TEST` reached *without* `cfg(test)` and without `feature = "test-util"` — impossible, enforced by the mutually exclusive `cfg` arms.
- A build that simultaneously satisfies both `cfg` arms — impossible, because `cfg(any(...))` and `cfg(not(any(...)))` partition the cfg evaluation.

### Terminal states

- Per-build terminal: either the test build links (state `S_TEST` reached) or it fails with `E0432/E0624` (state `S_PROD` reached but downstream tests asserted `pub`).

## W-2 Release/Verification Pipeline Workflow (Affected by This Bead)

This is the linear release-verification pipeline whose **Test** phase currently fails to compile. This bead fixes the **Test** phase's *build gate* only; no runtime phases are touched.

```
[Cargo Manifest Apply]
       │
       ▼
[Production Compile (cargo build)]           ← unaffected
       │
       ▼
[Test Compile (cargo build --tests)]         ← FIXED by vb-rz9ey
       │
       ▼
[Test Run (cargo test)]                      ← downstream of fix
       │
       ▼
[Downstream Negative Check]                  ← defensive gate added
   - cargo build -p vb_cli
   - cargo build -p workspace_tests
       │
       ▼
[Lockfile Review]
       │
       ▼
[Land]
```

### Per-phase contracts

| Phase | Owner | Required success criterion | Risk if violated |
|-------|-------|---------------------------|-----------------|
| Cargo Manifest Apply | `holzman-rust` | `[dev-dependencies]` contains the self-reference entry | Subsequent phases all fail |
| Production Compile | CI / `holzman-rust` | `cargo build -p vb_compile` succeeds with `pub(crate)` visibility | regresses production API |
| Test Compile | CI | `cargo build -p vb_compile --tests` exits 0 with 0 errors (baseline = 38 errors) | bead is not done |
| Test Run | CI | All integration tests pass | new regressions |
| Downstream Negative Check | `black-hat-reviewer` | `vb_cli`, `workspace_tests` still compile without `test-util` | silent API leak |
| Lockfile Review | `black-hat-reviewer` | `Cargo.lock` diff is exactly +1 line, the self-reference | accidental dep churn |

### Terminal outcomes

- **Pass**: all five compile/test gates green; lockfile diff is one line.
- **Fail**: any of the above gates red; revert the `Cargo.toml` and `Cargo.lock` changes, re-open bead.

### No loops, no retries

The pipeline is strictly forward. There is no "retry this phase without fixing root cause" path. The root cause is the missing `test-util` activation; if activation does not work, the fix is wrong, not the pipeline.

## W-3 What Workflow Is *Not* Modeled Here

- The runtime YAML → AST → IR → compiled-artifact workflow inside `vb_compile`. This is owned by other beads (`vb-xi2f.*`); it is unaffected.
- The Cargo feature-resolution algorithm itself. We take Cargo's documented behavior as ground truth.
- The integration-test behavioral coverage of the AST parser. Owned by `test-writer`; orthogonal to this bead.
