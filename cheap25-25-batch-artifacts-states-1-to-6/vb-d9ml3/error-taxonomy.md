# Error Taxonomy — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Decision (one-paragraph rationale)

The bead text offers two typed-error targets:

- **Option A (chosen)**: reuse `TrimError::IncompleteTrim { deleted_count: u64 }`
  (diagnostic code `0x4102`, registered as `JOURNAL_INCOMPLETE_TRIM`).
- **Option B (not chosen)**: converge on
  `JournalError::MalformedKeyspaceRow { prefix, expected_len, actual_len }`
  (diagnostic code `0x4030`, registered as `MALFORMED_KEYSPACE_ROW`).

The contract **commits to Option A** for three reasons:

1. The Round 10 issue 7 regression test (`snapshot_tests.rs:208-248`)
   contains a **structural** assertion
   `Err(crate::trimming::TrimError::IncompleteTrim { deleted_count: 0 })`
   at `snapshot_tests.rs:235`. Converging to `MalformedKeyspaceRow` would
   require rewriting this test, violating the bead's explicit "must keep
   passing" constraint.
2. The trim-diagnostic test (`trimming/tests.rs:934-987`) wraps the trim
   error as `JournalError::Trim(inner) -> inner == TrimError::IncompleteTrim { .. }`.
   The wrapping chain is already in place (`error/mod.rs:187` +
   `error/codes.rs:167`); preserving `IncompleteTrim` keeps the chain
   intact with no code-map change.
3. The bead text lists `IncompleteTrim` first in the typed-error choice.

`MalformedKeyspaceRow` is documented as a **precedent** (see
`boundary-map.md` §4) so future work in other keyspaces can follow the same
pattern; it is NOT introduced for the trim scanner in this bead.

---

## Error Variant Tree (relevant subset)

```
TrimError                                              (trimming/mod.rs)
├── Fjall(fjall::Error)                                -> 0x4001 (delegated)
├── Journal(JournalError)                              -> inner.diagnostic_code()
├── NoDurableSnapshot { run: RunId }                   -> 0x4101
├── RetentionPolicyBlocks { run: RunId }               -> 0x4103
└── IncompleteTrim { deleted_count: u64 }              -> 0x4102  <-- THE BEAD'S ERROR

JournalError                                           (error/mod.rs)
├── ... (many unrelated variants)
├── MalformedKeyspaceRow { prefix: u8,                -> 0x4030  (reference; NOT introduced)
│                          expected_len: usize,
│                          actual_len: usize }
└── Trim(Box<TrimError>)                               -> inner.diagnostic_code()
                                                       -> 0x4102 for IncompleteTrim
```

---

## Per-Variant Contracts

### `TrimError::IncompleteTrim { deleted_count: u64 }`  (CHOSEN — preserved)

| Field | Domain meaning | Range |
|-------|----------------|-------|
| `deleted_count` | Number of events ALREADY removed from the LSM batch BEFORE the abort. The batch was NOT committed. | `0..=u64::MAX` (saturating) |

**Preconditions (when this variant is constructed):**

- The scanner observed a raw key whose `key.len() != MAX_*_KEY_LEN` (primary
  trigger at `logic.rs:36, 77, 222`); OR
- A secondary guard fired (`decode_storage_key` returned a non-`RunSnapshot`
  variant; slice bounds violated; `try_into` failed; etc.).

**Postconditions:**

- The LSM batch has been abandoned (not committed).
- No future trim/count call on the same `(run, safe_point)` will succeed
  until the offending row is repaired or removed.
- `diagnostic_code() == 0x4102` (`INCOMPLETE_TRIM_CODE`).
- `symbolic_code() == JOURNAL_INCOMPLETE_TRIM` (registered).

**Display**: `thiserror`'s `#[error("trim operation incomplete")]` — the
`deleted_count` is omitted from the display string to avoid log noise
(it is observable via `diagnostic_code` + the typed field).

