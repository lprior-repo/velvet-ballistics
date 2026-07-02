# Proof Coverage Matrix — vb-svvr7

## Bead: vb-svvr7 — IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)

Maps every contract clause (CC-TB-1..CC-TB-10) to the proof-obligation rows that discharge it, with exact evidence commands and behavior-affecting flags.

## Coverage Summary

| Contract Clause | Statement (terse) | Discharged By | Behavior-Affecting |
|---|---|---|---|
| CC-TB-1 | `Ok` implies exact length | PO-TB-PROP-01, PO-TB-UNIT-01 (regression) | false |
| CC-TB-2 | `<` length returns `DecodeFailed` | PO-TB-UNIT-01 (regression via existing `test_decode_data_too_short`, `decode_rejects_truncated_header`) | false |
| CC-TB-3 | `>` length returns `TrailingBytes` | PO-TB-PROP-01, PO-TB-UNIT-01 | false |
| CC-TB-4 | `TrailingBytes` is a unit variant | PO-TB-UNIT-01 (`test_postcard_error_variants`) | false |
| CC-TB-5 | `Display` is non-empty and distinct from `DecodeFailed` | PO-TB-UNIT-01 (`test_postcard_error_variants` covers `format!` non-empty + discriminant) | false |
| CC-TB-6 | `decode_postcard_json` propagates `TrailingBytes` | PO-TB-UNIT-01 (`decode_postcard_json_propagates_trailing_bytes`) | false |
| CC-TB-7 | `encode_postcard` emits exactly `HEADER_SIZE + p.len()` bytes | PO-TB-UNIT-01 (regression via existing `test_encode_postcard`, `test_roundtrip`) | false |
| CC-TB-8 | Encoder/decoder roundtrip | PO-TB-UNIT-01 (regression via `test_roundtrip`) | false |
| CC-TB-9 | Fix is purely additive; cross-crate parity | PO-TB-CLIPPY-01 | false |
| CC-TB-10 | Preserves INV-005 + POST-007 | PO-TB-LINT-01 (canonical CI gate covers the invariants) | false |

All obligations are `behavior_affecting: false` per bead-scope policy.

## Detailed Coverage Rows

### C1: CC-TB-1 — `Ok` implies exact length

**Source**: `crates/vb_cli/src/cli_postcard/validation.rs:71-101`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-PROP-01 | proptest | `PROPTEST_CASES=10000 cargo test -p vb_cli --test cli_postcard_properties --release strict_length_no_trailing_bytes` | `test result: ok` with 1 passed; property asserts exact `Err(PostcardError::TrailingBytes)` equality for any trailing length in `[1, 4096]`, which is the contrapositive of `Ok ⇒ exact length` | false |
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::decode_accepts_exact_length_frame` | `test result: ok`; the new `decode_accepts_exact_length_frame` test asserts `decode_postcard(&encoded).is_ok() && encoded.len() == HEADER_SIZE + payload.len()` | false |

### C2: CC-TB-2 — `<` length returns `DecodeFailed` (regression preserved)

**Source**: `crates/vb_cli/src/cli_postcard/validation.rs:72-74, 87-89`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::test_decode_data_too_short cli_postcard::tests::decode_rejects_truncated_header` | `test result: ok`; both existing tests must still pass; the tightened `!=` length check at line 87-89 still returns `DecodeFailed` for `data.len() < payload_end` | false |

### C3: CC-TB-3 — `>` length returns `TrailingBytes` (bug closure)

**Source**: `crates/vb_cli/src/cli_postcard/validation.rs:87-89`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-PROP-01 | proptest | (same as C1) | The proptest asserts `Err(PostcardError::TrailingBytes)` exactly, with `data.len() > HEADER_SIZE + payload_len` AND every earlier stage passing. This is the bug-closure property. | false |
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::decode_rejects_trailing_bytes_after_valid_frame` | `test result: ok`; the new unit test asserts `Err(PostcardError::TrailingBytes)` for valid encode + 1 trailing byte | false |

### C4: CC-TB-4 — `TrailingBytes` is a unit variant

**Source**: `crates/vb_cli/src/cli_postcard/error.rs:7-30`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::test_postcard_error_variants` | `test result: ok`; the new test asserts `std::mem::discriminant(Err(PostcardError::TrailingBytes)) != std::mem::discriminant(Err(PostcardError::DecodeFailed))` and `format!("{}", PostcardError::TrailingBytes) != ""` and `format!("{}", PostcardError::TrailingBytes) != format!("{}", PostcardError::DecodeFailed)` | false |

### C5: CC-TB-5 — `Display` non-empty + distinguishable

**Source**: `crates/vb_cli/src/cli_postcard/error.rs:32-48`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | (covered by C4 `test_postcard_error_variants`) | The same test asserts `format!()` is non-empty and distinct from `DecodeFailed`; the Display arm is `write!(f, "postcard decode failed: trailing bytes after valid frame")` | false |

### C6: CC-TB-6 — `decode_postcard_json` propagates `TrailingBytes`

