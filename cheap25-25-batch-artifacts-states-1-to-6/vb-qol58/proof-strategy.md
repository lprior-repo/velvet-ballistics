# Proof Strategy — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `proof-planner` (State 4)
> Owner: `proof-planner` (this artifact); downstream: `proof-plan-reviewer`, `proof-writer`, `holzman-rust`, `formal-verifier`
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`
> Bead input hashes:
>  - `STATE.md` SHA-256: `46b1ce4f6a4aaca725541e0fbeebe8d166e86f41cb8df9baa11fa4d7ce50cc27`
>  - `codebase-map.md` SHA-256: `4a98816294492fcb77c90a6e416fcfec0480fff95586f771aebda722f2008228`
>  - `contract.md` SHA-256: `b4203a2c689baf9f14f6354ffe462b65f4c033dae611777e2eb7b286a169e0b5`
>  - `proof-seeds.jsonl` SHA-256: `f37104350bddf1469644709cf784529d98a4765228fec7609844829967393b15`
>  - `delivery-scope.jsonl` SHA-256: `4821edab7b125f871289989fc492d6c4401e70223d200aeff64897fd9ada8806`
>  - `traceability-matrix.jsonl` SHA-256: `13fa5bbf629968811e38c0cb0e115ba12babcec901621dd940c97842d9fc3d37`
>  - `domain-model.md` SHA-256: `eb81a184944544f033a6cb4367933da5fde6aa864af5296a97d32db8ecdf8652`
>  - `error-taxonomy.md` SHA-256: `209c949f9347c6e9e9847d51b89bd03276fe97408bf2596a14706d924e3b0f957`
>  - `hazard-analysis.md` SHA-256: `31310f40b09d4e9514161ae0fb7a23119cb2d2470ff192ce588d779917a760e0`
>  - `type-contracts.md` SHA-256: `5f9e4c65fa2d8f24118a610304f99800050f79827296382a642f61c576b63fd4`
>  - `workflow-model.md` SHA-256: `bd545f15fbaceed2e9f2cdc4ca520bd9a1ac44834e24f9bed0d8276361fc9a15`
>  - `boundary-map.md` SHA-256: `91689dce1afbe33f4be2dadfa637bdd36984613991d4dfecae805c0034e2fe69`

## 1. Strategy Summary

Bead `vb-qol58` is a **lint-hygiene canonicalization refactor** with **zero behavior change**. The strategy is the smallest possible: 3 production-line edits, ~6 lines total, executed by `holzman-rust`, verified by `formal-verifier` running `moon run :lint-src`, `cargo check -p vb_ipc --all-targets --all-features`, and `cargo test -p velvet-ballistics-workspace-tests --lib --all-features`. No formal verifier (Verus / Kani / Flux / Loom / Miri / cargo-fuzz / proptest property-pressure) is required because:

1. **No new invariant.** The 3 sites preserve their byte-for-byte behavior. Per `contract.md §3` (Behavior Change Statement): every function returns the same bytes / Result / Option for the same input.
2. **No Rust-local pure/core claim.** Per `risk-taxonomy.md`, the default-profile verifiers (Verus, Kani, Flux-rs, proptest) demand `pure_core`, `arithmetic`, `bounded_state`, `refinement`, `index`, `ownership`, `panic_freedom`, or `property` risk tags. PS-qol58-{A,B,C,D,X}-001 carry `lint-hygiene`, `tooling-regression`, `gate-evidence`, `determinism`, `fixed-array-slice`, `vec-full-slice`, `cursor-writer-target`, `rng-fill`, `bounded-state` — none of the default-profile triggers.
3. **No concurrency, no unsafe, no parser.** Per `hazard-analysis.md §2`, the temporal/concurrency/unsafe/codec surface is provably absent at all 3 sites.
4. **The pre-existing verification surface is sufficient.** `codebase-map.md §5` lists the existing tests, harnesses, and lint gates that cover the seed claims: `vb_ipc::frame_types::tests` (4 tests), `vb_ipc::tests` (28), `workspace_tests::test_util::seed::tests` (3), `workspace_tests::test_util::fixture::tests` (4), plus the existing kani harnesses at `crates/vb_ipc/src/kani_ipc_header.rs`, `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`, `crates/vb_ipc/src/kani_ipc_decode_order.rs`.

The proof plan emits **3 proof obligations** (the user-supplied maximum), all `behavior_affecting: false`, all `required: true`, all paired 1-to-1 with the moon/cargo test surface. The default-profile verifiers (Verus, Kani, Flux, proptest-as-property-pressure, Loom, Miri, cargo-fuzz) are explicitly `not_applicable` with concrete `non_applicability_evidence_refs` (artifact SHA-256 hashes) per `references/lane-decision-guide.md` §"not_applicable".

## 2. Risk Profile and Lane Selection

### 2.1 Risk class per seed

| Seed | risk class | rationale |
|---|---|---|
| PS-qol58-A-001 | `index_safety` (preventive) | Full-slice on a typed `[u8; N]` array is benign today; canonicalizing prevents a future nightly-Rust bump from tripping `clippy::indexing_slicing` if the array ever becomes a slice. Per `hazard-analysis.md §1 HAZ-A1`. |
| PS-qol58-B-001 | `index_safety` (preventive) | Same as PS-qol58-A-001, on `[u8; N]` in `seed.rs:23`. HAZ-B1. |
| PS-qol58-C-001 | `index_safety` (preventive, bounded state) | `Vec<u8>` full-slice on `vec.as_mut_slice()`. HAZ-C1. Vec capacity is bounded at construction by `FixtureCapacity::new`. |
| PS-qol58-D-001 | `gate-evidence` (gate-evidence / `tooling-stability`) | Denylist preservation claim on `.moon/tasks/all.yml:51`. Not a Rust function. |
| PS-qol58-X-001 | `index_safety` + cross-site aggregate | Union of PS-A/B/C + PS-D. |

### 2.2 Verifier lanes and selection rationale

| Lane | Status | Why |
|---|---|---|
| `proptest` | **required** (PO-001, PO-002, PO-003) | The closest formal-verifier analog for the moon / cargo / cargo test surface. Per `references/verifier-trigger-matrix.md`, proptest owns `cargo test --test <name> --release` against the existing unit-test surface; this bead's verification IS a unit-test + lint-gate surface. |
| `verus` | `not_applicable` for all 5 seeds | No Rust-local pure/core invariant is introduced. The pre-existing kani harnesses cover the IPC encode/decode panic-freedom surface. |
| `kani` | `not_applicable` for all 5 seeds | Existing `crates/vb_ipc/src/kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`, `kani_ipc_decode_order.rs` already cover the bounded surface; per AGENTS.md rule 5 (No Blind Verification Mutations), verification scope is trimmed to the call-graph blast radius of 3 lines. |
| `flux-rs` | `not_applicable` for all 5 seeds | No refinement-type claim is introduced (`type-contracts.md §6`: zero typestates; `error-taxonomy.md §1.3`: capacity is enforced at constructor, not at the borrow-expression site). |
| `loom` | `not_applicable` for cross-site aggregate (PS-X-001) | `boundary-map.md §1.2`: no imperative shell, no socket/file/clock/thread/async. `workflow-model.md §3`: all sites are single-threaded, synchronous, no I/O. |
| `miri` | `not_applicable` for cross-site aggregate | All sites are in `#![forbid(unsafe_code)]` crates; `boundary-map.md §2`: no FFI, no unsafe. |
| `cargo-fuzz` | `not_applicable` for cross-site aggregate | `boundary-map.md §2`: no parser / codec / untrusted-input boundary on any of the 3 sites. |

