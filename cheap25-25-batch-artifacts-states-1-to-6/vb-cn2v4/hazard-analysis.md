# Hazard Analysis: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Rust-Core Invariant Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Encoder returns `Ok(bytes)` for `run == 0`. | The same module's decoder refuses to round-trip those bytes; persistence becomes asymmetric. | `require_non_zero_run(run)?` fires first in every run-bearing encoder; no path may emit `run == 0` bytes. |
| Guard skipped for one of the three `index_*` encoders. | `index_status_key` / `index_workflow_key` / `index_action_key` emit `run == 0` bytes that the decoder refuses. | Explicit `require_non_zero_run(run)?` call required in each of `index_status_key`, `index_workflow_key`, `index_action_key` at the top of the body (these fns bypass `run_only_key` / `sequenced_run_key`). |
| Guard order swapped (after `to_u8_checked` or after `seq == u64::MAX`). | `RunId(0)` inputs return `IndexStatusStateCollision` or `SequenceOverflow` instead of `InvalidRunId`. | The contract pins the order: `require_non_zero_run` is the FIRST check inside every run-bearing encoder body; `to_u8_checked` runs second; `seq == u64::MAX` runs second in `sequenced_run_key` (and is the only check currently there, so the new guard becomes the second check after the seq test). |
| `headers.rs::run_header` manual check accidentally removed without confirmation. | Behaviour-equivalent — encoder now rejects — but the test suite's title `invalid_key_prefix_returns_typed_error` in `storage_contract_pack_runner.rs:151-168` may rely on the manual-check path for a clearer error context. | The contract permits either choice; whichever choice is made must keep the production test `invalid_key_prefix_returns_typed_error` (renaming is suggested in the codebase map for clarity). |
| Test suite not flipped in lockstep with production change. | Tests that assert `Ok(...)` for `RunId::new(0)` start failing after the encoder rejects. | The 18-test flip list in `workflow-model.md` § Test Workflow Flips is mandatory and must happen in the same change. |

## Temporal Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Append path returns `Err(InvalidEvent)` from `is_valid()` for `RunId(0)` today; tomorrow it returns `Err(InvalidRunId)` from the encoder. | Downstream callers that pattern-match on `InvalidEvent` for "RunId(0 was passed" diagnostics will no longer fire; they must broaden to include `InvalidRunId`. | The contract documents this shift; downstream tests that explicitly assert `InvalidEvent` for `RunId(0)` must be flipped. No call site silently swallows the old error. |
| Encoding-after-rejection ordering in `sequenced_run_key`. | If `run == 0` and `seq == u64::MAX`, the older `seq`-first ordering returns `SequenceOverflow`; the new ordering returns `InvalidRunId`. | The contract pins the order: `seq` check first (unchanged), `run` check second (NEW). This keeps `SequenceOverflow` reachable for `RunId != 0` cases. |

## Bounded-State Hazards

- The `ArrayVec<u8, N>` bounds inside every encoder (e.g.
  `ArrayVec::<u8, JOURNAL_KEY_BYTES>`) are unreachable when the new
  guard fires first, so the `KeyCapacity` error variant remains
  unreachable in normal use. No new bounded-state risk is
  introduced.
- The `IndexStatusStateCollision` variant is unreachable for
  `RunId(0)` (the new guard fires first); it remains reachable for
  `RunId != 0` with `Other(v < 3)`. No bounded-state risk is
  introduced.

## Concurrency / Scheduling Hazards

- The encoder is purely synchronous; no scheduling hazards.
- No locks, atomics, channels, or task ownership are involved.
- Loom is NOT required for this bead (no concurrency in the
  encoder paths).

## Hostile / Invalid Input Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Persisted keyspace contains `RunId(0)` rows from before the encoder was tightened. | Decoder refuses them; existing scan paths that hit a `RunId(0)` row surface `KeyDecodeError::InvalidRunId` (decoder-side). The encoder tightening itself cannot produce such rows, so the asymmetric gap closes going forward. | The contract notes that no such rows exist (decoder was already strict). If a future migration imports a foreign keyspace, decoder-side rejection is the contract's defence. |
| Fuzz target `fuzz_storage_keys` (if any) feeding `RunId(0)` to every typed encoder. | The encoder must surface `InvalidRunId` for every run-bearing variant. | The contract pins this behaviour; a fuzz target is a natural verification surface. |
| Proptest range `run_val in 1u64..=1000u64` already excludes zero in `all_key_functions_are_deterministic`. | A new property is needed: `encoder_rejects_zero_run_id_for_every_prefix` covering all six run-bearing variants. | Out of scope for this state; flagged for test-planner. |

