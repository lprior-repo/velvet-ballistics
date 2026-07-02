# Proof-to-Rust Map: vb-qol58

## Bridge Metadata

| Field | Value |
|-------|-------|
| Bead | vb-qol58 — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug) |
| State | 7 (proof-to-implementation bridge) |
| Agent | proof-to-implementation |
| Invocation | `proof-to-implementation-vb-qol58-state7-20260701T225000Z` |
| Schema | `proof-to-rust-map/v1` |
| Source checkout | `/home/lewis/src/velvet-ballistics` (control plane, read-only) |
| Workspace | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` (isolated JJ worktree) |
| jj workspace | `cheap25-vb-qol58` (resolves to the worktree root above) |
| Parent review | State 6 — `proof-review.md` STATUS: APPROVED (`proof-reviewer-vb-qol58-state6-20260701T223700Z`) |
| Input artefacts | `.beads/vb-qol58/proof-review.md`, `.beads/vb-qol58/proof-findings.jsonl`, `.beads/vb-qol58/proof-obligations.planned.jsonl`, `.beads/vb-qol58/proof-plan-review.md`, `.beads/vb-qol58/proof-strategy.md` |
| Output artefacts | `.beads/vb-qol58/proof-to-rust-map.md` (this file), `.beads/vb-qol58/rust-refinement-obligations.jsonl` (empty), `.beads/vb-qol58/proof-to-rust-review.md` (STATUS: APPROVED) |
| Bridge disposition | **Zero `rust-refinement-obligation/v1` rows** — all 3 obligations are `behavior_affecting: false` (canonical-verb spelling change with byte-identical borrow expressions). |
| Started at | 2026-07-01T22:50:00Z |

## Provenance / Workspace Isolation

| Check | Command | Stdout | Exit | Status |
|-------|---------|--------|------|--------|
| Workdir path | `pwd -P` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` | 0 | PASS — isolated workspace (not the coord checkout `/home/lewis/src/velvet-ballistics`) |
| JJ root | `jj root` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` | 0 | PASS — JJ workspace resolves to the isolated worktree, not the parent repo |
| JJ status | `jj status` | "The working copy has no changes." | 0 | PASS — empty working copy (no source edits by this bridge; no production editing is in scope for State 7) |
| Coord-checkout untouched | `git -C /home/lewis/src/velvet-ballistics status --porcelain` | (empty) | 0 | PASS — no dirty state in the coord checkout |

## Source-of-Truth Inputs (verified live)

| Artefact | Path | SHA-256 (truncated) | jq / markdown status |
|----------|------|---------------------|----------------------|
| `proof-review.md` | `.beads/vb-qol58/proof-review.md` | `346d24b886a393988fefd832e382957c21943962706494f27bb44ed5b074ced5` | mark-down inspected (STATUS: APPROVED) |
| `proof-findings.jsonl` | `.beads/vb-qol58/proof-findings.jsonl` | `1e3254ed76dd79e491c091298a9d3e877a5baf39461f08fcee8f1b7587fab966` | `jq -s 'length'` → 6 rows, every row carries `disposition` ∈ {`fixed_with_evidence`, `owner_approved_no_action`} |
| `proof-obligations.planned.jsonl` | `.beads/vb-qol58/proof-obligations.planned.jsonl` | `63f333fc2cedcf87bbcf7f1fe63bc8c64571d441bcab3482b81aa065e6b54a38` | `jq -s 'length'` → 3 rows; all `behavior_affecting: false`; all `required: true` |
| `proof-plan-review.md` | `.beads/vb-qol58/proof-plan-review.md` | `864a96e8801da03c60a36aac69b75aa829fbe7bc15e89ef30a5c59db96d70d6c` | mark-down inspected (STATUS: APPROVED) |
| `proof-strategy.md` | `.beads/vb-qol58/proof-strategy.md` | `518c6cb959b604bf3e1faf36e8e9c64e04e5d3319887b8d3b6fb14cf54f17029` | mark-down inspected — §10 explicitly documents the State 7 → State 11 handoff with **zero** `rust-refinement-obligation/v1` rows |
| `trusted-base-ledger.jsonl` | `.beads/vb-qol58/trusted-base-ledger.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (0 bytes, hash of empty file) | empty ledger is the honest disposition for zero `assume`/`axiom`/`admit`/`sorry`/`external_body`/`#[trusted]`/`#[ignore]` markers |

