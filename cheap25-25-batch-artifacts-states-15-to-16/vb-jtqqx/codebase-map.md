# Codebase Map: vb-jtqqx — Side-index malformed-key tests actually decode malformed keys (P1)

## Workspace and input gate
- Isolated workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx` (verified with `pwd -P`).
- jj workspace root: same path (`jj root` returned `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx`).
- jj working-copy commit: `5c31d163a53e` on workspace `cheap25-vb-jtqqx` (parent commit `rsvywymk 1d6c017f` — AGENTS.md round10 forward-port; upstream main `2c8ea33c9`).
- Source checkout `/home/lewis/src/velvet-ballistics` is coord-only and not edited.
- Inputs read: `.beads/vb-jtqqx/STATE.md`, `baseline-report.md`, `global-readiness-report.md`, `transcript-state1.txt`, `agent-invocation-ledger.jsonl`, and the prior-bead doc reference (the test file header credits vb-3h3k).

## Bead target summary
The PO-008 ("Malformed index keys") proptests in `crates/workspace_tests/tests/journal_side_index_contracts.rs` claim to verify that the side-index decoder rejects malformed keys, but every assertion currently runs against the **valid** encoding produced by the builder (length-only). The actual "truncated/extra-byte/unknown-prefix" byte sequences are constructed and then **discarded** without ever being passed to the decoder. This bead is a test-only repair: replace the no-op assertions with real `decode_storage_key` calls against crafted malformed byte sequences so the decoder's typed-error paths (`KeyDecodeError::{EmptyKey, UnknownPrefix, KeyLengthMismatch, InvalidRunId, ReservedSeqSentinel}` → in keyspace context `JournalError::MalformedKeyspaceRow`) are exercised.

## Files and symbols mapped

### Primary scope (test file to repair)
- `crates/workspace_tests/tests/journal_side_index_contracts.rs`:
  - Lines 183-257 — PO-008 proptest block. Three named tests are the bug site:
    - `index_action_key_decode_error_on_short_input` (lines 195-218) — declares `truncate_len in 1u8..=12u8`, computes `&valid_key[..(valid_len - truncate_len)]`, binds it to `_short_key` (leading underscore = unused), and asserts only `valid_len == 13`. The truncated slice is never decoded. **No decode call exists in the body.**
    - `index_status_key_decode_error_on_wrong_length` (lines 222-238) — generates `_extra_bytes in 0u8..=10u8` and uses it nowhere; the body only asserts `valid_key.len() == 18` and `valid_key.len() >= 18`. **No extra-byte or wrong-length decode call exists.**
    - `index_workflow_key_decode_error_on_wrong_length` (lines 242-256) — same pattern: `_extra_bytes in 0u8..=10u8` is generated and never used; body only asserts `valid_key.len() == 13`. **No decode call exists.**
  - Lines 1-13 — file-level header and trusted-base preamble (TB-001..TB-004) which mention `index_*_key()` "bounded, panic-free"; the malformed-key tests currently give **zero behavioural coverage** of that claim.
  - Lines 14-31 — file-level `#![forbid(unsafe_code)]`, imports (`vb_storage::{EventSeq, FjallJournal, IndexStatusState, JournalError, JournalEvent, JournalWriteBatch}` plus `vb_storage::keys` brought in via `vb_storage::keys::index_action_key`/`index_status_key`/`index_workflow_key`).
  - Lines 22-31 — `JOURNAL_KEY_PROPTEST_CASES = 128` proptest budget already configured (no need to retune for the repair).

