# Proof Coverage Matrix — vb-jtqqx

Maps each contract clause (from `contract.md`) to proof obligations,
verifier lanes, and the corresponding source-line evidence. The matrix
is the planning-time view of how the PO-008 block is closed; the
formal-verifier (State 12) populates the disposition column with raw
`cargo nextest run` and `cargo clippy` output.

## Bead scope

- **In scope**: test-only repair of
  `crates/workspace_tests/tests/journal_side_index_contracts.rs:183-257`
  (PO-008 proptest block).
- **Out of scope (read-only)**: `crates/vb_storage/src/keys.rs:281-295`
  (try_key_prefix), `crates/vb_storage/src/keys.rs:346-434`
  (decode_storage_key), `crates/vb_storage/src/constants.rs:38-43,
  77-79` (prefix bytes and length envelopes),
  `crates/vb_storage/src/error/key_decode.rs:8-31` (KeyDecodeError
  taxonomy).
- **Behavior affecting**: `false` (no production source change).

## Per-contract-clause coverage

### SIDEX-MAL-001 — Decoder-call requirement (PS-MAL-001)

| Proof obligation | Verifier lane | Test function | Decoder call shape | Expected error |
|---|---|---|---|---|
| PO-MAL-001 (req-1) | proptest (VLD-001) | `index_action_key_decode_error_on_short_input` | `decode_storage_key(&short)` | `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: <13 - truncate_len> })` |
| PO-MAL-001 (req-2) | proptest (VLD-001) | `index_action_key_decode_error_on_short_input` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| PO-MAL-001 (req-3) | proptest (VLD-001) | `index_action_key_decode_error_on_short_input` | `decode_storage_key(&mismatch)` (`vec![0x30; 13]`) | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })` |
| PO-MAL-001 (req-4) | proptest (VLD-021) | `index_action_key_decode_error_on_short_input` | `try_key_prefix(&[])` | `Err(KeyDecodeError::EmptyKey)` |
| PO-MAL-001 (req-5) | proptest (VLD-001) | `index_status_key_decode_error_on_wrong_length` | `decode_storage_key(&oversize)` (`valid + vec![0u8; extra]`) | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + extra })` |
| PO-MAL-001 (req-6) | proptest (VLD-001) | `index_status_key_decode_error_on_wrong_length` | `decode_storage_key(&short17)` | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 17 })` |
| PO-MAL-001 (req-7) | proptest (VLD-010) | `index_status_key_decode_error_on_wrong_length` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| PO-MAL-001 (req-8) | proptest (VLD-001) | `index_status_key_decode_error_on_wrong_length` | `decode_storage_key(&mismatch)` (`vec![0x32; 18]`) | `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 })` |
| PO-MAL-001 (req-9) | proptest (VLD-001) | `index_workflow_key_decode_error_on_wrong_length` | `decode_storage_key(&oversize)` | `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + extra })` |
| PO-MAL-001 (req-10) | proptest (VLD-001) | `index_workflow_key_decode_error_on_wrong_length` | `decode_storage_key(&short)` | `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: <13 - truncate_len> })` |
| PO-MAL-001 (req-11) | proptest (VLD-010) | `index_workflow_key_decode_error_on_wrong_length` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| PO-MAL-001 (req-12) | proptest (VLD-001) | `index_workflow_key_decode_error_on_wrong_length` | `decode_storage_key(&mismatch)` (`vec![0x30; 13]`) | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })` |
| PO-MAL-001 (req-13) | proptest (VLD-022) | `index_workflow_key_decode_error_on_wrong_length` | `try_key_prefix(&[0xFF, ...])` | `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })` |

### SIDEX-MAL-002 — Strategies wired (PS-MAL-002, PS-MAL-003, PS-MAL-009)

