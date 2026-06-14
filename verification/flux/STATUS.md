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

## Trusted Annotation Detail

Every `#[flux_rs::trusted]` annotation now has a `/// TRUSTED BOUNDARY justification:` doc comment
explaining why it exists and what compensating evidence validates it. Below is the summary.

### Production Files (crates/)

| File | Count | PO / Bead | Domain | Compensating Evidence |
|---|---|---|---|---|
| `vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` | 12 | PO-FLUX-001/002/003 | Cancel/kill lifecycle invariants | Kani PO-KANI-001/002/003; unit tests |
| `vb_storage/src/codec/flux_validation.rs` | 11 | PO-FLUX-004/005 | Kind-28 admission + replay contiguity | Kani PO-KANI-004/005; proptest PO-PROP-004 |

### Verification Model Files (crates/vb_runtime/src/verification/flux/vb_fzgdn/)

| File | Count | POB | Domain | Compensating Evidence |
|---|---|---|---|---|
| `ps_001.rs` | 4 | POB-vb-fzgdn-003 | TimerDeadline u64 arithmetic | Kani; unit tests (timer_wheel.rs) |
| `ps_002.rs` | 3 | POB-vb-fzgdn-008 | PendingTimer field constraints | Kani; unit tests (transitions.rs) |
| `ps_003.rs` | 2 | POB-vb-fzgdn-013 | Timer authority validation | Kani; all authority-check paths |
| `ps_004.rs` | 2 | POB-vb-fzgdn-017 | Generation advancement bounds | Kani; unit tests |
| `ps_005.rs` | 1 | POB-vb-fzgdn-021 | Duplicate key idempotency | Kani; insert-idempotency tests |
| `ps_007.rs` | 2 | POB-vb-fzgdn-030 | Monotonic clock advancement | Kani; integration tests |
| `ps_008.rs` | 2 | POB-vb-fzgdn-035 | Bounded capacity admission | Kani; config validation tests |
| `ps_009.rs` | 1 | POB-vb-fzgdn-039 | Zero-duration branch determinism | Kani; integration tests |
| `ps_010.rs` | 2 | POB-vb-fzgdn-044 | Atomic fire + enqueue consistency | Kani; queue overflow tests |
| **Total** | **19** | | | |

### Verification Mirror Files (verification/flux/vb-fzgdn/)

| File | Count | POB | Domain | Compensating Evidence |
|---|---|---|---|---|
| `PS-001-refinements.rs` | 1 | POB-vb-fzgdn-003 | TimerDeadline u64 arithmetic | Kani; unit tests |
| `PS-002-refinements.rs` | 3 | POB-vb-fzgdn-008 | PendingTimer field constraints | Kani; unit tests |
| `PS-003-refinements.rs` | 2 | POB-vb-fzgdn-013 | Timer authority validation | Kani |
| `PS-004-refinements.rs` | 2 | POB-vb-fzgdn-017 | Generation advancement bounds | Kani |
| `PS-005-refinements.rs` | 1 | POB-vb-fzgdn-021 | Duplicate key idempotency | Kani |
| `PS-007-refinements.rs` | 2 | POB-vb-fzgdn-030 | Monotonic clock advancement | Kani; integration tests |
| `PS-008-refinements.rs` | 2 | POB-vb-fzgdn-035 | Bounded capacity admission | Kani; config validation tests |
| `PS-009-refinements.rs` | 1 | POB-vb-fzgdn-039 | Zero-duration branch determinism | Kani; integration tests |
| `PS-010-refinements.rs` | 2 | POB-vb-fzgdn-044 | Atomic fire + enqueue consistency | Kani |
| **Total** | **16** | | | |

### Common Justification Patterns

All trusted annotations are justified by one or more of:

1. **Flux cannot reason about external types/methods** — `std::time::Instant`, `indexmap::IndexMap`, cross-crate calls, newtype unwraps. Compensating evidence: Kani harnesses test all paths.
2. **Flux cannot prove loop invariants** — slice iteration, nested loops for gap/duplicate detection. Compensating evidence: proptest (PO-PROP-004) and unit tests.
3. **Flux cannot cross enum type boundaries** — refined enum → production enum conversion. Compensating evidence: exhaustive match + unit tests.
4. **Type-level refinement contracts** — `SafeGeneration(gen)` with `gen < u64::MAX`. Compensating evidence: Kani bounds-checks all next_generation paths.

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
