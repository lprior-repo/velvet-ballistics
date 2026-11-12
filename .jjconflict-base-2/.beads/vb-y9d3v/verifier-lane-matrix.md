# Verifier Lane Matrix — vb-y9d3v

Decision matrix: one row per proof seed, one column per verifier. Cells show `required`, `not_applicable`, or `blocked_tooling`.

## Lane Decisions by Seed

| Seed ID | Risk Tags | Verus | Kani | Flux-rs | proptest | Loom | Miri | cargo-fuzz | TLA_PLUS_REMOVED+ |
|---|---|---|---|---|---|---|---|---|---|
| vb-y9d3v-seed-001 | stale-attempt, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-002 | future-attempt, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-003 | retry-fence, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-004 | stale-authority, rust-local, type-contract | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-005 | single-terminal, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-006 | typed-error, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-007 | verus, rust-local-proof | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-008 | kani, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-009 | flux-rs, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-010 | proptest, rust-local | required | required | required | required | not_applicable | not_applicable | not_applicable | not_applicable |
| vb-y9d3v-seed-011 | fuzz, codec | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | required | not_applicable |
| vb-y9d3v-seed-012 | temporal, tla-plus | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |

## Lane Applicability Summary

| Verifier | Required (seeds) | Not Applicable (seeds) | Blocked |
|---|---|---|---|
| Verus | 001-010 (10) | 011-012 (2) | 0 |
| Kani | 001-010 (10) | 011-012 (2) | 0 |
| Flux-rs | 001-010 (10) | 011-012 (2) | 0 |
| proptest | 001-010 (10) | 011-012 (2) | 0 |
| Loom | 0 | 001-012 (12) | 0 |
| Miri | 0 | 001-012 (12) | 0 |
| cargo-fuzz | 011 (1) | 001-010, 012 (11) | 0 |
| TLA_PLUS_REMOVED+ | 0 | 001-012 (12) | 0 |

## Default Lane Not-Applicable Evidence for Seed 012

Seed 012 (`REQ-tla_removed-action-authority`) is explicitly scoped as temporal design evidence per its notes: *"TLA_PLUS_REMOVED+ is temporal design evidence, not Rust implementation proof."* The underlying Rust-local invariants for action authority are covered by seeds 001-011. Default Rust lanes are `not_applicable` for this seed because:
- The seed's domain claim targets temporal lifecycle ordering, not Rust implementation correctness
- All related Rust-local properties (attempt equality, future rejection, capacity bounds, non-mutation) are separately covered by seeds 001-011 with full default lane profiles
- Requiring Kani/Verus/Flux/proptest for a purely temporal-model seed would duplicate the other 11 seeds' obligations without additional behavioral coverage

## Loom Not-Applicable Evidence (All Seeds)

Per `boundary-map.md` §Async/Concurrency Boundary: *"Runtime execution is shard-owned and synchronous until suspension. External completions/failures can arrive after cancellation, terminal removal, retry replacement, or timer replacement; generation fences must handle reordering."* The generation fence operates at the shard boundary synchronously:
- `validate_action_completion` / `validate_ticket_attempt` are pure synchronous functions
- `preflight_action_completion` performs ordered checks before any mutation
- `handle_action_completion` / `handle_action_failure` execute within a single shard task
- No atomics, locks, channels, or concurrent task ownership exist in the authority check path
- All in-scope files enforce `#![forbid(unsafe_code)]`

Existing Loom model files (`crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`, `action_completion_cancel.rs`) are prior-art models that exercise cancellation scenarios but do not target the synchronous authority fence itself. These models may be revived in a separate bead for timeout/concurrent-cancellation verification.

## Miri Not-Applicable Evidence (All Seeds)

- All in-scope production files enforce `#![forbid(unsafe_code)]`
- No `unsafe` blocks, FFI declarations, raw pointers, `MaybeUninit`, or provenance-sensitive constructs exist in: `crates/vb_core/src/action.rs`, `crates/vb_runtime/src/shard/helpers.rs`, `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`, `chunk_003.rs`, `crates/vb_runtime/src/shard/transitions.rs`, `crates/vb_runtime/src/shard/timer_wheel.rs`, `crates/vb_runtime/src/engine/action.rs`
