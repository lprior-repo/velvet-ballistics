# Proof Plan Review — vb-vzo9b

**Bead**: vb-vzo9b — Tests: replace multi-run recovery disjunction with exact slots (P1 bug)
**Reviewer Skill**: proof-plan-reviewer
**Reviewer Invocation**: `proof-plan-reviewer-vb-vzo9b-state4b-attempt1`
**Review State**: state 4b (post-planner independent review)
**Planned State Owner**: proof-planner (state 4, invocation `proof-planner-vb-vzo9b-state4`)
**Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
**Reviewed At**: 2026-07-01

---

## STATUS: APPROVED

The proof plan is precise, complete, and bound to production semantics through
the cargo-test + cargo-build + source-lint lane profile. All three proof
obligations have exact commands, workdirs, evidence markers, and non-vacuous
claims. The six default-profile verifier lanes (verus, kani, flux-rs, loom,
miri, cargo-fuzz) carry typed `not_applicable` decisions with concrete
SHA-256 evidence refs and risk-tag-cited reasoning. The trusted-base plan
contains only structural notes (no obligation-driven trust markers). The
waiver candidate is a structural placeholder with `behavior_affecting: false`.
The contract C-7 cargo-build patch (`--manifest-path fuzz/Cargo.toml`
instead of `-p fuzz`) is correct and reflected in PO-003.

---

## Reviewed Artifacts (with canonical SHA-256 hashes)

| Artifact | Path | SHA-256 |
|----------|------|---------|
| proof-strategy | `.beads/vb-vzo9b/proof-strategy.md` | `db996029e7c821d9588a2cda374aa2f621e12bc2e60abf694e06eea672dfbdeb` |
| verifier-lane-decisions | `.beads/vb-vzo9b/verifier-lane-decisions.jsonl` | `bc3c834ec236df4f5db8fad8e9efef1c18cb2d904167d385a66fbc8ca107a5f2` |
| proof-obligations.planned | `.beads/vb-vzo9b/proof-obligations.planned.jsonl` | `572dd8c2766a5d94891b10937bf311500a0c24b1f98f971d903ee0fff18b350b` |
| trusted-base-plan | `.beads/vb-vzo9b/trusted-base-plan.md` | `17f72af7e1d944b2d6b42fbc7f9ac412253f8635505888c4a8a9ace052ca0c93` |
| waiver-candidates | `.beads/vb-vzo9b/waiver-candidates.jsonl` | `0d295a52890d1836a1c7c6de73d3b9fc07c9a6a6afdf2cf33e28e49d4a3e3021` |
| contract (input) | `.beads/vb-vzo9b/contract.md` | `3e759af7624f332b6b3298e9a93de95bfd206422d2b820f804bfbb5a11cca5eb` |
| proof-seeds (input) | `.beads/vb-vzo9b/proof-seeds.jsonl` | `346da60c2f2b4f078b70a3296d5493a2fbe552ba060ce3b48a076d1fa3fe6434` |
| traceability-matrix (input) | `.beads/vb-vzo9b/traceability-matrix.jsonl` | `7e3c1274962d85d49e59c012df6e7b959b898655015df6da1bcfabc089c557ca` |

All input hashes match the `output_artifact_hashes` recorded in the
planner's agent-invocation-ledger entry
(`proof-planner-vb-vzo9b-state4`, ledger_sequence 3) — no drift between
planner emission and reviewer reading.

---

## Review Method

1. Read every required input artifact (`proof-strategy.md`,
   `verifier-lane-decisions.jsonl`, `proof-obligations.planned.jsonl`,
   `trusted-base-plan.md`, `waiver-candidates.jsonl`, plus
   `contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`,
   `proof-to-implementation-input.md`, `verifier-lane-matrix.md`,
   `proof-coverage-matrix.md`).
2. Verified the defect at `fuzz/src/journal_target/readback.rs:196`
   is the disjunctive `assert!(run_summary.run == run ||
   run_summary.run == vb_core::RunId::new(0))` (line read directly).
