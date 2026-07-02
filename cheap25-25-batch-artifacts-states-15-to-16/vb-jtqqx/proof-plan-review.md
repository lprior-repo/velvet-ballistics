reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: vb-jtqqx-state4-proof-plan-review-attempt1
planner_invocation_id: vb-jtqqx-state4-proof-planner-attempt1
review_state: 4
reviewed_at: 2026-07-01T17:00:00Z

# Proof Plan Review: vb-jtqqx

## Review Metadata

- **bead_id**: vb-jtqqx
- **bead_title**: Tests: make side-index malformed-key tests decode malformed keys (P1)
- **scope**: Test-only repair bounded to
  `crates/workspace_tests/tests/journal_side_index_contracts.rs:183-257`
  (PO-008 proptest block). Decoder under test (read-only):
  `crates/vb_storage/src/keys.rs:346-434` (`decode_storage_key`) and
  `:281-295` (`try_key_prefix`).
- **behavior_affecting**: false (no production source changes; the repair
  rewires existing proptest strategy inputs into `decode_storage_key`
  calls and adds field-level assertions on `KeyDecodeError`).
- **workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx`
- **controller**: femdation (this reviewer is a direct child of femdation;
  no sub-agents were spawned).

## Reviewed Artifacts

| Artifact | Hash (sha256) | Status |
|----------|---------------|--------|
| `proof-strategy.md` | `5e99f19f497677dafb289f561814f3dd990f7addb511291365e0f859b8531af4` | reviewed |
| `verifier-lane-decisions.jsonl` (24 rows) | `3a05c4f60d7aeaa3ee53e94b9f78d3c97ff148949cff91a8f196e6217fe02633` | reviewed |
| `proof-obligations.planned.jsonl` (2 rows) | `abbbe53fa9cc7c3cbef6ce285ba83b12c7d58ecfdb2061d2791a5b81460114ab` | reviewed |
| `trusted-base-plan.md` | `445550c827219e14fcd7ba4a7c3dd6b022088b90170ba766748f6d0f6aa46c1a` | reviewed |
| `waiver-candidates.jsonl` (6 rows) | `1d988dfeffec44b0bda86166bd280ba6a49d0dba0831440718e0adcb7ac829a4` | reviewed |
| `contract.md` | `3ab0f24c385ec24c77fa8277ccae6124d57f9856dbfae1526f8376084948113e` | reviewed |
| `error-taxonomy.md` | `136ecc55723b433cc180edce1d6d4d2d4352ed51649df4d6629ec4c2da6e58dc` | reviewed |
| `proof-seeds.jsonl` (20 rows) | `b0e5ce274923dbbb805df1b6848004076f775ec081102bd610f0d1c750e2e828` | reviewed |
| `traceability-matrix.jsonl` (20 rows) | `e8bf337cecea9cf11e4d732292f2485e010266f55e7e5645798c1979e0c44a31` | reviewed |
| `proof-coverage-matrix.md` | `c3bb515cd2b2fe2c011ecbcf85c8bd924cdb465c16e733a845f84953067e692d` | reviewed |
| `verifier-lane-matrix.md` | `2835e8aaaafaa751d5acc31a6c70d57910403132593b95b0d9a1852afce379ce` | reviewed |
| `domain-model.md` | `19112b88cdd73405d8d763fe425605626d57a02957c446b88849350edb7a2d63` | reviewed |
| `type-contracts.md` | `9d2248476da9e4007dd15e299319ff3b38d907034e16bcd29c7f626b43813456` | reviewed |
| `boundary-map.md` | `3c95d410e6dd1417b41fbff6b77d25834b0b9576109b1031ea8934cc28c69feb` | reviewed |
| `hazard-analysis.md` | `78af9b3c103dd44ad5549eeceb0a27b5d1d22f2d8d697b507ddc25c8e2a64674` | reviewed |
| `workflow-model.md` | `86b8b51f580f741989b4162be94a7ef3408d0c87e0ae4203c18c3c66db3f30cf` | reviewed |
| `delivery-scope.jsonl` (8 rows) | (reviewed) | reviewed |
| `agent-invocation-ledger.jsonl` (seq 1..2) | `912c950390971a87b1313af4d18a3df69e4dca00bd0c2fe9fc34c0276a8ab169` | reviewed (will append seq 3) |
| Decoder source (read-only): `crates/vb_storage/src/keys.rs:281-295, 346-434` | n/a (read-only) | reviewed |
| Decoder error vocab (read-only): `crates/vb_storage/src/error/key_decode.rs:8-31` | n/a (read-only) | reviewed |
| Constants (read-only): `crates/vb_storage/src/constants.rs:38-43, 77-79` | n/a (read-only) | reviewed |
| Test file (in-scope surface): `crates/workspace_tests/tests/journal_side_index_contracts.rs:14, 23, 183-257` | n/a (in-scope) | reviewed |

## Review Summary

### Lane Decision Coverage: PASS

- 24 lane decisions (`VLD-jtqqx-001..024`) covering 18 proof seeds
  (PS-MAL-001..018) plus PS-MAL-019/020 follow-up seeds (no lane
  decisions because they are `out_of_scope_followup` for a future
  bead per `traceability-matrix.jsonl:19-20`).
- Every active seed has a row for **every verifier in the
  default Rust-Implementation profile** (proptest, verus, kani,
  flux-rs) plus the conditional lanes (loom, miri, cargo-fuzz). No
  silent omissions.
- 18 lanes are `applicability: required` (the proptest body for each
  active proof seed, double-bound to PO-MAL-001 or PO-MAL-002).
- 6 lanes are `applicability: not_applicable` (verus, kani, flux-rs,
  loom, miri, cargo-fuzz), each with concrete
  `non_applicability_evidence_refs` and a `limitation_kind`.
- Independent review rows: 24 `verifier-lane-review/v1` rows
  (`VLR-jtqqx-001..024`), all `reviewer_disposition: accepted`,
  emitted to `.beads/vb-jtqqx/verifier-lane-review.jsonl`. Planner and
  reviewer invocation IDs are distinct
  (`vb-jtqqx-state4-proof-planner-attempt1` vs.
  `vb-jtqqx-state4-proof-plan-review-attempt1`).

### Non-Applicability Evidence: PASS

Every `not_applicable` lane decision carries concrete file:line
evidence and a `limitation_kind`. Spot-checked against
`crates/vb_storage/src/keys.rs` and
`crates/workspace_tests/tests/journal_side_index_contracts.rs`:

| Verifier | Limitation kind | Concrete evidence (verified) |
|---|---|---|
| `kani` | `surface_absent` | `keys.rs:281-295, 346-434` — pure `match` arms, no loops, no recursion, no unsafe indexing. `key_array::<N>` at `keys.rs:305-314` uses `bytes.get(range).ok_or_else(...)` and `<[u8; N]>::try_from(slice).map_err(...)` — panic-free. |
| `verus` | `surface_absent` | No production source change; `KeyDecodeError` is the contract surface (`error/key_decode.rs:8-31`) and is already bound at the type level. Future Verus spec would require `mechanism: STRONG` (`#[path = ...]`) per the proof-planner Production Binding Plan; out of scope here. |
| `flux-rs` | `risk_out_of_scope` | No refinement types in scope; `KeyDecodeError` is a sum type, not a `flux_rs::refined_by::*` annotation surface. `boundary-map.md` confirms no refinement-type surface in scope. |
| `loom` | `surface_absent` | `journal_side_index_contracts.rs:1-31` shows only `proptest!` macro imports and `#![forbid(unsafe_code)]`; no `Arc`, `Mutex`, channels, or `Send`/`Sync` markers. Proptest seeds run sequentially. |
| `miri` | `surface_absent` | `#![forbid(unsafe_code)]` at `journal_side_index_contracts.rs:14`, `keys.rs:1`, and `error/key_decode.rs:1`. Zero unsafe blocks, FFI, raw pointers, or `MaybeUninit` in scope. |
| `cargo-fuzz` | `superseded_by_other_lane_with_evidence` | `JOURNAL_KEY_PROPTEST_CASES = 128` is randomized coverage of malformed payloads. Canonical fixture at `crates/vb_storage/src/preview/tests.rs:111-180` (`preview_keyspace_fails_closed`, `preview_keyspace_fail_closed_unknown_prefix`) exercises the parser through a real `KeyspaceScanPolicy` path. Future cargo-fuzz target captured in `PS-MAL-020`. |

