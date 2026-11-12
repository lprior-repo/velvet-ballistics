# vb-y9d3v State 11: holzman-rust Implementation

**Bead:** vb-y9d3v
**State:** 11 (holzman-rust IMPLEMENTATION)
**Delegate:** holzman-rust
**Agent-invocation-ledger seq:** 17 — "vb-y9d3v-state11-holzman-rust-attempt1"
**Date:** 2026-05-30

## Reference Files Read

1. `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
2. `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
3. `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
4. `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
5. `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
6. `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`

## G005 Fix: Future-Attempt Rejection

### Problem
`validate_ticket_attempt` in `crates/vb_runtime/src/shard/helpers.rs` accepted action completions with `ticket.attempt > current` (future attempts) as valid, returning `Ok(())`. This violated ACT-005 which requires exact attempt matching.

### Production Code Change

**File:** `crates/vb_runtime/src/shard/helpers.rs`, lines 93-98

Added future-attempt rejection to `validate_ticket_attempt`:
```rust
// Future-attempt rejection (G005): reject completions that reference a
// later attempt than what is recorded. Zero means no attempt has been
// scheduled yet, so any positive attempt is acceptable.
if current > 0 && ticket.attempt > current {
    return Err(RuntimeError::InvalidActionCompletion);
}
```

Key design decision: `current == 0` means no attempt has been scheduled yet, so any positive attempt is acceptable. Only reject when `current > 0` AND `ticket.attempt > current`.

### Test Updates

1. **`helpers/tests.rs:2217`** — `validate_action_completion_rejects_future_attempt_when_attempt_exceeds_current`: Changed from G005-expected-failure (accepting both Ok and Err) to exact assertion: `assert_eq!(result, Err(RuntimeError::InvalidActionCompletion))`.

2. **`helpers/tests.rs:2728`** — Proptest `prop_validate_ticket_attempt_classifies_all_attempt_relations`: Changed from G005 gap handler to exact assertion: `prop_assert_eq!(result, Err(RuntimeError::InvalidActionCompletion), ...)`.

3. **`lifecycle_tests/chunk_004.rs:260`** — `future_attempt_completion_does_not_mutate_state`: Changed from accepting both Ok/Err to exact assertion: `assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion), ...)`. Fixed unused variable warnings (`journal_snap_before`, `trace_before`, `trace_after`).

4. **`lifecycle_tests/chunk_004.rs:2`** — `future_attempt_completion_rejected_when_current_attempt_exists`: Changed expected result from `Ok(true)` to `Err(RuntimeError::InvalidActionCompletion)`.

5. **`vb_jggy_lifecycle_tests.rs:835`** — `validate_ticket_attempt_accepts_valid_ticket`: Changed ticket from attempt=2 (future) to attempt=1 (matching current).

6. **`vb_jggy_lifecycle_tests.rs:913`** — `future_attempt_within_capacity_is_accepted`: Renamed to `future_attempt_within_capacity_is_rejected`, expects `Err(RuntimeError::InvalidActionCompletion)`.

7. **`vb_jggy_lifecycle_tests.rs:409`** — `action_failed_carries_attempt_field`: Changed ActionFailed ticket from attempt=2 to attempt=1 (matching current), updated journal assertion to attempt=1.

## Power-of-Ten Rules Affected

| Rule # | Rule | Status |
|--------|------|--------|
| 1 | Simple control flow | SATISFIED — added a single `if` guard, no recursion, no panic-driven flow |
| 2 | Fixed loop bounds | NOT APPLICABLE — no loops added |
| 3 | No post-init dynamic allocation | SATISFIED — this is a pure validation function, no allocation |
| 4 | Short functions | SATISFIED — `validate_ticket_attempt` is 28 lines |
| 5 | Invariant density | SATISFIED — invariant encoded in type-level check returning typed error |
| 6 | Smallest scope | SATISFIED — `current` declared and used within local scope |
| 7 | Checked returns | SATISFIED — returns `RuntimeError::InvalidActionCompletion`, never ignored |
| 8 | Limited macros | SATISFIED — no macros used |
| 9 | Restricted pointers | SATISFIED — no raw pointers, trait objects, or unsafe |
| 10 | Zero warnings | SATISFIED — only pre-existing `cfg(verus)` warning outside touched code |

## Zero-Panic Rules

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, `dbg!` in any touched production code
- No unchecked indexing or arithmetic
- No lossy `as` conversions
- No ignored fallible results
- Production `assert!` macros only in test code (all under `#[cfg(test)]`)

## Performance Layer

**No performance claim made.** This is a correctness change adding a validation gate. No benchmarks run.

## Gate Results

### Commands Run

| Command | Result |
|---------|--------|
| `cargo fmt -- crates/vb_runtime/src/shard/helpers.rs crates/vb_runtime/src/shard/helpers/tests.rs crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` | PASS (no changes needed) |
| `cargo clippy --workspace --lib --bins --examples -- -D warnings -D unsafe_code ...` | PASS (only pre-existing `cfg(verus)` warning in verification/mod.rs:25) |
| `cargo check --workspace --all-targets` | PASS (0 errors, 4 pre-existing warnings) |
| `cargo test --workspace` | PASS: 12,793 passed, 27 ignored, 0 failed |
| Production panic macro scan (helpers.rs lines 1-373) | PASS: zero `assert!`/`assert_eq!`/`assert_ne!`/`unreachable!` macros in production code |

### Skipped Gates

| Gate | Reason |
|------|--------|
| `cargo check --all-features` | `vb-y9d3v-flux-refinements` feature enables Flux syntax that requires `cargo-flux` toolchain — pre-existing issue |
| `cargo audit` | Not available in this environment |
| `cargo deny check` | Not available in this environment |
| `cargo geiger` | Not available in this environment |
| `cargo machete` | Not available in this environment |
| `moon ci` | `moon` CLI not available in this environment |

## Residual Risks

1. **Flux refinement specs** (`verification/flux/vb_y9d3v_action_ticket_refinements.rs`) may need updating to reflect the new future-attempt rejection behavior, but these are only compiled under `cargo flux` and are gated behind `vb-y9d3v-flux-refinements` feature.
2. **Kani proof harnesses** (`verification/kani/kani_attempt_fence_harnesses.rs`) may need their assertions updated to account for future-attempt rejection in `validate_ticket_attempt` — these are gated behind `#[cfg(kani)]`.
3. **Verus proof models** (`verification/verus/vb_y9d3v_action_fence.rs`) may need `ensures` postcondition updates — these are gated behind `#[cfg(verus)]`.
4. None of the above affect runtime behavior; they are verification-only artifacts.

## Summary

G005 gap is CLOSED. Future-attempt action completions (`ticket.attempt > current` where `current > 0`) are now rejected with `Err(RuntimeError::InvalidActionCompletion)`.
