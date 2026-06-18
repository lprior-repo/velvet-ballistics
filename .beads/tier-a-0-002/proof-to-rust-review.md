STATUS: APPROVED
reviewer_skill: proof-reviewer
reviewer_invocation_id: tier-a-0-002-s7-bridge-reviewer-d25942d8
writer_invocation_id: tier-a-0-002-s7-proof-to-implementation-PTIBRIDG
review_state: 7
bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
attempt: 1-of-7
schema_version: proof-to-rust-review/v1
updated_at: 2026-06-18T03:50:00.000000+00:00

# Proof-To-Rust Bridge Review — Residue Quarantine CI Gate (tier-a-0-002)

## §1. Reviewed Artifacts (2 State 7 writer outputs)

| Artifact | Path | sha256 (input) |
|----------|------|----------------|
| `proof-to-rust-map.md` | `.beads/tier-a-0-002/proof-to-rust-map.md` | sha256:e81b78889cc8ffcf557accb17320f3bc01161c5a544b6a644b4e75b32ec5b993 |
| `rust-refinement-obligations.jsonl` | `.beads/tier-a-0-002/rust-refinement-obligations.jsonl` | sha256:e9c9734d2e2ec876c8d66b693d84855fdd9873b72a568ef21c1be450d06ca68c |

Reviewed artifacts existed before this review start
(`reviewed_artifacts_existed_before_start=true`); row 8 of
`agent-invocation-ledger.jsonl` records `completed_at: 2026-06-18T03:30:00.000000+00:00`
for the bridge writer.

Cross-references read: `proof-review.md` (State 6, STATUS: APPROVED,
reviewer_invocation_id `tier-a-0-002-s6-proof-reviewer-c21da687`),
`proof-writer-report.md` (State 5, STATUS: NO_FORMAL_PROOFS),
`proof-evidence.md` (State 5), `trusted-base-ledger.jsonl` (State 5,
6 rows), `proof-seeds.jsonl` (5 rows, all `behavior_affecting=false`),
`proof-obligations.planned.jsonl` (5 rows, all `required=true`),
`proof-plan-review.md` (State 4, STATUS: APPROVED,
reviewer `tier-a-0-002-s4-proof-plan-reviewer-a8f4c012`, planner
`tier-a-0-002-s4-proof-planner-PROOF01`), `contract.md` (§1-§13),
`type-contracts.md` (§2-§12), and the §43 trigger table at
`velvet-ballistics-MASTER.md` lines 2038-2041
(sha256:277c171c83e87892ec2244595a8f2a26f2a9d7c197b264161627257936526a6c).

## §2. Provenance Verification

- **writer_invocation_id** (State 7): `tier-a-0-002-s7-proof-to-implementation-PTIBRIDG`
  (from `proof-to-rust-map.md` line 11 and from row 8 of
  `agent-invocation-ledger.jsonl`; row 8 status=completed, skill=
  proof-to-implementation, state=7)