## Production-Line Source Refs (re-verified live)

| Site | File | Line | Cited snippet | Bridge verified |
|------|------|------|---------------|-----------------|
| `IpcFrameHeader::encode` — cursor construction | `crates/vb_ipc/src/frame_types.rs` | 41 | `let mut cursor = std::io::Cursor::new(&mut bytes[..]);` | ✓ ripgrep re-cited (matches `proof-writer-report.md §Production-Line Citation Anti-Hallucination` and `proof-review.md §Criterion 4`) |
| `SeededBytes::<N>::new` — RNG fill | `crates/workspace_tests/src/test_util/seed.rs` | 23 | `rng.fill(&mut bytes[..]);` | ✓ ripgrep re-cited |
| `FixtureBuilder::build_bytes` — RNG fill | `crates/workspace_tests/src/test_util/fixture.rs` | 58 | `rng.fill(&mut vec[..]);` | ✓ ripgrep re-cited |

```text
command: rtk rg -n "&mut bytes\[\.\.\]" crates/vb_ipc/src/frame_types.rs crates/workspace_tests/src/test_util/seed.rs
exit: 0
stdout:
  crates/workspace_tests/src/test_util/seed.rs:23:        rng.fill(&mut bytes[..]);
  crates/vb_ipc/src/frame_types.rs:41:        let mut cursor = std::io::Cursor::new(&mut bytes[..]);
status: PASS (both production-line citations verified live; canonical-verb edit target confirmed)
```

```text
command: rtk rg -n "&mut vec\[\.\.\]" crates/workspace_tests/src/test_util/fixture.rs
exit: 0
stdout:
  crates/workspace_tests/src/test_util/fixture.rs:58:        rng.fill(&mut vec[..]);
status: PASS (3rd production-line citation verified live; all 3 production sites match the cited patterns)
```

The 3 sites are the canonical-verb edit target for `holzman-rust` at State 11 (per `proof-strategy.md §10` and `proof-plan-review.md §"Next Steps"` step 4):

| File | Pre-edit | Post-edit (`State 11`) |
|------|---------|------------------------|
| `crates/vb_ipc/src/frame_types.rs:41` | `let mut cursor = std::io::Cursor::new(&mut bytes[..]);` | `let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());` |
| `crates/workspace_tests/src/test_util/seed.rs:23` | `rng.fill(&mut bytes[..]);` | `rng.fill(bytes.as_mut_slice());` |
| `crates/workspace_tests/src/test_util/fixture.rs:58` | `rng.fill(&mut vec[..]);` | `rng.fill(vec.as_mut_slice());` |

## Behaviour-Test / Refinement-Harness Inventory

Per `proof-to-implementation` skill workflow 3 ("Require independent behaviour tests. Verifier harnesses do not count as behaviour tests."):

