# Hazard Analysis — vb-qxjgx

Each hazard below is identified, classified, and bounded. Where a
hazard is closed by the contract, the contract says so explicitly. Where
a hazard is open, the contract names the owner state and the gate.

## 1. Hazard Inventory and Classification

| ID  | Hazard                                                            | Class                | Closed by contract?                                                                                                  | Owner           |
| --- | ----------------------------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------- |
| H1  | Writer emits envelope id 12 for `StepSucceeded` post-fix          | Wire format          | **Closed.** `JournalEvent::record_kind()` for `StepSucceeded` is `StepSucceeded = 33`; there is no writer seam to override. | type-system    |
| H2  | Pre-fix journal with `StepSucceeded` payload + envelope id 12 fails to decode after split | Migration (read-side)| **Closed by back-compat lane.** Parity accepts `{12, 33}` for `StepSucceeded` payloads.                              | codec          |
| H3  | Pre-fix journal with `SlotWrittenEvent` payload + envelope id 12 incorrectly decodes as `StepSucceeded` post-fix | Semantic drift       | **Closed.** The OR pattern at `events.rs:406` is split into two arms; `SlotWrittenEvent` retains id 12.            | type-system    |
| H4  | Hidden env or config flag reintroduces a "compat mode"            | API / release        | **Closed.** The contract forbids boolean flags and per-call config for the dual-bind; the relationship is typed.     | contract       |
| H5  | Out-of-family ids (33 under non-journal magic, 33 in snapshot)    | Parser/codec         | **Closed.** `validate_kind_family(MAGIC_SNAPSHOT, 33)` returns `RecordKindFamilyMismatch`; same for blob/index.    | type-system    |
| H6  | Recovery summary switches from variant-keyed to id-keyed counting  | Semantic drift       | **Open at contract layer; closed at contract-binding clause.** Counting MUST remain variant-keyed; test-writer adds a regression assertion. | tests          |
| H7  | Postcard wire bytes for `RecordKind` shift when the enum grows    | Wire format          | **Open at parser-codec layer; closed by review of `restate_postcard_newtype_compat_tests.rs`.** The `id()` is `repr(u16)` and is independent; downstream envelope-goldens must be re-baselined. | tests          |
| H8  | `Kani`/`Flux`/`proptest` generators forget the new id             | Invariant drift      | **Closed at contract level.** All three generator sites are listed in §boundary-map; the contract requires them updated. | proof-writer  |
| H9  | Kani counterexample on `check_journal_family_exhaustive`          | Invariant drift      | **Closed at contract level.** Predicate extended to `(10..=29) ∪ {31, 32, 33}`; failure of any harness indicates a regression. | proof-writer  |
| H10 | `CURRENT_SCHEMA_VERSION = 1` accidentally bumped by another PR    | Migration / release  | **Open at version-control layer; defended by golden tests at `tests.rs:3925, 4223`.** This contract does not change the pin. | process        |
| H11 | Concurrency races during journal replay (multi-shard decoders)    | Concurrency          | **Closed.** The parity gate is a pure function of `(envelope, payload)`; no shared state.                             | type-system    |
| H12 | Borrow / lifetime mismatch in `decode_record` due to a new branch  | Unsafe/provenance    | **Closed.** All branches introduced are pure `match` arms; no new borrows.                                            | type-system    |
| H13 | `decode_journal_event` accidentally panics on truncated input      | Temporal / parser    | **Closed.** Existing decoder is exhaustive; the new branches return typed `Err` and never panic or `unwrap`.        | contract       |
| H14 | Performance regression in the parity gate (extra branch)          | Performance          | **Open at contract level; closed by fan-in win from `set/do/.../ask` rows in the durability matrix now using the correct id, plus the typed-branch outcome.** No performance claim is made in this contract. | profile-only   |
| H15 | External migration artifact reserves id 33                         | Release / wire format| **Open at discovery layer; resolved by `bd show vb-qxjgx` cross-check before merge.**                                | process        |
| H16 | Test fixtures encode `StepSucceeded` with id 12                     | Test drift           | **Open at fixture-update layer; closed by inverting the named fixture test.**                                       | test-writer    |
| H17 | Proptest candidate generation drift on `StepSucceeded` payload     | Property-test drift  | **Closed at contract level.** New id-arm in proptest generators is documented in §boundary-map.                       | test-writer    |
| H18 | New `RecordKind` variant accidentally admits a payload variant that does not exist | Wire format | **Closed at type-system level.** `StepSucceeded` is the only JournalEvent variant introduced; no orphan arm.        | type-system    |
| H19 | Replay mixes legacy and canonical ids within the same run           | Migration / replay   | **Open at replay-decision layer.** The contract states dual-tag tolerance is per-record, not per-run. If a run contains both id 12 and id 33 envelopes, both decode — the runtime invocation that triggered the write path always emits id 33, so a mixed run is most likely a developer migration from a pre-fix state. The contract holds that this is acceptable; replay integrity is preserved because both are `StepSucceeded` events. | contract       |
| H20 | Snapshot adapter reads a journal payload through the parity gate  | Parser/codec         | **Closed.** Snapshots use `MAGIC_SNAPSHOT`; parity gate is on the journal-event magic; no cross-talk.                | type-system    |
| H21 | Anyone accidentally serializes `RecordKind` directly via postcard  | Wire format          | **Open at usage layer.** Envelope stores the `u16` id, not the enum; the `restate_postcard_newtype_compat_tests.rs` covers direct serialization. Risk: serde-derive enum-tag reordering. Asserted in §boundary-map. | proof-writer   |
| H22 | Dev-stage journal interpreted as a durable artifact                 | Release             | **Open at process layer.** The contract assumes dev-stage journals are ephemeral; if that assumption is wrong, the contract is wrong. Mitigation: a follow-up bead if non-ephemeral artifacts appear. | process        |

