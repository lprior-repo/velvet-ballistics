# Trusted Base Plan: vb-vzo9b

## Plan Status

This bead is a TEST-ONLY repair. The fuzz body at `fuzz/src/journal_target/readback.rs:196` is replaced with a plain Rust `assert_eq!(RecoveryRuntimeSummary, RecoveryRuntimeSummary)` over a struct that already derives `Debug, Clone, Copy, PartialEq, Eq` at `crates/vb_storage/src/recovery/types.rs:546`. No new types, no new error variants, no `unsafe`, no `unwrap`/`expect`/`panic` outside the desired `assert_eq!` panic. Production code is read-only context (contract C-5).

Therefore **no trust markers are introduced by this bead**. The rows below are structural notes about pre-existing components the plan depends on; they do not raise `trusted-base-ledger/v1` rows because they are not obligation-driven assumption shapes.

| ID | Note | Marker Kind | Reason | Compensating Evidence |
|----|------|-------------|--------|----------------------|
| TB-NOTE-001 | `RecoveryRuntimeSummary` derives `Debug, Clone, Copy, PartialEq, Eq` | structural (not obligation-driven) | The new `assert_eq!` requires PartialEq+Eq; Copy allows the literal `expected_recovery_runtime_summary` to be constructed on the stack without cloning; Debug provides panic-formatting via the standard `assert_eq!` macro expansion. | `crates/vb_storage/src/recovery/types.rs:546` (derive list). Verified by `cargo build -p fuzz --bin recovery_decode` (PO-003): if the derive set were insufficient, the build would fail. |
| TB-NOTE-002 | Production `summarize_recovery_events` derives the 11 fields deterministically from a single `RunAccepted` event with `seq = EventSeq::new(1)` | structural (not obligation-driven) | The contract C-1 expected value is constructed by hand from the local `digest`, `run`, `seq` and the production constants (workflow = Some(digest), all counters = 0, terminal = None). | `crates/vb_storage/src/recovery/replay/summary/apply.rs:88-129` (build-and-loop derivation). Verified by `cargo test -p vb_storage --lib summarize_recovery_events` (PO-001): the existing recovery_unit_tests pin the same derivation against the same input shape. |
| TB-NOTE-003 | `assert_typed_recovery_error` enumerates the `RecoveryError` variants exhaustively | structural (not obligation-driven) | Both the empty-events branch of `fuzz_recovery_decode` and the frame-seed error sink rely on `assert_typed_recovery_error` to fail-closed on unexpected variants. The rewrite does not change this call site. | `fuzz/src/journal_target/errors.rs:57-72` (exhaustive match). Verified by `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` (PO-002): the existing tests exercise the empty-events `RecoveryError::NoRecoveryData { run: RunId::new(0) }` variant. |
| TB-NOTE-004 | No `unsafe` keyword in any touched file | structural (not obligation-driven) | All Holzman Rust rules forbid `unsafe`; the fuzz harness and the production recovery surface it calls are 100% safe Rust. | `fuzz/src/journal_target/readback.rs` (zero `unsafe`); `crates/vb_storage/src/recovery/replay/summary/apply.rs` (zero `unsafe`); `crates/vb_storage/src/recovery/types.rs` (zero `unsafe`). Verified by the `rg '\\bunsafe\\b'` exit code captured as part of PO-003's forbidden-pattern gate. |

## Trusted Base Summary

- **Total entries**: 4 structural notes (no obligation-driven trust markers)
- **Behavior-affecting**: 0 (all entries describe pre-existing structural facts; no production behavior is trusted away)
- **Review state**: planned (owner_state 4)

## Validation Plan

At State 6 (proof-reviewer), each structural note is verified by inspecting the corresponding source artifact referenced in the `Compensating Evidence` column. None of these notes requires a `trusted-base-ledger/v1` row because they are not `assume`, `axiom`, `admit`, `external_body`, `#[trusted]`, `#[ignore]`, `extern_spec`, `opaque`, stub, disabled check, or model reduction markers.

## Cross-Reference

- `proof-obligations.planned.jsonl` PO-001, PO-002, PO-003 — the only obligations; none carries `assumptions` that need a `trusted_base_refs` entry, because all assumptions are property-of-test-surface facts that the proof-writer does not introduce as new markers.
- `waiver-candidates.jsonl` — single structural placeholder row (no behavior-affecting waiver).
- `proof-strategy.md` "Trust Markers" section — confirms no new markers.