None of the `not_applicable` decisions are weak (no bare "too hard",
"not practical", or "not needed" reasoning); all carry concrete
file:line evidence_refs.

### Obligation Schema: PASS

The two `proof-obligation/v1` rows
(`.beads/vb-jtqqx/proof-obligations.planned.jsonl`) carry the full
required schema:

- `schema_version: proof-obligation/v1` ✅
- `id` (`PO-MAL-001`, `PO-MAL-002`) ✅
- `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier` ✅
- `artifact`, `target`, `command`, `workdir`, `expected_evidence` ✅
- `assumptions`, `model_bounds`, `tool_metadata`, `trusted_base_refs` ✅
- `required: true`, `behavior_affecting: false`, `mode: verify-proof` ✅
- `owner_state: 4`, `rerun_from: 4`, `status: planned` ✅
- No legacy alias fields (`layer`, `checker`, `claim`) ✅
- `target` is canonical (no alias drift) ✅

Both obligations are `verifier: proptest` with
`behavior_affecting: false`. **No Verus obligations exist** — the
Production Binding Plan gate is vacuous-by-design (no obligation to
bind). This is the intended shape per the proof-plan-reviewer
rubric: a Verus obligation that cannot be bound to production via
STRONG / WEAK_MIRROR / WEAK_EXTERN must NOT exist; the planner
correctly avoided creating one.

