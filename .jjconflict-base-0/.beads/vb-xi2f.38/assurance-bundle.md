# Assurance Bundle

**bead_id**: vb-xi2f.38
**source_checkout**: /home/lewis/src/velvet-ballistics
**isolated_workspace**: /home/lewis/src/vb-xi2f.38-ws
**commit_or_change**: a626cda0e (vb-xi2f.38: fix digest_step_primitive to hash Collect semantics)
**note**: HEAD at vb-xi2f.5 (0806ade88) does not compile; vb-xi2f.38 commit is ancestor

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| CC-DIGEST-001: Digest content-addressing for Collect | CC-DIGEST-001 | TLA+ TLC PASS (20 states); proptest 290 PASS (digest_collect tests) | proof-review.md: REJECTED | BLOCKED |
| CC-DIGEST-001a: Collect field hashing (variable, source, pages, items, body) | CC-DIGEST-001a | TLA+ PASS; proptest 290 PASS | proof-review.md: REJECTED | BLOCKED |
| CC-DIGEST-002: Digest determinism | CC-DIGEST-002 | proptest PASS | proof-review.md: REJECTED | BLOCKED |
| CC-DIGEST-003: Artifact digest depends on source digest | CC-DIGEST-003 | proptest PASS | proof-review.md: REJECTED | BLOCKED |
| CC-DIGEST-004: Collect lowering preserves semantics | CC-DIGEST-004 | TLA+ LoweringDeterminism PASS | proof-review.md: REJECTED | BLOCKED |
| CC-DIGEST-005: Digest mismatch detection | CC-DIGEST-005 | verification-ledger.jsonl: PO-012b NOT_EXECUTED | proof-review.md: REJECTED | MISSING_TEST |
| CC-DIGEST-006: No panic on Collect digest | CC-DIGEST-006 | verification-ledger.jsonl: PO-002 BLOCKED_TOOLING | proof-review.md: REJECTED | WAIVED |
| CC-DIGEST-007: Property-based digest equality | CC-DIGEST-007 | proptest PASS | proof-review.md: REJECTED | BLOCKED |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-001 | tla-plus | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl:1 | PASS | None |
| PO-002 | kani | `cargo kani --workspace --no-default-features` | verification-ledger.jsonl:2 | BLOCKED_TOOLING | FW-001 |
| PO-003 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:3 | PASS | None |
| PO-004 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:4 | PASS | None |
| PO-005 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:5 | PASS | None |
| PO-006 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:6 | PASS | None |
| PO-007 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:7 | PASS | None |
| PO-008 | tla-plus | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl:8 | PASS | None |
| PO-008b | tla-plus | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl:9 | PASS | None |
| PO-009 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:10 | PASS | None |
| PO-010 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:11 | PASS | None |
| PO-011 | verus | `cargo verus --workspace` | verification-ledger.jsonl:12 | BLOCKED_TOOLING | FW-002 |
| PO-012 | tla-plus | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl:13 | PASS | None |
| PO-012b | integration-test | null | verification-ledger.jsonl:14 | NOT_EXECUTED | None |
| PO-013 | kani | `cargo kani --workspace --no-default-features` | verification-ledger.jsonl:15 | BLOCKED_TOOLING | FW-001 |
| PO-014 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:16 | PASS | None |
| PO-015 | kani | `cargo kani --workspace --no-default-features` | verification-ledger.jsonl:17 | BLOCKED_TOOLING | FW-001 |
| PO-016 | kani | `cargo kani --workspace --no-default-features` | verification-ledger.jsonl:18 | BLOCKED_TOOLING | FW-001 |
| PO-017 | tla-plus | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl:19 | PASS | None |
| PO-018 | proptest | `cargo test -p vb_compile` | verification-ledger.jsonl:20 | PASS | None |
| PO-020 | kani | `cargo kani --workspace --no-default-features` | verification-ledger.jsonl:21 | BLOCKED_TOOLING | FW-001 |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| vb_compile tests | `cargo test -p vb_compile` | cargo test output | 243 passed; 2 FAILED (schema tests unrelated to digest) |
| TLA+ model check | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | verification-ledger.jsonl | PASS (20 states) |
| Moon CI lint gate | `moon ci` | moon ci output | FAILED (vb_compile compilation errors in parse.rs:95-96) |

