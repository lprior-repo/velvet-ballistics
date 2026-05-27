# Type Contracts — vb-om21

## Contracted Rust Shapes

The implementation state should express the model with strong types rather than primitive `u64`/`Option<u64>` behavior flags.

```text
RunEventPrefix = [u8; 9]
RunEventKey    = [u8; 17]
EventSeq       = existing vb_storage numeric sequence type
JournalTail    = newtype wrapping EventSeq, meaning next append/recovery boundary
TailMetadata   = enum { Missing, Present(JournalTail) }
TailScanMode   = enum { QueryAllowsEmpty, RecoveryRequiresJournal }
TailScanResult = enum { EmptyTailZero, Present { max_seq, tail } }
```

These are contract names, not required exact production names. Existing public types such as `FjallJournal`, `JournalError`, and `RecoveryError` remain authoritative where already present.

## Smart Constructors and Parsers

| Type | Constructor / parser contract |
|---|---|
| `RunEventPrefix` | Construct only through `run_prefix_key(run)`. No caller-provided byte prefixes enter core logic unchecked. |
| `RunEventKey` | Construct only through `run_event_key(run, seq)`. No manual concatenation outside key module. |
| `EncodedSequence` | Decode only after `key.len() == JOURNAL_KEY_BYTES` and `key.starts_with(run_prefix)`. |
| `JournalTail` | Construct from empty prefix as zero or from `max_seq.checked_add(1)`. Overflow must become typed error. |
| `TailMetadata` | Convert absent metadata to `Missing`; convert present numeric metadata through `JournalTail` validation. |

## Illegal States to Make Unrepresentable

1. A tail scan that has no `RunId`.
2. A scan result that mixes keys from multiple run prefixes.
3. A decoded sequence taken from a key shorter than `JOURNAL_KEY_BYTES`.
4. A metadata-present flag without a typed metadata value.
5. A recovery outcome that both succeeds and reports `TailMismatch`/`MissingJournal`.
6. A reconstructed tail created by unchecked `max_seq + 1`.
7. A comparison between declared and reconstructed tails that treats “declared below reconstructed” as success.

## Required Error Extensions / Semantics

The explored code does not currently expose `RecoveryError::TailMismatch` or `RecoveryError::MissingJournal`. The contract requires typed recovery errors equivalent to:

```text
RecoveryError::TailMismatch { run: RunId, declared: EventSeq, reconstructed: EventSeq }
RecoveryError::MissingJournal { run: RunId }
```

If implementation chooses names or location differently, contract parity still requires:

- `TailMismatch` semantics: declared/suspect tail is below key-derived tail; recovery fails closed.
- `MissingJournal` semantics: recovery requires journal data but the `run_event` prefix has no event keys.
- No fallback to stringly `ReplayDivergence` for these bead-specific conditions unless wrapped by a typed variant with structured fields.

## Ownership and Borrowing

- Tail scan borrows `FjallJournal` immutably.
- It may use a Fjall snapshot for consistent read view.
- It must not mutate journal state, allocate unbounded buffers, or collect all events merely to compute tail.
- It may store only O(1) scan state: latest sequence, observed flag, and current key parse scratch.

## Boundary Validations

- Fjall keys are untrusted storage bytes at the parser boundary.
- `run_event_key` / `run_prefix_key` outputs are trusted key constructors once their `Result` succeeds.
- Journal event payload decode is not required merely to reconstruct tail from keys, but recovery replay still must validate payload/run/sequence consistency when reading events.
