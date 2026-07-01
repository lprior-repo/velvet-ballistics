# Proof Strategy — vb-jtqqx (P1 test-only repair of side-index malformed-key proptests)

## Bead Summary

**Bead**: vb-jtqqx — "Tests: make side-index malformed-key tests decode malformed keys (P1)"
**State**: 1 → 4 (planning)
**Scope**: Test-only repair bounded to one file:
`crates/workspace_tests/tests/journal_side_index_contracts.rs` lines 183-257 (PO-008 proptest block).
**Decoder under test (read-only)**: `vb_storage::keys::decode_storage_key` at
`crates/vb_storage/src/keys.rs:346-434` and `vb_storage::keys::try_key_prefix` at
`crates/vb_storage/src/keys.rs:281-295`.

**Forbidden edits (per contract.md SIDEX-MAL-008)**:
- Must NOT modify `vb_storage/src/keys.rs` (the decoder).
- Must NOT modify the `PREFIX_INDEX_STATUS = 0x30` / `PREFIX_INDEX_WORKFLOW = 0x31` /
  `PREFIX_INDEX_ACTION = 0x32` constants.
- Must NOT modify `Cargo.toml`, `Cargo.lock`, dependency manifests, or
  `crates/vb_storage/**` source.

## Strategy Overview

This is a **test-only P1** repair. The three PO-008 proptests
(`index_action_key_decode_error_on_short_input`,
`index_status_key_decode_error_on_wrong_length`,
`index_workflow_key_decode_error_on_wrong_length`) currently construct
malformed byte sequences but discard them; the only assertions are against
the *valid* key's length. The repair rewires the existing proptest
strategies (`truncate_len`, `_extra_bytes`, `_short_key`) into
`decode_storage_key` (and `try_key_prefix`) calls and asserts on the typed
`KeyDecodeError` variants.

**Proof principle**: The cheapest verifier that proves the obligation is
the proptest framework itself — the test bodies ARE the proof. No
Kani/Verus/Flux/fuzz obligations are in scope for this P1 because no
production source changes are made and the decoder is already exercised
by the canonical fixture pattern at `crates/vb_storage/src/preview/tests.rs:111-180`.

