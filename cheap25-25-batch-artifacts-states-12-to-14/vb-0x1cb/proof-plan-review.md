# Proof Plan Review — vb-0x1cb

- bead_id: vb-0x1cb
- bead_title: Repair ignored-fallible-results source gate violation (P1)
- state: 4b (proof-plan-reviewer)
- review_state: approved_with_debt
- controller: femdation
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- lane_profile: rust_local_concurrency_empty
- reviewer_skill: proof-plan-reviewer
- reviewer_invocation_id: proof-plan-reviewer-vb-0x1cb-state4b
- planner_invocation_id: proof-planner-vb-0x1cb-state4
- captured_at: 2026-07-01T16:15:00Z

## Reviewed artifacts

| Artifact | Path | SHA-256 |
|----------|------|---------|
| proof-strategy.md | `.beads/vb-0x1cb/proof-strategy.md` | `f21c4e7b276b1ceb27b3c43685ecf977871d57f58efd20e2173702b2b4d221eb` |
| verifier-lane-decisions.jsonl | `.beads/vb-0x1cb/verifier-lane-decisions.jsonl` | `59f458ed593dcbef03da5da0335daaf1e53bc1ff0efd6c202ddbd244477cd225` |
| proof-obligations.planned.jsonl | `.beads/vb-0x1cb/proof-obligations.planned.jsonl` | `61f98959e1a7fb88549894e4be8e37e78a104d96e2b0e884f0433c091a5295d2` |
| trusted-base-plan.md | `.beads/vb-0x1cb/trusted-base-plan.md` | `2f52f99d8fdc1d7ef1d688690a93178f3fb821c73d911d39d6d0eb4884c22527` |
| waiver-candidates.jsonl | `.beads/vb-0x1cb/waiver-candidates.jsonl` | `14719bcfff045e058f43ee76aed18af29d38ad26ff0c1ba8cd7432661c516c6c` |
| contract.md | `.beads/vb-0x1cb/contract.md` | `fb69524675389a7e634530599f3a97623cd1ef8208510e9944afe7c057c2abf6` |
| proof-seeds.jsonl | `.beads/vb-0x1cb/proof-seeds.jsonl` | `3fe7202f6a2ef8c2d8de8dcabf812434bdfd10593b9ce338a75ecccf39a313fa` |
| traceability-matrix.jsonl | `.beads/vb-0x1cb/traceability-matrix.jsonl` | `f76c5e5290632620bf5a0227914bc2e5b51fe3f306af4d545b7663d87e75fb8c` |
| verifier-lane-matrix.md | `.beads/vb-0x1cb/verifier-lane-matrix.md` | `0dff6b125be804e0490c57919326595e1167724aa5bdffc227759003fab8bd7b` |

## Reviewer disposition summary

| Verifier | Lane decisions | Required | Not applicable | Reviewer disposition |
|----------|----------------|----------|----------------|---------------------|
| verus | 7 | 0 | 7 | accepted (all rows cite concrete contract lane_profile evidence; no production-binding obligation created) |
| kani | 7 | 0 | 7 | accepted (all rows cite `cfg(kani)` stub at `impl_parts/chunk_001.rs:206-217` or compile-time enforcement) |
| flux-rs | 7 | 1 (PO-005) | 6 | accepted (PO-005 is the one refinement obligation; production-binding path is `#[extern_spec]` per the proof-planner SKILL Flux exemption) |
| proptest | 7 | 4 (PO-001, PO-002, PO-006, behavior-test 1+2) | 3 | accepted (required rows drive `cargo test -p vb_runtime --lib …` and `PROPTEST_CASES=1024 …`; non-applicable rows cite refinement or static-text reason) |
| loom | 7 | 0 | 7 | accepted (single-threaded `Shard::tick`; `JournalWriteBatch` `!Send + !Sync`) |
| miri | 7 | 0 | 7 | accepted (`#![forbid(unsafe_code)]` in `lifecycle.rs:1` and `shard/mod.rs:1`; transitively forbids unsafe in chunk files via `include!`) |
| cargo-fuzz | 7 | 0 | 7 | accepted (no codec / parser / byte-level hostile input boundary) |
| cargo-clippy | 1 | 1 (PO-006) | 0 | accepted (canonical `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use`) |
| moon-source-gate | 1 | 1 (PO-007) | 0 | accepted (`moon run :lint-src` depends on `check-ignored-fallible-results.sh`) |
| bash-source-gate | 2 | 2 (PO-006, PO-007) | 0 | accepted (canonical `bash scripts/check-ignored-fallible-results.sh` evidence command) |
| **Total** | **53** | **9** | **44** | **all 53 accepted** |

## Verifier-lane-review

