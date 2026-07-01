# Type Contracts — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Constant-Level Contracts

### `JOURNAL_KEY_BYTES` (source of truth — UNCHANGED)

```rust
// crates/vb_storage/src/constants.rs:74
pub(crate) const JOURNAL_KEY_BYTES: usize = 17;
```

- **Type**: `usize` const.
- **Visibility**: `pub(crate)`.
- **Equation**: `JOURNAL_KEY_BYTES == 17` (compile-time).
- **Pre-/postconditions**: none (literal const).
- **Stability**: must not change without coordinated migration of all
  pre-encoded keys on disk; out of scope for this bead.

### `MAX_TRIM_KEY_LEN` (NEW — alias)

```rust
// crates/vb_storage/src/constants.rs (after JOURNAL_KEY_BYTES)
pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;
```

- **Type**: `usize` const alias.
- **Visibility**: `pub(crate)` (matches surrounding consts).
- **Equation**: `MAX_TRIM_KEY_LEN == JOURNAL_KEY_BYTES == 17` (compile-time).
- **Compile-time invariant**: the const initializer is a `const` reference to
  another `const`, so the compiler statically enforces equality. Any future
  bump to `JOURNAL_KEY_BYTES` automatically propagates to this alias.
- **Domain role**: named cap for the **trim** call sites that read raw
  `run_event` keys under prefix `0x11`. Replaces the magic literal `17` at
  `trimming/logic.rs:77` and `:222`.
- **Forbidden patterns**:
  - Defining `MAX_TRIM_KEY_LEN = 17` directly (loses the alias chain).
  - Marking it `pub` without a public-API contract review.
  - Using `MAX_TRIM_KEY_LEN` for non-trim scanners (use the dedicated
    constants for other keyspaces).

### `MAX_SNAPSHOT_KEY_LEN` (NEW — alias)

```rust
// crates/vb_storage/src/constants.rs (after MAX_TRIM_KEY_LEN)
pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;
```

- **Type**: `usize` const alias.
- **Visibility**: `pub(crate)`.
- **Equation**: `MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17`.
- **Domain role**: named cap for the **trim** call site that reads raw
  `run_snapshot` keys under prefix `0x12`. Replaces the magic literal `17`
  at `trimming/logic.rs:36`.
- **Forbidden patterns**: same as `MAX_TRIM_KEY_LEN`.

### Co-location rule

Both aliases MUST be declared in `crates/vb_storage/src/constants.rs`
immediately adjacent to `JOURNAL_KEY_BYTES` (the source of truth) so that a
single-file edit changes all three in lockstep. Pairing the declarations
with a doc-comment block that explains the journal key envelope
(`[prefix][run_id:u64 BE][seq:u64 BE]`) prevents future drift between the
three values.

---

## Smart-Constructor Contracts (already in place, contractually frozen)

### `run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>`

- **Return type**: `[u8; JOURNAL_KEY_BYTES]` — fixed-size array, length
  guaranteed by the type system. **Cannot emit a non-canonical length.**
- **Pre-/postconditions**: unchanged.
- **Contractual freeze**: the encoder is contractually frozen against the
  cap. Any future change that weakens this guarantee (e.g., variable-length
  return) must also update `MAX_TRIM_KEY_LEN` semantics.

### `run_snapshot_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>`

- Same guarantees as `run_event_key`. Encoder is contractually frozen
  against `MAX_SNAPSHOT_KEY_LEN`.

### `decode_storage_key(bytes: &[u8]) -> Result<StorageKey, KeyDecodeError>`

- **Precondition**: `bytes.len() == expected_len` for the detected prefix
  (enforced at `keys.rs:349` via `KeyDecodeError::KeyLengthMismatch`).
- **Postcondition**: `Ok(_)` implies the returned `StorageKey` variant
  matches the prefix byte.
- **Contractual role**: the secondary safety net for the trim scanner at
  `trimming/logic.rs:43`. Re-validates prefix + length after the scanner's
  primary length check.

---

## Boundary Scanner Contracts

### `latest_durable_snapshot_seq`

```rust
pub fn latest_durable_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>>
```

**Preconditions:**

- `run != RunId::new(0)` — implicit, inherited from the Fjall keyspace
  invariant (`RunHeader` row must exist for the run; scanned via
  `run_headers()` upstream).

**Postconditions:**

