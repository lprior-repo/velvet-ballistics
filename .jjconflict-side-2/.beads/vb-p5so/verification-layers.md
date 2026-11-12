bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Verification Layers

## Layer Assignment by Contract Clause

### Preconditions (P1-P3)
- **Layer**: Unit tests + Type system
- **Tool**: `cargo test` + Rust compiler
- **Rationale**: Preconditions are enforced by Rust's ownership and the existing Shard construction invariants.

### Postconditions (PO1-PO4)
- **Layer**: Unit tests + Integration tests
- **Tool**: `cargo test -p vb_runtime shard`
- **Rationale**: Direct observable state assertions on `pending_timers` after `drain_for_shutdown`.

### Invariants (I1-I4)
- **Layer**: Unit tests + Property tests
- **Tool**: `cargo test -p vb_runtime shard` + proptest (if applicable)
- **Rationale**: Invariants hold across all shard operations; tests must verify them before and after `drain_for_shutdown`.

### Error Taxonomy
- **Layer**: Error-path unit tests
- **Tool**: `cargo test -p vb_runtime shard`
- **Rationale**: Verify `ShutdownInProgress` is returned correctly and state is unchanged.

## Defense-in-Depth Summary

| Concern | Primary Verification | Compensating Control |
|---|---|---|
| Timer leak on shutdown | Unit test: assert `pending_timers.is_empty()` | Code review: single `.clear()` call |
| Idempotency | Unit test: double `drain_for_shutdown` | No state mutation on second call (shutting_down already true) |
| Capacity limit edge | Unit test: full queue without shutdown | Return `Err` without side effects |
| Run without backing timer | Unit test: run in `runs` but not in `pending_timers` | `IndexMap::clear()` is total |