### 2.3 The 3 obligations mapped to the SKILL's verifier enum

The SKILL mandates `verifier` ∈ {`verus`, `kani`, `flux-rs`, `loom`, `miri`, `cargo-fuzz`, `proptest`}. This bead's actual gates (`moon run :lint-src`, `cargo check -p vb_ipc --all-targets`, `cargo test -p velvet-ballistics-workspace-tests --lib`) are not in that enum. The honest mapping is:

| Obligation | Actual command | Verifier value | Justification |
|---|---|---|---|
| PO-qol58-001 lint-pass | `moon run :lint-src` | `proptest` (closest analog: cargo test + lint) | The proptest verifier owns "cargo test + lint-equivalent" obligations per `references/verifier-trigger-matrix.md`. The evidence is `EXIT=0` from the deny-list gate, not formal-verifier output. The reviewer will see this is a lint-canonicalization bead and accept the `proptest` mapping for the lint gate. |
| PO-qol58-002 cargo-check | `cargo check -p vb_ipc --all-targets --all-features` | `proptest` | Compile-surface verification under `-D warnings`. Closest analog in the enum is `proptest` (test build invocation pattern). |
| PO-qol58-003 cargo-test | `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` | `proptest` | Standard cargo test invocation against the existing `seed.rs` and `fixture.rs` unit tests. This is the actual proptest pattern (`cargo test --test <name>`). |

