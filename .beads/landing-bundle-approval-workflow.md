# Landing Bundle Approval Workflow

**Bead**: vb-zg5sb
**Date**: 2026-08-30
**Purpose**: Define the mandatory approval workflow for landing bundles. All unverified landing bundles are marked as blockers in `unverified-landing-bundles-ledger.md` until approved through this workflow.

---

## Policy

**No landing bundle may be pushed to main or a production bookmark without formal evidence approval.** This policy enforces AGENTS.md requirements:

- Formal verification mandates (GOD RULES 1-5)
- Zero-tolerance source lint
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`
- Every speed claim requires real benchmark evidence
- TLA+ specs must model bounded hardware limits (Rule 3)
- Kani harnesses must use `kani::Arbitrary`, not hardcoded data (Rule 1)
- Verus proofs must bind to production Rust code (Rule 2)

---

## Approval Workflow

### Phase 1: Pre-Submission Checklist

Before evidence is submitted for review, the implementer MUST verify:

| # | Gate | Command | Required Output |
|---|------|---------|-----------------|
| 1 | Source formatting | `rtk cargo fmt --all` | PASS (zero diffs) |
| 2 | Source length | `bash scripts/check-source-length.sh` | PASS (all files < 300 lines) |
| 3 | No forbidden patterns | `rg -n 'unwrap|expect|panic|todo!|unimplemented|dbg!' crates/` | Zero matches |
| 4 | No unsafe code | `rg -n 'unsafe' crates/` | Zero matches (except documented exceptions) |
| 5 | Compile check | `rtk cargo check --all-features` | PASS |
| 6 | Unit tests | `rtk cargo test --lib` | All tests PASS |
| 7 | Integration tests | `rtk cargo test --test` | All tests PASS |
| 8 | CI gate | `moon ci` | All tasks PASS |

**If any gate fails, the landing bundle is BLOCKED and must be repaired before proceeding.**

---

### Phase 2: Evidence Production

For each landing bundle, produce the required evidence artifacts:

#### 2.1 Required Evidence Files

| Artifact | Description | Required For |
|----------|-------------|-------------|
| `STATE.md` | Current bead state, progress, blockers | All beads |
| `contract-spec.md` | Type-level contract (newtypes, domain models) | All beads with behavioral logic |
| `proof-strategy.md` | Which verification lanes are used and why | Beads with safety-critical code |
| `test-plan.md` | Exhaustive test strategy (unit, integration, proptest, mutation) | All beads |
| `test-suite-review.md` | Test reviewer adversarial review | All beads |
| `black-hat-review.md` | Contract parity + DDD + zero-warning review | All beads |
| `proof-evidence.md` | Raw verifier command evidence (Verus/Kani/Flux/TLA+) | Beads using formal verification |
| `final-evidence-decision.md` | Final approval decision with STATUS | All beads before landing |
| `landing-report.md` | Branch info, commit info, files changed, integration status | All beads that land |

#### 2.2 Formal Verification Evidence (if applicable)

| Verification Type | Evidence Required | Verification Standard |
|-------------------|------------------|----------------------|
| Kani | Harness list, proof results, `#[cfg(kani)]` modules | GOD RULE 1: No hardcoded shapes; use `kani::Arbitrary` |
| Verus | Proof artifacts bound to production Rust via `#[path]` | GOD RULE 2: Must bind to production code, not vacuum models |
| TLA+ | `.tla` spec, `.cfg` file, TLC output, THEOREM statements | GOD RULE 3: Bounded model with realistic bounds; all specs must have TypeOK |
| Proptest | Property tests for invariants, boundary conditions, mutation resistance | Standard behavior tests |
| Loom | Concurrency permutation tests for lock-free code | Required for any shared-state concurrency |
| Fuzz | Fuzz targets for structured inputs | Required for parsers, codecs, wire formats |

#### 2.3 TLA+ Specific Requirements

Every `.tla` file MUST have:

1. **TypeOK invariant**: Constrains all variables to finite, meaningful bounds
2. **THEOREM statements**: `THEOREM Spec => []InvariantName` for each safety invariant
3. **Fairness in cfg**: If the spec defines WF/SF fairness, the cfg must include it
4. **Non-trivial bounds**: At least `RunId = {1, 2}` for cross-run interference checking
5. **Nonvacuity probes**: cfg variant that intentionally violates each invariant to confirm TLC can detect failure
6. **Liveness properties**: Any liveness claim in spec comments must be declared as PROPERTIES in cfg

Files in `unverified-landing-bundles-ledger.md` (Category 1) all fail these requirements.

