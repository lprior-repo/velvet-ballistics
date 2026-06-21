# RS-109-life: Submit can leak a frame on errors before RunState insertion

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001_submit.rs:161`
- **Confidence**: confirmed

## Description

`handle_submit_with_inputs_contracts_and_header_mode` takes a frame before seeding inputs and appending admission header events. Any error before the frame is moved into `RunState` returns without `release_frame`, leaking frame capacity.

## Evidence

The frame is acquired before several fallible operations:

```rust
let mut frame = self.take_frame_for(run, &workflow)?;
crate::shard::helpers::seed_input_slots(&mut frame, inputs)?;
...
self.append_admission_header_journal_event(
    run,
    RuntimeJournalEvent::RunSubmitted { ... },
)?;
```

The admission event append is also fallible before `RunState` owns the frame:

```rust
self.append_admission_header_journal_event(
    run,
    RuntimeJournalEvent::RunAdmission { admission: admission.clone() },
)?;
```

The frame is not released on any of those `?` paths.

## Adversarial Check

This is not a false leak if frames were meant to be dropped normally: terminal, cancel, and kill paths all explicitly call `self.release_frame(state.frame)`. That explicit release discipline means a frame taken during submit must also be returned on pre-insertion failures.

## Suggested Fix

Delay `take_frame_for` until after fallible header persistence where possible, or wrap the acquired frame in a local guard that calls `release_frame` unless ownership is transferred into `RunState`. Ensure `seed_input_slots` and both admission header append failures release the frame.