| Proof obligation | Verifier lane | Strategy | Wired into | Notes |
|---|---|---|---|---|
| PO-MAL-002 (PS-MAL-002) | proptest (VLD-008) | `truncate_len in 1u8..=12u8` (action) | truncated-payload constructor | VLD-008 enforces that the strategy input is consumed by the body (not discarded with `_`). |
| PO-MAL-002 (PS-MAL-003) | proptest (VLD-009) | `_extra_bytes in 0u8..=10u8` (status, workflow) | oversize-payload constructor | VLD-009 enforces that the leading underscore is removed and the strategy is bound to a let binding. |
| PO-MAL-002 (PS-MAL-009) | proptest (VLD-015) | every strategy on PO-008 signature | at least one payload shape each | VLD-015 enforces no `_`-discard pattern; cargo clippy's `unused_variables` lint catches the violation. |

### SIDEX-MAL-003 — Reachable variant coverage (PS-MAL-013, PS-MAL-015, PS-MAL-016)

| Variant | Owner test | Verifier lane | Decoder call | Source branch |
|---|---|---|---|---|
| `EmptyKey` | action test | proptest (VLD-021) | `try_key_prefix(&[])` | `keys.rs:282` |
| `UnknownPrefix` | workflow test | proptest (VLD-022) | `try_key_prefix(&[0xFF, ...])` | `keys.rs:293` |
| `KeyLengthMismatch` (IndexAction) | action test | proptest (VLD-001, VLD-017) | truncated + within-family | `keys.rs:349-355` |
| `KeyLengthMismatch` (IndexWorkflow) | workflow test | proptest (VLD-001, VLD-017) | truncated + within-family | `keys.rs:349-355` |
| `KeyLengthMismatch` (IndexStatus) | status test | proptest (VLD-001, VLD-017) | truncated + within-family | `keys.rs:349-355` |
| `InvalidRunId` (IndexAction) | action test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:423-425` |
| `InvalidRunId` (IndexWorkflow) | workflow test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:412-414` |
| `InvalidRunId` (IndexStatus) | status test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:400-402` |
| `ReservedSeqSentinel` | (forbidden) | n/a (VLD-012 not_applicable) | n/a | `keys.rs:386` is unreachable from side-index payloads |

### SIDEX-MAL-004 — Budget preserved (PS-MAL-017)

| Constant | Line | Verifier lane | Decision |
|---|---|---|---|
| `JOURNAL_KEY_PROPTEST_CASES = 128` | `journal_side_index_contracts.rs:23` | proptest (VLD-023) | Constant unchanged; cargo nextest run reports 128 cases per proptest. |

### SIDEX-MAL-005 — File-level `forbid(unsafe_code)` (PS-MAL-018)

| Lint | Line | Verifier lane | Decision |
|---|---|---|---|
| `#![forbid(unsafe_code)]` | `journal_side_index_contracts.rs:14` | proptest (VLD-024) | File-level lint preserved; `cargo clippy ... -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` enforces Holzman-Rust zero-tolerance. |

### SIDEX-MAL-006 — No unwrap/expect/panic on decoder results (PS-MAL-007)

| Rule | Verifier lane | Decision |
|---|---|---|
| All decoder results matched under `prop_assert!`, never `unwrap`/`expect`/`panic` | proptest (VLD-013) | Holzman-Rust zero-tolerance enforced at compile time by `cargo clippy -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` and at run time by proptest's `prop_assert!` macro. |

### SIDEX-MAL-007 — No membership probe (PS-MAL-008)

| Rule | Verifier lane | Decision |
|---|---|---|
| PO-008 block must not call `FjallJournal::has_*_index_entry`, `temp_journal()`, or `KeyspaceScanPolicy::*` | proptest (VLD-014) | Import graph is the static check; `temp_journal()` is used by other PO blocks (PO-002, PO-004, PO-009, PO-010, PO-013, PO-014) but must not appear inside the PO-008 block. |

### SIDEX-MAL-008 — Bounded to one test file

| Constraint | Verifier lane | Decision |
|---|---|---|
| No edits to `Cargo.toml`, `vb_storage/**`, `Cargo.lock`, dependency manifests | proptest (PO-MAL-002) | `cargo clippy` compiles all dependent crates; any change to a non-test file would require recompilation. Source-diff evidence: only `crates/workspace_tests/tests/journal_side_index_contracts.rs` is modified. |

