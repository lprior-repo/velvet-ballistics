# Proof Coverage Matrix — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `proof-planner` (State 4)
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`

This matrix maps each `proof-obligation/v1` row to its contract clause, its behavior-affecting flag, its production-binding source ref, its evidence command, and its risk class. It is the planner's contract with `proof-to-implementation`, `formal-verifier`, and the reviewer.

## 1. Obligation Summary

| ID | Contract | Behavior-Affecting | Production Source Ref | Risk Class | Verifier | Command |
|---|---|:---:|---|---|---|---|
| PO-qol58-001 | C-1+C-2+C-3+C-4 | false | (cross-site aggregate; 3 sites) | index_safety | proptest | `moon run :lint-src` |
| PO-qol58-002 | C-1 | false | `crates/vb_ipc/src/frame_types.rs::IpcFrameHeader::encode` | index_safety | proptest | `cargo check -p vb_ipc --all-targets --all-features` |
| PO-qol58-003 | C-2+C-3 | false | `crates/workspace_tests/src/test_util/seed.rs::SeededBytes::new` + `crates/workspace_tests/src/test_util/fixture.rs::FixtureBuilder::build_bytes` | index_safety | proptest | `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` |

**3 obligations**, all `behavior_affecting: false`, all `required: true`, all `mode: verify-proof`.

## 2. Per-Obligation Coverage Detail

### 2.1 PO-qol58-001 — Cross-site aggregate (lint-pass)

- **Requirement:** `REQ-LINT-CANONICALIZE-ALL-PROD-SITES` (PS-qol58-X-001 + PS-qol58-D-001 cross-cite)
- **Contract clause:** `C-1+C-2+C-3+C-4`
- **Behavior-affecting:** `false`
- **Risk class:** `index_safety` (preventive; the lint-deny-list preservation is `gate-evidence` but classified under the underlying lint risk).
- **Target (production source):** `crates/vb_ipc::frame_types::IpcFrameHeader::encode` + `crates::workspace_tests::test_util::seed::SeededBytes::new` + `crates::workspace_tests::test_util::fixture::FixtureBuilder::build_bytes`.
- **Verifier:** `proptest` (the actual command is `moon run :lint-src`; verifier is the closest enum match).
- **Command:** `moon run :lint-src` (workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`).
- **Expected evidence:** `moon run :lint-src` exits 0; the deny-list flags in `.moon/tasks/all.yml:51` are unchanged post-refactor; no new clippy warning; raw command log captured.
- **Required:** true.
- **Owner state:** 4 (`planned`).
- **Rerun from:** 4.
- **Paired lane decisions:** VLD-qol58-A-001-proptest, VLD-qol58-B-001-proptest, VLD-qol58-C-001-proptest, VLD-qol58-D-001-proptest, VLD-qol58-X-001-proptest.
- **Trusted base refs:** `TB-qol58-lint-denylist-preserved`.
- **Bridge (proof-to-implementation):** None required (behavior_affecting=false; no `rust-refinement-obligation/v1` row needed).

### 2.2 PO-qol58-002 — IPC header encode canonicalization (cargo-check)

- **Requirement:** `REQ-LINT-CANONICALIZE-IPC-HEADER-ENCODE` (PS-qol58-A-001)
- **Contract clause:** `C-1`
- **Behavior-affecting:** `false`
- **Risk class:** `index_safety` (preventive; full-slice on `[u8; IPC_HEADER_LEN]` is benign today).
- **Target:** `crates/vb_ipc::frame_types::IpcFrameHeader::encode` (line 41 is the refactor site).
- **Verifier:** `proptest` (closest enum match for cargo check + cargo test invocation).
- **Command:** `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` (workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`).
- **Expected evidence:** `cargo check -p vb_ipc --all-targets --all-features` exits 0 with no warnings; the same exit-0 from `cargo test -p vb_ipc` exercises the existing `frame_types::tests::roundtrip_encode_decode` and friends.
- **Required:** true.
- **Owner state:** 4.
- **Rerun from:** 4.
- **Paired lane decisions:** VLD-qol58-A-001-proptest (and VLD-qol58-X-001-proptest for cross-cite).
- **Trusted base refs:** `TB-qol58-encode-byte-layout-preserved`.
- **Bridge:** None required (behavior_affecting=false).

### 2.3 PO-qol58-003 — Test-utility RNG fill canonicalization (cargo-test)

- **Requirement:** `REQ-LINT-CANONICALIZE-SEEDED-BYTES-NEW` (PS-qol58-B-001) + `REQ-LINT-CANONICALIZE-FIXTURE-BUILDER-BUILD-BYTES` (PS-qol58-C-001)
- **Contract clause:** `C-2+C-3`
- **Behavior-affecting:** `false`
- **Risk class:** `index_safety` (preventive).
- **Target:** `crates::workspace_tests::test_util::seed::SeededBytes::new` + `crates::workspace_tests::test_util::fixture::FixtureBuilder::build_bytes`.
- **Verifier:** `proptest`.
- **Command:** `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` (workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`).
- **Expected evidence:** `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` reports `test result: ok. N passed; 0 failed`; the 7 named tests (3 from `seed.rs:33-50`, 4 from `fixture.rs:67-90`) all pass.
- **Required:** true.
- **Owner state:** 4.
- **Rerun from:** 4.
- **Paired lane decisions:** VLD-qol58-B-001-proptest, VLD-qol58-C-001-proptest (and VLD-qol58-X-001-proptest for cross-cite).
- **Trusted base refs:** `TB-qol58-testutil-rng-determinism`.
- **Bridge:** None required (behavior_affecting=false).

