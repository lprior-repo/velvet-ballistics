---
bead_id: vb-rz9ey
title: Fix vb_compile test compilation: WorkflowSourceParts private (Cargo self-reference, P0)
state: 6 (proof-reviewer)
reviewer_skill: proof-reviewer
reviewer_invocation_id: femdation-cheap25-batch-vb-rz9ey-state6-proof-reviewer
writer_invocation_id: femdation-cheap25-batch-vb-rz9ey-state5-proof-writer
planner_invocation_id: femdation-cheap25-batch-vb-rz9ey-state4-proof-planner
plan_reviewer_invocation_id: femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer
host_session_id: femdation-cheap25-batch
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
scope_class: cargo-manifest-metadata-only
behavior_affecting: false
disposition: NO_PROOF_WORK
contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
proof_strategy_sha256: f9765849970a049eefd2fb04a4ef6cda1201b67aa1f16c0c5fcf49099d7f27f7
trusted_base_plan_sha256: 15ad62c6a6843af437a3aed89258e5665a8764d4324ca800313a8ad22367f1d2
proof_obligations_planned_sha256: a8dc5fae7a553f693c97085e196c51c5da2f2675e354d4b16027cb214e092983
verifier_lane_decisions_sha256: 9a577a51995a11468a46b0a9b7d97a487368d4ebb1ff8f5eec9a37ce225fde50
proof_plan_review_sha256: 1de7e9ea8e41bf635503baf04b8da7c4c357af3727e3feba7fc4845c2a3e715f
proof_writer_report_sha256: 8472b72f2a4ab0569841bd00caeb9da6fee847776e6463f2dfdebbc02e6feced
proof_evidence_sha256: 14b93c4a3a9acce2cbdc2a625fa1af4ed9b9203bf180b5d1c25f717241fd36e3
trusted_base_ledger_sha256: 18717abd393b87cf7083144a06a1357d8998d63ba2d86d9e34c5a485ee5b97ae
proof_findings_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
binding_classification: N/A (no Verus obligations emitted)
review_state: 6
review_completed_at: 2026-07-01T18:35:00Z
authored_by: proof-reviewer (direct child of femdation; no sub-agents)
---

# Proof Review — vb-rz9ey

**Bead**: vb-rz9ey — Fix `vb_compile` test compilation: `WorkflowSourceParts` private (Cargo self-reference, P0)
**Scope class**: `cargo-manifest-metadata-only`
**Behavior-affecting**: `false`
**State**: 6 (proof-reviewer) — reviewing State 5 (proof-writer) output

---

## 1. Review Metadata

