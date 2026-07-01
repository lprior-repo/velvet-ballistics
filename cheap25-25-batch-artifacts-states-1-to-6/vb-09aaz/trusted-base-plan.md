# Trusted Base Plan — vb-09aaz

bead_id: vb-09aaz
state: 4 (proof-planner)

## Overview

This document enumerates the surfaces that the proofs assume to be
correct without proving them inside this bead's obligations. Each
trusted surface is justified by either a type-system invariant, a
compile-time guarantee, a verified external crate, or an established
crate-level convention.

## Trusted Surfaces

### 1. Rust Standard Library (trusted — well-tested)

- `std::collections::HashSet::contains`, `HashSet::insert` — well-tested
  safe container; no panics on the G8 path.
- `std::option::Option::is_some`, `Option::unwrap_or` — pure functions.
- `std::result::Result::map_err`, `Result::?` propagation — exhaustive
  error handling.
- `std::cmp` — total ordering for u64/u32/u16 comparisons.
- `core::convert::identity` — used in the Verus mirror's `map_err` body.

**Justification**: Standard library, no unsafe code in this crate.

### 2. Compile-Time Enforced Constants (trusted)

- `INDEX_ACTION_KEY_BYTES = 13` (`crates/vb_storage/src/constants.rs:79`)
  — `pub const u8` value.
- `PREFIX_INDEX_ACTION = 0x32` (`crates/vb_storage/src/constants.rs:43`)
  — single byte prefix.
- `JOURNAL_KEY_BYTES = 17` (`crates/vb_storage/src/constants.rs`) — used
  in the staged_event_keys HashSet type.
- `MAX_BATCH_COUNT = 10_000` (`crates/vb_storage/src/constants.rs:100`)
  — used by the G4 guard.
- `MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576` (`constants.rs:88`).
- `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` (`batch/types.rs:10`).
- `ACTION_KIND_MAGIC_JOURNAL_EVENT = 0x4A45` — record framing magic.

**Justification**: These are `pub const` values; the Rust compiler
enforces their values at compile time. The Verus mirror inlines them
as `pub const SPEC_*` to avoid a `pub const` panic in `--crate-type=lib`
mode (documented in `vb-vzcuf-PS-008.rs:92-102`).

### 3. Type-System Enforced Invariants (trusted)

- `JournalWriteBatch<'j>` is `!Send + !Sync` via
  `PhantomData<*mut FjallJournal>` (`batch/types.rs:18-21`).
- `JournalError::KeyCapacity` is a unit variant at
  `error/mod.rs:28-29` with diagnostic code at
  `error/codes.rs:103, 196`. Fieldless.
- `JournalError::BatchAborted` is a unit variant at
  `error/mod.rs:42-43`. Fieldless.
- `ActionId(u16)`, `RunId(u64)`, `StepIdx(u16)` — bounded newtype
  wrappers from `vb_core`. Encoding to fixed-length bytes is
  total for these ranges.
- `SpecJournalWriteBatch` mirror struct in
  `verification/verus/extern_vb_vzcuf_PS_008.rs` and `_PS_009.rs` —
  the production-mirror abstraction.

**Justification**: The Rust type system prevents illegal states
(unrepresentable at compile time).

### 4. Fjall Database Substrate (trusted — external crate)

- `fjall::OwnedWriteBatch::insert`, `len`, `is_empty` — durable
  storage primitive. The `OwnedWriteBatch` is committed atomically by
  `FjallJournal::commit`.
- `fjall::Keyspace::contains_key`, `insert` — durable storage
  primitive. Used by `journal.events.contains_key` at append_event.rs:61.
- `FjallJournal::stage_pending_action_index_op` (at
  `batch/action_index.rs:106-126`) — internal helper. Returns
  `Err(JournalError::KeyCapacity)` on `index_action_key` overflow.
- The atomicity guarantee: an `OwnedWriteBatch` is either fully
  committed or fully dropped.

**Justification**: Fjall is the trusted persistence substrate. The
atomicity guarantee is the foundation for master §49
Crash-Consistency Rule compliance. The Verus mirror abstracts
Fjall as opaque (`#[verifier::external]`).

### 5. postcard Codec (trusted — external crate)

- `postcard::to_stdvec` / `postcard::from_bytes` — encode/decode
  primitives. Returns `Result` for malformed input; never panics.
- `encode_record(MAGIC_JOURNAL_EVENT, ...)` in the storage codec
  layer — the trusted wrapper for `JournalEvent` envelope encoding.
  Returns `Err(JournalError::Encode)` or
  `Err(JournalError::PayloadTooLarge { len, max })` on failure.

**Justification**: postcard is a well-established Rust crate. Its
error paths are typed and never panic on malformed input.

### 6. `index_action_key` Constructor (trusted — production key encoder)

- `keys::index_action_key(action: ActionId, run: RunId, step: StepIdx)
  -> Result<[u8; INDEX_ACTION_KEY_BYTES], JournalError>` at
  `crates/vb_storage/src/keys.rs:139-155`.
- Encoding layout: `[0x32][action u16 be][run u64 be][step u16 be]`
  = 1 + 2 + 8 + 2 = 13 bytes. For `ActionId(u16) × RunId(u64) ×
  StepIdx(u16)` inputs, the encoding always fits the fixed-length
  buffer; `KeyCapacity` is DEFENSIVELY REACHABLE but unreachable
  under nominal inputs.
- `try_push` / `try_extend_from_slice` / `into_inner` — ArrayVec
  primitives. Total functions over fixed-capacity buffers.