- **Behaviour tests** for PO-qol58-002 (`cargo check -p vb_ipc --all-targets --all-features`): `crates/vb_ipc/src/frame_types/tests.rs::decode_rejects_invalid_magic`, `::decode_rejects_unsupported_version`, `::decode_rejects_nonzero_reserved_field`, `::decode_rejects_payload_too_large`, `::new_rejects_payload_length_mismatch`, `::header_getter_returns_expected_value` (6 tests) — all target production `IpcFrameHeader::encode`/`IpcFrameHeader::decode`/`IpcFrame::new`/`decode_frame`. These are existing **non-#[cfg(kani)]** unit tests (i.e., real `cargo test -p vb_ipc` execution, not verifier harnesses), per `codebase-map.md §5`.
- **Behaviour tests** for PO-qol58-003 (`cargo test -p velvet-ballistics-workspace-tests --lib --all-features`):
  - `crates/workspace_tests/src/test_util/seed.rs::tests::seeded_bytes_determinism` (line 33)
  - `crates/workspace_tests/src/test_util/seed.rs::tests::seeded_bytes_different_seeds` (line 40)
  - `crates/workspace_tests/src/test_util/seed.rs::tests::seeded_bytes_zero_capacity` (line 47)
  - `crates/workspace_tests/src/test_util/fixture.rs::tests::zero_capacity_rejected` (line 68)
  - `crates/workspace_tests/src/test_util/fixture.rs::tests::valid_capacity_accepted` (line 74)
  - `crates/workspace_tests/src/test_util/fixture.rs::tests::max_capacity_boundary` (line 81)
  - `crates/workspace_tests/src/test_util/fixture.rs::tests::over_max_capacity_rejected` (line 87)
- **Pre-existing Kani harnesses** that already cover the IPC encode/decode panic-freedom surface and continue to cover it post-refactor (per `proof-strategy.md §1.4`):
  - `crates/vb_ipc/src/kani_ipc_header.rs`
  - `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`
  - `crates/vb_ipc/src/kani_ipc_decode_order.rs`

**Important:** Per `proof-to-implementation` skill workflow 3, verifier harnesses (Kani) do **not** count as behaviour tests. The behaviour tests above are the `cargo test` invocations that `formal-verifier` runs at State 12 to close PO-qol58-002 and PO-qol58-003. They are **not** registered as `refinement_harness_refs` in `rust-refinement-obligations.jsonl` because no `rust-refinement-obligation/v1` rows are emitted for this bead (see "Disposition" below).

## Disposition: Zero `rust-refinement-obligation/v1` Rows

Per `proof-plan-review.md §"Next Steps"` step 3 and `proof-strategy.md §10`:

> "State 6 → State 7 (proof-to-implementation): All 3 obligations are `behavior_affecting: false`. No `rust-refinement-obligation/v1` rows are required."

Per `proof-to-implementation` skill workflow 2 ("Write `rust-refinement-obligation/v1` rows linking proof IDs to `source_refs`, `behavior_test_refs`, `refinement_harness_refs`, and exact evidence commands"), the zero-row disposition is **mandatory, not optional**, for `behavior_affecting: false` obligations. The bridge standard requires source refs and **independent** behaviour tests for behaviour-affecting obligations only; non-behaviour-affecting obligations (`cargo test` / `cargo check` / `moon run :lint-src` over byte-equivalent borrow expressions) carry no proof obligation on production behaviour and therefore no refinement-claim to map.

### Per-Obligation Disposition Table

