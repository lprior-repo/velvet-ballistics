# Proof Strategy — vb-5bqmr

## Bead

- **bead_id**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
- **isolated_workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`
- **go-skill state**: 4 (proof-planner)
- **owner**: proof-planner (no sub-agents)
- **planned_at**: 2026-07-01
- **scope_mode**: focused — single-call graph blast radius, no broad re-scan

## 1. Goal and risk posture

The bead fixes a P1 silent-downgrade bug: `decode_slot_written_extra` in
`crates/vb_storage/src/slot_extra.rs:60-69` currently routes any byte slice
that does not equal `b"VBSE\x01"` (the legacy-stripped prefix) into
`DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)`. A writer that emits a
recognised-but-newer version (e.g. `b"VBSE\x02…"`) is silently reclassified
as legacy frame extra and the durable taint metadata is discarded.

The fix:

1. Hoist the prefix into two constants:
   `SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and
   `SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01`. Retain
   `SLOT_WRITTEN_EXTRA_PREFIX` as a compositional derivation of the two.
2. Add `SlotWrittenExtraError::VersionMismatch { found: u8 }` to the
   existing `#[non_exhaustive]` enum.
3. Tighten `decode_slot_written_extra` into three explicit arms
   (decode / version-mismatch / legacy) that are mutually exclusive and
   exhaustive over `&[u8]`.
4. Propagate the new variant through:
   - `crates/vb_storage/src/recovery/replay/summary/hydrate.rs:209-235`
     (`decoded_slot_taint`) — explicit match arm + `tracing::warn!`.
   - `crates/vb_runtime/src/primitives/collect.rs:256-273`
     (`hydrate_slot_written_extra`) — explicit match arm + `tracing::warn!`.
   - `crates/vb_core/src/errors.rs` — add `CollectExtraHydrationFailureKind::VersionMismatch`.

### 1.1 Risk classification

| Risk class | Tag(s) | Present? |
|---|---|---|
| `parser/codec` | `parser`, `codec`, `bounded_state`, `rejection` | YES |
| `bounded_transition` | `bounded_state` | YES (length-bounded input) |
| `refinement` | `refinement`, `index` | YES (prefix-length invariant) |
| `equality` (round-trip) | `property`, `equality` | YES (encoder→decoder) |
| `hostile_input` | `parser`, `codec`, `hostile_input`, `persisted_bytes` | YES at the disk boundary; fuzz out-of-scope (RED QUEEN §M3, `vb-1rqz7.15`) |
| `concurrency_interleaving` | — | NO (sync, pure, total) |
| `ub_safety` | — | NO (`#![forbid(unsafe_code)]`) |
| `temporal_*` | — | NO (no async, no scheduler) |
| `miri` UB family | — | NO (no `unsafe`) |

The default Rust behavior profile per `references/verifier-trigger-matrix.md` is
`verus + kani + flux-rs + proptest`. The user's explicit lane instruction
`Lanes: rust-local, kani, flux-rs, proptest` corresponds to:
- `rust-local` → `verus` (Rust-local pure/core invariants; the
  default-profile primary for `rust_local`/`pure_core`/`arithmetic`).
- `kani` → bounded symbolic, primary for `bounded_state`/`rejection`.
- `flux-rs` → refinement type, primary for `refinement`/`index`.
- `proptest` → property pressure, primary for `equality`/`property`.

`loom` is excluded (sync). `cargo-fuzz` is excluded for this bead (RED QUEEN
§M3 separately). `miri` is excluded (no `unsafe`).

## 2. Defense-in-depth lane rationale