- `Ok(None)` iff no snapshot exists for `run`.
- `Ok(Some(seq))` iff there exists a raw key `k` in the snapshot keyspace
  with `k.len() == MAX_SNAPSHOT_KEY_LEN` whose decoded form is
  `StorageKey::RunSnapshot { run, seq }`.
- `Err(TrimError::IncompleteTrim { deleted_count: 0 })` for any non-canonical
  raw key under the snapshot prefix (length != `MAX_SNAPSHOT_KEY_LEN` OR
  prefix collision).
- Never panics, never returns `Ok(Some(seq))` for a non-canonical key.

**Type-level enforcement:**

- The length check uses `MAX_SNAPSHOT_KEY_LEN` (not `17`). The replacement
  is line-for-line: `key.len() != MAX_SNAPSHOT_KEY_LEN` →
  `Err(TrimError::IncompleteTrim { deleted_count: 0 })`.

### `trim_events_for_run`

```rust
pub fn trim_events_for_run(&self, run: RunId, policy: TrimPolicy) -> TrimResult<TrimmedRunResult>
```

**Preconditions:**

- Caller has invoked `latest_durable_snapshot_seq(run)` upstream
  (the function calls it internally on line 54, so callers need not).
- `policy` is a valid `TrimPolicy` (struct invariants on the type).

**Postconditions:**

- On `Ok`: returns a `TrimmedRunResult { run, deleted_count, cutoff_seq, status }`
  with `deleted_count == 0` and `status == NoOp` if no events were eligible.
- On `Err(TrimError::IncompleteTrim { deleted_count })`: the partial count
  is the number of events already removed from the LSM batch before the
  abort. The LSM batch is **not** committed when this error is returned.
- On `Err(TrimError::NoDurableSnapshot { .. })`: no events were inspected.
- On `Err(TrimError::RetentionPolicyBlocks { .. })`: no events were
  inspected beyond the policy check.
- Never panics; never mutates the LSM tree on `Err`.

**Type-level enforcement:**

- The loop body length check uses `MAX_TRIM_KEY_LEN`. The replacement is
  line-for-line: `key.len() != MAX_TRIM_KEY_LEN` →
  `Err(TrimError::IncompleteTrim { deleted_count })`.
- The downstream `key.get(9..17)` slice bounds remain valid because
  `9 + 8 == MAX_TRIM_KEY_LEN`.

### `count_trimmable_events` (private helper)

```rust
fn count_trimmable_events(&self, run: RunId, safe_point: EventSeq) -> Result<u64, JournalError>
```

**Preconditions:**

- `run != RunId::new(0)` (inherited).
- `safe_point` is a valid `EventSeq` for `run` (caller is the diagnostic
  function which already filtered `latest_durable_snapshot_seq`).

**Postconditions:**

- `Ok(count)` with the number of events whose `seq < safe_point.get()`,
  computed only from canonical-length keys.
- `Err(JournalError::Trim(Box::new(TrimError::IncompleteTrim { deleted_count })))`
  on the first non-canonical raw key. The wrapped `TrimError::IncompleteTrim`
  carries the partial count.
- Never panics; never returns `Ok(0)` when a malformed key was observed.

**Type-level enforcement:**

- The length check uses `MAX_TRIM_KEY_LEN`. The replacement is line-for-line:
  `key.len() != MAX_TRIM_KEY_LEN` →
  `Err(JournalError::from(TrimError::IncompleteTrim { deleted_count: count }))`.

---

## Error Variant Contracts (preserved verbatim)

### `TrimError::IncompleteTrim { deleted_count: u64 }`

- **Diagnostic code**: `0x4102` (`TrimError::INCOMPLETE_TRIM_CODE`,
  `trimming/mod.rs:62`).
- **Semantic code**: `JOURNAL_INCOMPLETE_TRIM` (registered in
  `vb_core::CODE_REGISTRY`).
- **Carries**: the number of events that were ALREADY removed before the
  scanner aborted. Zero means the abort happened at the first observation.
- **Wraps to**: `JournalError::Trim(Box<TrimError>)` via
  `error/mod.rs:187`. `JournalError::Trim(inner).diagnostic_code()` delegates
  to `inner.diagnostic_code()` (`error/codes.rs:167`), so the chain
  `JournalError::Trim(TrimError::IncompleteTrim)` carries `0x4102`.

### `JournalError::MalformedKeyspaceRow { prefix, expected_len, actual_len }` (NOT chosen for trim path)

