# Flux RS Verification Status

Generated: 2026-06-14

## Inventory

| Metric | Count |
|---|---|
| Files referencing `flux_rs` in `crates/` | 31 |
| Files referencing `flux_rs` in `verification/` | ~45 (`.rs` + `.flux`) |
| Total `flux_rs` annotations in `crates/` | 174 |
| Total `flux_rs` annotations in `verification/` (`.rs` only) | 130 |
| Total `flux_rs` annotations in `verification/` (`.flux` only) | 38 |
| `#[flux_rs::trusted]` annotations in `crates/` | 44 |
| `#[flux_rs::trusted]` annotations in `verification/` | 17 |
| `#[trusted]` (bare) annotations | 0 |
| Crates depending on `flux-rs` | 3 (`vb_queue_semantics`, `vb_runtime` (opt), `vb_storage` (opt)) |
| `cargo flux` CLI | Available |

## Verification Status

**No active Flux verification is running in CI or on commit.**

All Flux annotations are unverified — no `cargo flux` invocation is wired into:
- CI pipelines (`moon ci`)
- Pre-commit hooks
- Any test suite
- The existing `scripts/flux-check-package.sh` script (present but dormant)

The `#[flux_rs::trusted]` annotations bypass verification entirely. These cover cancel/kill lifecycle logic in `vb_runtime` and all model files under `verification/flux/`.

Within `crates/vb_compile/src/`, several `#[flux_rs::sig]` annotations exist alongside commented-out refinements (e.g. `body_step_width_flux.rs` has 3 commented refinements suggesting prior bounds that were never proven).

## Recommendation

1. **Choose a verification entry point.** The most viable first target is `vb_queue_semantics` — it has a non-optional `flux-rs` dependency, no `#[trusted]` annotations, and is scoped to queue-index refinements.

2. **Wire `scripts/flux-check-package.sh` into `moon ci`** for the chosen crate. Start with a permissive surface (no `--deny` warnings) to establish a baseline.

3. **Retire unverified annotations.** Either run `cargo flux` on each annotated crate, or strip dead annotations. Currently 44 `#[flux_rs::trusted]` annotations in production crates provide no assurance.

4. **Expand only from proven base.** Do not add new Flux refinements to production crates until at least one crate has passing `cargo flux` in CI.