### Decoder under test (must remain stable; read-only mapping)
- `crates/vb_storage/src/keys.rs`:
  - Lines 281-295 — `pub fn try_key_prefix(bytes: &[u8]) -> Result<KeyPrefix, KeyDecodeError>`. Returns `EmptyKey` for `[]`, `UnknownPrefix { prefix }` for any byte not in the nine known prefixes. This is the entry point that the repaired tests should call for the empty/unknown-prefix cases.
  - Lines 346-434 — `pub fn decode_storage_key(bytes: &[u8]) -> Result<StorageKey, KeyDecodeError>`. Length-check happens at lines 349-355 (returns `KeyLengthMismatch { prefix, expected, actual }`). Per-variant field rejections:
    - `IndexStatus` (lines 395-408): decodes `state_byte`, `timestamp`, `run_val`; rejects `InvalidRunId` (run==0) at lines 400-402.
    - `IndexWorkflow` (lines 409-419): decodes `workflow_val` (u32 BE), `run_val` (u64 BE); rejects `InvalidRunId` (run==0) at lines 412-414.
    - `IndexAction` (lines 420-432): decodes `action_val` (u16 BE), `run_val` (u64 BE), `step_val` (u16 BE); rejects `InvalidRunId` (run==0) at lines 423-425.
  - Lines 451-474 — `decode_run_event_key` exists for `RunEvent` variant only (not directly relevant to side-index repair, but documents the convention: `KeyDecodeError` is converted into `JournalError::MalformedKeyspaceRow`).

### Decoding primitives / constants the repair must reuse
- `crates/vb_storage/src/constants.rs`:
  - `PREFIX_INDEX_STATUS: u8 = 0x30` (line 39)
  - `PREFIX_INDEX_WORKFLOW: u8 = 0x31` (line 41)
  - `PREFIX_INDEX_ACTION: u8 = 0x32` (line 43)
  - `INDEX_STATUS_KEY_BYTES: usize = 18` (line 77) → 1 prefix + 1 state + 8 timestamp + 8 run = 18
  - `INDEX_WORKFLOW_KEY_BYTES: usize = 13` (line 78) → 1 prefix + 4 workflow + 8 run = 13
  - `INDEX_ACTION_KEY_BYTES: usize = 13` (line 79) → 1 prefix + 2 action + 8 run + 2 step = 13
  - `MIN_OTHER_STATUS_BYTE` is referenced for state-byte collision rejection in the encoder (separate concern from the malformed decoder path).

### Error vocabulary the repair must assert against
- `crates/vb_storage/src/error/key_decode.rs` (lines 8-31):
  - `enum KeyDecodeError { EmptyKey, UnknownPrefix { prefix: u8 }, KeyLengthMismatch { prefix, expected, actual }, InvalidRunId, ReservedSeqSentinel }` — re-exported as `vb_storage::KeyDecodeError` (`crates/vb_storage/src/lib.rs:202`).
- `crates/vb_storage/src/error/mod.rs` (lines 98-105):
  - `JournalError::MalformedKeyspaceRow { prefix, expected_len, actual_len }` — what the keyspace-level surfaces will return when `decode_storage_key` rejects. Tests at the pure-decoder layer can assert `KeyDecodeError` directly; integration tests against `FjallJournal` (or `preview_keyspace`) must match on `MalformedKeyspaceRow`.

### Side-index entry points currently used by other PO tests in the same file
- `crates/vb_storage/src/journal/core.rs`:
  - Lines 165-167 — `FjallJournal::has_action_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError>` — used at lines 112-113 of the test file (PO-002, PO-014). Takes a raw `AsRef<[u8]>` so it will accept malformed bytes; in contrast to the decoder, this is a keyspace membership probe and never decodes. Repair should NOT lean on this for the decoder-path test (it does not exercise the malformed path).
  - Lines 177-179 — `has_status_index_entry` (analogous).
  - Lines 189-191 — `has_workflow_index_entry` (analogous).

### Other tests proving the "decode malformed bytes → typed error" pattern (fixture reference, not modification target)
- `crates/vb_storage/src/preview/tests.rs`:
  - Lines 70-109 — `preview_keyspace_skips_malformed` — uses `KeyspaceScanPolicy::default_doctor()` (SkipMalformed), plants a 3-byte `vec![0x10, 0xAB, 0xCD]` "key" in a typed-entry list, asserts the surrounding valid entries still appear. This is the doctest pattern the new malformed-byte tests should mirror.
  - Lines 111-150 — `preview_keyspace_fails_closed` — uses `KeyspaceScanPolicy::default_production()` (FailClosed), plants `vec![0x10, 0x00, 0x00, 0x00]` (4 bytes, expected 9), asserts the result is `Err(JournalError::MalformedKeyspaceRow { prefix: 0x10, expected_len: 9, actual_len: 4 })`.
  - Lines 152-180 — `preview_keyspace_fail_closed_unknown_prefix` — plants `vec![0xFF, 0x01, 0x02, 0x03]`, asserts `MalformedKeyspaceRow { prefix: 0xFF, expected_len: 0, actual_len: 4 }`.
