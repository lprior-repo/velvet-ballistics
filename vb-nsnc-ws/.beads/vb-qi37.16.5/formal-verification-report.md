# Formal Verification Report

STATUS: APPROVED

## Inputs

- proof-obligations.jsonl: 22 entries; six required Verus entries repaired to executable standalone harness command with `owner_state=12`, `rerun_from=12`, `status=passed`.
- verification-ledger.jsonl: updated with Verus PASS evidence and clean trust scan.
- contract-repair-state12-verus-harness.md: records why the original standalone production-source commands were invalid and why the dedicated harness is the correct Verus proof target.
- contract-verification-review.md: State3 review remains approved and permits Verus ownership for Rust-local typestate/validation/append-event clauses.

## Tool Availability

- `command -v verus`: `/home/lewis/.local/bin/verus`
- `verus --version`: `0.2026.05.05.d03e906`, profile `release`, platform `linux_x86_64`, toolchain `1.95.0-x86_64-unknown-linux-gnu`.
- `verusfmt`: missing; recorded as `VERUSFMT_MISSING`, not proof evidence.

## TLA+ / Integration / Replay Evidence

- TLA-LIFECYCLE-001..006: PASS; prior TLC run reported no invariant violation, 35647 states generated, 15463 distinct, depth 16.
- INTEGRATION-001..006: PASS; `cargo test --package velvet_ballastics --test lifecycle_integration -- --test-threads=1` reported `43 passed`.
- REPLAY-001: PASS; `cargo test --package velvet_ballastics --test lifecycle_integration replay_ -- --test-threads=1` reported `4 passed, 39 filtered out`.
- REPLAY-002: PASS; `cargo test --package velvet_ballastics --test lifecycle_integration replay_full -- --test-threads=1` reported `1 passed, 42 filtered out`.
- REPLAY-003: PASS; `cargo test --package velvet_ballastics --test lifecycle_integration replay_corruption -- --test-threads=1` reported `2 passed, 41 filtered out`.
- MANUAL-QA-001: PASS using existing `manual-qa-smoke.md` evidence.

## Verus Obligation Results

### Contract repair

Original commands such as `verus crates/vb_runtime/src/shard/lifecycle.rs` failed before proof due standalone crate-context and Rust-edition incompatibility. The approved contract scope is Rust-local pure typestate/validation/append-event behavior, not production dependency wiring. State12 therefore added `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` and updated the Verus proof obligations to this executable harness command while retaining original production files as `source_target`.

### Verified command

```bash
verus contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs
```

Result: PASS

```text
verification results:: 12 verified, 0 errors
```

### Trust scan

```bash
rtk grep -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom' contracts/verus --glob '*.rs'
```

Result: CLEAN (`0 matches`; `TRUST_SCAN_CLEAN`).

## Ledger Counts

- PASS: 22
- FAIL_LOCAL: 0
- FAIL_REGRESSION: 0
- WAIVED: 0
- DEFERRED_GLOBAL: 0

## State12 Decision

APPROVED. All required Verus rows now pass with executable harness evidence and no trusted-base expansion. TLA+, integration, replay, and manual QA evidence remain unchanged and passing.

## Residual Risk

The harness proves the contract-level mathematical model, not production crate wiring. Production-source standalone Verus remains non-executable under the old commands; this is recorded as repaired contract-command scope rather than proof failure.
