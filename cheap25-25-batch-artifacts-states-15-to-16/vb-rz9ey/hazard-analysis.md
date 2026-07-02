# Hazard Analysis — vb-rz9ey

- bead_id: `vb-rz9ey`
- skill_state: 3 (rust-contract)
- scope_class: `cargo-manifest-metadata-only`
- behavior_affecting: `false` (visibility invariant is already encoded in source; this bead only activates an existing cfg gate)

## Hazard Register

| ID | Hazard class | Description | Probability | Impact | Net risk | Mitigation | Owner lane |
|----|--------------|-------------|-------------|--------|----------|-----------|------------|
| H-01 | Public API surface leak | `test-util` accidentally promoted into `default` features, exposing `WorkflowSourceParts` to all consumers. | Low | High (silent API change) | **Medium** | Keep `default = []`; black-hat reviewer verifies `cargo doc` output. | `black-hat-reviewer` |
| H-02 | Wrong-feature activation | A future change adds a feature (e.g. `unsafe-internals`) to the dev-dep `features = [...]` list, changing the visibility footprint. | Low | Medium | Low | This bead scopes exactly to `["test-util"]`. Add a comment in `Cargo.toml` documenting intent. | `holzman-rust` |
| H-03 | Lockfile drift | `Cargo.lock` diff exceeds one line due to unrelated package churn. | Low | Low | Low | `git diff Cargo.lock` MUST be exactly one line. Flag any other diff. | `black-hat-reviewer`, `landing-skill` |
| H-04 | Self-reference placement | Self-reference accidentally placed in `[dependencies]` instead of `[dev-dependencies]`, activating `test-util` in production builds. | Low | High (full API leak) | **Medium** | Mandatory placement in `[dev-dependencies]`; review with `git diff` snippet. | `holzman-rust` |
| H-05 | Production-build regression | Visibility gate accidentally removed in this bead while editing the manifest. (Ironically, the bead does not edit source; this hazard is for source-rebase risk.) | Very low | High | Low | Source files (`workflow.rs:107-149`) are explicitly off-limits; black-hat reviewer verifies via `git diff --stat`. | `black-hat-reviewer` |
| H-06 | Field-shape divergence between cfg arms | A future edit to `WorkflowSourceParts` adds a field to one arm without mirroring it in the other. | Medium (cumulative across edits) | High (compile error in test build, silent in prod) | **Medium** | This bead does NOT introduce the divergence; existing code is identical-field. A future-proofing note belongs in a follow-up bead. Out-of-scope here. | (follow-up) |
| H-07 | Test-build Cargo-feature unification | Cargo's feature unification is global to a build. If a sibling crate (e.g. workspace_tests) ALSO requested `test-util` from `vb_compile`, it would propagate. Verified: no such request exists (workspace_tests uses no features). | Very low | Medium | Low | Negative-check via `cargo build -p workspace_tests` after fix. | `black-hat-reviewer` |
| H-08 | Downstream compile regression | A future change in `vb_cli` or `workspace_tests` starts importing `WorkflowSourceParts` directly. Their `Cargo.toml` would still not activate `test-util`, so their build would fail with `E0432`. | Low (latent) | Medium | Low | This bead does not introduce or fix this. Note as a follow-up. | (follow-up) |
| H-09 | Kani-harness latent defect | Six Kani harnesses at `src/kani_digest_ask_*.rs` and `src/kani_digest_step_primitive_no_panic.rs` use `crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts}`. `crate::ast` does not export `WorkflowSource`. They are gated by `#[cfg(all(kani, any(test, feature = "test-util")))]` and ARE NOT built by `cargo build --tests`. | Medium | Low (does not block cargo build; only blocks `cargo kani`) | Low | OUT OF SCOPE for vb-rz9ey. Flagged for future bead (`codebase-map.md` Q1). | (follow-up) |
| H-10 | `#[doc(hidden)]` lossy docs | Both cfg arms declare `#[doc(hidden)]`. Production build's `cargo doc` will not show `WorkflowSourceParts`, which is correct. Test build's `cargo doc --features test-util` will show it. Either is acceptable. | Very low | Low | Negligible | Docs are consistent with current intent. | (none) |
| H-11 | `cargo lockfile` formatting | Adding a self-reference may change Cargo.lock *formatting* beyond a single line if rerunning cargo on a stale lockfile. | Very low | Low | Negligible | Re-run `cargo metadata` after edit; commit the resulting lockfile verbatim. | `landing-skill` |
| H-12 | Idempotence of fix application | Running `cargo build -p vb_compile --tests` repeatedly must yield identical results. | Negligible | Low | Negligible | Cargo and rustc are deterministic w.r.t. this fix. | (none) |
| H-13 | Bead scope creep | A reviewer or future agent tries to "fix" related issues (Q1 Kani latent defect, tests `digest_ask_*` that already compile, etc.) inside vb-rz9ey. | Medium | Medium | **Medium** | Bead scope = Cargo manifest + Cargo.lock. Any other change requires a new bead. | `black-hat-reviewer` |
| H-14 | test-util exposure to other Cargo workspaces | Outside the `velvet-ballistics` workspace, `cargo install`-style users with their own `[patch]` chains could see `test-util` propagated. | Negligible (patches are opt-in) | Low | Negligible | This is a Cargo-wide convention. | (none) |

## Top Three Risks (Ordered)

1. **H-01 / H-04 (tied):** Public-API surface leak via `default = ["test-util"]` OR via placing the self-reference in `[dependencies]`. Both are surgical mistakes that black-hat review MUST catch.
2. **H-13:** Scope creep — touching tests or Kani harnesses outside the manifest is forbidden.
3. **H-06 / H-09:** Cumulative drift hazards — out of scope for this bead but worth tracking.

## Behavioral Impact Statement

This bead makes NO behavioral change at runtime. The visibility contract is preserved exactly as authored in `workflow.rs:107-149`. The cfg gate is the *single* place where visibility is decided; this bead only flips the feature flag that selects between the two arms for the test build. `#[doc(hidden)]` is applied on both arms; no new public-API symbol appears in the default-feature build.

## Pre-Flight Validation

A `holzman-rust` implementer MUST run, in order, before declaring completion:

```bash
# 1. The bead's primary test compile
cargo build -p vb_compile --tests --message-format=human

# 2. The downstream negative-checks
cargo build -p vb_cli --message-format=human
cargo build -p workspace_tests --message-format=human

# 3. The lockfile diff
git diff --stat Cargo.lock  # should show +1
git diff Cargo.lock         # should show only the self-reference addition
```

## Out-of-Scope Hazards (Recorded for Future Beads)

- **H-06**: Field-shape divergence between cfg arms.
- **H-09**: Kani-harness `crate::ast` import path.
- **H-08**: Downstream crates importing `WorkflowSourceParts` directly.