`verifier-lane-review.jsonl` (53 rows, one per planner lane decision) is at
`.beads/vb-0x1cb/verifier-lane-review.jsonl`. All rows carry
`reviewer_disposition: accepted` with `planner_invocation_id: proof-planner-vb-0x1cb-state4`
and `reviewer_invocation_id: proof-plan-reviewer-vb-0x1cb-state4b`. No
self-stamped reviewer fields in the planner artifacts; my invocation ID
differs from the planner's.

## Proof obligations (7)

`proof-obligations.planned.jsonl` defines 7 obligations, mapping 1:1 to the
user-specified verifier set:

| ID | Verifier | Target | Required |
|----|----------|--------|----------|
| PO-001 | proptest | `Shard::finish_run` rollback helper dual-failure matrix | yes |
| PO-002 | proptest | `Shard::fail_run_state` rollback helper dual-failure matrix | yes |
| PO-003 | cargo-test | `lifecycle_tests::chunk_005::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | yes |
| PO-004 | cargo-test | `lifecycle_tests::chunk_008::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | yes |
| PO-005 | flux-rs | `TraceEvent::RunRollbackFailed` size-bound extern_spec | yes |
| PO-006 | cargo-clippy | `transitions.rs` after annotation removal + allow row gone | yes |
| PO-007 | bash-source-gate / moon-source-gate | Source-gate postcondition (zero `transitions.rs` lines, exit 0) | yes |

Schema validation: all 7 obligations have the canonical `proof-obligation/v1`
fields (schema_version, id, requirement_id, contract_clause, domain_claim,
risk, risk_tags, verifier, artifact, target, command, workdir,
expected_evidence, assumptions, model_bounds, tool_metadata,
trusted_base_refs, required, behavior_affecting, mode, owner_state,
rerun_from, status). No legacy alias fields (e.g. `layer`, `checker`,
alias-only `claim`).

## Production binding gate (Verus obligations)

The plan has **zero** `verifier: verus` obligations. Every Verus lane
decision (VLD-001, -008, -015, -022, -029, -036, -043) is
`applicability: not_applicable` with `limitation_kind: not_required_by_contract`
citing `contract.md#c-7.lane-profile` and the bead instruction. Therefore the
GOD RULE 2 / SKILL production-binding gate (which applies only to Verus
obligations) is satisfied vacuously. The single Flux obligation (PO-005) is
exempt from the production-binding gate per the proof-planner SKILL §"Flux
production-binding exemption" — its binding is the canonical `#[extern_spec]`
pattern already used in `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs`.

## Behavior-waiver check

`waiver-candidates.jsonl` contains 2 rows (WC-001, WC-002). Both are
`behavior_affecting: false`. WC-001 is a no-waiver statement confirming the
plan needs no formal waiver; WC-002 is a negative-observation reminder that
no new allow row may reference `follow_up=vb-ttki3`. Neither row is a
behavior-affecting waiver. No `E_BEHAVIOR_WAIVER` finding.

## Forbidden-pattern check (per contract C-3 + bead instructions)

| Forbidden pattern | Plan compliance |
|-------------------|-----------------|
| `let _ = self.run_state_insert(run, state);` retained | Plan removes the discard at `transitions.rs:100` and `:202`. New `Shard::observe_run_state_rollback` helper is `#[must_use]` and bound via match. |
| `match … { Ok(_) \| Err(_) => {} }` retained | Plan does not introduce this pattern; the phantom `match` at line 146 referenced in the bead description is stale (per `codebase-map.md`). |
| `Err(secondary)` returned in place of `Err(primary)` | Plan explicitly preserves primary-wins via C-1, asserted in PO-001..PO-004. |
| `RuntimeError::Core { source: CoreError::InternalInvariantViolation { .. } }` | **NOT used.** Observability surface is `TraceEvent::RunRollbackFailed { run, site, primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> }` per C-3. |
| `eprintln!` or `tracing::error!` for the secondary surface | Plan does not introduce either call site for the secondary surface. |
| New allow row with `follow_up=vb-ttki3` | Plan deletes the only row that referenced `vb-ttki3`; black-hat-reviewer disposition in proof-strategy §3 (`follow_up linker rot`) and §10 forbids reuse. |
| `RuntimeError` new variants | Plan does not add a new variant; uses the existing `StorageJournalAppend { source: Arc<…> }` enum. |

## Source-gate evidence chain (per bead instruction)

- `bash scripts/check-ignored-fallible-results.sh` (PO-007) is the canonical
  source-gate evidence command per the bead instruction. PO-006 also
  re-runs it after clippy and after a follow-up `grep -RnH
  '#\[allow(clippy::let_underscore_must_use)\]'` over
  `crates/vb_runtime/src/shard/transitions.rs`.
- `moon run :lint-src` (PO-007) composes from
  `check-ignored-fallible-results.sh` per `.moon/tasks/all.yml:50-62,
  75-85` (cited in the strategy §2 / §3). Once the bash check is green,
  `moon :lint-src` is green by composition.
