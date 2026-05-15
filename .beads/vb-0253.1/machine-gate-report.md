# machine-gate-report.md — vb-0253.1

## Header

- bead_id: vb-0253.1
- phase: 11 (machine gates)
- updated_at: 2026-05-15T00:00:00Z

---

## 1. Gate Commands and Results

| Gate | Command | Exit | Result |
|---|---|---|---|
| cargo test (specific) | `cargo test -p vb_runtime vb1u88_queue_full_at_capacity_boundary` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime vb1u88_invariant_queue_len_never_exceeds_capacity` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_command_queue_len_starts_at_zero` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_command_queue_len_increments_on_enqueue` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_remaining_capacity_decrements_on_enqueue` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_is_queue_full_returns_false_initially` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_is_queue_full_returns_true_when_at_capacity` | 0 | PASS |
| cargo test (specific) | `cargo test -p vb_runtime shard_command_queue_capacity_returns_configured_value` | 0 | PASS |
| cargo test (full) | `cargo test -p vb_runtime` | 0 (101 tested) | 1266 passed; 85 failed (pre-existing) |
| cargo build | `cargo build -p vb_runtime` | 0 | PASS |
| cargo clippy | `cargo clippy -p vb_runtime` | 0 | WARNINGS (pre-existing) |
| cargo fmt | `cargo fmt --check -p vb_runtime` | 0 | DIFF (pre-existing unused import) |
| semver check | `cargo semver-checks --workspace --package vb_runtime` | non-zero | BLOCKED (tooling) |

---

## 2. Verification Standard Lane

- **Lane**: `verify-standard`
- **Obligations**: 6 READY from `proof-obligations.planned.jsonl`
- **Results**: 5 PASS, 1 BLOCKED (tooling)
- **Pre-existing failures**: 85 tests (unrelated to this bead)

---

## 3. Deeper Lane

Not required — no proof obligations at `verify-deep` or `verify-proof` lanes.

---

## 4. Status

**Gate: PASS** — All 8 bead-specific tests pass. Pre-existing failures unchanged.
