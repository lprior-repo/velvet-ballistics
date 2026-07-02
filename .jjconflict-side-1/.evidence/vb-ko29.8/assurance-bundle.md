# vb-ko29.8 Idempotency Assurance Bundle

Scope: final evidence package for epic `vb-ko29` idempotency / rerun safety, covering child beads `vb-ko29.1` through `vb-ko29.7` and final proof review `vb-ko29.8`.

Final status: **APPROVED FOR EVIDENCE PACKAGING**.

Approval inputs:

- Proof review: `.evidence/vb-ko29.8/proof-review.md` — `STATUS: APPROVED`, residual blockers: none.
- Test review: `.evidence/vb-ko29.5/test-review.md` — `STATUS: APPROVED`.
- Bridge summary: `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md` — `PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`.
- Refreshed proof review confirms the bridge JSONL note cleanup is complete and consistent with `IDEMP-DIRECT-STALE-CERTIFICATE-ADMISSION` status `TESTED`.
- Post-black-hat refresh confirms digest mismatch has precedence over stale-certificate rejection in the floor-aware public admission API.
- Black-hat review: `.evidence/vb-ko29.8/black-hat-review.md` — `STATUS: APPROVED`, blockers: none.

## Child evidence index

| Child bead | Report / index | Raw evidence pointers | Verdict |
|---|---|---|---|
| `vb-ko29.1` | `.evidence/vb-ko29.1/tla-idempotency-report.md` | `.evidence/vb-ko29.1/logs/IdempotencySafetyTypeOK.log`; `IdempotencySafety.log`; `IdempotencySafetyOverflow.log`; `IdempotencySafetyTerminalFinality.log`; `IdempotencySafetyDuplicateSuccess.log`; `IdempotencySafetyDuplicateFailure.log`; `IdempotencySafetyDivergentDigest.log`; `IdempotencySafetyCrashRecoverDuplicate.log`; `IdempotencySafetyRetryCollision.log`; `IdempotencySafetyStaleTracker.log` | PASS |
| `vb-ko29.2` | `.evidence/vb-ko29.2/verus-idempotency-binding-report.md`; `.evidence/vb-ko29.2/verus-idempotency-binding-map.jsonl` | `.evidence/vb-ko29.2/verus-idempotency-decision.log`; `verus-idempotency-replay-tracker.log`; `verus-idempotency-certificate-summary.log`; `verus-trust-scan.log`; `cargo-check-vb-compile.log` | PASS |
| `vb-ko29.3` | `.evidence/vb-ko29.3/kani-compile-unblock-report.md` | `.evidence/vb-ko29.3/cargo-kani-version.log`; `vb_core-kani-list-final-r3.log`; `vb_validate-kani-list-final-r2.log`; superseded reproductions: `vb_core-kani-list-before-cratedir.log`, `vb_validate-kani-list-before-cratedir.log` | PASS |
| `vb-ko29.4` | `.evidence/vb-ko29.4/kani-idempotency-generators-report.md` | `.evidence/vb-ko29.4/vb_core-verify_idempotency_missing_key_symbolic_contract_no_frame_write.log`; `vb_core-verify_idempotency_duplicate_invocation_is_stable-r2.log`; `vb_core-verify_idempotency_duplicate_success_clean_key.log`; `vb_core-verify_idempotency_duplicate_failure_tainted_key.log`; `vb_core-verify_idempotency_required_taint_variants_have_witnesses-r2.log`; `vb_core-verify_idempotency_boundary_key_lengths_pass_clean_frame.log`; `vb_core-verify_idempotency_frame_slot_bounds_no_panic.log`; `vb_core-verify_idempotency_retry_policy_matrix_no_frame_write.log`; `vb_core-idempotency_divergent_digest_symbolic_certificate_rejected-r2.log`; `vb_core-validate_action_outcome_certificate_stale_nonterminal.log`; `vb_core-validate_action_outcome_certificate_conflict_oob.log`; `vb_validate-kani_decision_001_all_combinations.log` | PASS |
| `vb-ko29.5` | `.evidence/vb-ko29.5/public-idempotency-tests-report.md`; `.evidence/vb-ko29.5/test-review.md` | `.evidence/vb-ko29.5/cargo-test-no-run.raw.log`; `cargo-test.raw.log`; `cargo-test-vb-runtime-admission.raw.log`; refreshed `.evidence/vb-ko29.8/idempotency-suite.raw.log`; `.evidence/vb-ko29.8/vb-runtime-admission.raw.log` | PASS / APPROVED |
| `vb-ko29.6` | `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md`; `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl` | Child proof/test logs referenced in each JSONL row; ordering closure raw evidence `.evidence/vb-ko29.8/idempotency-suite.raw.log`; overflow closure raw evidence `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log` and `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.exit` | PASS: 20 mapped rows, 0 blocked, 0 waived |
| `vb-ko29.7` | `.evidence/vb-ko29.7/loom-miri-idempotency-report.md` | `.evidence/vb-ko29.7/loom-idempotency.log`; `miri-idempotency-alt-20260404.log`; `miri-alt-20260404-version.log`; `rustfmt-touched-check.log`; superseded default-nightly Miri tooling note `miri-idempotency.log` | PASS with superseded tooling note retained |
| `vb-ko29.8` | `.evidence/vb-ko29.8/proof-review.md`; `.evidence/vb-ko29.8/black-hat-review.md`; this bundle; `.evidence/vb-ko29.8/proof-test-source-alignment.md`; `.evidence/vb-ko29.8/proof-test-source-alignment.jsonl` | latest targeted checks `.evidence/vb-ko29.8/idempotency-suite.raw.log`, `.evidence/vb-ko29.8/vb-runtime-admission.raw.log`, `.evidence/vb-ko29.8/touched-rustfmt-check.raw.log`; JSONL validation command run during packaging, exit 0 in session output | APPROVED |