## 3. Behavior-Affecting Coverage

All 3 obligations are `behavior_affecting: false`. This means:

- No `rust-refinement-obligation/v1` rows are required at State 7.
- No `E_BEHAVIOR_WAIVER` is possible (and none are present in `waiver-candidates.jsonl`).
- The `proof-to-implementation-input.md` bridge stub is not required (since every obligation is not behavior-affecting, the bridge produces zero rows).
- The `evidence-packaging` skill produces one requirement-to-evidence row per obligation; the landing gate consumes the bundle without needing refinement-bridge rows.

## 4. Coverage Gaps and Risks

### 4.1 Test-side patterns (out of default scope)

`delivery-scope.jsonl` rows 4-13 enumerate test-side `clippy::indexing_slicing`-class patterns in `crates/vb_ipc/src/{tests,frame/tests,frame_types/tests,client/tests,server/impl_tests}.rs`, `crates/vb_cli/tests/*`, and `crates/workspace_tests/tests/*`. These are **out of default scope** for this bead per `delivery-scope.jsonl:14` and `contract.md §7`. Follow-up beads can pick them up; this bead's verification does not cover them.

The 3 obligations are scoped narrowly to the 3 production sites per `delivery-scope.jsonl:14` (RECOMMENDED DEFAULT SCOPE).

### 4.2 Kani harness `#[cfg(kani)]` (out of scope)

`crates/vb_ipc/src/kani_*.rs` modules (21 + 10 + 10 + 12 + 30 + 1 = ~83 byte-slice patterns) are out of scope per `delivery-scope.jsonl:17` and `codebase-map.md §3.3`. Kani's own tool enforces its scoping; the `lint-src` gate excludes `cfg(kani)` modules per `.moon/tasks/all.yml:51`'s `--lib` target. No obligation is required.

### 4.3 Risk of behavior-preservation argument failing

Per AGENTS.md **GOD RULE 4** ("No Loop Oscillations — Fix the implementation rather than weakening the proof"), if `moon run :lint-src` reports a new warning caused by the refactor (e.g., the `as_mut_slice()` form introduces a different `clippy::indexing_slicing` violation), the implementation must be fixed, not the obligation relaxed. The expected evidence (`expected_evidence`) does not include any "if this fails, drop the flag" clause; if `moon run :lint-src` fails, the obligation is `FAIL_REGRESSION` per the formal-verifier authority.

## 5. Anti-Hallucination Markers

- The 3 obligations' targets are bound to production symbols (`crates/vb_ipc::frame_types::IpcFrameHeader::encode`, `SeededBytes::new`, `FixtureBuilder::build_bytes`), not file-only refs.
- The 3 obligations' commands are absolute-pathed and reproducible from the isolated workspace.
- The 3 obligations' expected_evidence cite concrete tool markers (`EXIT=0`, `test result: ok`).
- The 18 `not_applicable` lane decisions cite concrete SHA-256 evidence refs (artifact hashes), not vague reasons.
- No `cover!`-as-proof, no `assume` / `axiom` / `admit`, no `external_body` (none of the 3 obligations invoke a formal verifier).
- The 3 obligations are `behavior_affecting: false`; no `E_BEHAVIOR_WAIVER` is possible.
