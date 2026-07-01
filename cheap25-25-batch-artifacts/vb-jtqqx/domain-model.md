# Domain Model — vb-jtqqx Side-Index Malformed-Key Test Repair

## Scope

Bead `vb-jtqqx` is a **test-only P1 repair** in
`crates/workspace_tests/tests/journal_side_index_contracts.rs`, lines 183-257
(the PO-008 proptest block). The three proptest functions currently construct
truncated / extra-byte byte sequences and then **discard them** — the only
assertions are against the *valid* key's length. The decoder is never invoked
against the malformed payload, so the test gives zero behavioural coverage
of `vb_storage::keys::decode_storage_key`'s malformed-payload rejection
contract.

The State 3 pass writes contract artifacts only and edits no production Rust
and no test Rust. Downstream `test-writer` / `holzman-rust` own the actual
proptest body rewrite; the contract here is the normative specification
they must implement.

## Ubiquitous Language

| Term | Meaning | Source refs |
| --- | --- | --- |
| Side-index key | Storage key whose prefix is one of `PREFIX_INDEX_STATUS` (`0x30`), `PREFIX_INDEX_WORKFLOW` (`0x31`), or `PREFIX_INDEX_ACTION` (`0x32`). | `crates/vb_storage/src/constants.rs:38-43` |
| Valid side-index key | A byte sequence that round-trips through `index_*_key(...)` and `decode_storage_key(...)` to `Ok(StorageKey::Index{...})`. | `crates/vb_storage/src/keys.rs:100-155`, `346-432` |
| Malformed side-index payload | Any byte sequence that fails the decoder's prefix / length / domain-rule gates. Must produce a typed `KeyDecodeError`, never panic. | `crates/vb_storage/src/error/key_decode.rs:8-31` |
| Decoder under test | `vb_storage::keys::decode_storage_key(bytes: &[u8]) -> Result<StorageKey, KeyDecodeError>`. Pure parse, no I/O, no allocation. | `crates/vb_storage/src/keys.rs:346-434` |
| Prefix classifier | `vb_storage::keys::try_key_prefix(bytes: &[u8]) -> Result<KeyPrefix, KeyDecodeError>`. Exposes `EmptyKey` / `UnknownPrefix` independently of length. | `crates/vb_storage/src/keys.rs:281-295` |
| Length envelope | The exact byte count required for each prefix: `18` for status, `13` for workflow, `13` for action. The decoder rejects any other length as `KeyLengthMismatch`. | `crates/vb_storage/src/constants.rs:77-79` |
| RunId zero | A `run` field whose big-endian bytes decode to `u64::0`. The decoder rejects any valid-length side-index payload whose `run` field is `0` with `InvalidRunId`. | `crates/vb_storage/src/keys.rs:400-425` |
| Typed decoder error | `vb_storage::KeyDecodeError::{EmptyKey, UnknownPrefix{prefix}, KeyLengthMismatch{prefix, expected, actual}, InvalidRunId, ReservedSeqSentinel}`. The first four are reachable from side-index keys; `ReservedSeqSentinel` is not reachable from side-index payloads and is out of scope for this bead. | `crates/vb_storage/src/error/key_decode.rs:8-31` |
| Encoded state byte | First payload byte of an `index_status_key`, carrying the `IndexStatusState` tag (`Submitted=0`, `Active=1`, `Completed=2`, `Other(n>=3)`). Decoder accepts any byte and recovers `IndexStatusState` via `from_u8`. | `crates/vb_storage/src/types.rs:255-310` |
| Within-family prefix mismatch | A byte sequence whose prefix is one of the three side-index prefixes but whose length matches a *different* side-index variant's length envelope (e.g. `0x30` prefix at length 13). Decoder raises `KeyLengthMismatch` with the prefix that was actually present. | `crates/vb_storage/src/keys.rs:349-355`, `256-264` |
| Proptest budget | The `JOURNAL_KEY_PROPTEST_CASES = 128` cases-per-run cap currently set in the test file. The repair must not raise or lower this; it is the only budget knob. | `crates/workspace_tests/tests/journal_side_index_contracts.rs:23` |
| Validation-before-mutation | The runtime invariant that `decode_storage_key` is called on every read-path byte sequence *before* any storage mutation, journal append, or frame update. The malformed-key tests prove this gate exists in the decoder's pure layer. | `crates/vb_storage/src/keys.rs:346-355` |