## Requirement coverage summary

| Requirement family | Contract/source anchor | Proof/refinement evidence | Behavior evidence | Final disposition |
|---|---|---|---|---|
| Static idempotency decision parity | `vb_core::action`, `vb_validate::idempotency_contract`, `vb_compile::check_idempotency_gates` refs in `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.jsonl:1` | Verus decision proof, Kani all-combinations harness | Not required for internal verifier gate | PROVED |
| Key-required and taint/idempotency verifier gates | `crates/vb_core/src/action.rs:371-388` refs in bridge rows 2-4 | Kani symbolic generators and cover-backed harnesses in `.evidence/vb-ko29.4/` | Not required for internal verifier gates | PROVED |
| Certificate outcome and attestation gates | `crates/vb_core/src/action.rs:450-489`, storage/runtime admission refs in bridge rows 5, 14, 17, 18 | Kani certificate harnesses; Verus certificate summary | Public digest-conflict, stale-certificate admission, and wrong-digest-plus-stale ordering tests | PROVED / TESTED |
| Journal duplicate and cross-run isolation | storage journal source refs in bridge rows 6-7 | TLA duplicate/cross-run bounded model evidence | Public duplicate success/failure and cross-run journal tests | TESTED |
| Recovery crash/replay and durable fallback | storage replay/tracker refs in bridge rows 8, 12 | TLA crash/recover, Verus replay tracker, Loom eviction model | Public crash-restart and eviction durable replay tests | PROVED / TESTED |
| Runtime retry tracker duplicate handling | runtime idempotency refs in bridge row 9 | Loom same-scope collision model | Public retry-required duplicate dispatch denial | TESTED |
| Lifecycle retry collision, stale terminal retry, cross-run retry isolation | CLI/lifecycle refs in bridge rows 10, 11, 19 | TLA retry, terminal finality, stale tracker scenarios | Public CLI retry collision/stale/cross-run tests | TESTED |
| Digest divergence and digest-before-stale ordering | admission/storage digest refs in bridge row 13 and ordering row `IDEMP-DIGEST-BEFORE-STALE-CERTIFICATE-ORDERING` | TLA divergent digest and Kani digest harness | Public divergent artifact digest test; wrong-digest-plus-stale-floor ordering test in `.evidence/vb-ko29.8/idempotency-suite.raw.log` | TESTED |
| Event sequence overflow fail-safe | `crates/vb_storage/src/codec/mod.rs:46-70` refs in bridge row 15 | TLA overflow fail-safe | Storage overflow tests, raw log `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log` | TESTED |
| Miri no-UB representative tracker path | `crates/vb_runtime/src/idempotency.rs:49-232` refs in bridge row 16 | Miri command evidence under `nightly-2026-04-04` | Not a behavior-test requirement | PROVED |

## Waivers and blockers

- Bridge status reports `PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`.
- No `formal-waivers.jsonl` was created because no waiver rows are needed.
- Superseded evidence retained but not counted as pass: Kani timeout/failed predecessor logs in `vb-ko29.4`; Kani harness-filter miss in `vb-ko29.6`; default-nightly Miri tooling failure in `vb-ko29.7`.
- Prior stale bridge-note wording has been cleaned up and is no longer a blocker or documentation hygiene issue per refreshed proof review.
- Prior black-hat stale-ordering rejection is addressed by source ordering and refreshed public raw evidence.

## Reviewer verdicts

- Test reviewer: `.evidence/vb-ko29.5/test-review.md`, `STATUS: APPROVED`.
- Proof reviewer: `.evidence/vb-ko29.8/proof-review.md`, `STATUS: APPROVED`.
- Black-hat reviewer: `.evidence/vb-ko29.8/black-hat-review.md`, `STATUS: APPROVED`.

## Residual blockers

None recorded in the reviewed final package.