## Unsafe / Provenance Hazards

- No `unsafe` in the encoder paths (`#![forbid(unsafe_code)]` at
  `keys.rs:1`). Miri is NOT applicable.

## Storage / Codec Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Verus mirror `SpecKeyEncodeError` omits `InvalidRunId { run: u64 }`. | Production rejection has no Verus contract coverage; the mirror's assume_specification clause would either have to lie or be missing. | The contract REQUIRES the new `SpecKeyEncodeError::InvalidRunId` variant and the assume_specification clauses for `run_event_key`, `journal_key`, `encode_key`. |
| Production-binding drift between `SpecKeyEncodeError` and `JournalError::InvalidRunId`. | Verus mirror diverges from production; `scripts/check-verus-production-binding.sh` fails. | The contract requires the mirror to either include `#[path = ".../crates/vb_storage/src/error/mod.rs"]` for `JournalError` (STRONG), or to use a drift-gated mirror (WEAK), or to register an `ALLOWED_EXCEPTIONS` row with PO-XXXX. The binding ledger entry must be updated. |
| Mirror drift at `production_inner/vb_storage_keys_production.rs:79-80`. | Production comment block lists `run_event_key` as a blocker; the new rejection must surface in the comment. | The contract requires the comment block to mention the `InvalidRunId` rejection for completeness. |

## Performance Hazards

- The new guard is a single integer compare (`run.get() == 0`) per
  encoder call. The compare is on a value already in a register
  from the function argument; branch-prediction eliminates the
  cost in the hot path (always non-zero). No measurable overhead.
- No allocation introduced.
- No new heap pressure, no new locking, no new syscalls.

## Release / API Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Public API signature of `run_header_key`, `run_event_key`, `run_snapshot_key`, `index_status_key`, `index_workflow_key`, `index_action_key`, `encode_key_into`, `encode_key`, `journal_key`, `run_prefix_key` changes `Result` semantics for `RunId(0)`. | Downstream crate callers (currently in-crate only — `vb_storage` itself) that pattern-match on `Ok(...)` for `RunId(0)` will start receiving `Err(InvalidRunId)`. | The contract enumerates all in-crate callers and accepts the shift. CLI / external callers that passed `RunId(0)` to the public API now get a typed error (the same behaviour `headers.rs::run_header` already enforces manually). |
| Diagnostic code `0x4021` collides with a future allocation. | Already allocated and registered; no collision. | No change to the code registry. |
| Behaviour change is observable in tests. | 18 tests must be flipped in lockstep with production. | The flip list in `workflow-model.md` § Test Workflow Flips is mandatory. |

## Remaining Illegal-State Risks (Representable Today, After This Bead)

- The `headers.rs::run_header` manual check is now redundant; the
  contract permits removal but also permits keeping it. Either is
  legal. The chosen shape must be documented in the implementation
  agent's report.
- `RunId::ZERO` remains constructable and continues to be used as
  a `NoRecoveryData` placeholder. The encoder tightening does not
  affect those callers because they never reach a key encoder.
- The `IndexStatusStateCollision` and `SequenceOverflow` errors
  remain reachable for `RunId != 0` inputs; their diagnostic
  codes (`0x4017` and `0x400A` respectively) and symbolic names
  are unchanged.
- The Verus mirror's `SpecKeyEncodeError::InvalidRunId { run: u64
  }` carries the raw `u64`, not a `SpecRunId` newtype. The
  decoder-mirror `SpecKeyDecodeError::InvalidRunId` carries no
  payload. This asymmetry mirrors production and is acceptable.

## Hazards NOT Applicable to This Bead

- **Concurrency / Loom:** no concurrency in encoder paths.
- **TLA+ temporal models:** the encoder tightening is a pure
  function tightening; no temporal workflow to model. (Master
  rules: TLA+ is removed from the go-skill lifecycle.)
- **Miri:** no `unsafe` in encoder paths.
- **Distributed / replication:** not affected.
- **Auth / security:** not affected.