- **Diagnostic code**: `0x4030` (`MALFORMED_KEYSPACE_ROW_CODE`).
- **Semantic code**: `MALFORMED_KEYSPACE_ROW`.
- **Status**: **NOT introduced** for the trim scanner. This variant is the
  precedent used by `headers.rs:67-72`; the bead opts to preserve
  `IncompleteTrim` because the existing regression tests at
  `snapshot_tests.rs:208-248` and `trimming/tests.rs:875-987` are
  **structural** assertions on `IncompleteTrim { deleted_count: 0 }`.
  Converging on `MalformedKeyspaceRow` would require breaking those
  structural assertions, which the bead scope forbids ("must keep passing").

### Decision rationale (one paragraph for the proof planner)

The bead text offers both `TrimError::IncompleteTrim (0x4102)` and
`JournalError::MalformedKeyspaceRow (0x4030)` as the typed-error target.
The contract **commits to `TrimError::IncompleteTrim`** because:

1. The existing Round 10 issue 7 regression test (`snapshot_tests.rs:208-248`)
   asserts `Err(crate::trimming::TrimError::IncompleteTrim { deleted_count: 0 })`
   at `snapshot_tests.rs:235`. Converging to `MalformedKeyspaceRow` would
   require rewriting this test, violating the bead's "must keep passing"
   constraint.
2. `count_trimmable_events` already converts `TrimError::IncompleteTrim`
   to `JournalError::Trim(Box<TrimError>)` (`logic.rs:223`). The
   `From<TrimError> for JournalError` impl (`error/mod.rs:187`) plus
   `JournalError::diagnostic_code()` delegation (`error/codes.rs:167`)
   preserve the `0x4102` code through the chain.
3. The bead explicitly permits reuse of `IncompleteTrim` and lists it first
   in the typed-error choice.

The `MalformedKeyspaceRow` precedent is documented as a **reference** in the
boundary map (`boundary-map.md`) so future work in other keyspaces can
follow the same pattern.

---

## Forbidden Patterns (type-system-level)

| Pattern | Why forbidden |
|---------|---------------|
| `key.len() != 17` at the three magic-17 sites | Magic literal; non-self-documenting; bypasses the alias chain. The fix replaces `17` with `MAX_TRIM_KEY_LEN` or `MAX_SNAPSHOT_KEY_LEN` at all three sites. |
| Defining a new `TrimError` variant for overlong keys | The existing `IncompleteTrim` is already typed and carrying; adding a variant would force a code-map change and break the structural test assertions. |
| Panicking on `key.len() != MAX_*_KEY_LEN` | Holzmann Rust forbids panic at boundaries; trim scanners are storage boundaries. |
| Returning `Ok(None)` to mask a malformed key | Would silently skip a corrupt row and lose the fail-closed invariant. |
| `panic!` / `unwrap()` / `expect()` inside the length check | Holzmann Rust zero-tolerance for panic at boundaries. |
| `unsafe { ... }` in any of the call sites | `#![forbid(unsafe_code)]` is set on the crate; contract preserves this. |
| Calling `MAX_TRIM_KEY_LEN` for snapshot keys or vice versa | Each alias is scoped to its keyspace; using the wrong alias is a code-smell that a future lint should catch. |

---

## Type-Test Pinning (for the proof planner)

The contract pins the following public observable invariants, each backed by
either an existing test or a new test the planner must author:

| Invariant | Pinned by |
|-----------|-----------|
| `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17` | Compile-time; `cargo check` |
| `latest_durable_snapshot_seq` rejects overlong snapshot key with `IncompleteTrim { deleted_count: 0 }` | `snapshot_tests.rs:208-248` (existing, plus augmented overlong case) |
| `trim_events_for_run` rejects overlong event key with `IncompleteTrim { deleted_count }` | `trimming/tests.rs:875-932` (existing, plus augmented overlong case) |
| `count_trimmable_events` (via `trim_eligibility_diagnostic`) rejects overlong event key with `JournalError::Trim(IncompleteTrim)` | `trimming/tests.rs:934-987` (existing, plus augmented overlong case) |
| All `run_event_key(...)` and `run_snapshot_key(...)` outputs have length exactly `JOURNAL_KEY_BYTES` | `keys/tests.rs` length tests (existing); new proptest extension recommended |
| `decode_storage_key` rejects a key whose length != expected prefix length | `keys/tests.rs` length tests (existing) |

END OF TYPE CONTRACTS.