- `crates/vb_storage/src/tests.rs` (lines 1862-1904):
  - `cc002_run_headers_fails_closed_on_malformed_key` — runs against a real `FjallJournal` instance; plants `vec![PREFIX_RUN_HEADER, 0xAB, 0xCD]` (3 bytes, expected 9) directly into the `run_header` partition and asserts the typed error path. This is the closest behaviourally-correct exemplar and demonstrates the `FailClosed` pattern through the live journal surface.

### Existing key-decode unit tests (fixture reference)
- `crates/vb_storage/src/keys/tests.rs`:
  - Lines 20-46 (`workflow_source_key_*`), 73-99 (`run_header_key_*`), 142-176, 228-256, 260-287 — encode-only coverage. None of them currently exercises `decode_storage_key` with malformed input; the new proptest bodies must close that gap.

## What the repair must do (scope, not implementation)

For each of the three tests in `journal_side_index_contracts.rs` PO-008 block, replace the no-op body with assertions that call `vb_storage::keys::decode_storage_key` (and/or `try_key_prefix` for the empty/unknown-prefix cases) against **real malformed byte sequences**, matching on `KeyDecodeError::{EmptyKey, UnknownPrefix, KeyLengthMismatch, InvalidRunId}` for each side-index variant. Concrete construction recipes that already trip the decoder today:

1. `index_action_key_decode_error_on_short_input` — must build at least:
   - `[PREFIX_INDEX_ACTION]` (1 byte) and the empty `[]` slice → `EmptyKey` (via `try_key_prefix`) and `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 1 }` (via `decode_storage_key`).
   - A truncated slice of a valid 13-byte `index_action_key` whose run field is non-zero; the prefix is `0x32` and length < 13 → `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: <13 }`.
   - A 13-byte buffer with the correct prefix `0x32` but `run == 0` → `InvalidRunId`.
   - A 13-byte buffer with prefix `0x30` (`PREFIX_INDEX_STATUS`) but length 13 (status expects 18) → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 }` — exercises the wrong-prefix-within-index-family case.
2. `index_status_key_decode_error_on_wrong_length` — must build at least:
   - A 17-byte truncation of a valid 18-byte `index_status_key` (correct prefix, length mismatch) → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 17 }`.
   - An 18-byte buffer with prefix `0x32` (`PREFIX_INDEX_ACTION`, expects 13) → `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 }`.
   - An 18-byte buffer with prefix `0x30` and `run == 0` → `InvalidRunId`.
   - An unknown-prefix `vec![0xFF; 18]` → `UnknownPrefix { prefix: 0xFF }` (via `try_key_prefix`) and `KeyLengthMismatch { prefix: 0xFF, expected: 0, actual: 18 }`-equivalent surfacing via `decode_storage_key` (current decoder raises `UnknownPrefix` first).
3. `index_workflow_key_decode_error_on_wrong_length` — must build at least:
   - A truncated slice of a valid 13-byte `index_workflow_key` → `KeyLengthMismatch { prefix: 0x31, expected: 13, actual: <13 }`.
   - A 13-byte buffer with prefix `0x31` and `run == 0` → `InvalidRunId`.
   - A 13-byte buffer with prefix `0x30` (status expects 18) → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 }`.
   - An unknown-prefix `vec![0xFF; 13]` → `UnknownPrefix { prefix: 0xFF }`.

The repair must keep the file's `proptest!` macro framing, the existing `JOURNAL_KEY_PROPTEST_CASES = 128` budget, and the `#![forbid(unsafe_code)]` lint. It must keep the original `IndexStatusState`, `ActionId`, `RunId`, `StepIdx`, `WorkflowId` strategies intact (they are useful for picking valid field bytes) and add the malformed-byte construction inside each body. After repair each test must make at least one `assert!(matches!(..., Err(...)))` (or equivalent `prop_assert!`) against the real malformed payload.