### SIDEX-MAL-009 — `KeyLengthMismatch` field surfacing (PS-MAL-011)

| Field | Verifier lane | Decision |
|---|---|---|
| `prefix: <actual_prefix>` matches the first byte of the payload | proptest (VLD-017) | The `matches!` pattern with field surfacing is the static check; a body that uses the wrong prefix byte will fail the `prop_assert!`. |

### SIDEX-MAL-010 — Per-variant `InvalidRunId` (PS-MAL-004)

| Variant | Owner test | Verifier lane | Decoder call | Source branch |
|---|---|---|---|---|
| `IndexAction` `InvalidRunId` | action test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:423-425` |
| `IndexWorkflow` `InvalidRunId` | workflow test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:412-414` |
| `IndexStatus` `InvalidRunId` | status test | proptest (VLD-010, VLD-019) | `decode_storage_key(&zero_run)` | `keys.rs:400-402` |

### SIDEX-MAL-011 — Within-family prefix mismatch (PS-MAL-014)

| Mismatch shape | Owner test | Verifier lane | Expected `prefix` field | Expected `expected` field | Expected `actual` field |
|---|---|---|---|---|---|
| `vec![0x30; 13]` (status prefix, action length) | action test | proptest (VLD-020) | `0x30` | `18` | `13` |
| `vec![0x32; 18]` (action prefix, status length) | status test | proptest (VLD-020) | `0x32` | `13` | `18` |
| `vec![0x30; 13]` (status prefix, workflow length) | workflow test | proptest (VLD-020) | `0x30` | `18` | `13` |

### SIDEX-MAL-012 — Empty-key branch coverage (PS-MAL-015)

| Shape | Owner test | Verifier lane | Decoder call |
|---|---|---|---|
| `&[]` | action test | proptest (VLD-021) | `try_key_prefix(&[])` |

### SIDEX-MAL-013 — Unknown-prefix branch coverage (PS-MAL-016)

| Shape | Owner test | Verifier lane | Decoder call |
|---|---|---|---|
| `vec![0xFF; 13]` (or any length) | workflow test | proptest (VLD-022) | `try_key_prefix(&[0xFF, ...])` |

### SIDEX-MAL-014 — Truncate bound (PS-MAL-012)

| Bound | Range | Verifier lane | Decision |
|---|---|---|---|
| `truncate_len in 1u8..=12u8` (action) | `[1, 12]` → truncated length `[1, 12]` ∈ `[1, 13)` | proptest (VLD-018) | Bound preserved; `0u8..=12u8` would shift to `EmptyKey`. |
| `truncate_len in 1u8..=12u8` (workflow, new) | `[1, 12]` → truncated length `[1, 12]` ∈ `[1, 13)` | proptest (VLD-018) | New bound added by the repair. |
| `truncate_len in 1u8..=17u8` (status, new) | `[1, 17]` → truncated length `[1, 17]` ∈ `[1, 18)` | proptest (VLD-018) | New bound added by the repair. |

### SIDEX-MAL-015 — Oversize slice (PS-MAL-003)

| Strategy | Owner test | Verifier lane | Wired into |
|---|---|---|---|
| `extra_bytes in 0u8..=10u8` (status) | status test | proptest (VLD-009) | `valid_key + vec![0u8; extra_bytes]` |
| `extra_bytes in 0u8..=10u8` (workflow) | workflow test | proptest (VLD-009) | `valid_key + vec![0u8; extra_bytes]` |

### SIDEX-MAL-016 — ReservedSeqSentinel forbidden (PS-MAL-006)

| Variant | Verifier lane | Decision |
|---|---|---|
| `KeyDecodeError::ReservedSeqSentinel` | proptest (VLD-012) | Forbidden; side-index variants do not carry an `EventSeq` field. The contract explicitly excludes this variant from PO-008 coverage. |

### SIDEX-MAL-017 — `KeyCapacity` forbidden (PS-MAL-005)