**Justification**: Pure function with bounded input range; encoding
fits exactly. The Verus mirror abstracts the key as an exec arg
(analogous to `encode_ok: bool` for G5), which is the established
abstraction pattern.

### 7. Verus Production Mirror Pattern (trusted — established convention)

- `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`
  — drift-gated mirror at `crates/vb_storage/src/batch/append_event.rs:1-110`.
- `verification/verus/production_inner/vb_vzcuf_PS_009_production.rs`
  — secondary drift-gated mirror.
- `verification/verus/extern_vb_vzcuf_PS_008.rs` and
  `verification/verus/extern_vb_vzcuf_PS_009.rs` — extern surface
  binding via `#[path = "..."]`.
- DRIFT POLICY header at PS-008 L5-14 and PS-009 L5-32 — explicit
  regeneration contract.

**Justification**: The drift-gate mechanism (`scripts/check-production-inner-drift.sh`,
zero tolerance) is the binding enforcement. The production-binding gate
(`scripts/check-verus-production-binding.sh`) is mandatory per
AGENTS.md.

## Model Reductions and Assumptions

### Bounded State Space

- `MAX_BATCH_COUNT = 10_000` — hard limit per batch.
- `INDEX_ACTION_KEY_BYTES = 13` — fixed-length encoding.
- `JOURNAL_KEY_BYTES = 17` — fixed-length encoding.
- `byte_limit = Some(1_048_576)` — bounded byte budget per batch.

**Justification**: These are compile-time constants; the proofs use
them as the upper bounds of any unbounded iteration.

### No Concurrency

- `JournalWriteBatch<'j>` is `!Send + !Sync` (PhantomData<*mut FjallJournal>).
- The queued-writer path's `OwnedWriteBatch` is single-shot.
- The direct-path `append_unfsynced` builds a fresh batch and commits
  or drops in the same function.

**Justification**: No concurrent aliasing is possible; no loom or
scheduling proof required.

### No Unsafe / No FFI

- All scoped files carry `#![forbid(unsafe_code)]`. Zero unsafe
  blocks, no FFI, no raw pointers.

**Justification**: No miri / unsafe proof required.

### KeyCapacity Reachability (Defensive)

- Production `index_action_key` cannot fail for nominal inputs
  (`ActionId(u16) × RunId(u64) × StepIdx(u16)` fits 13 bytes exactly).
- The abort-on-fallible-step invariant is unconditional: even for
  paths that are practically unreachable, the contract requires
  `aborted = true` on `Err`.

**Justification**: This is the established defensive-reachability
contract in the codebase (mirrors the durable-duplicate guard G3
which is unreachable in normal operation but aborts unconditionally
on its `Err` path).

## Reduction Justification

The Verus mirror abstracts production as:

- `key: u64` for the journal event key (G1 abstracted as exec arg).
- `journal_has_key: bool` for the durable-duplicate lookup (G3
  projected).
- `encode_ok: bool` + `encoded_len: u64` for the codec step (G5
  projected).
- `byte_limit: Option<u64>` for the byte-admission guard (G6).
- `index_key_ok: bool` (NEW for G8) for the index-action-key step.

The mirror is sound because:

1. Each abstraction preserves the witness precondition required by
   the `assume_specification` match arm.
2. State-preservation predicates (`spec_state_preserved`,
   `spec_state_preserved_except_aborted`,
   `spec_state_after_ok`) capture the full observable mutation
   surface.
3. The exec wrapper at the bottom of PS-008/PS-009 exercises the
   bridge from `verus!` context, proving the contract is not used as
   a vacuum.
4. The drift-gate header enforces mirror regeneration on production
   change.

## Known Assumptions and Stub Boundaries

| Assumption | Location | Justification |
| --- | --- | --- |
| `index_action_key` cannot fail for nominal inputs | `keys.rs:139-155` | 13-byte fixed-length encoding; u16/u64/u16 inputs always fit |
| `commit()` short-circuits when `aborted == true` | `commit.rs:20-23` | existing mechanism; unchanged by fix |
| `JournalError::KeyCapacity` is a unit variant | `error/mod.rs:28-29` | established error taxonomy |
| `FjallJournal::stage_pending_action_index_op` returns `Err(KeyCapacity)` on `index_action_key` overflow | `batch/action_index.rs:106-126` | production helper; unchanged by fix |
| `OwnedWriteBatch` is atomic at commit | fjall crate | substrate contract |
| `SpecJournalWriteBatch` mirror is byte-for-byte equal to production field set | extern_vb_vzcuf_PS_008.rs binding ledger | drift-gate enforced |
| Drift-gate header is honored on every production change | `vb_vzcuf_PS_008_production.rs:5-14` | explicit regeneration contract |
| `assume_specification` is the canonical proof artifact | `vb-vzcuf-PS-008.rs:180-225` | established Verus pattern |

## Non-Behavior Waivers

None. All trusted surfaces are justified by either type-system
invariants, compile-time constants, established external crates, or
drift-gated mirrors.

The proof obligations in this plan do not introduce any
behavior-affecting waiver. The plan emits no waiver for the G8
abort invariant — the production fix is mandatory and the
verification is required.

## Trusted-Base Drift Watch

The proof-writer at State 5 must monitor:

1. Drift-gate header in PS-008 production mirror — any drift
   requires regeneration before re-verification.
2. Drift-gate header in PS-009 production mirror — same.
3. Production-binding gate (`scripts/check-verus-production-binding.sh`)
   — must pass on every CI run; failure is a hard rejection per
   AGENTS.md.
4. Mirror drift gate (`scripts/check-production-inner-drift.sh`) —
   zero tolerance; any drift fails the gate.