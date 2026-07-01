# Trusted-Base Plan — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `proof-planner` (State 4)
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`

This plan enumerates every trust marker that the 3 obligations rely on. Per `references/plan-quality-gates.md` Gate 8 (Trust Marker Ledger), every `assumptions` entry in a `proof-obligation/v1` row must have a matching `trusted-base-plan.md` row. This bead has **zero trust markers** that affect production behavior (no `assume`, `axiom`, `admit`, `external_body`, `#[trusted]`, `#[ignore]`, `opaque`, `extern_spec`, stub, or disabled check), so the ledger entries below are **assumptions that must hold outside the verification surface** rather than trust markers.

## 1. Trust Markers (none)

The 3 obligations do **not** introduce any of the following:
- `assume`, `axiom`, `admit`, `sorry`, `external_body`
- `#[trusted]`, `#[ignore]`, `opaque`, `extern_spec`
- Disabled check (`--no-default-checks`, `--no-memory-safety-checks`, `--no-overflow-checks`, `--no-unwinding-checks`, `--prove-safety-only`, `--only-codegen`, `--no-codegen`)
- Stub or model reduction
- Kani `cover!` as sole evidence

This is consistent with `AGENTS.md **GOD RULE 4**` ("No Loop Oscillations") and the AGENTS.md engineering rules forbidding `unsafe` / `unwrap` / `expect` / `panic` / `todo` / `unimplemented` / `dbg`.

## 2. Assumptions Ledger

Each `proof-obligation/v1` row's `assumptions` array is recorded here with a unique trust-base note ID. The `trusted_base_refs` on each obligation cites the corresponding note.

### 2.1 TB-qol58-lint-denylist-preserved

- **Trust kind:** assumption, not trust marker (the lint deny-list is YAML, not Rust code).
- **Reason:** `moon run :lint-src` is invoked at `/.moon/tasks/all.yml:46` and its deny-list at `.moon/tasks/all.yml:51` enforces `-D clippy::indexing_slicing -D clippy::get_unwrap -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::string_slice -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr` plus `-D warnings -W clippy::all -D unsafe_code`. This bead must not weaken this deny-list.
- **Scope:** workspace-wide (`cargo clippy --workspace --lib --bins --examples --all-features`).
- **Impact:** nil if preserved; catastrophic if weakened (would hide a `clippy::indexing_slicing` regression on test-side patterns in `delivery-scope.jsonl:4-13`).
- **Behavior-affecting:** false (the deny-list is a tooling concern; no production behavior change).
- **Compensating evidence:** `git diff .moon/tasks/all.yml` is expected to be empty post-refactor. The `proof-obligation/v1` row PO-qol58-001's `expected_evidence` cites this.
- **Owner:** `proof-plan-reviewer` (verifies deny-list byte-identity); `formal-verifier` (verifies `EXIT=0`).
- **Expiry:** at the bead's lifecycle end; re-justified on each follow-up bead that touches `.moon/tasks/all.yml`.
- **Status:** `proposed` (planner); `accepted` (reviewer); `closed` (formal-verifier after `verification-ledger/v1` PASS row).
- **Cited by obligations:** PO-qol58-001.

### 2.2 TB-qol58-encode-byte-layout-preserved

- **Trust kind:** assumption, not trust marker.
- **Reason:** the `cursor.write_uXX<LittleEndian>` call sequence at `crates/vb_ipc/src/frame_types.rs:42-62` is **unchanged** by this bead (per `delivery-scope.jsonl:1` and `contract.md §C-1`). The seven `IpcError::HeaderEncodeFailed` emit sites at lines 44, 47, 50, 53, 56, 59, 62 are byte-identical pre/post-refactor (per `error-taxonomy.md §1.1`). `IPC_HEADER_LEN: usize = 24` (and the wire layout: 4+2+2+2+2+8+4 = 24 bytes) is unchanged.
- **Scope:** `crates/vb_ipc/src/frame_types.rs::IpcFrameHeader::encode`.
- **Impact:** nil if preserved; wire-format-incompat if violated.
- **Behavior-affecting:** false (the layout is a pre-existing invariant, not introduced by this bead; this bead does not modify it).
- **Compensating evidence:** `cargo test -p vb_ipc` (specifically `frame_types::tests::roundtrip_encode_decode` at `frame_types/tests.rs`) exercises the wire layout end-to-end. PO-qol58-002's `expected_evidence` cites the existing test surface.
- **Owner:** `proof-plan-reviewer` (verifies the diff scope); `formal-verifier` (verifies `EXIT=0` from cargo check + cargo test).
- **Expiry:** at the bead's lifecycle end.
- **Status:** `proposed`.
- **Cited by obligations:** PO-qol58-002.