### Commands and Bounds: PASS

- `PO-MAL-001` command:
  `PROPTEST_CASES=128 cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts -- index_action_key_decode_error_on_short_input index_status_key_decode_error_on_wrong_length index_workflow_key_decode_error_on_wrong_length`
  Targets three named proptests (covers all 18 `required` lanes that
  roll up to PO-MAL-001 or PO-MAL-002). Workdir matches.
  `expected_evidence` specifies `test result: ok`, `JOURNAL_KEY_PROPTEST_CASES = 128` per test, and proptest-shaped failure reporting (no panic).
- `PO-MAL-002` command:
  `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic --message-format human`
  Targets the workspace-tests package with Holzman-Rust zero-tolerance
  lint flags. Workdir matches.
- Bounds:
  `JOURNAL_KEY_PROPTEST_CASES = 128` (preserved at line 23),
  `truncate_len in 1u8..=12u8` (action) and analogous bounds for the
  status / workflow tests
  ([1,12]∈[1,13); [1,17]∈[1,18)),
  `_extra_bytes in 0u8..=10u8`. All bounds are explicitly justified in
  `trusted-base-plan.md#Strategy Bounds`.

### TLA+ Compliance: PASS (N/A)

The repository has removed TLA+ per the master mandate; this bead has
no temporal / workflow obligations. No `verifier: tla+` lanes exist.

### Waiver Candidates: PASS

6 `waiver-candidate/v1` rows; **every row has `behavior_affecting:
false`**, `review_status: pending`, and ISO-8601 `expiry`. None are
behavior-affecting waivers. Per the proof-plan-reviewer rubric,
behavior-affecting waivers are invalid — none exist here. The 6
candidates are bookkeeping for the 6 `not_applicable` lane decisions
(no obligation is waived; the planner documents the rationale in both
`verifier-lane-decisions.jsonl` and `waiver-candidates.jsonl` for
traceability).

### Trusted Base Plan: PASS

`trusted-base-plan.md` covers:

- **In-scope surface**:
  `crates/workspace_tests/tests/journal_side_index_contracts.rs:14`
  (`#![forbid(unsafe_code)]`), `:23`
  (`JOURNAL_KEY_PROPTEST_CASES = 128`), `:183-257` (PO-008 block).
- **Out-of-scope surface** (read-only):
  `keys.rs:281-295` (`try_key_prefix`), `:346-434`
  (`decode_storage_key`), `constants.rs:38-43` (prefix bytes),
  `:77-79` (length envelopes), `error/key_decode.rs:8-31`
  (`KeyDecodeError` taxonomy), `lib.rs:202` (re-export).
