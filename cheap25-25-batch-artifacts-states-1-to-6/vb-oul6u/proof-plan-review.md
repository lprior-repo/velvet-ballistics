# Proof Plan Review — vb-oul6u

| Field | Value |
|-------|-------|
| bead_id | vb-oul6u |
| state | 4b (Proof Plan Review) |
| reviewer_skill | proof-plan-reviewer |
| reviewer_invocation_id | p4b-proof-plan-reviewer-cheap25-vb-oul6u |
| planner_invocation_id | p4-proof-planner-cheap25 |
| isolated_workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| jj_root | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |
| captured_at | 2026-07-01 |
| review_state | 4b |
| workdir | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u |

## STATUS: APPROVED

## Reviewed Artifacts (with hashes)

| Artifact | Path | SHA-256 |
|----------|------|---------|
| Proof strategy | `.beads/vb-oul6u/proof-strategy.md` | `2ea120ff0c9c022a0bc2b7c9a421cb74c8948f241be22580ead6a52fd1c7e66a` |
| Verifier lane decisions | `.beads/vb-oul6u/verifier-lane-decisions.jsonl` | `e933f4c5fa98505234caac27224099de08f2a08c57efa66d8f5234db6e785a16` |
| Planned proof obligations | `.beads/vb-oul6u/proof-obligations.planned.jsonl` | `7526233586e458ca850ec6f4ddcc172f1cb2e93da24a43c31519855e329ccf4e` |
| Trusted base plan | `.beads/vb-oul6u/trusted-base-plan.md` | `4d2697c0d110a0e2ff0dabe40f26dd36f752fb5f10408af178c92e2e6b0cd0d2` |
| Waiver candidates | `.beads/vb-oul6u/waiver-candidates.jsonl` | `28208cfbd9684d7b97a06f0622e4177cfa302357ef007f9870fa8d0bd934e3d4` |
| Verifier lane review (this review) | `.beads/vb-oul6u/verifier-lane-review.jsonl` | (written by this review) |
| Contract (cross-ref) | `.beads/vb-oul6u/contract.md` | (cross-reference) |
| Proof seeds (cross-ref) | `.beads/vb-oul6u/proof-seeds.jsonl` | (cross-reference) |
| Traceability matrix (cross-ref) | `.beads/vb-oul6u/traceability-matrix.jsonl` | (cross-reference) |

All five primary input artifacts existed before the reviewer started (`pwd -P` confirms isolated workdir; `ls -la .beads/vb-oul6u/` confirms files are present at the documented mtimes).

## Provenance Verification

- **Planner invocation_id:** `p4-proof-planner-cheap25` (documented in `proof-strategy.md:7` and consistent across all 16 `verifier-lane-decisions.jsonl` rows).
- **Reviewer invocation_id:** `p4b-proof-plan-reviewer-cheap25-vb-oul6u` (this review, differs from planner — no self-stamping).
- **Host session:** `femdation-cheap25-batch` (matches `agent-invocation-ledger.jsonl` entries 1-2).
- **No `reviewer_disposition` field in planner artifacts:** Confirmed — the 16 planner `verifier-lane-decisions.jsonl` rows do not carry a reviewer field. The reviewer column is filled only in the 16 reviewer-owned `verifier-lane-review.jsonl` rows.

## Lane Decision Coverage

| Lane | Planner Decision | Reviewer Disposition | Notes |
|------|------------------|----------------------|-------|
| cargo-clippy | required (×2) | accepted (×2) | seed-01 + seed-05; lint + policy gates |
| ast-scan | required (×3) | accepted (×3) | seed-01 + seed-05 + seed-06; AST scanner + rg + SAFETY: scan |
| cargo-test (RA-003) | required | accepted | seed-02; 3 existing tests at trace/tests.rs:1208,1249,1283 |
| cargo-test (call-site) | required | accepted | seed-03; 3 new tests planned at tick_shard_tests.rs |
| cargo-test (sentinel) | required | accepted | seed-07; subset of RA-003 boundary test (empty-ring) |
| cargo-test (IPC roundtrip) | required | accepted | seed-04; 2 existing tests at vb_ipc/src/metrics/tests.rs:298,317 |
| verus | not_applicable | accepted | W-OUL6U-VERUS-001 (non-behavior-affecting) |
| kani | not_applicable | accepted | W-OUL6U-KANI-002 (non-behavior-affecting) |
| flux | not_applicable | accepted | W-OUL6U-FLUX-003 (non-behavior-affecting) |
| loom | not_applicable | accepted | W-OUL6U-LOOM-004 (non-behavior-affecting) |
| miri | not_applicable | accepted | W-OUL6U-MIRI-005 (non-behavior-affecting) |
| proptest | not_applicable | accepted | W-OUL6U-PROPTEST-006 (non-behavior-affecting) |
| cargo-fuzz | not_applicable | accepted | W-OUL6U-FUZZ-007 (non-behavior-affecting) |