## Aggregate / Component View

The repair touches one test file and one pure-decoder module. There is no
production aggregate boundary crossing; the "aggregate" of interest is the
**malformed-payload classification** carried by `KeyDecodeError`.

### Decoder Aggregate (read-only)

- **Entry**: `decode_storage_key(bytes: &[u8])` — pure function, no I/O, no
  mutation. Caller hands it any byte slice, decoder returns either a typed
  `StorageKey` or a typed `KeyDecodeError`.
- **Gates** (in order):
  1. `try_key_prefix` → rejects `EmptyKey` (length 0) and `UnknownPrefix { prefix }`.
  2. Length check `bytes.len() == expected_len` → rejects
     `KeyLengthMismatch { prefix, expected, actual }` when length differs.
  3. Per-variant field decode + domain rules → rejects `InvalidRunId` for
     any side-index variant whose run field is `0`.
- **Boundary**: pure parse layer. The decoder never calls into Fjall, never
  mutates state, never returns `JournalError`. Translation into
  `JournalError::MalformedKeyspaceRow` happens one layer up at the keyspace
  iterator (`decode_run_event_key` shows the convention; the side-index
  membership probes do not currently translate because they are
  membership-only, see Out-of-Scope below).

### Test Aggregates (the three PO-008 tests)

- **`index_action_key_decode_error_on_short_input`** — proptest body that
  currently constructs a truncated slice, names it `_short_key`, and
  discards it. Must instead exercise the decoder against a battery of
  crafted malformed action-key payloads.
- **`index_status_key_decode_error_on_wrong_length`** — currently builds
  a valid 18-byte status key and the `_extra_bytes` strategy is unused.
  Must exercise the decoder against status-key wrong-length and
  within-family-prefix-mismatch payloads.
- **`index_workflow_key_decode_error_on_wrong_length`** — currently builds
  a valid 13-byte workflow key and the `_extra_bytes` strategy is unused.
  Must exercise the decoder against workflow-key wrong-length and
  within-family-prefix-mismatch payloads.

### Out-of-Scope Components (read-only context only)

- `FjallJournal::has_action_index_entry` / `has_status_index_entry` /
  `has_workflow_index_entry` are **membership-only probes** that take
  `AsRef<[u8]>` and never decode. The repair must not route through them.
- `preview_keyspace` / `KeyspaceScanPolicy` (`SkipMalformed` /
  `default_production`) — fixture reference only. The repair operates at
  the pure-decoder layer, not the keyspace-iterator layer.
- `cc002_run_headers_fails_closed_on_malformed_key` (`tests.rs:1862-1904`)
  — fixture reference only. It proves the `FailClosed` pattern through
  `FjallJournal`; the repair is bounded at the decoder layer.

## Domain Decisions