---

### Phase 3: Adversarial Review

#### 3.1 Proof Review (proof-reviewer skill)

The proof reviewer MUST check for:

- **LETHAL**: Vacuous invariants (prove nothing beyond type checking)
- **LETHAL**: Missing THEOREM statements (no machine-verifiable proof contract)
- **LETHAL**: Disconnected Verus specs (not bound to production Rust)
- **LETHAL**: Kani harnesses with hardcoded literal data (GOD RULE 1 violation)
- **LETHAL**: TLA+ specs without TypeOK (GOD RULE 3 violation)
- **HIGH**: Shallow bounds (toy model cannot catch real bugs)
- **HIGH**: Liveness claims without PROPERTIES in cfg
- **HIGH**: Fairness defined in spec but not in cfg

#### 3.2 Black-Hat Review

The black-hat reviewer MUST check for:

- Contract parity (every contract clause has evidence)
- DDD design (Scott Wlaschin style, illegal states unrepresentable)
- Zero-warning source lint
- No residual risks without explicit waiver

#### 3.3 Test Review (test-reviewer skill)

The test reviewer MUST check for:

- Coverage of public API surface
- Boundary conditions and error paths
- Mutation resistance
- Determinism (no flaky tests)
- No Mode 2 blockers (tests that don't prove anything)

---

### Phase 4: Evidence Decision

The implementer produces `final-evidence-decision.md` with one of:

| Decision | When to Use |
|----------|-------------|
| `STATUS: APPROVED` | All gates pass, all reviews pass, no blockers |
| `STATUS: REJECTED` | One or more gates or reviews have findings |
| `STATUS: BLOCKED` | Infrastructure issue prevents completion (document blocker) |

**REJECTED bundles must be repaired and re-submitted. Each re-review cycle must be documented.**

---

### Phase 5: Landing

Only after `final-evidence-decision.md` reports `STATUS: APPROVED`:

1. **Create landing report** (`landing-report.md`) with:
   - Branch name and creation date
   - Commit message and files changed
   - moon ci result
   - All review statuses

2. **Execute landing**:
   ```bash
   # From isolated workspace only
   git push origin <branch>
   git checkout main
   git merge <branch>
   git push origin main
   ```

3. **Close the bead**:
   ```bash
   bd close <bead-id>
   ```

4. **Update ledger**: Remove the bead from `unverified-landing-bundles-ledger.md` and archive its entry.

---

## Blocker Handling

### When a Bundle is Marked as a Blocker

If any landing bundle is found to be unverified (missing evidence, rejected review, infrastructure blocker):

1. **Record in ledger**: Add entry to `unverified-landing-bundles-ledger.md` with category and blocker reason
2. **Notify**: The next agent in the chain must check the ledger before claiming dependent work
3. **Do NOT merge**: The blocked bundle and its dependents must NOT be merged to main
4. **Repair plan**: Create a repair plan with priority and timeline

### Blocker Severity Levels

| Severity | Meaning | Action Required |
|----------|---------|-----------------|
| LETHAL | Evidence proves nothing; must be completely rewritten | Block all dependents immediately |
| HIGH | Evidence is materially deficient; significant rework needed | Block all dependents; repair before proceeding |
| MEDIUM | Evidence has gaps but core logic is sound; targeted repair needed | Allow dependents with caution; document gaps |
| LOW | Minor documentation or formatting issues | Allow landing with minor fixes |

---

## Quick Reference: Is a Landing Bundle Verified?

Answer YES to all 6 questions. If any answer is NO, the bundle is BLOCKED.

1. [ ] Does `final-evidence-decision.md` exist with `STATUS: APPROVED`?
2. [ ] Does `black-hat-review.md` exist with `STATUS: APPROVED`?
3. [ ] Does `test-suite-review.md` exist with `STATUS: APPROVED`?
4. [ ] Did `moon ci` report all tasks PASS?
5. [ ] Are all TLA+ specs in scope present with TypeOK + THEOREM statements?
6. [ ] Are there no LETHAL findings in the most recent proof review?

---

## Related Artifacts

- `unverified-landing-bundles-ledger.md` — Current unverified bundles (this workflow's input)
- `.beads/tla-spec-audit/proof-review.md` — TLA+ spec non-vacuity audit
- `.beads/proof-review-rounds-1-3.md` — Cross-cutting proof review (REJECTED)
- `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/verification-ledger.jsonl` — Example evidence ledger format

---

**END OF LANDING BUNDLE APPROVAL WORKFLOW**
