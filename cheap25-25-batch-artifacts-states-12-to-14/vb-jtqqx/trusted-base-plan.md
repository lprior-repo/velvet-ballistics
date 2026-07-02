# Trusted Base Plan — vb-jtqqx

## Bead Scope

- **In-scope surface** (modified by the repair):
  `crates/workspace_tests/tests/journal_side_index_contracts.rs` lines
  14 (`#![forbid(unsafe_code)]`), 23 (`JOURNAL_KEY_PROPTEST_CASES =
  128`), and 183-257 (the PO-008 proptest block).
- **Out-of-scope surface** (read-only, trusted as-is):
  - `crates/vb_storage/src/keys.rs:1` (`#![forbid(unsafe_code)]`),
    `:281-295` (`try_key_prefix`), `:346-434` (`decode_storage_key`).
  - `crates/vb_storage/src/constants.rs:38-43` (prefix bytes
    `PREFIX_INDEX_STATUS=0x30`, `PREFIX_INDEX_WORKFLOW=0x31`,
    `PREFIX_INDEX_ACTION=0x32`), `:77-79` (length envelopes
    `INDEX_STATUS_KEY_BYTES=18`, `INDEX_WORKFLOW_KEY_BYTES=13`,
    `INDEX_ACTION_KEY_BYTES=13`).
  - `crates/vb_storage/src/error/key_decode.rs:1` (`#![forbid(unsafe_code)]`),
    `:8-31` (`KeyDecodeError` enum, `#[non_exhaustive]`).
  - `crates/vb_storage/src/lib.rs:202` (`KeyDecodeError` re-export).

## Trusted Surfaces

### 1. Rust Standard Library (trusted)

| Item | Trusted because |
|---|---|
| `proptest::proptest!` macro and `prop_assert!` family | proptest@1.5; standard property-testing framework; failures surface as proptest-shaped `TestCaseError`, not panics. |
| `proptest::prelude::*` (`any`, `prop_assert`, `prop_assert_eq`, `prop_assert!`) | Same as above. |
| `std::vec::Vec`, slice indexing with `&bytes[..n]` for `n <= bytes.len()` | Safe Rust; the `if n < valid_len` guard prevents underflow per `SIDEX-MAL-014 / H-MAL-009`. |

**Justification**: Standard library + proptest crate; both are well-tested
and are the canonical property-testing surface in the workspace. No
`unsafe` in scope.

### 2. Decoder Surface (trusted, read-only)

| Item | Trusted because |
|---|---|
| `vb_storage::keys::try_key_prefix(bytes: &[u8]) -> Result<KeyPrefix, KeyDecodeError>` | Pure `match`-based function at `keys.rs:281-295`. No loops, no unsafe, no indexing that can panic. |
| `vb_storage::keys::decode_storage_key(bytes: &[u8]) -> Result<StorageKey, KeyDecodeError>` | Pure `match`-based function at `keys.rs:346-434`. The `key_array::<N>` helper at `:305-314` uses `<[u8; N]>::try_from(slice)` and falls back to `KeyLengthMismatch` on failure — panic-free. |
| `vb_storage::KeyDecodeError` (re-exported at `lib.rs:202`) | `#[non_exhaustive]` enum at `error/key_decode.rs:8-31`. New variants may be added in the future; the test bodies' `matches!(..., Err(KeyDecodeError::Variant { .. }))` patterns are forward-compatible. |

**Justification**: The decoder is read-only in this P1 and is the
contract the tests verify. Per `delivery-scope.jsonl:2` the decoder is
marked `decoder_unchanged_read_only`. The Holzman-Rust zero-tolerance
lint (`#![forbid(unsafe_code)]` at the file scope) prevents unsafe
introductions.

### 3. Constants (trusted, read-only)