1. **The decoder's pure layer is the contract boundary for malformed-payload rejection.** `decode_storage_key` is the only function under test. The test must call it directly with crafted bytes; it must not route through any I/O or membership-only probe.
2. **Every side-index variant must be exercised against every malformed shape it can reach.** `KeyLengthMismatch` is reachable for all three side-index prefixes. `InvalidRunId` is reachable for all three. `EmptyKey` and `UnknownPrefix` are prefix-stage errors and must be covered at least once across the three tests (not necessarily per test).
3. **Within-family prefix mismatch is a distinct `KeyLengthMismatch` case.** A `0x30` (status) prefix at length 13 must produce `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 }` — the decoder surfaces the *actual* prefix, not the one the caller expected. This case is in scope for the status and workflow tests and must not be silently dropped.
4. **Empty slice handling belongs to `try_key_prefix`.** The `decode_storage_key` entry point delegates to `try_key_prefix` for prefix classification and so inherits `EmptyKey`. Tests must assert on `EmptyKey` *or* `KeyLengthMismatch` according to which function they call; the empty-slice contract for `decode_storage_key` is `Err(KeyDecodeError::EmptyKey)`.
5. **The repair is bounded to one proptest block.** No `Cargo.toml` edit, no `vb_storage` source edit, no `[[test]]` manifest change, no dependency change. The `JOURNAL_KEY_PROPTEST_CASES = 128` budget is preserved exactly.
6. **`ReservedSeqSentinel` is unreachable from side-index payloads.** The three side-index variants (`IndexStatus`, `IndexWorkflow`, `IndexAction`) do not carry an `EventSeq` field. The tests must not assert on `ReservedSeqSentinel`; doing so would be unreachable-test shamming.
7. **Truncated-vs-extra-byte are symmetric length-mismatch shapes.** A 12-byte truncation of a 13-byte action key and a 14-byte extra-byte padding of a 13-byte workflow key both produce `KeyLengthMismatch`. The tests must cover both shapes (truncation is the existing `_short_key`/`truncate_len` strategy; the existing `_extra_bytes` strategy is currently unused and must be wired in).
8. **Proptest strategies must wire the currently-discarded values.** `_short_key`, `_extra_bytes`, and `truncate_len` are not decorative; they must feed into the malformed-payload constructor. Removing them is forbidden (they are proptest inputs already on the test signature). Leaving them unwired is the current bug.

## Illegal States to Make Unrepresentable in the Proptest

| Illegal state in the test body | How the contract forbids it |
| --- | --- |
| A proptest body that constructs malformed bytes but never feeds them to `decode_storage_key` | Contract mandates at least one `prop_assert!(matches!(decode_storage_key(malformed), Err(KeyDecodeError::Variant)))` per body. |
| A proptest body that only asserts `valid_key.len() == N` | The valid-key length invariant is a side-product of `index_*_key(...)`; the *primary* assertion must target the decoder's typed error. |
| An assertion that uses `unwrap` / `expect` on a decoder result that is expected to be `Err` | Holzman-Rust zero-`unwrap` rule applies. Decoder results must be `match`-ed and asserted under `prop_assert!`. |
| A proptest body that exercises the decoder against valid bytes and expects `Ok` | The contract is malformed-key coverage; valid-key decode is already covered by `index_*_key` round-trip tests in `crates/vb_storage/src/keys/tests.rs`. |
| An `_extra_bytes` / `truncate_len` strategy that has no consumer in the test body | Strategies are proptest-side knobs that must drive the malformed-payload builder. Unbound strategies are dead weight and will surface as a black-hat finding. |
| A test that imports `vb_storage::KeyDecodeError` via a glob or via path-rewriting | Import path is `vb_storage::KeyDecodeError` (re-exported at `crates/vb_storage/src/lib.rs:202`) or `vb_storage::error::KeyDecodeError`. Path-rewriting to bypass the `vb_storage` public surface is forbidden. |

## Open Domain Questions

- **Should the malformed-decode coverage migrate into `crates/vb_storage/src/keys/tests.rs` as unit tests?** The proptest at workspace-test tier is the right place for property-driven coverage; per-variant unit tests in `keys/tests.rs` would complement but are out of scope for this P1.
- **Should a future bead add a Kani harness for `decode_storage_key` covering all five `KeyDecodeError` variants?** Out of scope here but flagged as a follow-up; see hazard `H-MAL-005`.
- **Should `FjallJournal::has_*_index_entry` be promoted from a membership-only probe to a decode-on-read probe?** Out of scope; that is a production-side decision. If a future bead wants to surface `MalformedKeyspaceRow` from the membership probes, it is independent of this test-only repair.