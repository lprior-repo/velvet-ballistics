# Boundary Map — vb-uwxct

This bead is TEST-ONLY. The boundary map below identifies the **layer
separation** between production code (which is NOT modified) and the test
specimens (which are the only edited artifacts). Each boundary is named with
its location, its purity/IO profile, and the legal direction of flow.

## Production Layer (untouched, reference only)

### Pure core — vb_storage keys

| Boundary | Path | Profile | Purity | Touched? |
|---|---|---|---|---|
| `sequenced_run_key` (private) | `crates/vb_storage/src/keys.rs:480-496` | pure core | deterministic, no I/O, no time, no allocator, no panic-path | NO |
| `journal_key` (private) | `crates/vb_storage/src/keys.rs:476-478` | pure core | deterministic, no I/O | NO |
| `run_event_key` (public) | `crates/vb_storage/src/keys.rs:81-83` | public API surface | deterministic | NO |
| `run_snapshot_key` (public) | `crates/vb_storage/src/keys.rs:85-91` | public API surface | deterministic | NO |
| `JournalError::SequenceOverflow` (variant) | `crates/vb_storage/src/error/mod.rs:69-70` | error type | unit variant | NO |

### Verifier mirror — Verus (reference only)

| Boundary | Path | Note |
|---|---|---|
| `SpecKeyEncodeError::SequenceOverflow` mirror | `verification/verus/extern_vb_storage_keys.rs:200,280,420` | Spec mirror retains variant; out of immediate scope |
| `journal_event_sequence_overflow_*` (PS_001/PS_002) | `verification/verus/production_inner/vb_vzcuf_PS_001_production.rs:144`, `…_PS_002_production.rs:179` | Mirror drift-catched by `scripts/check-production-inner-drift.sh`; out of scope |

## Test Layer (edited by this bead)

### Pure black-box boundary — proptests

The six proptests live in `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs`.
Each proptest consumes the public encoder surface (`run_event_key`) with
arbitrary `u64` inputs and asserts on the result. The flow is:

```
proptest!() arbitrary u64 sample
  |  (boundary)
  v
[prop_assume! / constraint] ── skip illegal inputs
  |  (boundary)
  v
run_event_key(RunId, EventSeq) ── production call, no mutation of production
  |  (boundary)
  v
Result<[u8; 17], JournalError> ── Result inspection
  |  (boundary)
  v
prop_assert!(...) ── property assertion (no panic on Ok)
  v
```

### Symbolic / model-checked boundary — Kani harness

The Kani harness lives in `crates/vb_storage/src/kani_typed_partitioned_ids.rs`.
It uses symbolic `u16` bitfields (hi/lo pairs to recover a packed `u64`)
to exercise `run_event_key` over a wide but constrained space. The flow is:

```
kani::any() ── SymbolicKeyInputs
  |  (boundary — symbolic)
  v
run_raw / seq_raw ── bitfield packing (deterministic)
  |  (boundary)
  v
run_event_key(RunId, EventSeq) ── production call
  |  (boundary)
  v
match Ok / Err ── typed-error discrimination (no `assert!(false)` blanket)
  v
```

The Kani harness is a special case: `kani::any()` cannot cheaply exclude
`seq == u64::MAX`, so the choice between `kani::assume(seq_value != u64::MAX)`
and explicit typed-error discrimination is a domain decision documented in
`type-contracts.md` §4.2.

## Async / Imperative Shell

This bead has **no async layer**. `run_event_key` is synchronous pure core.
There are no `async fn`, no `await`, no `tokio`/`futures` APIs, no channels.

## Storage / Network / Time / FFI / Unsafe

| Concern | Status in this bead |
|---|---|
| Fjall LSM | not touched; specimens only inspect return values |
| network | none |
| filesystem | none |
| clock/time | none |
| FFI | none |
| `unsafe` | forbidden by `#[forbid(unsafe_code)]` at file tops; not added |

## Hostile Input Boundaries

The proptest inputs are by definition hostile (full `u64` range). The repair
form recognizes that the encoder is well-defined only on `seq ∈ 0..u64::MAX`,
so the constraint `0u64..u64::MAX` (or equivalent `prop_assume!`) is itself
the hostile-input guard. After the bead, no specimen will panic on hostile
input — hostile input is either skipped or vacuously accepted.

## Test → Production Boundary

| Direction | Allowed? | Concrete checks |
|---|---|---|
| test → production | YES (read-only call) | `run_event_key(...).ok()` |
| production → test | NO | The encoder does not depend on test code; tests do not appear in production dependencies |
| test → test (intra-file) | YES | `event_key_seq_bytes` helper at line 1310+ (byte-slice ordering helper) |
| production spec → production (mirror) | YES, gated | `verification/verus/extern_vb_storage_keys.rs` mirrors production under drift gate |

## Encapsulation Rules

1. Production code is **encapsulated** — out of reach for this bead.
2. Verifier specs are **encapsulated** unless the Kani harness returns a
   `cover!` claim requiring a Verus update; in that case the bridge goes via
   `proof-to-implementation`, NOT here.
3. The test layer is the **only edit surface**. All edits stay inside the
   seven specimen source spans identified in
   `.beads/vb-uwxct/codebase-map.md`.

## Diagram

```
                  ┌────────────────────────────────────────┐
                  │       PRODUCTION PURE CORE             │
                  │                                        │
                  │   sequenced_run_key (private)          │
                  │            ▲                           │
                  │            │ delegates                 │
                  │   ┌────────┴──────────────┐             │
                  │   │  run_event_key (pub)  │             │
                  │   │  run_snapshot_key(pub)│             │
                  │   └────────▲──────────────┘             │
                  └──────────┬│─────────────────────────────┘
                             ││ reads only
                             ││
       ┌─────────────────────┘└──────────────────────┐
       │                                            │
       │                                            │
   ┌───┴──────────────────────────┐    ┌─────────────┴──────────────┐
   │  PROPTEST LAYER              │    │  KANI HARNESS LAYER          │
   │  workspace_tests/...rs       │    │  vb_storage/kani_…rs         │
   │  6 fns 1326-1449             │    │  assert_key_contracts        │
   │                              │    │  vb_eepg_typed_partitioned…  │
   │  flow: any u64 → assume      │    │  flow: kani::any → assume    │
   │   → run_event_key → assert   │    │   → run_event_key → match    │
   │                              │    │                              │
   │  OUTPUTS (this bead's edits) │    │  OUTPUTS (this bead's edits) │
   │  - 6 proptest ranges tightened │  │  - 1 match arm accepts      │
   │    OR                        │    │    Err(SequenceOverflow)    │
   │  - 6 proptest match arms     │    │  - 1 kani::assume(           │
   │    added (vacuous accept)    │    │    seq_value != u64::MAX)    │
   └──────────────────────────────┘    └──────────────────────────────┘
```

Both layers depend **only downward** on the production encoder. There is no
back-edge. The bead's edits are confined to the two lower boxes.