- **Trusted surfaces**:
  proptest@1.5, decoder (read-only, panic-free via `bytes.get(...).ok_or_else(...)` /
  `<[u8; N]>::try_from(slice).map_err(...)`), constants (`pub(crate)`,
  cited verbatim in test bodies), canonical fixtures
  (`preview_keyspace_*` at `preview/tests.rs:111-180` and
  `cc002_run_headers_fails_closed_on_malformed_key` at
  `tests.rs:1862-1904`), type invariants (decoder returns
  `Result<_, KeyDecodeError>` for all inputs), tooling
  (`cargo nextest run`, `cargo clippy -D warnings ...`).

No behavior-affecting trusted-base entries. Reduction justification
explains why proptest's 128-case budget is statistically sufficient
for a `match`-only decoder over a 13- or 18-byte input space, with
explicit reasoning per decoder branch (length envelope, prefix
exhaustiveness, `InvalidRunId` run==0).

### Non-Vacuity: PASS

- **No Kani cover-only obligations exist** — Kani is `not_applicable`.
- **No Verus obligations exist** — vacuous Verus is structurally
  impossible.
- **proptest obligations assert on typed variants**: `prop_assert!(matches!(decode_storage_key(payload), Err(KeyDecodeError::Variant)))` is the static check on
  `KeyDecodeError` variants. The `cargo clippy -D clippy::unwrap_used`
  / `-D clippy::expect_used` / `-D clippy::panic` flags will surface
  any forbidden panic pattern at compile time.
- **Per-test shape coverage** is enumerated in
  `proof-coverage-matrix.md` and `error-taxonomy.md#Per-test required shapes`:
  each of the 3 PO-008 tests gets ≥3 payload shapes (truncated/oversize
  + per-variant `InvalidRunId` + within-family mismatch), with `EmptyKey` and `UnknownPrefix` distributed across tests.

### Bridge Planning: PASS

Per `proof-strategy.md#Execution Order` State 7 (proof-to-implementation
bridge map) is planned to map:

- `PO-MAL-001` → test file lines 195-218
  (`index_action_key_decode_error_on_short_input`),
  222-238 (`index_status_key_decode_error_on_wrong_length`),
  242-256 (`index_workflow_key_decode_error_on_wrong_length`).
- `PO-MAL-002` → test file lines 14 (`#![forbid(unsafe_code)]`),
  23 (`JOURNAL_KEY_PROPTEST_CASES = 128`), and the PO-008 block
  (lines 183-257).
- All required code changes are bounded to one test file
  (`journal_side_index_contracts.rs`); no edits to `Cargo.toml`,
  `Cargo.lock`, dependency manifests, `crates/vb_storage/**`, or
  `crates/workspace_tests/**` outside the named test file (per
  contract `SIDEX-MAL-008`).

### Production Surface Confirmation (spot-check)

The following invariant claims in the strategy and obligations were
spot-checked against the read-only source:

- **`try_key_prefix`** at `crates/vb_storage/src/keys.rs:281-295`:
  returns `Err(KeyDecodeError::EmptyKey)` for empty slices
  (`bytes.first().ok_or(KeyDecodeError::EmptyKey)?`) and
  `Err(KeyDecodeError::UnknownPrefix { prefix })` for any byte outside
  the nine known prefixes. ✅ Matches `PS-MAL-015, PS-MAL-016`
  evidence refs (`keys.rs:282`, `keys.rs:293`).
- **`decode_storage_key`** at `crates/vb_storage/src/keys.rs:346-434`:
  - Length check at `:349-355`:
    `KeyLengthMismatch { prefix: prefix.to_u8(), expected: expected_len, actual: bytes.len() }`. ✅ Matches `PS-MAL-011` field-level surfacing.
  - `IndexStatus` `InvalidRunId` branch at `:400-402`. ✅
  - `IndexWorkflow` `InvalidRunId` branch at `:412-414`. ✅
  - `IndexAction` `InvalidRunId` branch at `:423-425`. ✅
  - `ReservedSeqSentinel` is unreachable from side-index payloads
    (only reachable from `RunEvent` / `RunSnapshot` at `:386`).
    ✅ Matches `PS-MAL-006` / `contract.md#SIDEX-MAL-016`.
