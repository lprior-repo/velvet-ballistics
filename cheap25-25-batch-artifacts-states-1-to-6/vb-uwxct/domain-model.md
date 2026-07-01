# Domain Model — vb-uwxct

- **bead_id**: vb-uwxct
- **title**: Tests: make max-sequence and key tests reject only exact overflow cases (P1 bug)
- **kind**: TEST-ONLY REPAIR — no production Rust changes, no verifier artifact changes
- **isolated_workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
- **runtime_skill_provenance**: `runtime-skill-provenance.json` (rust-contract)

## Scope Statement

The work is exclusively a tightening of seven test/harness artifacts so that they
**accept the typed `Err(JournalError::SequenceOverflow)` rejection** when
`seq.get() == u64::MAX` and **continue to enforce their original shape** for every
other `seq.get() ∈ 0..u64::MAX`. The production encoder at
`crates/vb_storage/src/keys.rs:480-496` and its public surface
(`run_event_key`, `run_snapshot_key`, `journal_key`) are NOT modified.

## Ubiquitous Language

| Term | Meaning in this bead |
|---|---|
| **Sequence sentinel** | `seq.get() == u64::MAX` — the reserved value that the encoder MUST reject with `Err(JournalError::SequenceOverflow)`. The decoder rejects it with `KeyDecodeError::ReservedSeqSentinel` (SC-002). |
| **Encodable sequence** | Any `seq.get() ∈ 0..u64::MAX` — succeeds and yields a 17-byte `[0x11][run_be_8][seq_be_8]` (event key) or `[0x12][run_be_8][seq_be_8]` (snapshot key). |
| **Exact-overflow rejection** | The contract that the encoder returns `Err(JournalError::SequenceOverflow)` **iff** the input sequence is exactly the sentinel. No off-by-one. No "plus or minus one". No partial range. |
| **Test specimen** | A test function in `workspace_tests` or a `#[kani::proof]` harness in `vb_storage` that exercises the encoder with one or more `u64` sequence samples. |
| **Over-rejection** | The defect a test specimen exhibits when it calls `run_event_key(...).expect("...")` on a full-range input and the panicking `.expect()` fires precisely at the one input the encoder is supposed to reject. |
| **Tightened specimen** | The test specimen after this bead is applied: it either (a) constrains proptest input ranges to `0u64..u64::MAX`, (b) adds `prop_assume!(seq != u64::MAX)`, or (c) `match`-binds the result and treats `Err(JournalError::SequenceOverflow)` as the contractually expected rejection. |

## Domain Decision

Illegal states in the **encoder** are already unrepresentable: the production
function `sequenced_run_key` is `fn` (private) and its only call sites
(`journal_key`, `run_event_key`, `run_snapshot_key`) cannot accept a sentinel
without observing the typed `Err`. The illegal state this bead closes is in the
**test specimen**: full-range proptest inputs plus an `.expect()` make the
specimen itself panic, which is indistinguishable from a real encoder bug.

The forbidden state under this contract is therefore:

```
specimen_input.seq == u64::MAX
  ∧ specimen uses run_event_key(...).expect(_)
  ⇒ specimen panics (defect)
```

After the bead the only remaining legal states are:

1. `specimen_input.seq ∈ 0..u64::MAX` and the property under test holds
   (success), OR
2. `specimen_input.seq == u64::MAX` and the specimen either skips the case
   (`prop_assume!`) or `match`-treats the typed `Err(JournalError::SequenceOverflow)`
   as contract-conformant (no panic).

## Entities, Value Objects, and Policies

### Value object — `EventSeq` (newtype, defined in `vb_core`)

A newtype wrapping `u64` with a smart constructor `EventSeq::new(...)`. This
bead does NOT modify `EventSeq`; it relies on `EventSeq::new` being a pure pass-
through so the boundary check `seq.get() == u64::MAX` at
`keys.rs:485` is the single source of truth for the sentinel.

### Entity — `run_event_key` (pub fn)

```
pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; 17], JournalError>
```

