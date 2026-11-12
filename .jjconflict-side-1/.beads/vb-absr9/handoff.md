# Handoff: vb-absr9

## Completed

### 1. Production Clippy source quality fixes

**Fixed 3 `too_many_arguments` diagnostics:**

| Function | Location | Fix |
|----------|----------|-----|
| `issue_action_ticket` | `crates/vb_core/src/action/ticket.rs` | Introduced `NewActionTicket` param struct; function now takes 1 arg |
| `dispatch_action_and_signal` | `crates/vb_runtime/src/engine/execute/dispatch/action_signal.rs` | Introduced `ActionDispatchContext` param struct; function now takes 1 arg |
| `handle_do` | `crates/vb_runtime/src/engine/execute/handlers_suspend.rs` | Takes `ActionDispatchContext` + 3 action-specific params (4 total) |

**Files modified for production source quality:**
- `crates/vb_core/src/action/ticket.rs` — `NewActionTicket` struct + refactored `issue_action_ticket`
- `crates/vb_core/src/action.rs` — added `NewActionTicket` re-export
- `crates/vb_core/src/lib.rs` — added `NewActionTicket` re-export
- `crates/vb_core/src/action/tests.rs` — updated 3 test callers to use `NewActionTicket`
- `crates/vb_runtime/src/engine/execute/dispatch.rs` — construct `ActionDispatchContext` at call site
- `crates/vb_runtime/src/engine/execute/dispatch/action_signal.rs` — `ActionDispatchContext` struct + refactored functions
- `crates/vb_runtime/src/engine/execute/handlers_suspend.rs` — `handle_do` uses context; removed unused imports

**Affected call sites updated:**
- 3 test callers in `crates/vb_core/src/action/tests.rs`
- 1 production caller in `crates/vb_runtime/src/engine/execute/dispatch.rs`

### 2. Test fallibility fixes (vb_r4fm9_event_generation.rs)

**Removed all 6 `.expect()` calls:**

| Line | Original | Replacement |
|------|----------|-------------|
| 87 | `ActionName::new(...).expect(...)` | `.map_err(...)?` with Result return |
| 111 | `postcard::to_allocvec(...).expect(...)` | `.map_err(...)?` with Result return |
| 112 | `u32::try_from(...).expect(...)` | `.map_err(...)?` with typed error |
| 126 | `.expect("no durable retry checkpoint...")` | `.ok_or_else(...)?` with Result return |
| 130 | `postcard::from_bytes(...).expect(...)` | `.map_err(...)?` with Result return |
| 236 | `.expect("expected ActionScheduledTicket...")` | `match` with `return Err(...)` |

**Helper functions made fallible:**
- `action_contract() -> Result<ActionContract, String>`
- `action_output(value) -> Result<ActionOutputReady, String>`
- `latest_retry_checkpoint(events) -> Result<RunCheckpointV1, String>`

**Removed 1 `panic!` call:**
- Line 128: `panic!("no durable retry checkpoint in journal")` — replaced with Result propagation

## Verification

### Production Clippy strict: PASSED
```bash
env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings" \
  rustup run nightly-2026-04-28 cargo clippy --quiet --workspace --lib --bins --examples --all-features \
  -- -D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented \
  -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap \
  -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use \
  -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr
```

### Production check: PASSED
```bash
env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings" \
  rustup run nightly-2026-04-28 cargo check --quiet --workspace --lib --bins --examples --all-features
```

### Target tests: PASSED (3/3)
```bash
rustup run nightly-2026-04-28 cargo test --quiet --package vb_runtime --test vb_r4fm9_event_generation
```

## Remaining known issues (not owned by this bead)

### Fixture compilation errors (owned by LOCAL3090 vb-3gthe)
The following test compilation errors remain from the ticket migration merge. They are caused by the `compute_action_idempotency_key` signature change (now 4 args including `run_attempt`) and the `ActionTicket` struct gaining the `run_attempt` field. These affect fixtures/tests owned by other workers:

- `crates/vb_validate/tests/idempotency_contract_red.rs:626` — missing `run_attempt` field
- `crates/vb_storage/src/codec/tests.rs:29,35` — missing `run_attempt` field + 3-arg call
- `crates/vb_storage/src/journal/regression_tests_vb_1rqz7.rs:33` — missing `run_attempt` field
- `crates/vb_storage/src/journal/regression_tests_vb_xsu4g.rs:45,106` — missing `run_attempt` field
- `crates/vb_core/src/action/journal_tests.rs:9` — missing `run_attempt` field
- `crates/vb_core/src/action/tests.rs` (many lines) — missing `run_attempt` field + 3-arg calls
- `crates/vb_storage/src/index_maintenance_tests.rs:87` — missing `run_attempt` field
- `crates/vb_storage/src/recovery/replay/summary/tests.rs` (many lines) — missing `run_attempt` field + 3-arg calls
- `crates/vb_storage/src/recovery/tests.rs` (many lines) — missing `run_attempt` field + 3-arg calls
- `crates/vb_storage/src/tests.rs:2982` — missing `run_attempt` field

These will be resolved by the LOCAL3090 fixture migration work (vb-3gthe) before final CI.

### Formatting
`crates/vb_storage/src/types/flux_sequence.rs` has a pre-existing `cargo fmt` issue with `#[flux_rs::sig(...)]` attribute parsing. Not related to this bead's changes.

## API changes for merge documentation

1. **`NewActionTicket` struct** added to `crates/vb_core/src/action/ticket.rs` and re-exported via `action.rs` and `lib.rs`
2. **`issue_action_ticket(spec: NewActionTicket)`** — signature changed from 8 positional args to 1 struct arg
3. **`ActionDispatchContext` struct** added to `crates/vb_runtime/src/engine/execute/dispatch/action_signal.rs` (pub(crate))
4. **`dispatch_action_and_signal(ctx: &mut ActionDispatchContext)`** — signature changed from 8 positional args to 1 struct arg
5. **`handle_do(ctx: &mut ActionDispatchContext, action, input, node_id)`** — signature changed from 8 positional args to 4 args (context + action-specific)