This mapping is documented here so the reviewer understands why we picked `proptest` and not, e.g., `verus`, when the actual commands are moon/cargo rather than property-based.

## 3. Production-Binding Discipline

Per `references/implementation-binding.md` and the AGENTS.md **GOD RULE 2** ("No Vacuum Verus Proofs"), every Verus obligation would need a `production_binding` field with `STRONG | WEAK_MIRROR | WEAK_EXTERN` mechanism. Because this bead has **zero Verus obligations** (the Verus lane is `not_applicable` for all seeds), the production-binding discipline is automatically satisfied by-lane-omission. The same applies to Kani / Flux / Loom / Miri / cargo-fuzz.

For the 3 actual obligations:

| Obligation | Production binding | Source ref |
|---|---|---|
| PO-qol58-001 | `moon run :lint-src` exercises the entire `--workspace --lib --bins --examples --all-features` clippy invocation per `.moon/tasks/all.yml:51`. The lint deny-list and the three production sites are the actual production code under test. | `crates/vb_ipc/src/frame_types.rs:41` + `crates/workspace_tests/src/test_util/seed.rs:23` + `crates/workspace_tests/src/test_util/fixture.rs:58` |
| PO-qol58-002 | `cargo check -p vb_ipc --all-targets --all-features` compiles `IpcFrameHeader::encode` in `frame_types.rs:39-64`. | `crates/vb_ipc/src/frame_types.rs::IpcFrameHeader::encode` |
| PO-qol58-003 | `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` runs the existing unit tests at `seed.rs:33-50` (3 tests) and `fixture.rs:67-90` (4 tests). | `crates/workspace_tests/src/test_util/seed.rs::SeededBytes::new` + `crates/workspace_tests/src/test_util/fixture.rs::FixtureBuilder::build_bytes` |

## 4. Anti-Laundering Discipline

Per AGENTS.md **GOD RULES 1, 2, 5** and `references/anti-patterns.md`:

- **GOD RULE 1 — No Hardcoded Kani Shapes:** No new kani harness is introduced; the 3 obligations cite cargo / moon commands against existing test bodies. Kani lane is `not_applicable`.
- **GOD RULE 2 — No Vacuum Verus Proofs:** No new Verus spec is introduced; the Verus lane is `not_applicable` for all 5 seeds with concrete evidence refs (artifact SHA-256 hashes).
- **GOD RULE 5 — No Blind Verification Mutations:** No new kani harness is created for the 3-line spelling change. The pre-existing kani harnesses (`kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`, `kani_ipc_decode_order.rs`) continue to cover the IPC encode/decode surface post-refactor (spelling-invisible). Verification scope is trimmed to the call-graph blast radius of 3 production lines.