**Three evidence lanes** (per the bead prompt's `cargo-test / proptest /
source-lint` framing, mapped to the proof-planner's verifier taxonomy):

| User-facing lane | Verifier in obligation | Evidence command | What it proves |
|---|---|---|---|
| `cargo-test` | `proptest` | `cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` | The three PO-008 proptests run and pass; 128 cases per proptest. |
| `proptest` | `proptest` | `cargo nextest run ... -- index_action_key_decode_error_on_short_input` (and the two named siblings) | Each proptest body invokes `decode_storage_key` (or `try_key_prefix`) on a crafted malformed payload and asserts on a typed `KeyDecodeError` variant. |
| `source-lint` | `proptest` (Holzman-Rust gate) | `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings` | The PO-008 block has no `unwrap` / `expect` / `panic` / `todo` / `unimplemented` / `dbg!`; `#![forbid(unsafe_code)]` is preserved; `JOURNAL_KEY_PROPTEST_CASES = 128` is preserved. |

The two `proof-obligation/v1` rows below are both `verifier: proptest` —
one for decoder-rejection coverage, one for structural preservation
(Holzman-Rust + budget + lint).

## Anti-laundering posture

Per the skill's GOD RULES and the contract's forbidden-edits list:

- **No production source changes**: the decoder at `keys.rs:346-434` is
  read-only. No Kani harness, no Verus spec, no Flux refinement is
  planned. A future Kani scope-up is captured in `proof-seeds.jsonl` as
  `PS-MAL-019` (out of scope for vb-jtqqx).
- **No vacuum Verus proofs**: No Verus obligations are created. If a
  future bead requires Verus, the production-binding gate
  (`scripts/check-verus-production-binding.sh`) will require
  `mechanism: STRONG` (or one of the two `WEAK_*` mechanisms); the
  current bead does not need this gate because no Verus obligations
  exist.
- **No proptest discard with `_`**: Contract clause SIDEX-MAL-002
  forbids the leading-underscore pattern; the repair wires every
  declared strategy into the malformed-payload constructor.
- **No `unwrap` / `expect` / `panic` in the PO-008 block**: the decoder
  is allowed to return `Err`; the test must not panic on that path
  (H-MAL-004 / PS-MAL-007).

## Risk Class Mapping

The seed-level risk tags are mapped to the proof-planner's
`DEFAULT_RISK_PROFILE` taxonomy as follows:

| Seed-level risk tag(s) | proof-planner `risk` | Default profile requires |
|---|---|---|
| `malformed-key`, `decoder`, `proptest`, `fail-closed` | `rejection` | `kani` + `proptest` |
| `malformed-key`, `proptest`, `test-only` | `rejection` | `kani` + `proptest` |
| `decoder`, `invalid-run-id`, `proptest` | `rejection` | `kani` + `proptest` |
| `typed-error`, `decoder` (no behavioral change) | (no default-profile risk; structural obligation) | (none — covered by proptest) |
| `holzman-rust`, `proptest` | `panic_freedom` | `verus` + `kani` + `proptest` |
| `holzman-rust`, `lint` | `panic_freedom` | `verus` + `kani` + `proptest` |
| `typed-error`, `decoder` (field surfacing) | `field_sensitivity` | `proptest` + `kani` |
| `test-only`, `constants` | (no default-profile risk; structural obligation) | (none — covered by proptest) |
| `test-only` (budget) | (no default-profile risk; structural obligation) | (none — covered by proptest) |

Default-profile verifiers that are **not applicable** to this test-only
P1 repair are explicitly enumerated in `verifier-lane-decisions.jsonl`
with concrete `non_applicability_evidence_refs` and a `limitation_kind`
of `surface_absent` or `superseded_by_other_lane_with_evidence`. The
non-applicable decisions are not silent omissions.

## Obligation-by-Obligation Strategy

### PO-MAL-001 — Decoder-rejection coverage of all four reachable `KeyDecodeError` variants

**Verifier**: `proptest`
**Risk**: `rejection`
**Risk tags**: `malformed-key`, `decoder`, `proptest`, `fail-closed`
**Domain claim**: Each of the three PO-008 proptest bodies invokes
`vb_storage::keys::decode_storage_key` (or `vb_storage::keys::try_key_prefix`)
against at least one crafted malformed byte sequence and asserts on a
typed `KeyDecodeError` variant via `prop_assert!(matches!(...))`. Across
the three tests, the four reachable variants `EmptyKey`,
`UnknownPrefix`, `KeyLengthMismatch` (with multiple `actual` lengths and
multiple `prefix` bytes), and `InvalidRunId` (one per side-index variant)
are each exercised at least once. `ReservedSeqSentinel` is forbidden
because the three side-index variants do not carry an `EventSeq` field
(SIDEX-MAL-016 / H-MAL-003).

**Per-test shape (per `error-taxonomy.md#Per-test required shapes`)**:

| Test | Required shapes |
|---|---|
| `index_action_key_decode_error_on_short_input` | (1) truncated slice of valid 13-byte key → `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: <short_len> }`; (2) 13-byte zero-run payload → `InvalidRunId` (exercises `keys.rs:423-425`); (3) within-family-mismatch `vec![0x30; 13]` → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 }`; (4) empty slice → `EmptyKey` (via `try_key_prefix`, exercising `keys.rs:282`). |
| `index_status_key_decode_error_on_wrong_length` | (1) oversize key `valid_key + vec![0u8; _extra_bytes]` → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + _extra_bytes }`; (2) truncated 17-byte key → `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 17 }`; (3) 18-byte zero-run payload → `InvalidRunId` (exercises `keys.rs:400-402`); (4) within-family-mismatch `vec![0x32; 18]` → `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 }`. |
| `index_workflow_key_decode_error_on_wrong_length` | (1) oversize key → `KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + _extra_bytes }`; (2) truncated 13-byte key (requires adding a `truncate_len` strategy) → `KeyLengthMismatch { prefix: 0x31, expected: 13, actual: <short_len> }`; (3) 13-byte zero-run payload → `InvalidRunId` (exercises `keys.rs:412-414`); (4) unknown prefix `vec![0xFF; 13]` → `UnknownPrefix { prefix: 0xFF }`. |

**Artifact target**: `crates/workspace_tests/tests/journal_side_index_contracts.rs:183-257`
**Test functions**: `index_action_key_decode_error_on_short_input`,
`index_status_key_decode_error_on_wrong_length`,
`index_workflow_key_decode_error_on_wrong_length`.

**Why proptest (not Kani / Verus / Flux / fuzz)**:
- The test bodies are themselves proptests; `cargo nextest run` is the
  natural execution surface.
- The decoder is read-only (not modified by this P1); a Kani harness on
  `decode_storage_key` is flagged in `PS-MAL-019` as out-of-scope
  follow-up work for a future bead.
- No arithmetic / index-safety / deep invariants warrant Verus or Flux.
- Proptest's `JOURNAL_KEY_PROPTEST_CASES = 128` budget provides
  randomized coverage at the right granularity for a test-only repair.

**Command**:
```
cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts
```

Plus per-test single-filter runs:
```
cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts -- index_action_key_decode_error_on_short_input
cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts -- index_status_key_decode_error_on_wrong_length
cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts -- index_workflow_key_decode_error_on_wrong_length
```