Pure, deterministic, no I/O, no time, no allocator. Delegates to
`sequenced_run_key(PREFIX_RUN_EVENT, run, seq)`. The bead only reads this;
the bead does not modify it.

### Policy — Exact-overflow rejection (production)

```
if seq.get() == u64::MAX { return Err(JournalError::SequenceOverflow); }
```

This single if-statement at `keys.rs:485-487` is the policy. The bead does
not modify this; the bead's job is to update specimens so they honor this
policy instead of masking it with a panic.

### Policy — Test specimen exact-overflow honesty

For each of the seven specimens this bead is bound to:

- A specimen that asserts "keys of valid sequences succeed and have shape X"
  must interpret `Err(JournalError::SequenceOverflow)` either as a domain-
  vacuous case (skip via `prop_assume!` or constraint `0u64..u64::MAX`) OR as
  an explicit, contract-correct "this is the one rejected value" assertion.
- A specimen must NEVER `.expect()` on a full-range input.

## Aggregate Boundaries

The aggregate for this bead is the entire
`(RunId, EventSeq) -> [u8; 17]` encoding surface:

- `journal_key` (private) — sole write path
- `run_event_key` (public) — event encoding
- `run_snapshot_key` (public) — snapshot encoding (same overflow policy)

This bead does not touch any of these. The aggregate's invariant under test is
the policy above. The boundary check at `keys.rs:485` is correct and is the
contractually-canonical rejection. All seven specimens must be re-aligned to it.

## Bounded Cardinalities

| Quantity | Bound | Why |
|---|---|---|
| Specimens repaired by this bead | exactly 7 | 6 proptests in `restate_journal_tail_scan_fallback_tests.rs:1326-1449` + 1 Kani harness `kani_typed_partitioned_ids.rs:43-115` |
| Allowed `.expect()` lines in the seven repaired specimens | 0 | All removed or replaced with match/assume |
| `prop_assume!` clauses added | `≤ 14` | up to 2 per proptest (run/seq) where the full range contracts; the Kani harness may add `kani::assume(seq_value != u64::MAX)` OR keep the packed `seq_hi == 0xFFFF && seq_lo == 0xFFFF` reachability but `match` the typed Err |
| Proptest `0u64..u64::MAX` constrain clauses added | `≤ 10` | one per proptest `seq` argument across all six proptests (the `s1`/`s2` proptests share constrained types) |
| Untouched proptests | 1 | `big_endian_bytes_preserve_ordering` is pure byte ordering with no encoder call |

## Contract Surface

| Symbol | Visibility | Source | Lane |
|---|---|---|---|
| `sequenced_run_key` | private | `crates/vb_storage/src/keys.rs:480-496` | production reference, no write |
| `journal_key` | private | `crates/vb_storage/src/keys.rs:476-478` | production reference, no write |
| `run_event_key` | `pub` | `crates/vb_storage/src/keys.rs:81-83` | production reference, no write |
| `run_snapshot_key` | `pub` | `crates/vb_storage/src/keys.rs:85-91` | production reference, no write |
| `JournalError::SequenceOverflow` | `pub` enum variant | `crates/vb_storage/src/error/mod.rs:69-70` | production reference, no write |
| `run_event_key_lexicographic_ordering` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1326-1351` | TIGHTEN |
| `sequence_bytes_roundtrip_through_key_encoding` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1355-1369` | TIGHTEN |
| `run_event_key_always_17_bytes` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1373-1386` | TIGHTEN |
| `run_event_key_always_has_correct_prefix` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1390-1401` | TIGHTEN |
| `different_runs_have_different_event_key_prefixes` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1405-1423` | TIGHTEN |
| `same_run_different_seq_keys_differ_in_seq_bytes` (proptest) | workspace_tests | `restate_journal_tail_scan_fallback_tests.rs:1427-1449` | TIGHTEN |
| `vb_eepg_typed_partitioned_ids` -> `assert_key_contracts` | Kani harness | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-115` | TIGHTEN |