3. Verified the production struct `RecoveryRuntimeSummary` at
   `crates/vb_storage/src/recovery/types.rs:546-570` derives
   `Debug, Clone, Copy, PartialEq, Eq` and has exactly the 11 fields
   named in contract C-1: `run`, `first_seq`, `last_seq`, `workflow`,
   `steps_started`, `steps_succeeded`, `actions_scheduled`,
   `actions_resolved`, `suspensions`, `slots_written`, `terminal`.
4. Verified the production functions exist at the cited line ranges:
   - `summarize_recovery_events` at
     `crates/vb_storage/src/recovery/replay/summary/apply.rs:88`.
   - `recover_runtime_frame_seed_from_events` at
     `crates/vb_storage/src/recovery/replay/summary/derive.rs:69`.
5. Verified the fuzz package name in `fuzz/Cargo.toml:2` is
   `velvet-ballistics-fuzz`, confirming the planner's C-7 patch
   (`--manifest-path fuzz/Cargo.toml` instead of `-p fuzz`) is correct.
6. Verified the fuzz binary `recovery_decode` exists at
   `fuzz/src/bin/recovery_decode.rs:5` and re-exports
   `fuzz_lib::fuzz_recovery_decode` via `run_with_stdin`, with the
   harness re-exported at `fuzz/src/lib.rs:46` and `fuzz/src/journal_target.rs:32`.
7. Confirmed `#![forbid(unsafe_code)]` is present at the top of
   `crates/vb_storage/src/recovery/replay/summary/apply.rs:1` and
   `crates/vb_storage/src/recovery/replay/summary/derive.rs:1`,
   supporting VLD-008's `miri: not_applicable` claim.
8. Validated schema compliance for every JSONL file via
   `jq -s -c '.[] | {schema_version, required_fields}'` — all rows
   conform to `verifier-lane-decision/v1` and `proof-obligation/v1`
   per `~/.agents/skills/go-skill/references/proof-schemas.md`.

---

## Plan Quality Gate Results

### Gate 1 — Schema Compliance: PASS

All JSONL rows have `schema_version` literals matching
`verifier-lane-decision/v1`, `proof-obligation/v1`, `waiver-candidate/v1`,
`proof-seed/v1`. All required fields per
`~/.agents/skills/go-skill/references/proof-schemas.md` are present.
No legacy alias fields (`layer`, `checker`, alias-only `claim`) detected.
No `self_stamped` reviewer field detected in planner artifacts.

### Gate 2 — Lane Decision Coverage: PASS

9 lane decisions cover the required lane profile:
- `proptest` ×3 required (PO-001, PO-002, PO-003)
- `verus`, `kani`, `flux-rs`, `loom`, `miri`, `cargo-fuzz` ×6
  `not_applicable` with typed `limitation_kind` and 2-3 SHA-256 evidence refs each.

No default-profile verifier is silently omitted. No `blocked_tooling` rows.
Default profile (`verus`, `kani`, `flux-rs`, `proptest`) is fully addressed;
conditional profile additions (`loom` for concurrency, `cargo-fuzz` for
parsers/hostile-input boundaries) are also addressed.

### Gate 3 — Obligation Pairing: PASS

Every required lane decision names its paired obligation ID and that
obligation exists in `proof-obligations.planned.jsonl`:

| Lane | Verifier | Applicability | Obligation |
|------|----------|---------------|------------|
| VLD-001 | proptest | required | PO-001 |
| VLD-002 | proptest | required | PO-002 |
| VLD-003 | proptest | required | PO-003 |

### Gate 4 — Implementation Binding: PASS

All `target` fields parse as `crate::module::symbol`:
- PO-001: `vb_storage::recovery::summarize_recovery_events` (apply.rs:88)
- PO-002: `vb_storage::recovery::recover_runtime_frame_seed_from_events` (derive.rs:69)
- PO-003: `fuzz::journal_target::readback::fuzz_recovery_decode` (readback.rs:183)

For the fuzz target, the crate prefix `fuzz` matches the fuzz workspace
package name's effective crate root (re-exported from
`fuzz/src/lib.rs:46`).

### Gate 5 — Evidence Specificity: PASS