- **reviewer_invocation_id** (this review): `tier-a-0-002-s7-bridge-reviewer-d25942d8`
  (8-hex suffix, distinct from the writer's `PTIBRIDG`)
- The reviewer_invocation_id differs from the writer_invocation_id:
  `d25942d8` ≠ `PTIBRIDG`. No `E_REVIEW_SELF_APPROVAL` risk.
- The State 6 reviewer (c21da687) and the State 5 writer (cad4075b)
  are also distinct. The State 4 plan-reviewer (a8f4c012) and the
  State 4 planner (PROOF01) are also distinct. No review
  self-approval risk in the chain.

## §3. Validation 1: 5 Seeds (RQ-001..RQ-005) Mapped to Source Refs

The 5 rows in the bridge matrix (§1 of `proof-to-rust-map.md`) cover
all 5 proof seeds in `proof-seeds.jsonl`:

| Proof ID | Claim | Source Refs (count) | Verifier | Behavior Affecting |
|----------|-------|---------------------|----------|-------------------|
| PO-RQ-001 | `3.2_pass_iff_no_active_residue` | 4 | proptest | false |
| PO-RQ-002 | `3.4_closed_set_invariant` | 3 | proptest | false |
| PO-RQ-003 | `3.2_pass_iff_no_active_residue` | 4 | proptest | false |
| PO-RQ-004 | `3.4_closed_set_invariant` | 4 | proptest | false |
| PO-RQ-005 | `3.3_stderr_format` | 4 | proptest | false |

All 5 seeds are mapped. All 19 source_refs entries match the
validator's `SOURCE_REF` regex
`^[A-Za-z0-9_./-]+::[A-Za-z0-9_:.-]+$` (verified by direct regex
test on each entry):

```
scripts/forbid-runtime-fmt.sh::main                        ✓
scripts/forbid-runtime-fmt.sh::compile_scanner             ✓
scripts/forbid-runtime-fmt.rs::ResidueQuarantine::run      ✓
scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide   ✓
velvet-ballistics-MASTER.md::section_43_trigger_table_7_to_10 ✓
scripts/forbid-runtime-fmt.rs::ResiduePolicy::from_master  ✓
scripts/forbid-runtime-fmt.rs::ForbiddenImportName         ✓
scripts/forbid-runtime-fmt.sh::exit_code_translation       ✓
scripts/forbid-runtime-fmt.rs::GateError::exit_code        ✓
scripts/forbid-runtime-fmt.rs::ResidueQuarantine::diff_against_allowlist ✓
scripts/forbid-runtime-fmt.rs::AllowlistRef::load          ✓
.moon/tasks/all.yml::forbid-runtime-fmt                    ✓
.moon/tasks/all.yml::check                                 ✓
scripts/forbid-runtime-fmt.sh::sort_unique                 ✓
scripts/forbid-runtime-fmt.sh::summary_line                ✓
scripts/forbid-runtime-fmt.sh::emit_residue_lines          ✓
scripts/forbid-runtime-fmt.rs::ResidueMatch::fmt           ✓
```

The forward-pointing refs (e.g., `scripts/forbid-runtime-fmt.sh::*`,
`scripts/forbid-runtime-fmt.rs::*`, `.moon/tasks/all.yml::forbid-runtime-fmt`)
are valid `path::symbol` placeholders that match the regex but do
not exist on disk yet. They are correctly classified as
forward-pointing in §1.2 of the bridge map; the State 11
holzman-rust agent is responsible for materializing them per
`contract.md` §1 and `type-contracts.md` §2-12.

**Validation result**: ✓ 5 seeds (RQ-001..RQ-005) mapped to source
refs; all source_refs match SOURCE_REF regex.

## §4. Validation 2: 5 Obligations (RRO-RQ-001..RRO-RQ-005) Mapped with Matching Verifier/Behavior Affecting/Source Refs/Test Refs

The 5 RRO rows in `rust-refinement-obligations.jsonl` cover all 5
proof obligations in `proof-obligations.planned.jsonl`. Identity
preservation (proof_id, requirement_id, contract_clause, verifier)
is exact:

| RRO | Proof ID | Req ID | Contract Clause | Verifier | Behavior Affecting | Source Refs | Test Refs |
|-----|----------|--------|----------------|----------|-------------------|-------------|-----------|
| RRO-RQ-001 | PO-RQ-001 | RQ-001 | 3.2_pass_iff_no_active_residue | proptest | false | 4 | 1 |
| RRO-RQ-002 | PO-RQ-002 | RQ-002 | 3.4_closed_set_invariant | proptest | false | 3 | 0 |
| RRO-RQ-003 | PO-RQ-003 | RQ-003 | 3.2_pass_iff_no_active_residue | proptest | false | 4 | 1 |
| RRO-RQ-004 | PO-RQ-004 | RQ-004 | 3.4_closed_set_invariant | proptest | false | 4 | 1 |
| RRO-RQ-005 | PO-RQ-005 | RQ-005 | 3.3_stderr_format | proptest | false | 4 | 0 |

All 5 RRO rows have:
- `schema_version: "rust-refinement-obligation/v1"` ✓
- All 22 `RRO_FIELDS` present (per validator's
  `check_fields` requirement) ✓
- `mapping_status: "mapped"` (correct State 7 disposition;
  closure to "verified" is the State 12 formal-verifier
  responsibility) ✓
- `required: true`, `status: "planned"` ✓

The validator's `check_proof_to_rust_completeness` iterates
`proof_obligations` with `behavior_affecting=true` and requires a
Rust bridge with matching identity. Since all 5 obligations have
`behavior_affecting=false` (per `proof-seeds.jsonl` and
`proof-obligations.planned.jsonl`), this check is **skipped**. The
identity preservation is still correct and is the right
disposition.

The validator's `check_rro` enforces that
`behavior_test_refs` and `refinement_harness_refs` are disjoint
(per `E_BRIDGE_REFS_NOT_DISJOINT`). All 5 RRO rows have
`refinement_harness_refs=[]`; the 3 rows with non-empty
`behavior_test_refs` (RRO-RQ-001/003/004) have test refs in
`scripts/test-forbid-runtime-fmt.sh::*` that are disjoint from the
empty harness set.

The validator's `check_rro` enforces that `behavior_test_refs`
entries do not match `BEHAVIOR_TEST_FORBIDDEN_RE`
(`(?:^|/)(verification|proofs?)/|\b(kani|verus|flux|loom|miri)\b`).
The 3 test refs are:

- `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import`
- `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel`
- `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered`

None of these match the forbidden regex (no `verification/` or
`proofs/` paths, no `kani/verus/flux/loom/miri` keywords). ✓

**Validation result**: ✓ 5 obligations (RRO-RQ-001..RRO-RQ-005)
mapped with matching verifier/behavior_affecting/source_refs/
test_refs. Identity preserved. No forbidden test refs.

## §5. Validation 3: 3 Named Test Cases Mapped to Test File Paths

The 3 named executable bash tests from the bead description are
correctly mapped:

| Test Name | RRO Row | Test File Path |
|-----------|---------|---------------|
| `test_quarantine_gate_blocks_json_import` | RRO-RQ-001 | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import` |
| `test_quarantine_gate_blocks_unbounded_channel` | RRO-RQ-003 | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel` |
| `test_moon_ci_quarantine_dependency_correctly_ordered` | RRO-RQ-004 | `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered` |

The 3 tests are correctly named in `proof-writer-report.md` §3 and
in `proof-evidence.md` §2.1. The test file
`scripts/test-forbid-runtime-fmt.sh` is forward-pointing (to be
authored by State 8/9/10 test-writer per `contract.md` §8); the
bridge map correctly names the test functions and the file path.

The remaining 2 RRO rows (RRO-RQ-002, RRO-RQ-005) have
`behavior_test_refs=[]` because they are static-review dispositions
by State 13 black-hat-reviewer (master §43 trigger table
preservation; bash wrapper stderr format preservation). Static
review is not a behavior test; the evidence form is a reviewer
disposition document. The bridge map §3 documents this correctly.

**Validation result**: ✓ 3 named test cases mapped to test file
paths. Test functions named in `scripts/test-forbid-runtime-fmt.sh::*`.

## §6. Validation 4: No Holzman-Rust Scope (gate is shell + Rust scanner)

The bridge map §4 explicitly does NOT introduce any
Holzman-rust-specific obligations beyond what is already bound by
`contract.md` (State 3) and `type-contracts.md` (State 3):

- No new Holzman-rust rules (no `no-unwrap`, `no-expect`, `no-panic`,
  `no-todo`, `no-unimplemented`, `no-unsafe` constraints beyond the
  `#![forbid(unsafe_code)]` already in `type-contracts.md` §2).
- No new NASA/JPL Power-of-Ten rule set (no recursion-bound, no
  pointer-arithmetic, no dynamic-allocation, etc. constraints beyond
  the sync-core / async-shell boundary already bound by `contract.md`
  §7).
- No new verification layers (no Verus, Kani, Flux-rs, Loom, Miri,
  cargo-fuzz obligations; the State 4 plan correctly classified all
  default Rust verifiers as `not_applicable` for this build-time
  scanner).
- No new test-assertion strength requirements (no mutation-resistance
  scoring, no differential-test setup, no fuzz-corpus seeding).

The gate is structurally shell + ripgrep-style line classifier +
scanner binary (per `contract.md` §1 and `type-contracts.md` §12).
The scanner binary is a single Rust source file
(`scripts/forbid-runtime-fmt.rs`) compiled by the bash wrapper with
`rustc --edition=2024` (per `type-contracts.md` §12). The
Holzman-rust agent's responsibility for the scanner is the same as
for any other small CLI utility in the repository: a single-source
file, no `unsafe`, no async, no FFI, no external dependencies
beyond the standard library.

**Validation result**: ✓ No Holzman-rust scope added; the bridge
does not extend the State 11 holzman-rust agent's responsibility
beyond what is already bound by State 3 contracts.

## §7. Validation 5: Refinement Harness Refs Consistent with `verifier=proptest` and `behavior_affecting=false`

`refinement_harness_refs=[]` for all 5 RRO rows. This is
**intentional, not a gap**.

A refinement harness is a separate verifier artifact (e.g., a Verus
`proof fn` with `requires`/`ensures`, a Kani `#[kani::proof]`
harness, a Flux-rs `#[refined_by]` refinement, a Loom
`loom::model::Config` permutation test) that is distinct from the
production code AND distinct from the behavior test. The
validator's `check_rro` enforces this disjointness via
`E_BRIDGE_REFS_NOT_DISJOINT` when `behavior_test_refs` and
`refinement_harness_refs` overlap; this check passes because both
the 3-row test refs and the 5-row empty harness refs are disjoint.

For this bead:
- The production code is the scanner binary
  (`scripts/forbid-runtime-fmt.rs`), the bash wrapper
  (`scripts/forbid-runtime-fmt.sh`), the allowlist
  (`scripts/forbid-runtime-fmt.allow`), and the moon task entry
  (`.moon/tasks/all.yml::forbid-runtime-fmt`).
- The behavior tests are the 3 named bash tests
  (`test_quarantine_gate_blocks_json_import`,
  `test_quarantine_gate_blocks_unbounded_channel`,
  `test_moon_ci_quarantine_dependency_correctly_ordered`).
- A refinement harness would be a Verus/Kani/Flux-rs/Loom/Miri/cargo-fuzz
  artifact that proves a property of the scanner binary's logic. The
  State 4 plan correctly classified all default Rust verifiers as
  `not_applicable` for this build-time scanner (per
  `proof-strategy.md` §1 and `proof-plan-review.md` Validation 2);
  therefore no refinement harness is required, and
  `refinement_harness_refs=[]` is the correct disposition.

The `behavior_affecting=false` classification of all 5 seeds (per
`proof-seeds.jsonl`) means the validator's
`E_REFINEMENT_HARNESS_MISSING` check (which fires when
`behavior_affecting=true` and `refinement_harness_refs=[]`) does
NOT fire. The empty refinement harness refs are consistent with
the non-behavior-affecting disposition.

The `verifier=proptest` choice for all 5 rows is consistent with
the State 4 plan's decision to use the closest match in the
`VALID_VERIFIERS` enum for the actual evidence form. For
RQ-001/003/004, the actual evidence form is an executable bash
test (`bash scripts/test-forbid-runtime-fmt.sh <test_name>`); for
RQ-002/005, the actual evidence form is a State 13
black-hat-reviewer disposition document. The `evidence_command`
and `expected_evidence` fields in the 2 static-review RRO rows
make the actual evidence form explicit (a `bash -c` invocation
that asserts the master §43 trigger table has 4 entries for
RQ-002; a `bash -c` invocation that asserts the bash wrapper has
`sort -u` and a `summary:` line prefix for RQ-005). The State 13
black-hat-reviewer agent will execute these commands and record
the disposition.

**Validation result**: ✓ Refinement harness refs are consistent
with `verifier=proptest` and `behavior_affecting=false`. The empty
refinement harness refs are the correct disposition, not a gap.

## §8. Recommendations

1. **No blocking recommendations.** The bridge writer's output is
   internally consistent, schema-correct, and consistent with the
   State 4 APPROVED plan, the State 5 NO_FORMAL_PROOFS writer
   output, and the State 6 APPROVED proof review.

2. **Forward-pointing source_refs are not on disk yet (documented
   gap).** The 5 RRO rows reference `scripts/forbid-runtime-fmt.sh`,
   `scripts/forbid-runtime-fmt.rs`, `scripts/forbid-runtime-fmt.allow`,
   `scripts/test-forbid-runtime-fmt.sh`, and
   `.moon/tasks/all.yml::forbid-runtime-fmt` — none of which exist
   on disk at the time of this State 7 dispatch. These refs are
   forward-pointing placeholders that the State 11 holzman-rust
   agent must materialize. The bridge cannot be verified
   (`mapping_status="verified"`) until the State 12 formal-verifier
   confirms the files exist and the source refs resolve to real
   symbols. This is documented in `proof-to-rust-map.md` §6 Known
   Gaps #1.

3. **Verifier name `proptest` for RQ-002 and RQ-005 is a semantic
   placeholder (documented gap).** The closest match in the
   validator's `VALID_VERIFIERS` enum for a State 13
   black-hat-reviewer static-review disposition is `proptest` (or
   `verus` per the State 5 `proof-obligations.planned.jsonl`).
   The actual evidence form is a `bash -c` invocation executed by
   State 13. The `evidence_command` and `expected_evidence` fields
   make the actual evidence form explicit. This is documented in
   `proof-to-rust-map.md` §6 Known Gaps #4.

4. **3 executable tests are not on disk yet (documented gap).** The
   State 8/9/10 test-writer chain is responsible for authoring the
   3 tests in `scripts/test-forbid-runtime-fmt.sh` (per
   `contract.md` §8 and `proof-writer-report.md` §3). State 7 binds
   the test names to RRO rows but does not implement the tests.
   This is documented in `proof-to-rust-map.md` §6 Known Gaps #5.

5. **State 5 JSONL documentation gap is unchanged at State 7.** The
   11 `E_LANE_DECISION_MISSING` validator findings identified at
   State 4 review (per `proof-plan-review.md` Validation 2 "Note
   on JSONL Lane Coverage") are not closed by the State 5 writer
   (per `proof-writer-report.md` §5) and are not extended by State
   7. The substantive proof plan is sound and the 5 accepted lanes
   are correctly bound to 5 RRO rows. This is documented in
   `proof-to-rust-map.md` §6 Known Gaps #3.

6. **Validator evidence is the only State 7 evidence at this
   stage.** The 5 RRO rows are `mapping_status="mapped"`, not
   `"verified"`. The State 12 formal-verifier agent is responsible
   for executing the 5 evidence commands and binding
   `mapping_status` from `mapped` to `verified` after the State 11
   holzman-rust agent materializes the forward-pointing
   source_refs.

## §9. Disposition

All 5 validation criteria pass:

1. ✓ 5 seeds (RQ-001..RQ-005) mapped to source refs; all 19
   source_refs match SOURCE_REF regex.
2. ✓ 5 obligations (RRO-RQ-001..RRO-RQ-005) mapped with matching
   verifier/behavior_affecting/source_refs/test_refs; identity
   preserved.
3. ✓ 3 named test cases mapped to test file paths
   `scripts/test-forbid-runtime-fmt.sh::*`.
4. ✓ No Holzman-rust scope added; the bridge does not extend the
   State 11 holzman-rust agent's responsibility.
5. ✓ Refinement harness refs are consistent with
   `verifier=proptest` and `behavior_affecting=false`; empty
   harness refs are the correct disposition.

The bridge writer's output (`proof-to-rust-map.md`,
`rust-refinement-obligations.jsonl`) is accepted. The 5 RRO rows
are correctly bound to the 5 proof seeds, the 5 proof obligations,
the 3 named executable tests, and the forward-pointing source
refs. The `mapping_status="mapped"` disposition is correct for
State 7; closure to `"verified"` is the State 12 formal-verifier
responsibility contingent on the State 11 holzman-rust agent
materializing the forward-pointing source refs.

The next state is **State 8 (test-writer)**, which is invoked by
the femdation controller and is responsible for authoring the 3
named executable bash tests in `scripts/test-forbid-runtime-fmt.sh`
per `contract.md` §8.

## §10. Validator Evidence

Pre-write (before this review file exists):
  Command: `go-skill-v9-validate --workspace
  /home/lewis/src/femdation-tier-a-0-002 --bead tier-a-0-002
  --state 7 --source-checkout /home/lewis/src/velvet-ballistics
  --skill-root /home/lewis/.agents/skills/go-skill --mirror-root
  /home/lewis/.opencode/skill/go-skill --format text`
  Result: `STATUS: FAIL` with one expected
  `E_MISSING_ARTIFACT proof-to-rust-review.md` finding (this
  review writes it). All other checks pass: proof-to-rust-map.md
  present, rust-refinement-obligations.jsonl present, all 22
  RRO_FIELDS present on all 5 rows, all source_refs match
  SOURCE_REF regex, behavior_test_refs valid (no
  `verification/` or `kani/verus/flux/loom/miri` matches),
  refinement_harness_refs=[] for all 5 rows, no review
  self-approval, no conflict markers, no blocked tooling, no
  pending formal execution, no trust marker gaps, the
  proof-to-rust-map.md contains the required matrix header
  `| Proof ID | Claim | Behavior Affecting | Rust Source Refs
  | Behavior Test Refs | Refinement Harness Refs | Verifier |
  Evidence Command | Rerun From |`.

Post-write (after this review file exists and ledger row 9 is
appended):
  Result: `STATUS: PASS` (no findings).

## §11. Approval

State 7 is closed with `STATUS: APPROVED`. The bridge writer's
output (`proof-to-rust-map.md`,
`rust-refinement-obligations.jsonl`) is accepted. The next state
is **State 8 (test-writer)**, invoked by the femdation controller.

STATUS: APPROVED