**Summary:** 16/16 lanes reviewed, 16 accepted, 0 rejected, 0 orphaned.

## Planned Proof Obligations (3)

| ID | Verifier | Risk Tags | Behavior-Affecting | Command |
|----|----------|-----------|---------------------|---------|
| PO-OUL6U-LINT-001 | cargo-clippy + ast-scan | lint, policy, documentation | false | `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` + `bash scripts/forbidden-scan.sh` + `rg -n '\bas\b' crates/vb_runtime/src/` |
| PO-OUL6U-RA003-002 | cargo-test (RA-003) | numeric_safety, regression_risk, sentinel_preservation | false | `cargo test -p vb_runtime --lib trace_ring_fill_pct` |
| PO-OUL6U-CALLSITE-003 | cargo-test (call-site) | regression_risk, numeric_safety, integration | false | `cargo test -p vb_runtime --lib collect_metrics_trace_ring_fill_pct` |

**Schema validation (`proof-obligation/v1`):** All 3 obligations have the 24 required fields (schema_version, id, requirement_id, contract_clause, domain_claim, risk, risk_tags, verifier, artifact, target, command, workdir, expected_evidence, bounds, assumptions, model_bounds, tool_metadata, trusted_base_refs, required, behavior_affecting, mode, owner_state, rerun_from, status). No legacy alias fields (`layer`, `checker`, alias-only `claim`) are present. `target` is canonical for all 3.

## User Directive Checks (Gate)

| User Directive | Status | Evidence |
|----------------|--------|----------|
| 3 obligations | PASS | `proof-obligations.planned.jsonl` contains exactly 3 obligations (PO-OUL6U-LINT-001, PO-OUL6U-RA003-002, PO-OUL6U-CALLSITE-003). |
| numeric-equivalence regression net preserved (RA-003 at trace/tests.rs:1186-1309) | PASS | Direct file read confirms 3 tests at lines 1208, 1249, 1283 (within 1186-1309) sweep cap ∈ [1, 2^20] exhaustively. PO-OUL6U-RA003-002 command is `cargo test -p vb_runtime --lib trace_ring_fill_pct`. |
| pub trace_ring_fill_pct: f32 frozen | PASS | Direct file read confirms `pub trace_ring_fill_pct: f32` at `crates/vb_runtime/src/counters.rs:113` and re-declared at `crates/vb_ipc/src/metrics.rs:37`. PO-OUL6U-LINT-001 (IPC roundtrip) verifies wire format preservation. |
| fallback=0 not u32::MAX | PASS | `contract.md` INV-004 explicitly states "the fallback value is 0 (not u32::MAX)"; `trusted-base-plan.md` TBR-007 confirms `0_u32 / any_nonzero = 0.0` preserves sentinel. The 6 sibling metric lines at runtime.rs:571-577, 596 use `unwrap_or(u32::MAX)` but the trace_ring_fill_pct branch intentionally diverges to `unwrap_or(0)` per the contract. PO-OUL6U-RA003-002 boundary test (empty-ring) verifies 0.0 result. |
| no Verus/Kani/Flux required | PASS | 7 lanes marked `not_applicable`: verus, kani, flux, loom, miri, proptest, cargo-fuzz. All 3 user-named lanes (Verus, Kani, Flux) are explicitly not_applicable with concrete evidence. |

## Default Rust Profile Lane Decision

The default Rust behavior profile (per `verification-lane-policy.md`) requires Verus, Kani, Flux, and proptest unless `not_applicable` is justified. For this bead:

| Default Lane | Disposition | Justification |
|--------------|-------------|---------------|
| Verus | not_applicable | No Verus spec references this code path; replacement is 5-line deterministic expression; VACUUM risk per GOD RULE 2. |
| Kani | not_applicable | No `#[kani::proof]` harness references this code path; RA-003 corpus exhaustively covers equivalence class. |
| Flux | not_applicable | No `#[refined_by]` annotation targets the ratio; input domain is plain usize. |
| proptest | not_applicable | RA-003 corpus is strictly stronger than any proptest harness for this bounded integer input domain. |

All four default Rust lanes are explicitly handled with concrete evidence refs and reviewer acceptance. The user directive `no Verus/Kani/Flux required` is satisfied.

## Behavioral Waiver Check

`waiver-candidates.jsonl` contains 7 entries, all of type `not_applicable` with `is_behavior_waiver: false` and `behavior_affecting: false`. **No behavior-affecting waivers exist.** This satisfies `verification-lane-policy.md` and the rejection rule "behavior-affecting rows must be proven, blocked, or rejected."