**Constructor contract**: no public constructor. Only the scanner call
sites construct this variant (lines 37, 78, 82, 85, 223, 228, 232 in
`logic.rs`). The contract preserves all seven construction sites verbatim;
only the literal `17` is replaced with the named cap.

---

### `JournalError::Trim(Box<TrimError>)` (CHOSEN — preserved)

- Diagnostic code: delegates to `inner.diagnostic_code()` via
  `error/codes.rs:167`. For `inner = TrimError::IncompleteTrim`, the
  propagated code is `0x4102`.
- Semantic code: delegates to `inner.diagnostic_code().symbolic_code()` via
  `error/codes.rs:181-191`. For the incomplete-trim chain, the propagated
  semantic code is `JOURNAL_INCOMPLETE_TRIM`.

**Production construction site**: `count_trimmable_events` at
`logic.rs:223, 228, 232`. The contract preserves the construction:
`Err(JournalError::from(TrimError::IncompleteTrim { deleted_count }))`.

---

### `JournalError::MalformedKeyspaceRow { prefix, expected_len, actual_len }`  (NOT chosen for trim)

| Field | Domain meaning |
|-------|----------------|
| `prefix` | First byte of the raw key (e.g., `PREFIX_RUN_HEADER = 0x10`). |
| `expected_len` | The canonical byte length for that prefix (e.g., `RUN_ONLY_KEY_BYTES = 9`). |
| `actual_len` | The observed `key.len()` — strictly `!= expected_len`. |

- Diagnostic code: `0x4030` (`MALFORMED_KEYSPACE_ROW_CODE`).
- Semantic code: `MALFORMED_KEYSPACE_ROW`.
- Production construction site: `headers.rs:67-72`. The trim path does NOT
  introduce this variant in this bead.

**Why NOT chosen for trim**: see "Decision" above. The bead's typed-error
requirement is satisfied by the existing `IncompleteTrim` variant, which is
already structured (carries `deleted_count: u64`) and registered
(`JOURNAL_INCOMPLETE_TRIM`, code `0x4102`).

---

## Diagnostic Code Stability

The following invariants MUST hold after the fix:

