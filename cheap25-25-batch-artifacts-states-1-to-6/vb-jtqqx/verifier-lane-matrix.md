# Verifier Lane Matrix — vb-jtqqx

Maps each proof seed (from `proof-seeds.jsonl`) to its assigned verifier
lanes. Verifiers are evaluated against the proof-planner's
`DEFAULT_RISK_PROFILE`; lanes marked **required** are paired with a
`proof-obligation/v1` row in `proof-obligations.planned.jsonl`; lanes
marked **not_applicable** are paired with an explicit
`verifier-lane-decision/v1` row carrying `non_applicability_evidence_refs`
and a `limitation_kind`.

## Bead scope reminder

- **In-scope verifier lanes**: `proptest` (the test bodies are
  themselves proptests). The three user-facing lanes
  `cargo-test / proptest / source-lint` map to `proptest` obligations
  in the proof-planner's verifier taxonomy; the underlying execution
  surface is `cargo nextest run` and `cargo clippy -D warnings`.
- **Out-of-scope verifier lanes**: `verus`, `kani`, `flux-rs`, `loom`,
  `miri`, `cargo-fuzz` (each requires a non-applicable decision with
  concrete evidence refs).

## Matrix

| Proof Seed ID | Description | proptest | verus | kani | flux-rs | loom | miri | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|
| PS-MAL-001 | Each PO-008 body invokes `decode_storage_key` against a malformed payload and asserts on typed `KeyDecodeError` | ✅ | — | — | — | — | — | — |
| PS-MAL-002 | `truncate_len` strategy drives the truncated-payload constructor (action test) | ✅ | — | — | — | — | — | — |
| PS-MAL-003 | `extra_bytes` strategy drives the oversize-payload constructor (status, workflow tests) | ✅ | — | — | — | — | — | — |
| PS-MAL-004 | Per-variant `InvalidRunId` coverage (action, workflow, status) | ✅ | — | — | — | — | — | — |
| PS-MAL-005 | `KeyDecodeError` vocabulary; no `KeyCapacity` / `ReservedSeqSentinel` | ✅ | — | — | — | — | — | — |
| PS-MAL-006 | `ReservedSeqSentinel` is unreachable from side-index payloads | ✅ | — | — | — | — | — | — |
| PS-MAL-007 | No `unwrap`/`expect` on decoder results (Holzman-Rust) | ✅ | — | — | — | — | — | — |
| PS-MAL-008 | No membership probe (`FjallJournal::has_*_index_entry`) in PO-008 | ✅ | — | — | — | — | — | — |
| PS-MAL-009 | Every proptest strategy on PO-008 signature is wired (no `_`-discard) | ✅ | — | — | — | — | — | — |
| PS-MAL-010 | Literal prefix bytes cite `constants.rs:38-43, 77-79` in comments | ✅ | — | — | — | — | — | — |
| PS-MAL-011 | `KeyLengthMismatch { prefix: <actual> }` field surfacing | ✅ | — | — | — | — | — | — |
| PS-MAL-012 | Truncate bound `1u8..=12u8` (action) and analogous for status/workflow | ✅ | — | — | — | — | — | — |
| PS-MAL-013 | Per-variant `InvalidRunId` coverage (companion to PS-MAL-004) | ✅ | — | — | — | — | — | — |
| PS-MAL-014 | Within-family prefix mismatch (per side-index prefix) | ✅ | — | — | — | — | — | — |
| PS-MAL-015 | `EmptyKey` branch coverage (action test) | ✅ | — | — | — | — | — | — |
| PS-MAL-016 | `UnknownPrefix { 0xFF }` branch coverage (workflow test) | ✅ | — | — | — | — | — | — |
| PS-MAL-017 | `JOURNAL_KEY_PROPTEST_CASES = 128` budget preserved | ✅ | — | — | — | — | — | — |
| PS-MAL-018 | `#![forbid(unsafe_code)]` file-level lint preserved | ✅ | — | — | — | — | — | — |
| PS-MAL-019 | Future Kani harness for `decode_storage_key` (out-of-scope follow-up) | — | — | (out of scope) | — | — | — | — |
| PS-MAL-020 | Future cargo-fuzz target for `decode_storage_key` (out-of-scope follow-up) | — | — | — | — | — | — | (out of scope) |