- **`KeyDecodeError`** at
  `crates/vb_storage/src/error/key_decode.rs:8-31`: `#[non_exhaustive]`
  enum with the five documented variants. ✅ Stable test surface for
  forward-compatible `matches!` patterns.
- **`JOURNAL_KEY_PROPTEST_CASES = 128`** at
  `crates/workspace_tests/tests/journal_side_index_contracts.rs:23`: ✅
  Verified.
- **`#![forbid(unsafe_code)]`** at
  `crates/workspace_tests/tests/journal_side_index_contracts.rs:14`: ✅
  Verified.

### Review Provenance: PASS

- **Independent reviewer invocation**:
  `vb-jtqqx-state4-proof-plan-review-attempt1` (this review).
- **Planner invocation**: `vb-jtqqx-state4-proof-planner-attempt1`
  (proof-planner; the planner's state-4 row will be appended to the
  control-plane agent-invocation ledger per the femdation
  provenance).
- Planner and reviewer invocation IDs differ (independent review
  requirement satisfied).
- Host session: `femdation-cheap25-batch` (mirrors the State 2 row
  for `explore-vb-jtqqx-state2`).
- All `verifier-lane-review/v1` rows in
  `.beads/vb-jtqqx/verifier-lane-review.jsonl` carry the same
  planner_invocation_id and reviewer_invocation_id (cross-row
  consistency).
- A state-4 `agent-invocation/v1` row will be appended to
  `.beads/vb-jtqqx/agent-invocation-ledger.jsonl` upon approval; the
  chain links `previous_entry_hash = f15a93ba6fb71ce9e9803c2a3228ff1ae915377b5d8cee245c4c245e20bf460e`
  (= entry_hash of seq 2; the most recent log entry).

## Findings

No findings. The plan is implementation-bound (in-scope surface =
single test file `journal_side_index_contracts.rs:183-257`), the
production surface is read-only (no decoder change, no constant
change, no Cargo manifest change), the default-profile verifiers
(Kani, Verus, Flux-rs) are explicitly `not_applicable` with concrete
file:line evidence, the proptest obligations carry precise commands,
bounds, and expected evidence, the trusted-base plan covers the
read-only decoder and the canonical fixtures, and bridge planning is
present. No behavior-affecting waivers exist. The plan is precise
enough for `proof-writer` (which has nothing to write — no Kani /
Verus / Flux / Loom / fuzz artifacts for a test-only P1) and
`proof-to-implementation` at State 7.

### Inline observation (non-blocking; not a finding)

PO-MAL-001's command starts with the env-var prefix
`PROPTEST_CASES=128`. This variable is inert on proptest@1.5 — the
per-test case count is bound through
`journal_proptest_config(JOURNAL_KEY_PROPTEST_CASES)` (line 25-31 of
the test file), which guarantees 128 cases per proptest regardless
of any env var. The env-var prefix is harmless (it does not affect
behavior) and serves as documentation intent; the proof-writer /
formal-verifier does not need to change it. If the bead owner
prefers to drop the env-var prefix (e.g., for hygiene), it is a
single-line edit; if not, no action is required. No action is
required.

## Disposition

| Lane set | Disposition |
|---|---|
| VLD-001..024 (proptest, kani, verus, flux-rs, loom, miri, cargo-fuzz × PS-MAL-001..018) | accepted |
| PO-MAL-001 (decoder-rejection) | ready for proof-writer / formal-verifier (State 5 / State 12); no verifier artifacts required for this P1; holzman-rust / test-writer owns the repair at State 5b |
| PO-MAL-002 (structural preservation) | ready for proof-writer / formal-verifier (State 5 / State 12); same ownership as PO-MAL-001 |
| WC-jtqqx-001..006 (waiver candidates) | not behavior-affecting; bookkeeping for not_applicable lanes; pending review at State 12 |

## Conclusion

The proof plan is precise, evidence-grounded, and bounded to a
single test file with the decoder under test strictly read-only.
Default-profile verifiers are explicitly addressed with concrete
evidence. No silent omissions; no behavior-affecting waivers; no
vacuous obligations; no production source change; bridge planning
is present; commands are exact; bounds are justified; non-vacuity
is established.

STATUS: APPROVED