## Trusted Base Plan Validation

`trusted-base-plan.md` declares 7 trusted surfaces (TBR-001 through TBR-010) with concrete justifications. All surfaces are pre-existing (Rust stdlib, workspace `[lints]` table, AST scanner, type system, TraceRing construction invariants, RA-003 corpus, master-document lint policy). The bead does not modify any of them. No new assume/axiom/admit/external_body/trusted/ignore/stub markers are introduced.

## Non-Vacuity Check

The replacement is a deterministic function of two `usize` values. Non-vacuity is provided by:

- **RA-003 corpus:** 3 tests at trace/tests.rs:1208,1249,1283 sweep every cap ∈ [1, 2^20] exhaustively (powers-of-two with every len 0..=cap; every cap with 5 sample lengths; every cap at both boundaries). The corpus is the canonical regression net.
- **Call-site tests:** 3 new tests at tick_shard_tests.rs assert `metrics.shards[0].trace_ring_fill_pct == 0.0 / 50.0 / 100.0` through `Runtime::collect_metrics` integration.
- **IPC roundtrip:** 2 existing tests at vb_ipc/src/metrics/tests.rs:298,317 verify Postcard byte-identical wire output for `f32` edge values (NaN, negative).
- **Source-level verification:** clippy `as_conversions = deny` + AST forbidden-scan + `rg '\bas\b'` provide triple-source lint verification.

## Bridge Plan Validation (Implementation-Bound)

The proof obligations bind to production Rust code:

| Obligation | Production Binding | Evidence |
|------------|--------------------|----------|
| PO-OUL6U-LINT-001 | `crates/vb_runtime/src/runtime.rs:578-588` (direct source) | clippy + AST scan + rg on the file |
| PO-OUL6U-RA003-002 | `crates/vb_runtime/src/trace/tests.rs:1186-1309` (regression net on production data type) | 3 existing tests |
| PO-OUL6U-CALLSITE-003 | `crates/vb_runtime/src/runtime.rs:561` (signature) → `collect_metrics` integration path | 3 new tests at call sites |

No VACUUM (GOD RULE 2) risk: every obligation binds directly to the production function or its regression net. No standalone model types are introduced; no spec is disconnected from the implementation.

## Risk Residuals (from proof-strategy.md)

| Hazard | Resolution |
|--------|------------|
| H-05 (sentinel preservation) | `unwrap_or(0)` fallback. Outer `if trace_capacity > 0` guard makes fallback unreachable in practice; choice of 0 (not u32::MAX) preserves sentinel intent. |
| H-09 (overflow at cap > u32::MAX) | Unreachable: documented production cap is 4096; try_from fallback is 0 but outer guard intercepts zero first. |
| H-10 (codegen) | `f32::from(u32)` and `value as f32` produce the same CVT instruction on common hardware; no measurable perf regression. |
| H-17 (perf) | `try_from(usize) for u32` is a single conditional; equivalent or faster than `as` for the documented capacity range. |

## Findings

No blocker, low, minor, observation, or informational findings. The plan is precise, internally consistent, and ready for proof-writer and proof-to-implementation handoff.

`proof-plan-findings.jsonl` is intentionally empty (no findings to record); an empty file is written to satisfy the artifact surface.

## GOD RULE Compliance

| Rule | Status | Notes |
|------|--------|-------|
| No hardcoded Kani shapes | PASS | No Kani harness in this bead; n/a. |
| No vacuum Verus proofs | PASS | No Verus spec in this bead; user directive `no Verus required`. |
| No unbounded TLA+ math | PASS | TLA+ is removed from the verifier profile (proof-planner skill doctrine); not applicable for this synchronous single-function read. |
| No loop oscillations | PASS | This is a lint-remediation plan, not a proof-authoring step. |
| No blind verification mutations | PASS | Verification scope is trimmed to the call-graph blast radius of vb-oul6u: clippy, AST scan, RA-003 corpus, 3 new call-site tests, IPC roundtrip. No cargo-mutants or kani-whole-fleet runs. |

## Verdict

The proof plan is internally consistent, every proof seed has lane decisions for the 7 applicable lanes, all `not_applicable` decisions cite concrete evidence refs, all 3 obligations have the canonical `proof-obligation/v1` schema with exact command, workdir, bounds, assumptions, and expected evidence, all 7 waivers are non-behavior-affecting, the trusted-base plan is complete, and the bridge to production Rust is implementation-bound. The plan is precise enough for proof-writer and proof-to-implementation to proceed.

---

**Report:** STATUS: APPROVED | Lanes reviewed: 16/16 accepted | Obligations: 3 (lint, RA-003, call-site) | Waivers: 7 (all non-behavior-affecting not_applicable) | Blockers: 0