| Constant | Value | Trusted because |
|---|---|---|
| `PREFIX_INDEX_STATUS` | `0x30` | `pub(crate)` at `constants.rs:39`; used internally; the test file uses literal `0x30` with a comment citing `constants.rs:38-43` (per `type-contracts.md#Public API Compatibility Notes`). |
| `PREFIX_INDEX_WORKFLOW` | `0x31` | `pub(crate)` at `constants.rs:41`; same as above. |
| `PREFIX_INDEX_ACTION` | `0x32` | `pub(crate)` at `constants.rs:43`; same as above. |
| `INDEX_STATUS_KEY_BYTES` | `18` | `pub(crate)` at `constants.rs:77`; same as above. |
| `INDEX_WORKFLOW_KEY_BYTES` | `13` | `pub(crate)` at `constants.rs:78`; same as above. |
| `INDEX_ACTION_KEY_BYTES` | `13` | `pub(crate)` at `constants.rs:79`; same as above. |

**Justification**: The constants are `pub(crate)` and not visible to
the test file. The repair uses literal values with a citation comment
to make future drift detectable (per `PS-MAL-010 / H-MAL-007`). A
future bead may widen visibility; the P1 deliberately avoids that to
keep the repair bounded to one test file.

### 4. Canonical Fixtures (trusted, read-only)

| Fixture | Location | Trusted because |
|---|---|---|
| `preview_keyspace_skips_malformed` | `crates/vb_storage/src/preview/tests.rs:70-109` | Reference pattern for "build malformed bytes → assert typed error". Passes today. |
| `preview_keyspace_fails_closed` | `crates/vb_storage/src/preview/tests.rs:111-150` | Same as above; uses `KeyspaceScanPolicy::default_production()` and asserts on `JournalError::MalformedKeyspaceRow { prefix: 0x10, expected_len: 9, actual_len: 4 }`. |
| `preview_keyspace_fail_closed_unknown_prefix` | `crates/vb_storage/src/preview/tests.rs:152-180` | Same as above; asserts on `MalformedKeyspaceRow { prefix: 0xFF, expected_len: 0, actual_len: 4 }`. |
| `cc002_run_headers_fails_closed_on_malformed_key` | `crates/vb_storage/src/tests.rs:1862-1904` | Behaviourally-correct exemplar: plants `vec![PREFIX_RUN_HEADER, 0xAB, 0xCD]` (3 bytes, expected 9) directly into the `run_header` partition and asserts the typed error path. |
| Encode-only unit tests in `keys/tests.rs` | `crates/vb_storage/src/keys/tests.rs:20-287` | Encode-side coverage; not malformed-decode coverage. Trusted as a reference for the encoder contract. |

**Justification**: These fixtures demonstrate the
"build-malformed-bytes → assert-typed-error" pattern that the PO-008
repair must mirror. They are not modified by this P1
(`fixture_only_do_not_modify` per `delivery-scope.jsonl:6`).

### 5. Type Invariants (trusted, enforced by type system)

| Invariant | Trusted because |
|---|---|
| `KeyDecodeError` is `#[non_exhaustive]` | Adding a new variant in the future will not break the test bodies' `matches!(..., Err(KeyDecodeError::Variant { .. }))` patterns. |
| `try_key_prefix` returns `Result<KeyPrefix, KeyDecodeError>` | Total function for all `&[u8]` inputs; no panic path. |
| `decode_storage_key` returns `Result<StorageKey, KeyDecodeError>` | Total function for all `&[u8]` inputs; uses `bytes.get(range).ok_or_else(...)` to prevent indexing panic. |
| `proptest::ProptestConfig { cases, failure_persistence: None, .. }` | Local helper at `journal_side_index_contracts.rs:25-31`; the `None` failure persistence prevents stale-failure artifacts between runs. |

### 6. Build / Tooling (trusted)

| Tool | Trusted because |
|---|---|
| `cargo nextest run` | Canonical test runner in the workspace (per `AGENTS.md` Build And CI: "moon ci is canonical. Prefer moon ci over ad-hoc Cargo gates"; nextest is the per-test runner). |
| `cargo clippy` with `-D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` | Holzman-Rust zero-tolerance gate; zero warnings required. |
| `proptest@1.5` | Pinned in workspace `Cargo.lock`; standard property-testing framework. |
| `rust-toolchain.toml` pinned nightly | `docs/rust-governance.md` governance; the repair must not introduce features outside the whitelist. |