**Note**: `moon ci` shows `lint-src` FAILED due to compilation errors in `crates/vb_compile/src/ast/parse.rs:95-96` where `trigger_str(...).into()` cannot convert `&str` to `Option<Box<str>>`.

**Note**: `crates/vb_compile/src/tests/digest_collect_tests.rs` referenced in proof-evidence.md does NOT exist in source checkout.

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| proof-review | proof-review.md | REJECTED (state 6) | 1 CRITICAL (Kani harness doesn't call production code), 4 HIGH (tooling blockers, disconnected Verus spec, missing TLA+ invariant, absent proptest evidence), 3 MEDIUM |
| test-plan-review | test-plan-review.md | MISSING | Artifact does not exist |
| black-hat-review | black-hat-review.md | MISSING | Artifact does not exist |
| formal-verification | formal-verification-report.md | PARTIAL PASS | TLA+ PASS, proptest PASS (290), Kani BLOCKED_TOOLING, Verus BLOCKED_TOOLING |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| FW-001: Kani BLOCKED_TOOLING (PO-002,013,015,016,020) | Kani 0.67.0 internal compiler error on vb_compile | vb-xi2f.38 | Needs re-run after Kani fix | TLA+ TLC PASS, proptest 290 PASS |
| FW-002: Verus BLOCKED_TOOLING (PO-011) | `cargo verus --workspace` invalid command | vb-xi2f.38 | Needs re-run with correct Verus invocation | TLA+ LoweringDeterminism PASS |
| PO-012b: integration-test NOT_EXECUTED | Test artifact_digest_mismatch does not exist | vb-xi2f.38 | Needs test creation | None |
| MISSING: test-plan-review.md | Artifact not created | vb-xi2f.38 | Must be created before landing | None |
| MISSING: black-hat-review.md | Artifact not created | vb-xi2f.38 | Must be created before landing | None |
| MISSING: machine-gate-report.md | Artifact not created | vb-xi2f.38 | Must be created before landing | None |
| MISSING: regression-diff.md | Artifact not created | vb-xi2f.38 | Must be created before landing | None |
| SOURCE: vb_compile compilation error | vb-xi2f.5 (HEAD) broke webhook trigger parsing | vb-xi2f.5 | Must fix parse.rs:95-96 | None |

---

## Truth Serum Audit

- report: `.beads/vb-xi2f.38/truth-serum-report.md`
- status: **UNVERIFIED** — `truth-serum` tool not available in active execution context
- findings below

### Anti-Hallucination Shield Findings

| Rule | Status | Evidence |
|---|---|---|
| raw_evidence_only | ✅ PASS | All evidence drawn from command output, exit statuses, and artifact contents |
| traceability_kernel | ❌ FAIL | CC-DIGEST-005 (PO-012b) has NO test evidence; CC-DIGEST-006 only has tooling waiver |
| truth_serum_required | ⚠️ UNVERIFIED | truth-serum tool not available; manual audit performed |
| no_new_claims | ✅ PASS | No new correctness claims made during packaging |

### Missing Evidence (Truth Serum Findings)

1. **BLOCKER**: `test-plan-review.md` is MISSING — required by evidence-packaging workflow
2. **BLOCKER**: `black-hat-review.md` is MISSING — required by evidence-packaging workflow
3. **BLOCKER**: `machine-gate-report.md` is MISSING — required by evidence-packaging workflow
4. **BLOCKER**: `regression-diff.md` is MISSING — required by evidence-packaging workflow
5. **BLOCKER**: `proof-review.md` shows STATUS: REJECTED — proof artifacts not approved
6. **BLOCKER**: Source checkout at HEAD (`vb-xi2f.5`) has compilation errors — `moon ci lint-src` FAILS
7. **BLOCKER**: `crates/vb_compile/src/tests/digest_collect_tests.rs` does NOT exist (referenced in proof-evidence.md lines 93-103)
8. **WAIVER UNAPPROVED**: FW-001 and FW-002 have `approved_by: null` — waivers not formally approved
9. **CLAIM DISCREPANCY**: User context says "309 tests passed including 18 digest_collect tests" — actual test run shows 243 passed, 2 failed; `digest_collect_tests.rs` file missing