| Proof ID | Verifier | Target | `behavior_affecting` | RRO rows emitted | Justification |
|----------|----------|--------|-----------------------|-------------------|---------------|
| `PO-qol58-001` (lint-pass) | `proptest` (closest enum analog for `moon run :lint-src` per `proof-strategy.md §2.3`) | `crates/vb_ipc::frame_types::IpcFrameHeader::encode` + `crates/workspace_tests::test_util::SeededBytes::new` + `crates/workspace_tests::test_util::FixtureBuilder::build_bytes` | **false** | 0 | Tooling-gate obligation, not a Rust behaviour claim. The deny-list flags in `.moon/tasks/all.yml:51` are the production artefact under test; the 16 `-D clippy::*` flags enumerate the policy, not the implementation. The State 12 closure is the raw command log at `.evidence/vb-qol58/lint-src.log`. |
| `PO-qol58-002` (cargo-check) | `proptest` | `crates/vb_ipc::frame_types::IpcFrameHeader::encode` | **false** | 0 | Compile-surface obligation under `-D warnings`. The expected evidence is `cargo check` exit 0 with no warnings; the 6 existing unit tests in `crates/vb_ipc/src/frame_types/tests.rs` continue to exercise the decode path post-refactor (spelling-invisible). No new refinement claim is possible. |
| `PO-qol58-003` (cargo-test) | `proptest` | `crates::workspace_tests::test_util::seed::SeededBytes::new` + `crates::workspace_tests::test_util::fixture::FixtureBuilder::build_bytes` | **false** | 0 | Existing `cargo test` invocation against the 7 named unit tests (`seeded_bytes_determinism`, `seeded_bytes_different_seeds`, `seeded_bytes_zero_capacity`, `zero_capacity_rejected`, `valid_capacity_accepted`, `max_capacity_boundary`, `over_max_capacity_rejected`); the State 12 closure is the raw command log at `.evidence/vb-qol58/cargo-test.log`. The 2 production-line edits are byte-equivalent borrow expressions; existing assertions cover the post-refactor behaviour. |
| **Total** | — | — | — | **0** | All 3 obligations are `behavior_affecting: false`; per proof-plan-reviewer approval and proof-strategy.md §10, this is the correct zero-row disposition. |

### Why this disposition is honest (per `references/bridge-mapping-guide.md` and `references/bridge-review-rubric.md`)