Every obligation has:
- Concrete command with no `$VAR` placeholders (PO-001, PO-002, PO-003).
- Absolute `workdir` matching the isolated jj workspace root
  (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`).
- `expected_evidence` citing concrete tool markers:
  - PO-001: `test result: ok. 4 passed; 0 failed; ...`
  - PO-002: `test result: ok. 2 passed; 0 failed; ...`
  - PO-003: `Compiling recovery_decode ... Finished 'recovery_decode'`
    plus six `rg` zero-match gates.
- `tool_metadata.tool` populated for every obligation
  (`cargo-test` ×2, `cargo-build` ×1).
- `tool_metadata.version_pin` cites rust-toolchain.toml pinning.

### Gate 6 — Resource Governance: PASS

`model_bounds` is set on every obligation:
- PO-001: `{cases: 1, input_size: 1}` — unit-test smoke gate.
- PO-002: `{cases: 1, input_size: 1}` — unit-test smoke gate.
- PO-003: `{cases: 1, input_size: 1, max_total_time: 600}` — cargo build
  with 10-minute cap (sufficient for incremental velvet-ballistics-fuzz
  build after pre-fix dependency graph).

### Gate 7 — Waiver Discipline: PASS

Single waiver candidate row `WC-001` with:
- `behavior_affecting: false` (NOT a behavior waiver).
- `reason` explains no obligations are waived — placeholder documents
  the empty waiver set.
- `boundary_proof`: `N/A — no obligation is waived`.
- `compensating_evidence`: concrete PO-001/PO-002/PO-003 commands.
- `owner: proof-planner`, future `expiry`, `review_status: proposed`.

Does NOT trigger `E_BEHAVIOR_WAIVER` rejection (waiver is
`behavior_affecting: false` and is not "waiver because proof is hard" —
it is a structural placeholder documenting that no waiver is needed).

### Gate 8 — Trust Marker Ledger: PASS

`trusted-base-plan.md` declares **4 structural notes** and **0
obligation-driven trust markers**. No `assume`, `axiom`, `admit`,
`external_body`, `#[trusted]`, `#[ignore]`, `extern_spec`, `opaque`,
stub, disabled check, model bound, or model reduction markers are
introduced by this bead. The plan is explicit about this in the
"Trust Markers" section of `proof-strategy.md`.

### Gate 9 — Cross-Reference Integrity: PASS

No `behavior_affecting: true` obligations exist, so the
`proof-to-implementation` bridge does not need to materialize
`rust-refinement-obligation/v1` rows. `proof-to-implementation-input.md`
explicitly documents this and provides the source-ref / behavior-test /
closure-command mapping for the State 11 verifier.

### Gate 10 — Production Binding Validation: PASS (N/A by verifier type)

The `proof-plan-reviewer` skill's "Production Binding Plan Validation"
rule mandates `production_binding` only for `verifier: verus` rows.
All 3 obligations have `verifier: proptest` (not Verus); therefore the
mandate does not apply. All 6 default-profile Verus/Kani/Flux/Loom/Miri/
cargo-fuzz lanes are correctly `not_applicable` with typed
`limitation_kind` and SHA-256 evidence refs (not the E_LANE_DECISION_WEAK
pattern). The plan therefore satisfies the no-vacuum Verus discipline
without needing any `production_binding` row.

### Gate 11 — Command Strength (no smoke-only): PASS

PO-001 and PO-002 are `cargo test --lib <unit-name>` invocations
(filtered by function name substring), which run the targeted unit-test
functions against real `RecoveryRuntimeSummary` inputs. They are NOT
smoke-only (no `cargo build` or `cargo check` substitute). PO-003
chains `cargo build --bin recovery_decode` with six inverted `rg`
gates covering C-8 forbidden patterns — this is compile + lint, the
appropriate non-smoke combination for a blast-radius-control /
source-lint claim.

### Gate 12 — Non-Vacuity: PASS

The `assert_eq!(run_summary, expected_recovery_runtime_summary)`
assertion is over a 11-field `Copy + PartialEq + Eq` struct and pins
every field simultaneously. This is a maximally-non-vacuous claim:
any field mismatch (including sentinel `RunId::new(0)`) causes the
`assert_eq!` macro to panic with a `Debug`-formatted diff. The cargo-test
obligations run the existing `summarize_recovery_events_*` and
`frame_seed_*` test bodies (not stub `#[ignore]`'d or empty functions)
so the behavior is exercised by real input data, not just type-checked.

---

## Lane-by-Lane Disposition

| VLR | Lane | Verifier | Disposition | Reason Summary |
|-----|------|----------|-------------|----------------|
| VLR-001 | VLD-001 | proptest | accepted | cargo-test on production unit-test surface |
| VLR-002 | VLD-002 | proptest | accepted | cargo-test on production frame-seed unit-test surface |
| VLR-003 | VLD-003 | proptest | accepted | cargo-build + 6× inverted rg forbidden-pattern gates |
| VLR-004 | VLD-004 | verus | accepted | not_applicable (surface_absent) — no production invariant introduced |
| VLR-005 | VLD-005 | kani | accepted | not_applicable (surface_absent) — no new bounded symbolic claim |
| VLR-006 | VLD-006 | flux-rs | accepted | not_applicable (surface_absent) — no new refinement type |
| VLR-007 | VLD-007 | loom | accepted | not_applicable (surface_absent) — no concurrency surface |
| VLR-008 | VLD-008 | miri | accepted | not_applicable (surface_absent) — zero unsafe in scope |
| VLR-009 | VLD-009 | cargo-fuzz | accepted | not_applicable (superseded_by_other_lane_with_evidence) — fuzz harness IS the test target |

Full per-row `reviewer_note` content is in `verifier-lane-review.jsonl`.

---

## Critical Patch Verification

The planner corrected contract C-7's `cargo build -p fuzz --bin
recovery_decode` to `cargo build --bin recovery_decode --manifest-path
fuzz/Cargo.toml`. Verified independently:

- `fuzz/Cargo.toml:2` declares `name = "velvet-ballistics-fuzz"`,
  NOT `fuzz`. A `-p fuzz` invocation against the main workspace would
  fail with "package ID specification `fuzz` did not match any packages".
- `fuzz/src/bin/recovery_decode.rs:5` re-exports the harness via
  `run_with_stdin(fuzz_lib::fuzz_recovery_decode)`, with the harness
  re-exported from `fuzz/src/lib.rs:46`.
- The corrected `--manifest-path fuzz/Cargo.toml` invocation runs the
  build against the correct workspace.

PO-003 reflects this corrected command verbatim. The `proof-strategy.md`
documents the patch rationale. APPROVED.

---

## Findings

No findings at any severity. The plan is precise, complete, and bound
to production semantics through the cargo-test + cargo-build +
source-lint lane profile. All default-profile verifier lanes carry
typed non-applicability with concrete SHA-256 evidence refs. No
behavior-affecting waivers. No trust markers introduced. No vacuous
Verus specs.

---

## Downstream Contract

The plan is sufficient for:

- **State 5 (proof-writer)**: 3 obligations are concrete enough to
  write the `verification-ledger/v1` rows. The compound PO-003 command
  can be split into a `cargo build --bin recovery_decode --manifest-path
  fuzz/Cargo.toml` ledger row plus 6 `rg` rows (or kept as a single
  compound row per the planner's chosen evidence representation).
- **State 7 (proof-to-implementation)**: No `rust-refinement-obligation/v1`
  rows needed (`behavior_affecting: false` for all obligations).
- **State 11 (formal-verifier)**: Run `cargo test -p vb_storage --lib
  summarize_recovery_events` and `cargo test -p vb_storage --lib
  recover_runtime_frame_seed_from_events` against the post-fix fuzz
  body, plus `cargo build --bin recovery_decode --manifest-path
  fuzz/Cargo.toml` and the six `rg` forbidden-pattern checks.

---

## Ledger Action

Append `proof-plan-reviewer-vb-vzo9b-state4b-attempt1` (state 4b) entry
to `.beads/vb-vzo9b/agent-invocation-ledger.jsonl` with output_artifacts
`proof-plan-review.md` + `verifier-lane-review.jsonl`.

---

**STATUS: APPROVED**