# Rust Contract: vb-absr9

## Objective
Fix strict source Clippy diagnostics introduced by the ticket-generation source merge, and remove all `.expect()`/`.unwrap()` calls from the new event-generation test harness.

## Changes

### 1. `issue_action_ticket` 8-argument refactoring (clippy::too_many_arguments)

**Problem:** `issue_action_ticket` takes 8 positional arguments, exceeding Clippy's 7-argument threshold under `-D warnings`.

**Solution:** Introduce a typed parameter struct `NewActionTicket` that groups all 8 fields into a single constructor parameter. This is a legitimate type/domain grouping: all fields collectively specify a single action ticket instance.

**New API:**
```rust
pub struct NewActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,
    pub action: ActionId,
    pub run_attempt: u16,
    pub attempt: u16,
    pub idempotency_key: u128,
    pub capacity: u16,
}

pub fn issue_action_ticket(spec: NewActionTicket) -> ActionTicket;
```

**Semantics preserved:**
- Function is pure; returns the ticket struct verbatim from the spec.
- No computation or validation is added.
- All fields are public for explicit, named construction.

**Affected call sites (3):**
- `crates/vb_core/src/action/tests.rs::issue_action_ticket_captures_all_fields`
- `crates/vb_core/src/action/tests.rs::issue_action_ticket_with_zero_key_is_valid`
- `crates/vb_core/src/action/tests.rs::issue_action_ticket_with_max_values`

All updated to use struct literal syntax with explicit field names.

### 2. Event generation test fallibility (`vb_r4fm9_event_generation.rs`)

**Problem:** 6 `.expect()` calls introduced by the event-generation test harness violate the no-`expect` policy.

**Solution:** Replace with fallible `Result` helpers that propagate errors via `?` into typed test error values. Assertions remain exact.

**Affected test file:**
- `crates/vb_runtime/tests/vb_r4fm9_event_generation.rs`

**Changes:**
- 6 `.expect()` calls removed
- Replaced with typed error propagation using helper functions
- All 6 original test assertions preserved exactly

## Verification Commands

```bash
# Source Clippy strict (pinned nightly)
env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings" \
  rustup run nightly-2026-04-28 cargo clippy --quiet --workspace --lib --bins --examples --all-features \
  -- -D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented \
  -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap \
  -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use \
  -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr

# Production check
env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings" \
  rustup run nightly-2026-04-28 cargo check --quiet --workspace --all-targets --all-features

# Formatting
rustup run nightly-2026-04-28 cargo fmt --all --check
```