- Pre-repair the `JustifiedException|…|transitions.rs|…` row in
  `scripts/ignored-fallible-results.allow:4` is the only allow-file entry
  that maps the `DISCARD-006` exception for `transitions.rs`. The plan
  deletes the row, so post-repair the bash check emits zero lines
  containing `transitions.rs` on stdout.

## Non-vacuity (per skill)

- Proptest (PO-001, PO-002): driven via `prop_compose!` over
  `{journal_rejects, slot_full} ∈ {0,1}²` plus constructed
  `Result<Option<RunState>, RuntimeError>` values. Proptest directly drives
  the helper, bypassing the kani stub restriction. **No hardcoded
  `WorkflowParts` or `RunFrame` structures** (GOD RULE 1).
- Cargo-test (PO-003, PO-004): mirror `LegacyStepFailsJournal` from
  `lifecycle_tests/chunk_004.rs:236-339` using a `SharedRuntimeJournal` stub
  that returns `Err(StorageJournalAppend { source: Arc(WriteLockPoisoned) })`
  for one journal event variant and `Ok(())` for all others. **No hardcoded
  graphs**; the journal stub is parameterised over the event variant.
- Flux (PO-005): `#[extern_spec]` over the production `TraceEvent` and
  `RollbackSite` enums, with `#[refined_by]` and a size predicate. **No
  vacuous shadow model**; the extern spec resolves against production
  types via the canonical `vb_y9d3v_action_ticket_refinements.rs` pattern
  (GOD RULE 2 binding).
- Source-gate (PO-006, PO-007): static text scan of the post-repair tree.
  No mock or stub; runs against the actual files.

## Trusted base

`trusted-base-plan.md` enumerates 8 entries:

- 1 external_body (`TBR-001`): proptest `Arbitrary` impl for `RunId` and
  journal-rejection `bool` flag.
- 3 assumes (`TBR-005`, `TBR-006`, `TBR-007`): Flux nightly toolchain,
  `Arc<RuntimeError>` bounded allocation, bash interpreter + `rg`.
- 1 stub (`TBR-003`): `SharedRuntimeJournal` test stub rejecting specific
  journal event variants (mirror pattern, no production `FjallJournal`
  exercised in the test).
- 3 extern_specs (`TBR-002`, `TBR-004`, `TBR-008`): `pub(crate)` access to
  the helper and `trace_ring`; Flux extern_spec mirror; `#[must_use]`
  enforcement via clippy.

**0 behavior-affecting** entries. All 8 are reviewable at state 6 (proof-reviewer);
none waive behavior. The `is_terminal_for_run` decision for
`RunRollbackFailed` is explicitly `false` per C-3, and the `run_id` arm
returns `*run` per the C-3 contract clause.

## Findings summary

`proof-plan-findings.jsonl` contains 4 low / observation findings, all
dispositioned:

| Severity | Disposition | Owner | Subject |
|----------|-------------|-------|---------|
| low | owner_approved_debt | proof-writer | PO-005 size predicate may not discharge against default-layout 32-byte variant; proof-writer adjusts the bound. |
| low | owner_approved_no_action | proof-planner | `engaged-as-stub` label in strategy table is illustrative; VLD-016 is authoritative. |
| observation | owner_approved_no_action | proof-planner | miri non-applicability line refs point at blank lines in chunk_005/008; the substantive `forbid(unsafe_code)` is in `lifecycle.rs:1` and `mod.rs:1`. |
| observation | owner_approved_no_action | proof-planner | loom non-applicability line refs for `runtime.rs:174/198` are imprecise; substantive claim is correct. |

No `blocker` finding. Approval proceeds.

## Decision

Plan is precise enough for proof-writer (state 5) and proof-to-implementation
(state 7):

1. All 7 proof obligations are required, bound to a real verifier, and
   cite an executable command with a workdir.
2. All 53 lane decisions carry `accepted` disposition with
   non-self-stamped reviewer invocation IDs.
3. No behavior waiver exists. The 2 waiver candidates are
   `behavior_affecting: false`.
4. Production-binding gate is satisfied vacuously (no Verus obligations;
   Flux is exempt).
5. Forbidden patterns from the contract and bead instruction are
   observed: the observability surface is `TraceEvent::RunRollbackFailed`,
   NOT `RuntimeError::Core::InternalInvariantViolation`; the deleted
   `follow_up=vb-ttki3` row is not reintroduced in any new allow row.
6. The 7 obligations map 1:1 to the verifier set the user requested:
   proptest (PO-001, PO-002), cargo-test (PO-003, PO-004), flux-rs
   (PO-005), cargo-clippy (PO-006), moon-source-gate + bash-source-gate
   (PO-007).

STATUS: APPROVED