| Lane | Role | Binding mechanism |
|---|---|---|
| **verus** (`rust-local`) | PRIMARY on the discriminator body — proves the three-arm classification for **arbitrary** `bytes: &[u8]`, no length bound. Proves the `VersionMismatch {found: 0x01}` is unreachable from the decoder (C-ERR-002). | STRONG via `#[path = "crates/vb_storage/src/slot_extra.rs"]` + `assume_specification` bridge (no `external_body`, no `assume`, no `axiom` per GOD RULE 2). Drift-gate is `scripts/check-verus-production-binding.sh`. |
| **kani** | COMPANION on bounded symbolic — proves the partition holds for `len ∈ [0..256]` with symbolic version byte, and proves the legacy arm makes zero allocations (C-NEG-006). `kani::cover!` reachability proves each arm is hit. | direct `cargo kani -p vb_storage --harness <name>` with `kani::any()` for symbolic bytes. |
| **flux-rs** | PRIMARY on the prefix-length refinement — proves `MAGIC.len() + 1 == 5` at the type level, and the relationship between `bytes.len()`, the magic prefix, and the discriminator arm classification. | `cargo flux --lib -p vb_storage` against an in-tree `#[refined_by]` + `#[invariant]` annotation on the public constants and `decode_slot_written_extra` signature. No `#[trusted]`/`#[ignore]` broadening introduced. |
| **proptest** | COMPANION on property pressure — proves the version-mismatch, decode, and round-trip paths over a strategy-generated input space. Captures the tracing log emission for the translation sites (C-REC-002, C-RUN-002). | `PROPTEST_CASES=10000 cargo test --test <name> --release` against new test files under `crates/vb_storage/tests/` and `crates/vb_runtime/tests/`. |

### 2.1 Why no fuzz this bead

`decode_slot_written_extra` is a parser/codec boundary and would normally
trigger `cargo-fuzz` as a primary. `fuzz/research/red-queen-strategy.md:399`
explicitly flags this as a missing P0 fuzz target; the existing plan
(`vb-1rqz7.15` / §M3) addresses it as a separate bead. This bead is
deliberately scoped to the version-mismatch discriminator fix; fuzz is
out-of-scope. The proptest strategy covers the same input boundary at a
smaller budget; the fuzz gap is tracked in `delivery-scope.jsonl` row 16.

### 2.2 Why no Miri / Loom this bead

- **Miri**: `crates/vb_storage/src/slot_extra.rs:1` carries
  `#![forbid(unsafe_code)]`. The new decoder body uses only safe APIs
  (`split_at_checked`, `split_first`, `bytes.starts_with`, `postcard::from_bytes`).
  No UB surface to model.
- **Loom**: the function is `fn decode_slot_written_extra(bytes: &[u8]) -> Result<…>`
  — sync, pure, total. No `Send`/`Sync` boundary, no channel, no atomic, no
  cancel/drop. No concurrency surface to schedule.

## 3. Production binding plan (GOD RULE 2)

The Verus obligation MUST bind to production via one of the three mechanisms.
The plan:

```yaml
production_binding:
  mechanism: STRONG
  production_path: crates/vb_storage/src/slot_extra.rs
  production_lines: 60-69 (NEW body)
  assume_specification_targets:
    - production::decode_slot_written_extra
  exec_wrapper_required: true
  drift_detection: build-time
  drift_gate_script: scripts/check-verus-production-binding.sh
```

The artifact will be `verification/verus/vb_storage/slot_extra_decode_partition.rs`
with `#[path = "crates/vb_storage/src/slot_extra.rs"] mod production;` and an
`assume_specification[ production::decode_slot_written_extra ](...)` bridge.

A spec that defines a shadow enum `SpecSlotWrittenExtraError` without
`#[path]` would be VACUUM (GOD RULE 2 violation) and is forbidden. The
`check-verus-production-binding.sh` gate enforces this at `moon ci` time.

## 4. Evidence discipline

Each planned obligation:

- Names the exact `command` (no placeholders).
- Names the absolute `workdir` (the isolated workspace root).
- Specifies the success marker (e.g., `VERIFICATION:- SUCCESSFUL`,
  `test result: ok. 1 passed; 0 failed`).
- Encodes `model_bounds` for bounded lanes:
  - Kani: `-j 1`, `mem_high=20G`, `mem_max=24G`, `unwind=8`,
    `input_size=256`.
  - Flux: nightly toolchain pin in `tool_metadata.version_pin`.
  - Proptest: `PROPTEST_CASES=10000`, `input_size=256`.