**Expected evidence**:
- Each proptest reports `test result: ok` (proptest-shape).
- Per-proptest case count is exactly `JOURNAL_KEY_PROPTEST_CASES = 128`.
- The proptest shrinks at least once (via `failure_persistence: None`
  the shrink output goes to stderr); the minimal failing case
  demonstrates that the malformed payload actually reaches the
  decoder.

### PO-MAL-002 — Structural preservation (Holzman-Rust + budget + lint)

**Verifier**: `proptest` (Holzman-Rust gate as part of the proptest contract)
**Risk**: `panic_freedom`
**Risk tags**: `holzman-rust`, `lint`, `test-only`, `proptest`
**Domain claim**: The PO-008 block satisfies the structural
invariants: `JOURNAL_KEY_PROPTEST_CASES = 128` is preserved at
`journal_side_index_contracts.rs:23`; `#![forbid(unsafe_code)]` is
preserved at `journal_side_index_contracts.rs:14`; the `proptest!`
macro framing is preserved; no `unwrap` / `expect` / `panic` /
`todo` / `unimplemented` / `dbg!` is added; the `truncate_len`,
`_short_key`, and `_extra_bytes` strategies are wired into the
malformed-payload constructor (no `_`-discard); the
`KeyLengthMismatch` field surfacing (`prefix: <actual_prefix>`) is
field-checked per the error-taxonomy contract.

**Artifact target**: `crates/workspace_tests/tests/journal_side_index_contracts.rs:14, 23, 183-257`

**Command**:
```
cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings
```

**Expected evidence**:
- `clippy` exits 0 with `-D warnings` (no warnings, no clippy::unwrap_used,
  no clippy::expect_used, no clippy::panic — all forbidden in production
  code AND test code per Holzman-Rust).
- The `JOURNAL_KEY_PROPTEST_CASES` constant is unchanged at `128` at
  line 23.
- The `truncate_len` / `_extra_bytes` strategies on lines 200, 226, 245
  are read by the body (the leading-underscore `_` prefix is removed
  for `_extra_bytes`; `_short_key` becomes `short_key`).
- `KeyLengthMismatch` assertions pattern-match on the field
  `prefix: <actual_prefix_byte>` (not on `prefix: <expected_prefix>`).

**Why this is a separate obligation, not folded into PO-MAL-001**:
- PO-MAL-001 is about *what* the proptest asserts (decoder rejection).
- PO-MAL-002 is about *how* the proptest is structured
  (Holzman-Rust / zero-tolerance lint / budget / no-discard strategies).
  Splitting them lets `proof-reviewer` failure-isolate a missing
  assertion (PO-MAL-001) from a forbidden `unwrap` (PO-MAL-002).

## Default-Profile Verifier Decisions (not_applicable)

The following default-profile verifiers are **explicitly not_applicable**
to this test-only P1 repair. Each `verifier-lane-decision/v1` row
includes `non_applicability_evidence_refs` with concrete file:line or
SHA references and a `limitation_kind`:

| Verifier | Risk class | Limitation | Evidence ref |
|---|---|---|---|
| `verus` | `panic_freedom`, `rejection` | `surface_absent` | No production source change; decoder at `keys.rs:346-434` is read-only. No deep mathematical invariant in scope. The contract `decode_storage_key → KeyDecodeError` is already bound at the type level. |
| `kani` | `rejection`, `panic_freedom` | `surface_absent` | The decoder is a pure `match`-based function with no loops, recursion, or indexing; Kani on a `match`-only decoder is an exercise in pattern exhaustiveness, not a security-grade proof. A future Kani harness is flagged in `PS-MAL-019` for a future bead. |
| `flux-rs` | `rejection`, `panic_freedom` | `risk_out_of_scope` | No refinement types in scope. The contract uses `KeyDecodeError` directly; no `flux_rs::refined_by::*` annotations. Flux would add zero coverage for a test-only repair. |
| `loom` | `concurrency_interleaving`, `cancellation_safety` | `surface_absent` | The PO-008 proptest bodies are single-threaded; no `Arc`, `Mutex`, channels, or `Send`/`Sync` markers in the test scope. Proptest seeds run sequentially on the proptest framework's scheduler. |
| `miri` | `ub_safety` | `surface_absent` | The test file has `#![forbid(unsafe_code)]` at line 14; the decoder has `#![forbid(unsafe_code)]` at `keys.rs:1`. Zero `unsafe` blocks, FFI, raw pointers, or `MaybeUninit` in scope. Miri on safe-only code is a no-op. |
| `cargo-fuzz` | `hostile_input`, `parse_canonicalization` | `superseded_by_other_lane_with_evidence` | Proptest's `JOURNAL_KEY_PROPTEST_CASES = 128` budget already provides randomized malformed-payload coverage for the three side-index variants. A cargo-fuzz target on `decode_storage_key` is flagged in `PS-MAL-020` as future-bead work; the fixture at `crates/vb_storage/src/preview/tests.rs:111-180` already exercises the parser through a real `KeyspaceScanPolicy` path. |