## Existing tests/proofs located
- `crates/vb_storage/src/preview/tests.rs` — passes today; provides the canonical "build malformed bytes → assert typed error" recipe that the repair must copy.
- `crates/vb_storage/src/tests.rs` — `cc002_run_headers_fails_closed_on_malformed_key` (line 1864) — passes today; proves the `FailClosed` policy through the real `FjallJournal` surface. The repair is scoped at the pure-decoder layer (proptest), not the journal layer, so this test is fixture-only.
- `crates/vb_storage/src/keys/tests.rs` — encode-only coverage; no malformed-decode coverage exists. The repair is the first malformed-decode coverage at the workspace-test tier.
- No Kani harness currently models `decode_storage_key` on malformed input (checked via grep on `kani::*` in `crates/vb_storage/src`); proptest + a future Kani scope-up are downstream considerations, not blockers for this test-only P1.

## Gaps downstream must close (post-repair, out of bead scope)
1. Decide whether to mirror the malformed-decode tests at the live `FjallJournal` surface (`has_action_index_entry`/`has_status_index_entry`/`has_workflow_index_entry` take raw bytes today and never decode). If a closed-loop "decode on read" policy is desired for the side-index, that requires production-side changes; the test-only repair here does not.
2. (Optional) Promote one canonical malformed-input shape into a Kani harness for `decode_storage_key` covering all five `KeyDecodeError` variants. Not in scope for this P1 test repair.
3. The current `_extra_bytes` proptest strategies in the two `wrong_length` tests are unused and should be either wired into the malformed-payload builder or removed to avoid proptest budget waste — recommended wiring inside each body.

## Risk tags
- `test-only`, `malformed-key`, `decoder`, `validation-before-mutation`, `fail-closed`, `proptest`, `public-api` (no production public API changes; the test file's proptest bodies are the only mutable surface).
- Not behavior-affecting in production — no `vb_storage` source, no `Cargo.toml`, no dependency change. The repair is bounded to one test file's PO-008 block.

## Recommended downstream owners
- `rust-contract`: skip — no domain-model or typestate work; the contract `decode_storage_key → KeyDecodeError` is already correctly bound.
- `proof-planner`: skip Verus/Kani/Flux obligations — the test repair is plain proptest; existing proof scope is unchanged.
- `test-planner` / `test-writer`: own the repair. Follow the `crates/vb_storage/src/preview/tests.rs` and `crates/vb_storage/src/tests.rs:cc002_run_headers_fails_closed_on_malformed_key` recipes. Preserve `JOURNAL_KEY_PROPTEST_CASES = 128` and the `#![forbid(unsafe_code)]` file lint.
- `holzman-rust`: own the rewrite — Holzman-safe proptest body, no `unwrap`/`expect`, every assertion under `prop_assert!` or wrapped in `Result<(), TestCaseError>` so failures are proptest-shaped.
- Black-hat review: focus on whether each variant of `KeyDecodeError` is exercised and whether the truncated/extra-byte prefixes match `PREFIX_INDEX_ACTION/WORKFLOW/STATUS` exactly.

## Recommended focused evidence commands
- `cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` (must compile and run; proptest seeds surface via `failure_persistence: None`).
- `cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts -- index_action_key_decode_error_on_short_input` and the two named siblings — for single-filter run during repair.
- `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings` (zero-warning source gate per AGENTS.md).
- `moon run :lint-src` and `moon run :check` per AGENTS.md if a moon gate is desired; the workspace-tests `[[test]]` manifest entry already names `journal_side_index_contracts` so no Cargo.toml edit is required.

## Blockers
None for the test repair. No production source, public API, dependency, or proof artifact changes are needed.