- Records the `expected_evidence` artifact path (raw log location).
- The Kani harness uses `kani::any()` for symbolic bytes — never a
  hardcoded structural input (GOD RULE 1).

## 5. Anti-laundering

- No `assume(...)` / `axiom` / `admit` / `external_body` in executable
  Verus proof code. The `proof_decode_partition` proof body uses standard
  Verus idioms (`assert(...)`, `use_type_invariant`, `reveal`, `assert by (...)`).
- No `kani::cover!` as sole evidence. Each Kani harness has a paired
  `kani::assert` or postcondition expressing the property.
- No proptest that asserts only `is_ok()` / `is_err()`. Each proptest
  asserts exact values (`assert_eq!(found, expected_byte)`,
  `assert!(matches!(err, SlotWrittenExtraError::VersionMismatch { found: x }
  if x == bytes[4]))`).
- No local model that copies production logic. The bridge row in
  `proof-to-implementation-input.md` (and the
  `proof-writer`-produced `proof-to-rust-map.md`) will name the
  source_refs + behavior_test_refs + refinement_harness_refs and assert
  disjointness.
- No behavior-affecting waivers. The bead is fully provable by the four
  chosen lanes; no `E_BEHAVIOR_WAIVER` row is emitted.

## 6. Self-imposed forbidden behaviors (bead-specific)

These are binding constraints from the user prompt and the contract:

1. **Must NOT silently downgrade** magic-with-unknown-version to
   `LegacyFrameExtra`. The new `VersionMismatch` arm is the only valid
   classification for the magic-but-unknown-version branch. Verus (for-all)
   + Kani (symbolic) + Proptest (strategy) all bound this behavior.
2. **Must keep `recovery_bdd_tests.rs:3158-3211` (the legacy-frame BDD
   scenario using `b"\x01\x02\x03\x04"`) passing unchanged.** This is
   captured by `vb-5bqmr.ps.v1-bdd-legacy-regression` and the
   corresponding C-NEG-001 / C-DEC-003 obligations.
3. **Must keep `recovery/tests.rs:2332` corrupt-v1 helper classified as
   `DecodeFailed`, not `VersionMismatch`.** The `b"VBSE\x01\xff\xff\xff"`
   envelope has magic + version match, so the postcard path is taken; the
   anti-invariant in PO-PROP-002 (round-trip) covers this.

## 7. Obligation count budget

| Lane | Obligations | Count |
|---|---|---|
| verus (`rust-local`) | PO-VERUS-001 (discriminator partition) | 1 |
| kani | PO-KANI-001 (unknown-version rejection + unreachable VersionMismatch{0x01}), PO-KANI-002 (partition + legacy zero-alloc cover) | 2 |
| flux-rs | PO-FLUX-001 (prefix-length + MAGIC composition refinement) | 1 |
| proptest | PO-PROP-001 (unknown-version property + anti-invariant), PO-PROP-002 (round-trip + corrupt-v1 / legacy anti-invariants), PO-PROP-003 (hydrate + collect translation + warn-log capture) | 3 |
| **Total** | | **7** |

This is within the user-stated 6-8 obligation budget. All 7 obligations are
`behavior_affecting: true` except PO-VERUS-001 (which proves a behavior
invariant so is also behavior-affecting), PO-KANI-002 (the partition proof
itself is behavior-affecting; the legacy zero-alloc cover is a sub-proof
captured under `kani::cover!` with paired `kani::assert`), PO-FLUX-001
(refinement proves behavior), PO-PROP-003 (translation correctness), and
PO-PROP-002 (round-trip is behavior).

## 8. Handoff

After planning:

- `proof-plan-reviewer` (State 4b) dispositions each lane decision.
- `proof-writer` (State 5) authors Verus spec, Kani harnesses, Flux
  refinement annotations, and proptest modules.
- `proof-to-implementation` (State 7) materializes the bridge
  (`rust-refinement-obligation/v1` rows) per the
  `proof-to-implementation-input.md` stub this planner writes.
- `formal-verifier` (State 12) executes each obligation's command and
  writes `verification-ledger/v1` rows.

The planner does NOT claim PASS. Reviewer owns disposition; verifier owns
closure.