## 2. Rust-Core Invariant Hazards (H1, H3, H5, H12, H13, H18, H20)

These hazards all arise from type-level invariants. The contract closes
them at the type level by:

- Removing the OR pattern that collapses two variants (H1, H3).
- Pairing the enum variant with a single arm in `record_kind()` and a
  single id in `is_known_record_kind` and `validate_kind_family`
  (H5, H18).
- Returning typed `Result` for every new branch; no panics, no
  unwraps (H12, H13).
- Keeping the parity gate as a pure function (H20).

## 3. Temporal Hazards (H19, H13, H10)

The temporal hazards are mostly replay-related. The contract holds:

- H19: mixed legacy/canonical ids in the same run are accepted; both
  decode to the same `StepSucceeded` variant.
- H13: existing decoder already handles truncated input; the new
  branches add typed `Err` returns.
- H10: `CURRENT_SCHEMA_VERSION` is pinned. No version-bump path exists
  in this bead.

## 4. Concurrency Hazards (H11)

The parity gate is a pure function of `(envelope, payload)`. There is no
shared state, no thread-local, no atomic. Even if two shards decode the
same journal concurrently, the result is identical. Concurrency is
covered by the standard "pure function ⇒ deterministic" reasoning,
which the proof-writer should make explicit via a proptest that
exercises concurrent decodes.

## 5. Unsafe / Provenance Hazards (H12)

There is no unsafe code in the codec or storage pipeline. The contract
forbids `unsafe` even at the boundary. The new branches match
`envelope_kind` against a typed constant set and return a typed error.
There is no provenance to corrupt.

## 6. Hostile-Input Hazards (H2, H5, H7, H21)

- H2: pre-fix journals with envelope id 12 + `StepSucceeded` payload are
  accepted; the contract holds this is a hostile-input case the codec
  must handle gracefully.
- H5: an attacker crafting id 33 inside `MAGIC_SNAPSHOT` bytes is
  rejected at the family gate.
- H7: postcard wire-byte drift on `RecordKind`. The contract does not
  assert that the postcard enum-tag for `RecordKind` is stable; the
  `restate_postcard_newtype_compat_tests.rs` must be re-reviewed and
  re-baselined if any. The `id()` is a stable `u16` literal and is
  independent of the Rust enum-tag.

## 7. Performance Hazards (H14)

No performance claim is made in this contract. The contract forbids
performance claims unless accompanied by evidence. The intent is that:

- The parity gate adds at most one extra `match` arm in the dual-bind
  branch; both branches are branch-predictor-friendly because the
  variant is the discriminator.
- The durability matrix edit is mechanical text substitution; no
  runtime cost.
- The Kani/Flux/proptest updates are static.

## 8. Release / API Hazards (H4, H10, H15, H22)

- H4 is closed at contract: no compat-mode flag.
- H10 is pinned at golden tests; any change to `CURRENT_SCHEMA_VERSION`
  breaks the pin and must be a separate decision.
- H15 is open at process level: check for external migration artifacts
  before merge.
- H22 is open at process level: dev-stage journals assumed ephemeral.
  If that assumption proves false, the contract is insufficient and
  must be amended before shipping to a non-ephemeral environment.

## 9. Unsafe-Construct Anti-Patterns (no `unsafe`, no `unwrap`, etc.)

The contract enforces the master-AGENTS forbidden-construct list at the
bead boundary:

- No `unsafe` blocks.
- No `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, `unimplemented!()`,
  `dbg!()` in any new code path.
- No unchecked indexing, slicing, casts, or arithmetic.
- No YAML/JSON/HTTP in the runtime core.

The boundaries for this requirement are:

- The pure-core predicate (`JournalEvent::legacy_envelope_bindings`,
  `EnforceKindParity for JournalEvent`, `validate_kind_family`,
  `validate_known_kind`).
- The boundary function `decode_journal_event`.
- The proptest and Kani generators.

## 10. Summary of Hazards Closed vs Open

The contract closes:

- All Rust-core invariants (H1, H3, H5, H12, H13, H18, H20).
- All parser/codec invariants for the new id (H5, H7 partially — see
  open).
- Concurrency (H11) by reduction to pure functions.
- All hostile-input paths defined above (H2, H5, H7 partial).

The contract leaves open:

- Performance (H14 — fan-in closes it).
- Wire-byte drift of `RecordKind` postcard serialization (H7, H21).
- Fixture drift (H16).
- External migration artifact reservation (H15).
- Dev-stage ephemeral assumption (H22).
- Schema-version pin (H10) at process layer.
- Recovery summary must remain variant-keyed (H6) at test layer.
- Mixed legacy/canonical in same run (H19) — defined behavior; runtime
  still validates identity and sequence.