| Variant | Verifier lane | Decision |
|---|---|---|
| `JournalError::KeyCapacity` | proptest (VLD-011) | Forbidden; `KeyCapacity` is the encoder-side error surfaced by `ArrayVec::try_push`, not the decoder-side error. The PO-008 docstring at `journal_side_index_contracts.rs:184` may be updated to reflect the decoder contract. |

### SIDEX-MAL-018 — Error vocabulary import path (PS-MAL-018)

| Import | Verifier lane | Decision |
|---|---|---|
| `KeyDecodeError` via `vb_storage::KeyDecodeError` (the public re-export at `lib.rs:202`) or `vb_storage::error::KeyDecodeError` | proptest (VLD-024) | Re-export path is the canonical import; no path-rewriting. |

## Coverage summary

| Category | Total proof seeds | proptest | verus | kani | flux-rs | loom | miri | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|
| Decoder-call requirement (SIDEX-MAL-001) | 1 | 1 ✅ | — | — | — | — | — | — |
| Strategies wired (SIDEX-MAL-002) | 3 | 3 ✅ | — | — | — | — | — | — |
| Reachable variant coverage (SIDEX-MAL-003) | 3 | 3 ✅ | — | — | — | — | — | — |
| Budget preserved (SIDEX-MAL-004) | 1 | 1 ✅ | — | — | — | — | — | — |
| File-level forbid(unsafe_code) (SIDEX-MAL-005) | 1 | 1 ✅ | — | — | — | — | — | — |
| No unwrap/expect (SIDEX-MAL-006) | 1 | 1 ✅ | — | — | — | — | — | — |
| No membership probe (SIDEX-MAL-007) | 1 | 1 ✅ | — | — | — | — | — | — |
| Bounded to one test file (SIDEX-MAL-008) | 0 (structural) | 1 ✅ | — | — | — | — | — | — |
| Field surfacing (SIDEX-MAL-009) | 1 | 1 ✅ | — | — | — | — | — | — |
| Per-variant InvalidRunId (SIDEX-MAL-010) | 2 | 2 ✅ | — | — | — | — | — | — |
| Within-family mismatch (SIDEX-MAL-011) | 1 | 1 ✅ | — | — | — | — | — | — |
| Empty-key branch (SIDEX-MAL-012) | 1 | 1 ✅ | — | — | — | — | — | — |
| Unknown-prefix branch (SIDEX-MAL-013) | 1 | 1 ✅ | — | — | — | — | — | — |
| Truncate bound (SIDEX-MAL-014) | 1 | 1 ✅ | — | — | — | — | — | — |
| Oversize slice (SIDEX-MAL-015) | 1 | 1 ✅ | — | — | — | — | — | — |
| ReservedSeqSentinel forbidden (SIDEX-MAL-016) | 1 | 1 ✅ | — | — | — | — | — | — |
| KeyCapacity forbidden (SIDEX-MAL-017) | 1 | 1 ✅ | — | — | — | — | — | — |
| Import path (SIDEX-MAL-018) | 1 | 1 ✅ | — | — | — | — | — | — |
| **Total** | **21** | **21** | **—** | **—** | **—** | **—** | **—** | **—** |

**Legend**:
- ✅ = active lane; obligation exists and is paired.
- — = not applicable; `verifier-lane-decisions.jsonl` carries an explicit
  `not_applicable` row with `non_applicability_evidence_refs` and a
  `limitation_kind` (see `proof-strategy.md#Default-Profile Verifier
  Decisions` for the not-applicable rationale).

## Cross-references

- `contract.md#SIDEX-MAL-001..018` — normative clauses.
- `proof-seeds.jsonl` — proof seeds `PS-MAL-001..020` (PS-MAL-019, PS-MAL-020
  are out_of_scope_followup for a future bead).
- `traceability-matrix.jsonl` — clause-to-source-line mapping.
- `verifier-lane-decisions.jsonl` — 24 `verifier-lane-decision/v1` rows.
- `proof-obligations.planned.jsonl` — 2 `proof-obligation/v1` rows.
- `proof-strategy.md#Default-Profile Verifier Decisions` — not-applicable
  rationale for verus / kani / flux-rs / loom / miri / cargo-fuzz.
