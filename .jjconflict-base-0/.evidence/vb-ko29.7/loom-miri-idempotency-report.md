# vb-ko29.7 Loom/Miri Idempotency Report

## Scope

- `LOOM-IDEMPOTENCY-001`: same-scope retry admission/completion collision interleavings for `IdempotencyTracker` through Loom local sync indirection.
- `LOOM-IDEMPOTENCY-002`: capacity-one eviction versus duplicate/conflicting completion interleavings for `IdempotencyTracker` through Loom local sync indirection.
- `MIRI-IDEMPOTENCY-001`: representative safe idempotency tracker retry/duplicate/eviction data-structure path under Miri.
- `PO-007` repair note: existing `timer_fired_cancel` Loom model used removed builder chain methods; changed to current `loom::model` entrypoint so scoped Loom compilation can proceed.

## Results

| Obligation | Classification | Evidence |
| --- | --- | --- |
| `LOOM-IDEMPOTENCY-001` | PASS | `.evidence/vb-ko29.7/loom-idempotency.log`, exit `0` |
| `LOOM-IDEMPOTENCY-002` | PASS | `.evidence/vb-ko29.7/loom-idempotency.log`, exit `0` |
| `MIRI-IDEMPOTENCY-001` | PASS | `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log`, exit `0` |
| default nightly Miri attempt | SUPERSEDED_TOOLING_NOTE | `.evidence/vb-ko29.7/miri-idempotency.log`, exit `1`; discovery exit `1` |
| alternate Miri toolchain version | PASS | `.evidence/vb-ko29.7/miri-alt-20260404-version.log`, exit `0` |
| touched-file formatting | PASS | `.evidence/vb-ko29.7/rustfmt-touched-check.log`, exit `0` |

## Commands

```text
RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime --lib models::loom::idempotency_retry_eviction -- --nocapture
exit: 0
```

```text
cargo +nightly-2026-04-04 miri test -p vb_runtime --test vb_ko29_7_idempotency_miri -- --nocapture
exit: 0
classification: PASS
```

```text
rustup run nightly-2026-04-04 rustc --version --verbose && cargo +nightly-2026-04-04 miri --version
exit: 0
classification: PASS / toolchain-version evidence
```

```text
cargo +nightly miri test -p vb_runtime --test vb_ko29_7_idempotency_miri -- --nocapture
exit: 1
classification: SUPERSEDED_TOOLING_NOTE; default nightly rust-src layout remains broken, but alternate installed nightly completed the scoped Miri obligation
```

```text
rustup which --toolchain nightly rustc; rustup component list --installed --toolchain nightly; rustup component add rust-src --toolchain nightly; test -d "$(rustup run nightly rustc --print sysroot)/lib/rustlib/src/rust/library"
exit: 1
classification: SUPERSEDED_TOOLING_NOTE discovery
```

```text
rustfmt --edition 2024 --check crates/vb_runtime/src/models/loom/idempotency_retry_eviction.rs crates/vb_runtime/tests/vb_ko29_7_idempotency_miri.rs crates/vb_runtime/src/models/loom/timer_fired_cancel.rs
exit: 0
```

## Assumptions and Bounds

- Loom models use bounded `loom::model::Builder` settings: `max_branches = 1000`, `preemption_bound = Some(3)`.
- The tracker is modeled behind `crate::models::sync::sync::Mutex`; `IdempotencyTracker` itself remains synchronous/non-concurrent.
- The eviction/conflict model intentionally records the volatile tracker gap: after capacity-one eviction, a stale old key can be accepted locally. The model asserts capacity and explicit outcome only; durable journal fallback remains the authority for stale-key rejection beyond volatile capacity.
- No production behavior was changed.

## Blockers

- No remaining blocker for `MIRI-IDEMPOTENCY-001`: the alternate installed toolchain `nightly-2026-04-04` ran the scoped Miri test successfully.
- Superseded tooling note: default `+nightly` Miri/rust-src remains inconsistent. Raw default-nightly output reports: `fatal error: given Rust source directory /home/lewis/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library does not exist.` Discovery confirms `rust-src` is listed installed, but the expected source directory test exits `1`. This is retained as environment/tooling evidence only, not the final Miri classification.