## Model Reductions and Assumptions

### Proptest Budget (per-test)

- `JOURNAL_KEY_PROPTEST_CASES = 128` at `journal_side_index_contracts.rs:23`.
- This is the canonical budget for all three PO-008 proptests.
- The repair preserves this constant verbatim (per `SIDEX-MAL-004 /
  PS-MAL-017`). Any drift in the budget would be observable in the
  `cargo nextest run` output.

### Strategy Bounds

| Strategy | Bound | Justification |
|---|---|---|
| `truncate_len in 1u8..=12u8` (action) | `[1, 12]` | Truncated length `[1, 12]` ∈ `[1, 13)`; the assertion maps to `KeyLengthMismatch` not `EmptyKey` (per `SIDEX-MAL-014 / H-MAL-009 / PS-MAL-012`). |
| `truncate_len in 1u8..=12u8` (workflow, new) | `[1, 12]` | Same as above. |
| `truncate_len in 1u8..=17u8` (status, new) | `[1, 17]` | Truncated length `[1, 17]` ∈ `[1, 18)`; same logic. |
| `extra_bytes in 0u8..=10u8` (status, workflow) | `[0, 10]` | Oversize length `[18, 28]` (status) or `[13, 23]` (workflow); the assertion maps to `KeyLengthMismatch` (per `SIDEX-MAL-015 / PS-MAL-003`). |
| `action_val in 1u16..=100u16` | `[1, 100]` | `action != 0` is the valid range; preserved from the existing strategy. |
| `run_val in 1u64..=1000u64` | `[1, 1000]` | `run != 0` is the valid range; preserved from the existing strategy. The zero-run payload is built as a separate literal `vec![0x32, 0x00, 0x01, 0x00, 0x00, ...]` per the type-contracts.md recipe, not via the strategy. |
| `step_val in 0u16..=50u16` | `[0, 50]` | Valid range; preserved from the existing strategy. |
| `state in 0u8..=2u8` | `[0, 2]` | `IndexStatusState::from_u8` accepts `0..=2`; preserved. |
| `timestamp in 0u64..=1000u64` | `[0, 1000]` | Valid range; preserved. |
| `workflow_val in 1u32..=100u32` | `[1, 100]` | `WorkflowId::new(workflow)` accepts any `u32`; preserved. |

### Decoder Length Envelopes (read-only)

| Variant | Prefix | Length envelope | Source |
|---|---|---|---|
| `IndexStatus` | `0x30` | `18` | `constants.rs:39, 77` |
| `IndexWorkflow` | `0x31` | `13` | `constants.rs:41, 78` |
| `IndexAction` | `0x32` | `13` | `constants.rs:43, 79` |

The decoder is trusted to enforce these length envelopes at
`keys.rs:349-355` (the `bytes.len() != expected_len` check). The test
bodies' `KeyLengthMismatch` assertions field-check `prefix`,
`expected`, and `actual` against this table.

### Holzman-Rust Zero-Tolerance Surface

- **No `unwrap` / `expect` / `panic` / `todo` / `unimplemented` /
  `dbg!`** in the PO-008 block (per `SIDEX-MAL-006 / PS-MAL-007 /
  AGENTS.md Engineering Rules`).
- **No `unsafe`** anywhere in the test file
  (`#![forbid(unsafe_code)]` at line 14).
- **No unchecked indexing**: all slice accesses are guarded by `if n <
  valid_len` (per the existing pattern at `journal_side_index_contracts.rs:212`).
- **No `KEYCAPACITY` references**: `KeyCapacity` is the encoder-side
  error; the PO-008 block uses `KeyDecodeError` (per `SIDEX-MAL-017 /
  H-MAL-002 / PS-MAL-005`).

## Known Assumptions and Stub Boundaries