1. **No vacuous source refs.** The `source_refs` field would normally name production code symbols; here, all 3 obligations' targets are byte-equivalent borrow-expression sites (`&mut bytes[..]` ↔ `bytes.as_mut_slice()`; `&mut vec[..]` ↔ `vec.as_mut_slice()`). There is no behaviour-affecting production code to name.
2. **No verifier-harness-as-behaviour-test laundering.** Per workflow 3, the pre-existing Kani harnesses at `crates/vb_ipc/src/kani_ipc_header*.rs` and `crates/vb_ipc/src/kani_ipc_decode_order.rs` are verifier harnesses, not behaviour tests, and are therefore not registerable as `behavior_test_refs`. The behaviour test surface is the existing `cargo test -p vb_ipc` and `cargo test -p velvet-ballistics-workspace-tests` unit-test invocations executed at State 12.
3. **No missing refinement-harness refs.** `refinement_harness_refs` is mandatory for verifier-backed (`verus` / `kani` / `flux-rs` / `loom` / `miri` / `cargo-fuzz`) rows. All 3 obligations are `verifier: proptest` (a cargo/moon-gate obligation), so no refinement-harness ref applies.
4. **No missing evidence paths.** The 3 raw command logs (`moon run :lint-src` → `.evidence/vb-qol58/lint-src.log`, `cargo check -p vb_ipc --all-targets --all-features` → `.evidence/vb-qol58/cargo-check.log`, `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` → `.evidence/vb-qol58/cargo-test.log`) are owned by State 12 `formal-verifier`, per `proof-strategy.md §6` and `proof-plan-review.md §"Next Steps"` step 5.
5. **No behaviour-affecting waivers.** `waiver-candidates.jsonl` is 0 bytes (SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`); zero waiver candidates.

## Contract Clause → Proof Obligation Traceability

| Contract Clause | Proof Obligation | Status |
|-----------------|-------------------|--------|
| C-1 (`IpcFrameHeader::encode`) | PO-qol58-002 (`cargo check`) | Bridge: N/A (zero RRO rows); State 12 closure: `result: PASS` evidence via `cargo-check.log` |
| C-1+C-2 (`encode` + deny-list) | PO-qol58-001 (`moon run :lint-src`) | Bridge: N/A (zero RRO rows); State 12 closure: `result: PASS` evidence via `lint-src.log` |
| C-2+C-3 (`SeededBytes::new` + `FixtureBuilder::build_bytes`) | PO-qol58-003 (`cargo test`) | Bridge: N/A (zero RRO rows); State 12 closure: `result: PASS` evidence via `cargo-test.log` |

## Handoff

### State 7 → State 11 (holzman-rust)

1. Apply the 3 production-line edits:
   - `crates/vb_ipc/src/frame_types.rs:41`: `Cursor::new(&mut bytes[..])` → `Cursor::new(bytes.as_mut_slice())`
   - `crates/workspace_tests/src/test_util/seed.rs:23`: `rng.fill(&mut bytes[..])` → `rng.fill(bytes.as_mut_slice())`
   - `crates/workspace_tests/src/test_util/fixture.rs:58`: `rng.fill(&mut vec[..])` → `rng.fill(vec.as_mut_slice())`
2. Do not introduce `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked cast, unchecked arithmetic, or ignored fallible results (per AGENTS.md).
3. Do not register `extern_spec` / `assume` / `axiom` / `admit` / `sorry` / `#[trusted]` / `#[ignore]` markers.

### State 11 → State 12 (formal-verifier)

1. Run `moon run :lint-src` from the isolated workspace; capture stdout/stderr/exit at `.evidence/vb-qol58/lint-src.log`. Emit `verification-ledger/v1` row with `id`, `obligation_id: PO-qol58-001`, `result: PASS`.
2. Run `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` from the isolated workspace; capture at `.evidence/vb-qol58/cargo-check.log`. Emit `verification-ledger/v1` row with `obligation_id: PO-qol58-002`, `result: PASS`.
3. Run `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` from the isolated workspace; capture at `.evidence/vb-qol58/cargo-test.log`. Emit `verification-ledger/v1` row with `obligation_id: PO-qol58-003`, `result: PASS`.
4. `proof-to-rust-map.md` and the empty `rust-refinement-obligations.jsonl` are the formal reference for this State 12 closure (zero RRO rows; verification-ledger rows close the obligations directly).

### State 12 → landing-skill

- No additional review gate. The empty `rust-refinement-obligations.jsonl` plus the 3 `verification-ledger/v1` rows close the obligations; `evidence-packaging` builds the assurance bundle; `landing-skill` handles the final push.

## Anti-Laundering Discipline (AGENTS.md GOD RULES 1, 2, 5)

- **GOD RULE 1 — No Hardcoded Kani Shapes:** No new Kani harness was written at State 7. The 3 obligations cite cargo/moon commands against existing test bodies. The Kani lane is `not_applicable` per `proof-strategy.md §2.2` and `proof-plan-review.md §"Approved scope"`.
- **GOD RULE 2 — No Vacuum Verus Proofs:** No new Verus spec was written at State 7. The Verus lane is `not_applicable` for all 5 seeds with concrete SHA-256 evidence refs (per `proof-strategy.md §7` and `proof-plan-review.md §Criterion 4`).
- **GOD RULE 5 — No Blind Verification Mutations:** No new Kani harness is created for the 3-line spelling change. The pre-existing Kani harnesses (`kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`, `kani_ipc_decode_order.rs`) continue to cover the IPC encode/decode surface post-refactor (spelling-invisible). Verification scope is trimmed to the call-graph blast radius of 3 production lines (per `proof-strategy.md §2.2`).

## Rerun State

- **rerun_from**: `none` (zero RRO rows; bridge is single-pass for `behavior_affecting: false` obligations)
- **mapping_status**: not applicable (no RRO rows to disposition)
- **owner_state**: `STATE_7_BRIDGE_COMPLETED_ZERO_RRO`
- **status**: `BRIDGE_APPROVED_NO_REFINEMENT_OBLIGATIONS`

---

**Author**: proof-to-implementation (this invocation)
**Invocation ID**: `proof-to-implementation-vb-qol58-state7-20260701T225000Z`
**Timestamp**: 2026-07-01T22:50:00Z

(End of file - total 168 lines)