**Legend**:
- ✅ = active lane; obligation planned in
  `proof-obligations.planned.jsonl` (`PO-MAL-001` or `PO-MAL-002`).
- — = not applicable; `verifier-lane-decisions.jsonl` carries the
  `not_applicable` row with `non_applicability_evidence_refs` and a
  `limitation_kind`.
- (out of scope) = the seed itself is marked
  `out_of_scope_followup` in `traceability-matrix.jsonl` rows 19-20;
  no obligation or lane decision is created for vb-jtqqx.

## Per-test per-lane allocation

| Test function | proptest obligation | Risk | Default profile requires | Verifier-lane decisions |
|---|---|---|---|---|
| `index_action_key_decode_error_on_short_input` | PO-MAL-001 | `rejection` | kani + proptest | VLD-001 (proptest required), VLD-002 (kani not_applicable), VLD-003 (verus not_applicable), VLD-004 (flux-rs not_applicable), VLD-005 (loom not_applicable), VLD-006 (miri not_applicable), VLD-007 (cargo-fuzz not_applicable) |
| `index_status_key_decode_error_on_wrong_length` | PO-MAL-001 | `rejection` | kani + proptest | (same set, applied per seed PS-MAL-001, PS-MAL-003) |
| `index_workflow_key_decode_error_on_wrong_length` | PO-MAL-001 | `rejection` | kani + proptest | (same set, applied per seed PS-MAL-001, PS-MAL-003) |
| PO-008 block (structural) | PO-MAL-002 | `panic_freedom` | verus + kani + proptest | VLD-013 (proptest required), VLD-024 (proptest required, for #![forbid(unsafe_code)]) |

## Non-Applicable Lanes

| Lane | Proof seed(s) | Reason | Limitation kind |
|---|---|---|---|
| verus | PS-MAL-001 (and the seed cluster) | No production source change. The decoder at `keys.rs:346-434` is read-only; the contract `decode_storage_key → KeyDecodeError` is already bound at the type level. A Verus spec would require `mechanism: STRONG` (or `WEAK_*`) production binding per the proof-planner SKILL.md Production Binding Plan. | `surface_absent` |
| kani | PS-MAL-001 (and the seed cluster) | The decoder is a pure `match`-based function with no loops, recursion, or unsafe indexing. Kani on a match-only decoder reduces to pattern exhaustiveness. A future Kani harness is captured in PS-MAL-019 as out-of-scope follow-up. | `surface_absent` |
| flux-rs | PS-MAL-001 (and the seed cluster) | No refinement types in scope. The contract uses `KeyDecodeError` directly (a sum type), not `flux_rs::refined_by::*` annotations. | `risk_out_of_scope` |
| loom | PS-MAL-001 (and the seed cluster) | The PO-008 proptest bodies are single-threaded. No `Arc`, `Mutex`, channels, or `Send`/`Sync` markers in the test scope. Proptest seeds run sequentially. | `surface_absent` |
| miri | PS-MAL-001 (and the seed cluster) | Both the test file (`#![forbid(unsafe_code)]` at `journal_side_index_contracts.rs:14`) and the decoder (`#![forbid(unsafe_code)]` at `keys.rs:1`, `error/key_decode.rs:1`) carry the strongest source-lint. Zero `unsafe` blocks, FFI, raw pointers, or `MaybeUninit` in scope. | `surface_absent` |
| cargo-fuzz | PS-MAL-001 (and the seed cluster) | Proptest's `JOURNAL_KEY_PROPTEST_CASES = 128` budget already provides randomized malformed-payload coverage. A future cargo-fuzz target is captured in PS-MAL-020 as out-of-scope follow-up. The canonical fixture at `crates/vb_storage/src/preview/tests.rs:111-180` already exercises the parser through a real `KeyspaceScanPolicy` path. | `superseded_by_other_lane_with_evidence` |

## Risk-profile coverage check

The `DEFAULT_RISK_PROFILE` (per `proof-planner` `lib.rs:87-106`) requires
the following default-profile verifiers per risk class:

| Risk | Default profile | In-scope obligation | Default-profile lanes required | Default-profile lanes planned | Status |
|---|---|---|---|---|---|
| `rejection` | kani + proptest | PO-MAL-001 | kani, proptest | proptest ✅, kani ❌ (not_applicable with VLD-002) | Closed: kani not_applicable with concrete evidence (PS-MAL-019 future scope-up); proptest is the active lane. |
| `panic_freedom` | verus + kani + proptest | PO-MAL-002 | verus, kani, proptest | proptest ✅, verus ❌ (not_applicable per risk_absent), kani ❌ (not_applicable per surface_absent) | Closed: proptest is the active lane; verus and kani are explicitly not_applicable with concrete evidence (file-level `#![forbid(unsafe_code)]`, no arithmetic / index risk in scope). |
| `field_sensitivity` | proptest + kani | PO-MAL-001 (PS-MAL-011 sub-claim) | proptest, kani | proptest ✅, kani ❌ (not_applicable per surface_absent) | Closed: proptest covers field-level surfacing via `matches!` pattern with field surfacing; kani is not applicable. |
| `equality` | proptest + verus | (no in-scope obligation) | — | — | N/A: no equality obligation in this P1. |
| `ordering` | proptest + verus | (no in-scope obligation) | — | — | N/A: no ordering obligation in this P1. |
| `bounded_transition` | kani + verus | (no in-scope obligation) | — | — | N/A: no state-machine obligation in this P1. |
| `hostile_input` | cargo-fuzz + kani + proptest | PO-MAL-001 | cargo-fuzz, kani, proptest | proptest ✅, kani ❌ (not_applicable per surface_absent), cargo-fuzz ❌ (not_applicable per superseded_by_other_lane_with_evidence) | Closed: proptest is the active lane; kani and cargo-fuzz are explicitly not_applicable with concrete evidence. |
| `parse_canonicalization` | cargo-fuzz + verus + kani | (no in-scope obligation) | — | — | N/A: no canonicalization obligation; the parser is a pure match, not a parser-canonicalizer. |
| `concurrency_interleaving`, `cancellation_safety`, `shutdown_drain`, `temporal_liveness`, `temporal_safety`, `ub_safety`, `illegal_state`, `refinement` | (varied) | (no in-scope obligation) | — | — | N/A: none of these risk classes apply to a single-threaded proptest body on a `match`-only decoder. |

**Net status**: every default-profile verifier with `applicability:
required` is paired with a `proof-obligation/v1` row; every
default-profile verifier with `applicability: not_applicable` has
concrete `non_applicability_evidence_refs` and a `limitation_kind` in
`verifier-lane-decisions.jsonl`. No silent omissions.

## Lanes not in the proof-planner verifier taxonomy

The user-facing lanes `cargo-test`, `proptest`, and `source-lint` map
to the proof-planner verifier taxonomy as follows:

| User-facing lane | proof-planner verifier | Evidence command |
|---|---|---|
| `cargo-test` | `proptest` (PO-MAL-001) | `cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` |
| `proptest` | `proptest` (PO-MAL-001) | per-test single-filter run via `cargo nextest run ... -- <test_name>` |
| `source-lint` | `proptest` (PO-MAL-002, Holzman-Rust gate) | `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` |

The source-lint lane is enforced through `cargo clippy` rather than a
separate verifier because the zero-tolerance Holzman-Rust rules
(`no unwrap/expect/panic/todo/unimplemented/dbg!`) and the file-level
`#![forbid(unsafe_code)]` are all compile-time-checkable, and the
proof-planner's verifier taxonomy treats them as part of the proptest
obligation's structural contract.

## See also

- `proof-strategy.md` — high-level strategy and not-applicable rationale.
- `proof-coverage-matrix.md` — per-clause mapping.
- `proof-obligations.planned.jsonl` — 2 `proof-obligation/v1` rows.
- `verifier-lane-decisions.jsonl` — 24 `verifier-lane-decision/v1` rows
  (including the 6 not_applicable rows for verus / kani / flux-rs /
  loom / miri / cargo-fuzz).
- `waiver-candidates.jsonl` — 6 waiver candidates for the not-applicable
  lanes (no behavior-affecting waiver; all are `not_applicable` lane
  decisions, not waivers of obligation).