**Source**: `crates/vb_cli/src/cli_postcard/codec.rs:24-34`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::decode_postcard_json_propagates_trailing_bytes` | `test result: ok`; the new test asserts that valid encode + trailing bytes yields `Err(PostcardError::TrailingBytes)` from `decode_postcard_json` (not remapped to `DecodeFailed`) | false |

### C7: CC-TB-7 — `encode_postcard` emits exactly `HEADER_SIZE + p.len()` bytes (regression)

**Source**: `crates/vb_cli/src/cli_postcard/codec.rs:46-73`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | `cargo test -p vb_cli --lib cli_postcard::tests::test_encode_postcard cli_postcard::tests::test_roundtrip` | `test result: ok`; both existing tests must still pass after the fix | false |

### C8: CC-TB-8 — Encoder/decoder roundtrip (regression)

**Source**: `crates/vb_cli/src/cli_postcard/codec.rs:46-73` + `validation.rs:71-101`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | (covered by C7) | The existing `test_roundtrip` asserts `encode_postcard(v, k, p).is_ok() && decode_postcard(&encoded).is_ok()` plus payload equality | false |

### C9: CC-TB-9 — Fix is purely additive; cross-crate parity

**Source**: `crates/vb_cli/src/cli_postcard/validation.rs:87-89` + `crates/vb_ipc/src/frame.rs:44`

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-CLIPPY-01 | cargo-clippy | `cargo clippy -p vb_cli --all-targets -- -D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::indexing_slicing -D clippy::arithmetic_side_effects -D clippy::as_conversions` | `0 warnings emitted`; the new `TrailingBytes` unit variant matches the existing unit-shaped pattern; the `!=` length check matches the `vb_ipc/src/frame.rs:44` shape; no signature change; no visibility change; no dependency change | false |

### C10: CC-TB-10 — Preserves INV-005 (bounded allocation) + POST-007 (magic + header length before payload)

**Source**: `crates/vb_cli/src/cli_postcard/validation.rs:71-78` (header validation order) + `codec.rs:46-73` (encoder capacity)

| Obligation | Verifier | Command | Evidence | Behavior |
|---|---|---|---|---|
| PO-TB-LINT-01 | source-lint | `moon run :lint-src` | `moon :lint-src exits 0`; the `panic-surface`, `ignored-fallible-results`, `unsafe-audit`, `fmt`, and `cargo clippy --workspace --all-features -- -D warnings` sub-gates all pass; existing `test_decode_payload_too_large` and `test_decode_invalid_magic` continue to pass; no panic, no unwrap, no expect, no todo, no unimplemented, no dbg, no first-party unsafe | false |

## Lane-By-Contract Coverage Map

| Lane | Contract Clauses | Obligations |
|---|---|---|
| proptest | CC-TB-1, CC-TB-3 | PO-TB-PROP-01 |
| cargo-test | CC-TB-1, CC-TB-2, CC-TB-3, CC-TB-4, CC-TB-5, CC-TB-6, CC-TB-7, CC-TB-8 | PO-TB-UNIT-01 |
| cargo-clippy | CC-TB-9 | PO-TB-CLIPPY-01 |
| source-lint | CC-TB-10 | PO-TB-LINT-01 |

Every contract clause is covered by at least one required obligation. Every required obligation is paired with a `verifier-lane-decision/v1` row in `verifier-lane-decisions.jsonl`. The proptest obligation is the only one that is unique to a specific (req, cc, seed) tuple (PS-TB-01 / CC-TB-1 / REQ-TB-STRICT-LENGTH); the other three obligations aggregate multiple seeds and clauses (visible in the verifier-lane-decisions.jsonl cross-references).

## Defense Depth

| Risk class | Required lanes (matrix) | This plan |
|---|---|---|
| `rejection` | kani + proptest (per references/defense-depth-matrix.md) | proptest (primary) + cargo-test (unit companion) — kani not_applicable with evidence, so the kani row is replaced by cargo-test for the unit-shape discharge |

The `rejection` risk class is satisfied with 2 distinct lanes (proptest for property pressure, cargo-test for unit shape), matching the minimum defense depth of 2 distinct verifiers per risk class per `references/defense-depth-matrix.md`. The 4 required obligations map to 4 distinct verifier lanes (proptest, cargo-test, cargo-clippy, source-lint) and 4 distinct primary targets (the proptest target, the unit tests module, the cargo-clippy lib target, and the source-lint workspace target). The defense depth is at least 2 across all clauses, which is the minimum per the matrix; for parser claims the minimum is `cargo-fuzz + kani + proptest + verus` but the plan is a 4-lane minimal plan and the 4 absent lanes each carry typed `not_applicable` evidence.

## Self-Audit

- [x] Every contract clause CC-TB-1..CC-TB-10 is covered by at least one obligation.
- [x] Every obligation's `target` parses as `path::symbol` (`vb_cli::cli_postcard::decode_postcard`).
- [x] Every obligation's `verifier` matches the paired `verifier-lane-decision/v1` row's `verifier`.
- [x] Every obligation's `requirement_id` and `contract_clause` match a row in `proof-seeds.jsonl` and `contract.md`.
- [x] No `behavior_affecting: true` rows in `proof-obligations.planned.jsonl` (per user policy).
- [x] No `behavior_affecting: true` rows in `waiver-candidates.jsonl` (per schema and user policy).
- [x] No obligation uses `kani::cover!` as the sole property evidence.
- [x] No obligation uses Verus (so no `external_body` / `assume` / `axiom` risk).
- [x] No obligation uses Loom, Miri, Flux, or cargo-fuzz.