### 2.3 TB-qol58-testutil-rng-determinism

- **Trust kind:** assumption, not trust marker.
- **Reason:** the RNG constructor `StdRng::seed_from_u64(seed)` at both `crates/workspace_tests/src/test_util/seed.rs:21` and `crates/workspace_tests/src/test_util/fixture.rs:56` is unchanged. The fill window (`[u8; N]` for `seed.rs`; `Vec<u8>` of length `self.capacity.value` for `fixture.rs`) is unchanged. The `if N == 0 { return None }` short-circuit at `seed.rs:18-20` is preserved verbatim. `FixtureCapacity::MAX_CAPACITY = 1 MiB` at `fixture.rs:11` is preserved verbatim.
- **Scope:** `crates/workspace_tests/src/test_util/{seed,fixture}.rs`.
- **Impact:** nil if preserved; test-fixture drift if violated.
- **Behavior-affecting:** false (RNG determinism is a pre-existing property of the test util; this bead does not modify it).
- **Compensating evidence:** the 3 determinism unit tests at `seed.rs:33-50` (`seeded_bytes_determinism`, `seeded_bytes_different_seeds`, `seeded_bytes_zero_capacity`) and the 4 capacity-boundary unit tests at `fixture.rs:67-90` (`zero_capacity_rejected`, `valid_capacity_accepted`, `max_capacity_boundary`, `over_max_capacity_rejected`). PO-qol58-003's `expected_evidence` cites this.
- **Owner:** `proof-plan-reviewer`; `formal-verifier`.
- **Expiry:** at the bead's lifecycle end.
- **Status:** `proposed`.
- **Cited by obligations:** PO-qol58-003.

## 3. Cross-Cutting Trust Discipline

Per `references/plan-quality-gates.md` Gate 7 (Waiver Discipline) and Gate 8 (Trust Marker Ledger):

- This plan emits **zero** `E_BEHAVIOR_WAIVER` findings (no waiver row has `behavior_affecting: true`).
- This plan emits **zero** `E_TRUST_LEDGER_INCOMPLETE` findings (every `assumptions` entry in the 3 obligations has a matching note above).
- This plan emits **zero** `E_WAIVER_LIFECYCLE_INVALID` findings (no waiver rows; `waiver-candidates.jsonl` is empty).
- The `proof-obligation/v1` rows do **not** introduce `assert!` / `assert_eq!` in production code (per AGENTS.md engineering rules); the panic-surface script (`scripts/check-panic-surface.sh`) is upstream of this bead and unaffected.

## 4. Lifecycle and Ownership

| Note ID | Owner (proposed) | Owner (verification) | Status |
|---|---|---|---|
| TB-qol58-lint-denylist-preserved | `proof-plan-reviewer` | `formal-verifier` | proposed |
| TB-qol58-encode-byte-layout-preserved | `proof-plan-reviewer` | `formal-verifier` | proposed |
| TB-qol58-testutil-rng-determinism | `proof-plan-reviewer` | `formal-verifier` | proposed |

These notes will be promoted to `trusted-base-ledger/v1` rows at State 5 (proof-writer) and cited by the bridge at State 7 (proof-to-implementation); the formal-verifier closes them at State 12.

## 5. Anti-Hallucination Markers

- The 3 trust notes correspond 1-to-1 with the 3 obligation rows' `assumptions` arrays.
- The 3 trust notes cite concrete production symbols (`StdRng::seed_from_u64`, `cursor.write_uXX<LittleEndian>`, `.moon/tasks/all.yml:51`) read live from the isolated workspace.
- The compensating evidence cites concrete existing test names (`seeded_bytes_determinism`, `zero_capacity_rejected`, etc.) at concrete line ranges.
- No `assume(`, `axiom(`, `admit(`, `sorry(`, `external_body`, `#[trusted]`, `#[ignore]`, or `opaque` marker is introduced by this bead.