| Code | Variant | Status post-fix |
|------|---------|-----------------|
| `0x4001` | `JournalError::Fjall` | unchanged |
| `0x4030` | `JournalError::MalformedKeyspaceRow` | unchanged (still produced by `headers.rs`) |
| `0x4101` | `TrimError::NoDurableSnapshot` | unchanged |
| `0x4102` | `TrimError::IncompleteTrim` | **unchanged** (the bead's whole point) |
| `0x4103` | `TrimError::RetentionPolicyBlocks` | unchanged |

The `error_code_tests.rs:~244` test (which verifies
`count_trimmable_events` error code propagation) MUST continue to pass
without modification. The contract explicitly does not introduce a new
diagnostic code.

---

## Symbolic Code Stability

| Symbolic code | Variant | Status post-fix |
|---------------|---------|-----------------|
| `JOURNAL_INCOMPLETE_TRIM` | `TrimError::IncompleteTrim` | unchanged |
| `MALFORMED_KEYSPACE_ROW` | `JournalError::MalformedKeyspaceRow` | unchanged |

Both codes are registered in `vb_core::CODE_REGISTRY` and resolved via
`DiagnosticCode::symbolic_code()`. No new registrations are required for
this bead.

---

## Error-Chain Proof Obligations (for the proof planner)

The contract commits to the following test/proof surface:

| Chain | Test pinning |
|-------|--------------|
| `latest_durable_snapshot_seq -> Err(TrimError::IncompleteTrim { deleted_count: 0 })` | `snapshot_tests.rs:208-248` (existing 13-byte case) + new overlong case |
| `trim_events_for_run -> Err(TrimError::IncompleteTrim { deleted_count })` | `trimming/tests.rs:875-932` (existing 9-byte case) + new overlong case |
| `count_trimmable_events -> Err(JournalError::Trim(Box::new(TrimError::IncompleteTrim { deleted_count })))` | `trimming/tests.rs:934-987` (existing 9-byte case) + new overlong case |
| Diagnostic code propagation `0x4102` | `error_code_tests.rs:~244` (existing) |

The proof planner SHOULD author a property test that:

1. Plants raw keys of length `0..=256` under both `PREFIX_RUN_EVENT` and
   `PREFIX_RUN_SNAPSHOT`.
2. Invokes each trim scanner.
3. Asserts: for `len == 17`, no error is raised; for `len != 17`,
   `TrimError::IncompleteTrim { .. }` is raised (or its journal-wrapped
   variant); for `len == 17` with `PREFIX_RUN_EVENT` under the snapshot
   cursor (or vice versa), the secondary `decode_storage_key` check raises
   `TrimError::IncompleteTrim { deleted_count: 0 }`.

The proptest shape is described in `proof-seeds.jsonl` (see
`PS-CAP-PROPTEST-001`).

---

## Forbidden Error Patterns

| Pattern | Why forbidden |
|---------|---------------|
| Returning `Ok(None)` to mask a malformed snapshot key | Loses the fail-closed invariant; Round 10 issue 7 would recur. |
| Returning `Ok(0)` from `count_trimmable_events` when a malformed key was observed | Silently drops the row; violates the fail-closed invariant. |
| Introducing a new `TrimError::MalformedTrimKey { prefix, expected, actual }` variant | Would force a code-map change and break the existing structural test assertions. |
| Logging `warn!("malformed key, skipping")` and continuing the loop | Holzmann-Rust forbids "log and continue" on parser boundary errors; this is an explicit typed error. |
| Mapping `IncompleteTrim` to `MALFORMED_KEYSPACE_ROW` (0x4030) via `From` | Would collapse two distinct semantic codes; the chain must preserve `0x4102`. |
| Adding a new diagnostic code (e.g., `0x4104`) for overlong keys | Forbidden by the bead scope ("reuse `TrimError::IncompleteTrim (0x4102)`"); no code-map change permitted. |

---

## Diagnostic Code Authority (canonical reference)

The complete diagnostic code registry is in
`crates/vb_storage/src/error/codes.rs:1-95`. The trim-related codes are:

| Code | Symbolic name | Variant |
|------|---------------|---------|
| `0x4001` | `FJALL_ERROR` | `JournalError::Fjall` |
| `0x4101` | `JOURNAL_NO_DURABLE_SNAPSHOT` | `TrimError::NoDurableSnapshot` |
| `0x4102` | `JOURNAL_INCOMPLETE_TRIM` | `TrimError::IncompleteTrim` |
| `0x4103` | `JOURNAL_RETENTION_POLICY_BLOCKS` | `TrimError::RetentionPolicyBlocks` |
| `0x4030` | `MALFORMED_KEYSPACE_ROW` | `JournalError::MalformedKeyspaceRow` |

The bead changes NONE of these codes. The contract binds the implementation
to leave `error/codes.rs` untouched.

---

## Summary Table — Error Surface Post-Fix

| Aspect | Pre-fix | Post-fix |
|--------|---------|----------|
| Length literal at the three sites | `17` (magic number) | `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` |
| Typed error variant | `TrimError::IncompleteTrim { deleted_count }` | **unchanged** |
| Diagnostic code | `0x4102` | **unchanged** |
| Symbolic code | `JOURNAL_INCOMPLETE_TRIM` | **unchanged** |
| Code map (`error/codes.rs`) | untouched | **unchanged** |
| Public API | unchanged | **unchanged** |
| Test surface (`snapshot_tests.rs:208-248`) | passes (13-byte case) | passes + augmented overlong case |
| Test surface (`trimming/tests.rs:875-987`) | passes (9-byte cases) | passes + augmented overlong case |

END OF ERROR TAXONOMY.