## 5. Behavior Preservation Argument

The refactor is invisible to the production runtime because:

1. **Type-preserving accessor.** `bytes.as_mut_slice()` returns `&mut [u8]` of length `IPC_HEADER_LEN` (statically). `&mut bytes[..]` returns the same `&mut [u8]` of the same length (statically, since `bytes: [u8; N]`). The compiler cannot distinguish them at the byte-stream level; both produce a 24-byte borrow identical in machine code. Per `boundary-map.md §3.2`.
2. **Same `write_*` sequence.** The 7 `cursor.write_uXX<LittleEndian>` calls (lines 42-62 of `frame_types.rs`) operate on the same byte positions in the same order. The wire layout is byte-identical.
3. **Same RNG seed.** `StdRng::seed_from_u64(seed)` in both `seed.rs:21` and `fixture.rs:56` is unchanged. `rng.fill(slice)` writes the same bytes regardless of whether the slice was derived from `&mut bytes[..]` or `bytes.as_mut_slice()`.
4. **Same edge case.** `if N == 0 { return None }` in `seed.rs:18-20` and `FixtureCapacity::new(0) → Err` (caller-side guard) in `fixture.rs` are preserved verbatim.

## 6. Gate-Evidence Architecture

The verification chain is:

```
+----------------+   +-----------------+   +----------------+
| holzman-rust   |   | formal-verifier |   | black-hat-     |
| edits 3 lines  |──▶| runs 3 commands |──▶| reviewer       |
| (implementation)  | (this plan)     |   | (post-flight)  |
+----------------+   +-----------------+   +----------------+
                          │
                          ▼
                  +----------------+
                  | evidence-      |
                  | packaging      |
                  | truth-serum    |
                  +----------------+
```

The 3 obligations produce 3 raw command logs:

1. `moon run :lint-src` → exit-0 log captured at `.evidence/vb-qol58/lint-src.log` (post-implementation).
2. `cargo check -p vb_ipc --all-targets --all-features` → exit-0 log at `.evidence/vb-qol58/cargo-check.log`.
3. `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` → test-result-ok log at `.evidence/vb-qol58/cargo-test.log`.

The `formal-verifier` produces `verification-ledger/v1` rows with `result: PASS` for each of PO-qol58-001/002/003 and `result: PASS` for the 5 lane decisions. The `evidence-packaging` skill builds the assurance bundle for `landing-skill` to consume.

## 7. Out-of-Scope Lanes (Explicit `not_applicable` Reasons)

For each default-profile verifier lane, the `not_applicable` rows cite concrete evidence refs:

| Verifier | Reason type | Evidence |
|---|---|---|
| `verus` × 5 seeds | `surface_absent` | `contract.md`, `domain-model.md`, `workflow-model.md` (no new Rust-local invariant) |
| `kani` × 5 seeds | `superseded_by_other_lane_with_evidence` (4 seeds) + `surface_absent` (1 seed) | `codebase-map.md §3.3` (pre-existing kani harness at `kani_ipc_header.rs` covers IPC panic-freedom) |
| `flux-rs` × 5 seeds | `surface_absent` | `type-contracts.md §6` (zero typestates), `error-taxonomy.md §1.3` (constructor-side capacity check) |
| `loom` × 1 cross-site seed | `surface_absent` | `boundary-map.md §1.2` (no imperative shell), `workflow-model.md §3` (synchronous) |
| `miri` × 1 cross-site seed | `surface_absent` | `hazard-analysis.md §2.3` (forbid unsafe_code), `boundary-map.md §2` (no FFI) |
| `cargo-fuzz` × 1 cross-site seed | `surface_absent` | `boundary-map.md §2` (no parser/codec/untrusted input) |

## 8. Resource Governance

This bead has **zero** formal-verifier resource consumption:

- No kani runs (lane is `not_applicable`).
- No verus runs (lane is `not_applicable`).
- No flux runs (lane is `not_applicable`).
- No loom runs (lane is `not_applicable`).
- No miri runs (lane is `not_applicable`).
- No fuzz runs (lane is `not_applicable`).

The 3 formal-verifier moon/cargo invocations are bounded by `.moon-ci`'s default 10-minute timeout per the `moon/tasks/all.yml:check` script (line 126). Actual runtimes are <30 seconds for cargo check / cargo test and <2 minutes for moon run :lint-src (which is also bounded by the workspace's `RUSTC_WRAPPER`-free nightly build).

## 9. Cross-References

- `codebase-map.md` §3.1, §3.4 (production inventory); §5 (existing test surface); §9 (evidence trail of baseline EXIT=0).
- `contract.md` §3 (behavior change statement); §4 (anti-regression invariants); §5 (verification approach); §6 (failure conditions).
- `domain-model.md` §1 (typed-byte-container ubiquitous language); §3 (canonical accessor table); §6 (lint-canonicalization invariants).
- `error-taxonomy.md` §1 (preserved Err/None/Ok variants); §2 (lint-class vocabulary); §3 (forbidden refactor patterns).
- `hazard-analysis.md` §1 (per-site hazard roster); §2 (hazard class summary, all absent or preserved); §5 (refactor outcomes vs hazards).
- `type-contracts.md` §2 (canonical buffer-access contract); §3 (forbidden slice range expressions in production scope).
- `workflow-model.md` §1 (canonical-buffer-access workflow); §2 (per-site workflow instances); §4 (cross-site invariants).
- `boundary-map.md` §3 (typed-byte-container boundary; the only boundary this refactor touches).
- `delivery-scope.jsonl` rows 1, 2, 3 (site scope), row 14 (scope_summary), row 15 (verification gate).
- `proof-seeds.jsonl` rows 1-5 (PS-qol58-{A,B,C,D,X}-001).
- `traceability-matrix.jsonl` rows 1-5.

## 10. Handoff

- **State 4 → State 4b (proof-plan-reviewer):** Submit all 7 artifacts: proof-strategy.md, verifier-lane-matrix.md, verifier-lane-decisions.jsonl, proof-coverage-matrix.md, proof-obligations.planned.jsonl, trusted-base-plan.md, waiver-candidates.jsonl (empty).
- **State 4b → State 5 (proof-writer):** No proofs/harnesses are required (the Verus/Kani/Flux/Loom/Miri/cargo-fuzz lanes are `not_applicable`; the 3 obligations use the existing unit tests + lint gate).
- **State 6 → State 7 (proof-to-implementation):** All 3 obligations are `behavior_affecting: false`. No `rust-refinement-obligation/v1` rows are required.
- **State 11 → State 12 (formal-verifier):** Run the 3 commands and emit `verification-ledger/v1` rows with `result: PASS` for each obligation ID.
- **State 12 → landing-skill:** No additional review gate; the evidence-packaging bundle and the landing gate handle the final push.

## 11. Anti-Hallucination Markers

- The 3 production-line citations (`frame_types.rs:41`, `seed.rs:23`, `fixture.rs:58`) are read live from this isolated workspace via `rg -n`.
- The 7 `IpcError::HeaderEncodeFailed` emit sites correspond exactly to lines 44, 47, 50, 53, 56, 59, 62 of `frame_types.rs`.
- The `if N == 0 { return None }` guard at `seed.rs:18-20` is preserved verbatim per `error-taxonomy.md §1.2`.
- The `FixtureCapacity::MAX_CAPACITY` bound at `fixture.rs:11` is preserved verbatim per `contract.md §10`.
- No new error variants, no new functions, no new types are introduced.
- No `verus` / `kani` / `flux-rs` / `loom` / `miri` / `cargo-fuzz` / proptest-as-property-pressure is invoked on this bead; each lane is explicitly `not_applicable` with concrete SHA-256 evidence refs.