| Assumption | Location | Justification |
|---|---|---|
| `try_key_prefix` is panic-free for all `&[u8]` | `keys.rs:281-295` | Pure `match` on `bytes.first()`; `first()` returns `None` for empty slices, handled via `?`. |
| `decode_storage_key` is panic-free for all `&[u8]` | `keys.rs:346-434` | Pure `match` on prefix; `key_array` uses `bytes.get(range).ok_or_else(...)` and `<[u8; N]>::try_from(slice).map_err(...)` — no panic. |
| `KeyDecodeError` variants are stable | `error/key_decode.rs:8-31` | `#[non_exhaustive]` enum; future variants allowed but not present today. |
| `IndexStatusState::from_u8` accepts `0..=2` | `keys.rs:397` | Existing pattern; the `state in 0u8..=2u8` strategy is preserved. |
| proptest's `prop_assert!` is non-panicking on `false` | proptest@1.5 | Standard proptest contract; failures surface as `TestCaseError`. |
| `failure_persistence: None` | `journal_side_index_contracts.rs:28` | Local helper; prevents stale-failure artifacts. |

## Non-Behavior Waivers (none)

No waivers requested. The not-applicable verifier lanes
(`verus`, `kani`, `flux-rs`, `loom`, `miri`, `cargo-fuzz`) are
enumerated in `waiver-candidates.jsonl` as `not_applicable` lane
decisions, not as behavior-affecting waivers. No obligation is
waived; every default-profile verifier is either paired with a
`proof-obligation/v1` row or explicitly marked `not_applicable` with
concrete `non_applicability_evidence_refs` and a `limitation_kind`.

## Reduction Justification

The proptest model reduces the infinite space of malformed byte
sequences to a finite budget of 128 randomized cases per proptest.
This is justified because:

1. **The decoder is a pure `match`**: no loops, no recursion, no
   arithmetic overflow risk. 128 random cases over a 13- or 18-byte
   input space is statistically sufficient to cover the structural
   error branches.
2. **The decoder length envelopes are exact**: `bytes.len() !=
   expected_len` is a deterministic check. The proptest's
   `truncate_len` and `extra_bytes` strategies cover the
   truncated/oversize length space at the boundaries (`[1, 12]` and
   `[18, 28]`).
3. **The decoder prefix check is exhaustive**: `try_key_prefix` is a
   `match` on the first byte against nine known prefixes
   (`0x01, 0x02, 0x10, 0x11, 0x12, 0x20, 0x30, 0x31, 0x32`). Any other
   byte maps to `UnknownPrefix`. The proptest covers both the
   known-prefix cases (via the encoder) and the unknown-prefix case
   (`vec![0xFF; L]`).
4. **The decoder `InvalidRunId` check is exact**: `run_val == 0` is
   a deterministic branch. The proptest covers the `run == 0` case
   via a literal zero-run payload; the `run != 0` case is the
   encode-only roundtrip and is not in PO-008 scope.

The Kani exhaustiveness that would prove the decoder is panic-free
across all 2^192 (or larger) byte sequences is captured in
`proof-seeds.jsonl:19` as `PS-MAL-019` (future Kani scope-up) and
flagged in `codebase-map.md` as out-of-scope for this P1.

## Trusted-Base Change Tracking

This plan is at State 4 (planning). The trusted-base surface does not
change between State 4 and State 12 (closure) because:

- The decoder is read-only; no production source change.
- The constants are read-only; the literal `0x30/0x31/0x32/18/13`
  values used in the test bodies match the constants verbatim.
- The fixtures are read-only; the repair mirrors their pattern but
  does not modify them.
- The proptest budget (`JOURNAL_KEY_PROPTEST_CASES = 128`) is
  preserved at the constant definition; the repair does not raise or
  lower it.
- The Holzman-Rust zero-tolerance surface is preserved.

If a future bead modifies any of these surfaces, this
`trusted-base-plan.md` MUST be re-issued before the next planning
cycle.
