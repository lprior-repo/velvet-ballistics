# vb-ko29.8 Formal Verification Report

STATUS: PASS

Scope: final package report for idempotency / rerun-safety evidence produced by `vb-ko29.1` through `vb-ko29.7` and reviewed in `vb-ko29.8`.

## Lane results

| Lane | Evidence report | Raw evidence | Result |
|---|---|---|---|
| TLA+ | `.evidence/vb-ko29.1/tla-idempotency-report.md` | `.evidence/vb-ko29.1/logs/*.log` enumerated in that report | PASS |
| Verus | `.evidence/vb-ko29.2/verus-idempotency-binding-report.md`; `.evidence/vb-ko29.2/verus-idempotency-binding-map.jsonl` | `.evidence/vb-ko29.2/verus-idempotency-*.log`; `.evidence/vb-ko29.2/verus-trust-scan.log`; `.evidence/vb-ko29.2/cargo-check-vb-compile.log` | PASS |
| Kani compile unblock | `.evidence/vb-ko29.3/kani-compile-unblock-report.md` | `.evidence/vb-ko29.3/vb_core-kani-list-final-r3.log`; `.evidence/vb-ko29.3/vb_validate-kani-list-final-r2.log` | PASS |
| Kani idempotency generators | `.evidence/vb-ko29.4/kani-idempotency-generators-report.md` | scoped Kani logs listed in the report | PASS |
| Public behavior tests | `.evidence/vb-ko29.5/public-idempotency-tests-report.md`; `.evidence/vb-ko29.5/test-review.md` | latest `.evidence/vb-ko29.8/idempotency-suite.raw.log` (`13 passed; 0 failed`); latest `.evidence/vb-ko29.8/vb-runtime-admission.raw.log` (`60 passed; 0 failed` for admission-filtered unit tests) | PASS / APPROVED |
| Source-proof-test bridge | `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md`; `.jsonl` | child lane logs plus `.evidence/vb-ko29.8/idempotency-suite.raw.log` for ordering closure and `.evidence/vb-ko29.6/vb_storage-next-seq-overflow.rtk-run.log` for overflow closure | PASS: 8 PROVED, 12 TESTED, 0 BLOCKED, 0 WAIVED |
| Loom/Miri | `.evidence/vb-ko29.7/loom-miri-idempotency-report.md` | `.evidence/vb-ko29.7/loom-idempotency.log`; `.evidence/vb-ko29.7/miri-idempotency-alt-20260404.log`; `.evidence/vb-ko29.7/miri-alt-20260404-version.log` | PASS |
| Final proof review | `.evidence/vb-ko29.8/proof-review.md` | review cites raw evidence refs by lane | APPROVED |
| Black-hat review | `.evidence/vb-ko29.8/black-hat-review.md` | cites `.evidence/vb-ko29.8/idempotency-suite.raw.log` ordering test evidence | APPROVED |
| Formatting touched Rust | `.evidence/vb-ko29.8/touched-rustfmt-check.raw.log`; `.evidence/vb-ko29.8/rustfmt-touched-check.exit` | raw log is empty; exit file records `0` | PASS |

## Final disposition

The final proof reviewer approved the refreshed package with no residual blockers. The public behavior test reviewer approved the test evidence. The black-hat re-review approved the stale-certificate ordering fix. The bridge records 8 `PROVED`, 12 `TESTED`, 0 `BLOCKED`, and 0 `WAIVED` rows.

## Non-counted / superseded evidence retained

- `.evidence/vb-ko29.4/*` predecessor timeout/failure logs listed as superseded in the Kani generator report.
- `.evidence/vb-ko29.6/vb_storage-kani-next-seq-monotonic.log` and `.exit` are explicitly non-counted harness-filter miss evidence.
- `.evidence/vb-ko29.7/miri-idempotency.log` is a default-nightly tooling failure retained as a superseded tooling note; the counted Miri pass is under `nightly-2026-04-04`.
