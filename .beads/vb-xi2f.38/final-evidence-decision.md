# Final Evidence Decision: vb-xi2f.38

**bead**: vb-xi2f.38
**title**: P1: digest covers collect semantics
**date**: 2026-05-25
**auditor**: evidence-packaging agent

---

## STATUS: REJECTED

---

## Rationale

The evidence bundle for vb-xi2f.38 is **REJECTED** for the following blocking reasons:

### Blockers

| # | Blocker | Severity | Detail |
|---|---|---|---|
| 1 | Missing `test-plan-review.md` | CRITICAL | Mandatory artifact per evidence-packaging workflow does not exist |
| 2 | Missing `black-hat-review.md` | CRITICAL | Mandatory artifact per evidence-packaging workflow does not exist |
| 3 | Missing `machine-gate-report.md` | CRITICAL | Mandatory artifact per evidence-packaging workflow does not exist |
| 4 | Missing `regression-diff.md` | CRITICAL | Mandatory artifact per evidence-packaging workflow does not exist |
| 5 | `proof-review.md` STATUS: REJECTED | CRITICAL | Reviewer rejected at state 6 citing CRITICAL finding (Kani harness not calling production code) and 4 HIGH findings |
| 6 | Source checkout compilation failure | CRITICAL | HEAD (vb-xi2f.5, 0806ade88) has compilation errors in `vb_compile/src/ast/parse.rs:95-96` — `moon ci lint-src` FAILS |
| 7 | Moon CI canonical gate failing | CRITICAL | 5 tasks failed in `moon ci` including lint-src and source-length gates |
| 8 | Claimed test count unsupported | HIGH | User context claims "309 tests passed including 18 digest_collect tests"; actual: 243 passed, 2 failed; `digest_collect_tests.rs` MISSING |
| 9 | Formal waivers unapproved | HIGH | FW-001 and FW-002 have `approved_by: null` |
| 10 | PO-012b integration test not executed | HIGH | verification-ledger.jsonl shows NOT_EXECUTED for PO-012b |

### What Is Verifiably Correct

- The **implementation fix IS present** in source at:
  - `crates/vb_compile/src/compile/mod.rs:257-271` (explicit Collect match arm hashing variable, source, pages, items, body)
  - `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-178` (same fix in lowering module)

- The **TLA+ model checking PASSED** (20 states, no errors found)

- The **proptest tests PASSED** (290 tests per formal-verification-report.md, though `digest_collect_tests.rs` file is absent)

- **Formal waivers exist** for Kani/Verus tooling blockers (FW-001, FW-002)

### Required Actions Before Re-consideration

1. **Create or obtain `test-plan-review.md`** — evidence-packaging requires this mandatory artifact
2. **Create or obtain `black-hat-review.md`** — evidence-packaging requires this mandatory artifact
3. **Create or obtain `machine-gate-report.md`** — evidence-packaging requires this mandatory artifact
4. **Create or obtain `regression-diff.md`** — evidence-packaging requires this mandatory artifact
5. **Resolve `proof-review.md` REJECTED status** — either fix the CRITICAL/HIGH findings or obtain formal waiver from proof-reviewer
6. **Fix source checkout compilation** — vb-xi2f.5 broke `vb_compile/src/ast/parse.rs:95-96`; `moon ci lint-src` must pass
7. **Verify or correct test count** — the claimed "309 tests with 18 digest_collect" must be supported by actual command evidence
8. **Approve formal waivers** — FW-001, FW-002 must have `approved_by` set by authorized reviewer

---

## Non-Blocking Observations

- The implementation fix (Collect fields now hashed) matches the contract requirements (CC-DIGEST-001 through CC-DIGEST-007)
- TLA+ verification lane is strong (20 states, LoweringDeterminism invariant verified)
- The verification-ledger.jsonl is well-structured with 21 entries covering all obligations
- The `truth-serum` tool was not available, so this audit is manual — a proper truth-serum run may reveal additional issues

---

## Evidence References

- assurance-bundle.md: `.beads/vb-xi2f.38/assurance-bundle.md`
- truth-serum-report.md: `.beads/vb-xi2f.38/truth-serum-report.md`
- Source checkout: `/home/lewis/src/velvet-ballistics` (HEAD: 0806ade88 vb-xi2f.5)
- Bead commit: `a626cda0e` (vb-xi2f.38, ancestor of HEAD)

---

*Decision by evidence-packaging agent. Bead vb-xi2f.38 may not land until all blockers are resolved and STATUS: APPROVED is issued.*
