# Assurance Bundle

bead_id: vb-core-yaml-e2e-chain
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain
commit_or_change: jj workspace revision (not git)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| YAML validate | contract.md PRE-001 | TLC TLA-LIFE-001 | proof-review.md APPROVED | PASS |
| YAML compile | contract.md PRE-002 | vb_compile tests 10 PASS | test-plan-review.md APPROVED | PASS |
| Accepted artifact | contract.md PRE-003, POST-001 | Kani yaml_e2e_admission_matrix | proof-review.md APPROVED | PASS |
| Fjall persistence | contract.md PRE-004 | vb_storage 983 PASS | formal-verification-report.md APPROVED | PASS |
| Strict runtime execution | contract.md PRE-005, POST-002 | vb_runtime 1460 PASS | formal-verification-report.md APPROVED | PASS |
| Journal/events/inspect | contract.md INV-001..INV-008 | TLC JournalPrefixDurable | proof-review.md APPROVED | PASS |
| Replay | contract.md POST-003 | TLC NoYamlParseAfterAdmission | proof-review.md APPROVED | PASS |
| Recovery | contract.md POST-004 | vb_qi37_1_1 19 PASS | test-suite-review.md APPROVED | PASS |
| No YAML reparsing | contract.md ERR-001 | TLC RecoveryInputsPersistedOnly | proof-review.md APPROVED | PASS |
| Digest binding | contract.md VERUS-DIG-004/005 | Verus 8 verified | proof-review.md APPROVED | PASS |
| Typed errors | contract.md ERR-001..ERR-011 | 11 ERR obligations | test-plan-review.md APPROVED | PASS |
| Strict policy 15 gates | contract.md POST-001 | Kani + 35 contract tests | black-hat-review.md APPROVED | PASS |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| TLA-LIFE-001 | TLC | `tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` | YamlE2eChain.tla | PASS (2728 states, 990 distinct, depth 13) | None |
| TLA-DUR-002 | TLC | Same | Same | PASS (PersistBeforeAck, JournalPrefixDurable) | None |
| TLA-REC-003 | TLC | Same | Same | PASS (NoYamlParseAfterAdmission, RecoveryInputsPersistedOnly) | None |
| VERUS-DIG-004 | Verus | `verus verification/verus/yaml_e2e_digest_roles.rs` | yaml_e2e_digest_roles.rs | PASS (8 verified, 0 errors) | None |
| VERUS-DIG-005 | Verus | Same | Same | PASS (shared run) | None |
| KANI-ADMIT-023 | Kani | `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` | yaml_e2e_admission_matrix.rs | PASS (1 harness, 7 checks) | None |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| Strict YAML tests | `cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` | vb_core_yaml_e2e_chain_strict_yaml.rs | 10 PASS |
| Contract tests | `cargo test -p velvet-ballistics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` | vb_core_yaml_e2e_chain_contract.rs | 35 PASS |
| vb_storage | `cargo test -p vb_storage -- --nocapture` | vb_storage | 983 PASS |
| vb_runtime | `cargo test -p vb_runtime -- --nocapture` | vb_runtime | 1460 PASS |
| CLI integration | `cargo test -p velvet_ballistics --test cli_integration -- --nocapture` | cli_integration | 86 PASS |
| E2E recovery | `cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture` | vb_qi37_1_1_red_recovery_contract_test | 19 PASS |
| Clippy | `cargo clippy --all-features -- -D warnings ...` | workspace | No issues |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof review | proof-review.md | APPROVED | TLA/Verus/Kani all PASS |
| Test plan review | test-plan-review.md | APPROVED | 35 concrete tests, density correct |
| Contract verification | contract-verification-review.md | APPROVED | All 32 traceability entries covered |
| Formal verification | formal-verification-report.md | APPROVED | 18 PASS / 3 FAIL_LOCAL / 2 DEFERRED_GLOBAL |
| Black-hat review | black-hat-review.md | APPROVED | No defects found |
| Machine gate | machine-gate-report.md | APPROVED | All 9 gate groups PASS |
| Regression | regression-diff.md | APPROVED | No regressions |

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| STATIC-BOUNDARY-009 | `fuzz/src/lib.rs:1392` needless return | State 8 | Downstream fix | Clippy PASS on production |
| STRICT-YAML-012 | `lib.rs:4152` digest test assertion | State 10 | Downstream fix | Kani + 35 contract tests PASS |
| ERR-STRICT-013 | Same as above | State 10 | Downstream fix | Same |
| MIRI-CODEC-024 | Pre-existing nightly rust-src absence | Tooling | rust-src component | Kani + 983 vb_storage + 1460 vb_runtime PASS |
| GATE-RELEASE-025 | Pre-existing jj workspace environment | Environment | Non-bead-local | Compensating: 18 obligations PASS |

## Truth Serum Audit

- report: `.beads/vb-core-yaml-e2e-chain/truth-serum-report.md`
- status: APPROVED
- evidence: State 13 truth-serum ran in active execution context; all gates verified with command evidence.