| field | value |
|-------|-------|
| Reviewer skill | `proof-reviewer` |
| Reviewer invocation ID | `femdation-cheap25-batch-vb-rz9ey-state6-proof-reviewer` |
| Writer invocation ID | `femdation-cheap25-batch-vb-rz9ey-state5-proof-writer` (proof-writer) |
| Planner invocation ID | `femdation-cheap25-batch-vb-rz9ey-state4-proof-planner` (proof-planner) |
| Plan-reviewer invocation ID | `femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer` |
| Host session ID | `femdation-cheap25-batch` |
| Review state | 6 |
| Workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey` |
| Disposition | `NO_PROOF_WORK` (proof-writer correctly declared zero artifacts) |

**Independence check**: This reviewer's `invocation_id` (`...-state6-proof-reviewer`) is independent of the writer's (`...-state5-proof-writer`) and the plan-reviewer's (`...-state4-proof-plan-reviewer`). Same `host_session_id` (`femdation-cheap25-batch`) is the control-plane convention. No self-approval.

**Workspace check**: `pwd -P` returns `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey` (not the main checkout). `jj root` returns the same isolated workspace. `git rev-parse --show-toplevel` reports "not a git repository" (this repo is JJ-initialized only; `.jj/repo` points to `../../../velvet-ballistics/.jj/repo`, sharing store with main). Workspace isolation per `AGENTS.md` is satisfied.

---

## 2. Inputs Reviewed

| Artifact | sha256 | Path | Status |
|----------|--------|------|--------|
| `proof-writer-report.md` | `8472b72f2a4ab0569841bd00caeb9da6fee847776e6463f2dfdebbc02e6feced` | `.beads/vb-rz9ey/proof-writer-report.md` | present |
| `proof-evidence.md` | `14b93c4a3a9acce2cbdc2a625fa1af4ed9b9203bf180b5d1c25f717241fd36e3` | `.beads/vb-rz9ey/proof-evidence.md` | present |
| `trusted-base-ledger.jsonl` | `18717abd393b87cf7083144a06a1357d8998d63ba2d86d9e34c5a485ee5b97ae` | `.beads/vb-rz9ey/trusted-base-ledger.jsonl` | present, valid |
| `proof-plan-review.md` | `1de7e9ea8e41bf635503baf04b8da7c4c357af3727e3feba7fc4845c2a3e715f` | `.beads/vb-rz9ey/proof-plan-review.md` | present, `STATUS: APPROVED` |

**Hash verification**: All four on-disk artifacts match the SHA-256 listed in `proof-plan-review.md`'s Reviewed Artifacts table (line 33: `contract.md@e0cafa48...` is the bead's contract, not the input artifact set here). State-4b plan-reviewer already validated the higher-level artifacts; this state-6 review focuses on the four State-5 outputs.

---

## 3. NO_PROOF_WORK Disposition Validation

The proof-writer at State 5 declared `NO_PROOF_WORK` and produced an empty artifact bundle (proof-writer-report.md §"NO PROOF WORK — empty artifact bundle"). This disposition is reviewed here.

### 3.1 Expected vs. Actual Materialization

Per `proof-writer-report.md §"Empty placeholder artifact set"`:

| artifact | expected | actual | match |
|----------|----------|--------|-------|
| `proof-writer-report.md` | YES | YES (8472b72f...) | ✓ |
| `proof-evidence.md` | YES | YES (14b93c4a...) | ✓ |
| `trusted-base-ledger.jsonl` | YES (empty) | YES, single-row with `entries: []` and `schema_version: trusted-base-ledger/v1` | ✓ |
| `proof-obligations.written.jsonl` | NO | absent | ✓ |
| `verification/verus/*.rs` for `vb_compile` | NO | absent (the only file under `verification/verus/vb_compile/src/` is a placeholder `mod.rs` from bead `vb-czg3q`/`vb-xi2f.13` explicitly stating "No replacement modules are needed") | ✓ |
| `verification/kani/*.rs` for `vb_compile` | NO | absent (`verification/kani/` does not exist) | ✓ |
| `verification/flux/*.rs` for `vb_compile` | NO | absent (no `vb_compile/` subdirectory under `verification/flux/`) | ✓ |
| `crates/.../loom_*.rs` | NO | absent | ✓ |
| `crates/.../proptest_*.rs` | NO | absent (existing proptest harnesses are reused) | ✓ |
| `fuzz/fuzz_targets/*` for `WorkflowSourceParts` | NO | absent (only pre-existing `canonical_digest_ask.rs` exists, unrelated to this bead) | ✓ |

**Indiscriminate file-scan verification**: `rg -l "rz9ey|RZ9EY"` over `verification/`, `crates/`, `fuzz/` returns zero matches — confirming the proof-writer created no `verification/`, `crates/`, or `fuzz/` artifacts related to vb-rz9ey. The only `rz9ey`/`RZ9EY` references are in `.beads/vb-rz9ey/` (proper containment).

### 3.2 Materialized Counts

```text
materialized_proof_obligations: 0   (planned: 2; deferred to State-12 as PENDING_FORMAL_EXECUTION)
materialized_verifier_artifacts: 0  (planned: 2 lanes, both deferred; 12 lanes not_applicable)
```

**Verus vacuous-binding scan**: Zero Verus obligations emitted (VLD-002, VLD-009 both `not_applicable surface_absent`), so the `binding_classification` requirement is **N/A — no Verus artifacts to classify**. The mandatory Verus production-binding audit (per `proof-reviewer/SKILL.md` "MANDATORY: Verus Production-Binding Audit") does not apply for this bead. **No VACUUM artifact possible because no Verus file exists.**

**Kani/Flux/Loom vacuous-binding scan**: Zero obligations for all six other verifiers (VLD-003..VLD-007, VLD-010..VLD-014). Kani `cover!`-only risk, Flux `#[trusted]`/`#[ignore]` abuse risk, and Loom model mismatch risk are all structurally inapplicable.

---

## 4. Pre-Fix Evidence (§3 of proof-evidence.md)

`proof-evidence.md §3` carries the pre-fix `cargo build -p vb_compile --tests` baseline executed by the proof-writer. The evidence demonstrates the obligation premise: the build fails with `E0432` (unresolved import `vb_compile::WorkflowSourceParts`) and `E0624` (private associated function `WorkflowSource::new`) for 9 affected test files.

**On-disk verification of pre-fix state**:
- `crates/vb_compile/Cargo.toml` lines 17-18: `[dev-dependencies]` contains only `proptest.workspace = true`. No self-referencing `vb_compile = { path = ".", features = ["test-util"] }` entry. ✓ Matches `proof-evidence.md §3.1`.
- `crates/vb_compile/Cargo.toml` lines 21-23: `[features]` block with `default = []` and `test-util = []` (empty feature declarations). ✓
- `crates/vb_compile/src/lib.rs:241-242`: `#[cfg(any(test, feature = "test-util"))] pub use yaml_ast::types::WorkflowSourceParts;` — the cfg-gated re-export that makes the post-fix evidence non-vacuous. ✓
- `crates/vb_compile/src/lib.rs:185-198`: 6 `#[cfg(all(kani, any(test, feature = "test-util")))]` markers (matching the 6 Kani harnesses referenced in `proof-plan-review.md §3` and `proof-writer-report.md §"Why no proof artifacts materialize"`). The Kani harnesses are `cfg(kani)`-gated and do NOT participate in `cargo build --tests`. ✓

The pre-fix evidence is **authoritative for the obligation's premise** but does not close the obligation — only State-12's post-fix run does that.

---

## 5. Post-Fix Evidence Commands (§4 of proof-evidence.md) — PENDING_FORMAL_EXECUTION

`proof-evidence.md §4.2` and `§4.3` carry the exact post-fix evidence commands State-12 formal-verifier must execute:

| PO | command | expected outcome |
|----|---------|------------------|
| PO-001 | `cargo build -p vb_compile --tests --message-format=human` + `grep -cE 'error\[E0432\]'` + `grep -cE 'error\[E0624\]'` + `jj diff --stat Cargo.lock` + `awk` over `[dependencies]` + `moon run :lint-src` | exit 0; 0 E0432; 0 E0624; 9 affected test files compile; 1 insertion, 0 deletions in Cargo.lock; `test-util` not in `[dependencies]` |
| PO-002 | `cargo build -p vb_cli` + `cargo build -p workspace_tests` + `cargo doc -p vb_compile --no-deps \| grep -c WorkflowSourceParts` + `awk` over `[dependencies]` | both cargo builds exit 0; grep returns 0 (cfg-gate closed in production build); `test-util` not in `[dependencies]` |

**The commands match the obligation schema** in `proof-obligations.planned.jsonl`:
- PO-001 `command` (`cargo build -p vb_compile --tests --message-format=human`) and `workdir` (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey`) match the post-fix evidence command in §4.2 ✓
- PO-002 `command` (`(cargo build -p vb_cli && cargo build -p workspace_tests && cargo doc -p vb_compile --no-deps --message-format=human 2>&1 | grep -c WorkflowSourceParts)`) and `workdir` match the post-fix evidence command in §4.3 ✓

**No `BLOCKED_TOOLING` claim**: `cargo`, `jj`, `bash` are all available. The pre-fix cargo build invocation (§3.2) successfully emitted the expected `E0432`/`E0624` errors, confirming cargo is functional.

---

## 6. Trust-Marker Audit

`trusted-base-ledger.jsonl` is a single-row JSONL with:
- `schema_version: "trusted-base-ledger/v1"` ✓
- `bead_id: "vb-rz9ey"` ✓
- `contract_sha256: "e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66"` (matches on-disk `contract.md` sha256) ✓
- `entries: []` (zero trust markers) ✓
- `authored_by: "proof-writer"`, `authored_at: "2026-07-01T17:00:00Z"` ✓
- `note`: explicit statement that both `proof-obligation/v1` rows carry `trusted_base_refs=[]` and no `assume`/`axiom`/`admit`/`external_body`/`#[trusted]`/`#[ignore]`/`opaque`/`extern_spec` markers are introduced ✓

**jq validation**: `jq . < trusted-base-ledger.jsonl` parses cleanly; the row has the expected shape; `entries == []`; `schema_version` matches the canonical pattern.

**Cross-check against `trusted-base-plan.md`** (`sha256: 15ad62c6...`): the plan declares zero trusted-base entries, consistent with the empty ledger.

**Trust-marker scan** in `crates/vb_compile/`:
```text
$ rg -n "WorkflowSourceParts|test-util" crates/vb_compile/src/lib.rs | head
185:// Feature-gated behind test-util because these harnesses depend on
186:// WorkflowSourceParts which is pub(crate) in production and only
187:// re-exported as pub when test-util feature is active.
188:#[cfg(all(kani, any(test, feature = "test-util")))]
190:#[cfg(all(kani, any(test, feature = "test-util")))]
...
241:#[cfg(any(test, feature = "test-util"))]
242:pub use yaml_ast::types::WorkflowSourceParts;
```

These `#[cfg(...)]` attributes are **Cargo-feature gates, NOT proof-trust markers**. They are the production-Rust mechanism for visibility gating (no `#[trusted]`, no `#[ignore]`, no `opaque`, no `extern_spec`). No trust markers introduced by this bead.

---

## 7. Hash-Chain Validation

The `agent-invocation-ledger.jsonl` hash chain was validated using the canonical algorithm: `SHA-256(JSON.dumps(o, separators=(",", ":")))` after removing `entry_hash` from each row.

| row | invocation_id | expected entry_hash | computed | match |
|-----|---------------|---------------------|----------|-------|
| 1 | `go-skill-vb-rz9ey-state1` | `ae0fe480b62398675c17eda94638282f765864634fd781d63b86f93cadc44e58` | `ae0fe480b62398675c17eda94638282f765864634fd781d63b86f93cadc44e58` | ✓ |
| 2 | `explore-vb-rz9ey-state2` | `b8e12c0e12fc2ff097ec08175436468d238e73ef1f77efd06a0aa4dd8bd0a086` | `b8e12c0e12fc2ff097ec08175436468d238e73ef1f77efd06a0aa4dd8bd0a086` | ✓ |
| 3 | `femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer` | `8b30d58d6879431da681bc0d8b06fb6c31dbfe6447bd41d77bb5cc6150b275ec` | `8b30d58d6879431da681bc0d8b06fb6c31dbfe6447bd41d77bb5cc6150b275ec` | ✓ |
| 4 | `femdation-cheap25-batch-vb-rz9ey-state5-proof-writer` | `0dbf7794a5f9c9c5cba6c525847fcdb6cdd669fb87ed7c7782f1971fc5cda1cb` | `0dbf7794a5f9c9c5cba6c525847fcdb6cdd669fb87ed7c7782f1971fc5cda1cb` | ✓ |

**Linkage validation**: Each row's `previous_entry_hash` matches the previous row's `entry_hash`:
- Row 2 prev=`ae0fe480...` = Row 1 hash ✓
- Row 3 prev=`b8e12c0e...` = Row 2 hash ✓
- Row 4 prev=`8b30d58d...` = Row 3 hash ✓

The ledger's hash chain is **VALID** before the state-6 append. The state-6 row appended by this review uses `previous_entry_hash = 0dbf7794a5f9c9c5cba6c525847fcdb6cdd669fb87ed7c7782f1971fc5cda1cb` (= row 4's hash).

---

## 8. Cross-Cutting Verifications

### 8.1 Verifier-Lane-Decision Count Audit

`verifier-lane-decisions.jsonl` should have 14 rows (7 verifiers × 2 obligations) per `proof-plan-review.md §2`:

```text
$ jq -s 'group_by(.applicability) | map({applicability: .[0].applicability, count: length, limitation_kinds: [.[].limitation_kind] | unique})'
[
  {"applicability":"not_applicable", "count":12, "limitation_kinds":["surface_absent"]},
  {"applicability":"required",       "count":2,  "limitation_kinds":[""]}
]
```

| claim | verified | match |
|-------|----------|-------|
| 14 total rows | 14 (VLD-001..VLD-014) | ✓ |
| 12 `not_applicable` | 12 | ✓ |
| 2 `required` | 2 (VLD-001, VLD-008) | ✓ |
| All `not_applicable` have `limitation_kind: surface_absent` | All 12 | ✓ |

### 8.2 Proof-Obligation Schema Audit

`proof-obligations.planned.jsonl` should have 2 rows (PO-001, PO-002) per `proof-plan-review.md §5`:

```text
$ jq -s 'length' proof-obligations.planned.jsonl
2
```

Both rows have `verifier: "proptest"`, `behavior_affecting: false`, `trusted_base_refs: []`, `required: true`, `mode: "verify-proof"`, `status: "planned"`. No legacy alias fields detected.

### 8.3 `proof-writer-report.md` Self-Audit Checklist Cross-Check

| self-audit item | verified on disk |
|-----------------|-------------------|
| `proof-writer-report.md` present | ✓ (8472b72f...) |
| `proof-evidence.md` present with pre-fix baseline + PENDING_FORMAL_EXECUTION handoff | ✓ (14b93c4a...; §3 has pre-fix cargo build; §4 has post-fix commands) |
| `trusted-base-ledger.jsonl` present as empty placeholder | ✓ (18717abd...; `entries: []`) |
| No production Rust edited | ✓ (no jj commits with vb-rz9ey description; jj status shows only other beads' working-copy churn unrelated to vb-rz9ey) |
| No `verification/<verifier>/` artifact written for vb-rz9ey | ✓ (verified by `rg rz9ey verification/` returning zero matches) |
| No `proof-obligations.written.jsonl` written | ✓ (file does not exist; conventionally only present when materials were written) |
| All 14 verifier-lane decisions honored | ✓ (12 not_applicable surface_absent + 2 required deferred to state-12) |
| No `assume`/`axiom`/`admit`/`external_body`/`#[trusted]`/`#[ignore]`/`opaque`/`extern_spec` markers | ✓ (none added by this bead) |
| Hash chain valid | ✓ (algorithm verified against 4 existing entries; new state-6 row appended with valid `previous_entry_hash`) |

### 8.4 Non-Vacuity (N/A)

Per `proof-reviewer/SKILL.md` "Non-Vacuity Checks":
- **Verus standalone-model risk**: N/A — zero Verus obligations.
- **Kani `cover!`-only risk**: N/A — zero Kani obligations.
- **Flux refinement-trust abuse**: N/A — zero Flux obligations.
- **Loom model-mismatch risk**: N/A — zero Loom obligations.
- **Risky-path reachability (cargo-build evidence)**: The cargo build invocation IS the static-visibility-gate proof for PO-001 (rustc enforces `cfg(any(test, feature="test-util"))`); the cargo doc invocation IS the public-API-surface proof for PO-002. These are non-vacuous because:
  - PO-001: 9 distinct test files would fail to compile if `test-util` weren't activated in the test build. Pre-fix evidence shows real `E0432`/`E0624` errors; post-fix must eliminate them.
  - PO-002: `cargo doc --no-deps` uses default-features; absence of `WorkflowSourceParts` from the public doc surface proves the cfg-gate remains closed in production builds.

---

## 9. Findings

**Zero findings at every severity.** The proof-writer's NO_PROOF_WORK disposition is correctly declared, all evidence is present, the trusted-base ledger is valid empty, the hash chain is intact, and the post-fix evidence commands are workdir-aligned and concrete.

| severity | count | notes |
|----------|-------|-------|
| blocker | 0 | — |
| major | 0 | — |
| minor | 0 | — |
| observation | 0 | — |

### Disposition Table

| disposition | count |
|-------------|-------|
| `fixed_with_evidence` | 0 |
| `owner_approved_debt` | 0 |
| `owner_approved_no_action` | 0 |
| `blocker` | 0 |

---

## 10. State Transition

`vb-rz9ey` is approved to advance from State 5 (proof-writer) through State 6 (proof-reviewer) to:

- **State 7 (proof-to-implementation)**: bridge is already materialized as `proof-to-implementation-input.md` (per `proof-plan-review.md §"State Transition"`); no separate State-7 output needed.
- **State 8 (black-hat-reviewer)**: pending — verify the post-fix file diff matches the forbidden-mutation list (8 paths per `contract.md §3.3`) and that the required mutation is exactly one line in `[dev-dependencies]`.
- **State 12 (formal-verifier)**: pending — run PO-001 and PO-002 evidence commands from `proof-evidence.md §4.2` and `§4.3` after State-6 lands the self-reference; populate `verification-ledger.jsonl` with the per-PO verdict.

---

# STATUS: APPROVED