These decisions are written out in full in `verifier-lane-decisions.jsonl`.

## Execution Order

```
State 4 (this plan):
  - Plan PO-MAL-001 (proptest decoder-rejection)
  - Plan PO-MAL-002 (proptest structural preservation)
  - Document not_applicable decisions for verus/kani/flux/loom/miri/cargo-fuzz

State 4b (proof-plan-reviewer):
  - Disposition per lane decision
  - Reject missing contracts, weak commands, shallow bounds

State 5 (proof-writer — N/A, no proof code):
  - No Kani/Verus/Flux/loom/fuzz artifacts to write for this P1
  - The "proptest" verifier lanes point to existing proptest bodies
    that holzman-rust will rewrite

State 5b (test-writer / holzman-rust — owner of repair):
  - Rewrite the three PO-008 proptest bodies per
    `error-taxonomy.md#Per-test required shapes` and
    `type-contracts.md#Required Test Body Shape`
  - Run `cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` until green

State 7 (proof-to-implementation — bridge map):
  - Map PO-MAL-001 → test file lines 195-218, 222-238, 242-256
  - Map PO-MAL-002 → test file lines 14, 23, 183-257

State 12 (formal-verifier — closure):
  - Execute `cargo nextest run`; collect raw output
  - Execute `cargo clippy ... -D warnings`; collect raw output
  - Emit verification-ledger.jsonl rows
```

## Verifier Distribution Summary

| Verifier | Obligations | Coverage |
|---|---|---|
| proptest | PO-MAL-001, PO-MAL-002 | Decoder rejection + structural preservation |
| verus | (none) | Not applicable — read-only decoder, no deep invariants |
| kani | (none) | Not applicable — see PS-MAL-019 for future scope-up |
| flux-rs | (none) | Not applicable — no refinement types in scope |
| loom | (none) | Not applicable — single-threaded proptest bodies |
| miri | (none) | Not applicable — `#![forbid(unsafe_code)]` |
| cargo-fuzz | (none) | Not applicable — see PS-MAL-020 for future scope-up |

## Risk Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Proptest case budget drift | A proptest budget increase from 128 to a larger number could mask shrinking failures | PO-MAL-002 explicitly checks `JOURNAL_KEY_PROPTEST_CASES` constant is preserved; the constant is at line 23, not parameterized by proptest config. |
| Field-level `KeyLengthMismatch` assertion drift | A test that asserts `prefix: 0x32` against `vec![0x30; 13]` will silently mismatch on the wrong field | PO-MAL-001's `error-taxonomy.md#Variant Field Assertions` table specifies the actual `prefix` byte per shape; H-MAL-008 / PS-MAL-011 is closed by the repair. |
| `truncate_len` bound widening | Widening `1u8..=12u8` to `0u8..=13u8` causes the truncated slice to be empty, mapping to `EmptyKey` not `KeyLengthMismatch` | PO-MAL-001's truncated-slice shape is bounded per SIDEX-MAL-014 / H-MAL-009 / PS-MAL-012. The existing `1u8..=12u8` is correct; the workflow test's new `truncate_len` must use an analogous bound. |
| `_extra_bytes` strategy discarding | The leading underscore allows the strategy to be unused, restoring the original bug | PO-MAL-002 enforces `no _-discard strategies` per SIDEX-MAL-002 / H-MAL-006 / PS-MAL-009. The repair removes the leading underscore. |
| Per-variant `InvalidRunId` coverage gap | A test that asserts `InvalidRunId` only on the action variant leaves workflow/status branches unexercised | PO-MAL-001 requires a zero-run payload for each of the three side-index variants per SIDEX-MAL-010 / H-MAL-010 / PS-MAL-013. |
| ReservedSeqSentinel forbidden assertion | An over-eager reviewer could ask for `ReservedSeqSentinel` coverage | PO-MAL-001 explicitly excludes `ReservedSeqSentinel` per SIDEX-MAL-016 / H-MAL-003 / PS-MAL-006 — it is unreachable from side-index payloads. |
| Membership-probe routing | Routing the malformed payload through `FjallJournal::has_*_index_entry` instead of `decode_storage_key` | PO-MAL-001's command targets `decode_storage_key` directly per SIDEX-MAL-007 / H-MAL-005 / PS-MAL-008. The PO-008 block must not call `temp_journal()` or any membership probe. |
| `KeyCapacity` (encoder-side error) | Stale vocabulary referring to `JournalError::KeyCapacity` instead of `KeyDecodeError` | PO-MAL-001's vocabulary is `KeyDecodeError` per SIDEX-MAL-017 / H-MAL-002 / PS-MAL-005. The docstring at line 184 may be updated to reflect the decoder contract. |
