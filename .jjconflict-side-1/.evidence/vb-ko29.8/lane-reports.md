# vb-ko29.8 Lane Reports Index

This index records the final lane report for each child bead and does not replace raw evidence logs.

| Lane | Child bead | Report | Status |
|---|---|---|---|
| TLA+ bounded model checking | `vb-ko29.1` | `.evidence/vb-ko29.1/tla-idempotency-report.md` | PASS |
| Verus source binding | `vb-ko29.2` | `.evidence/vb-ko29.2/verus-idempotency-binding-report.md` | PASS |
| Kani compile unblock | `vb-ko29.3` | `.evidence/vb-ko29.3/kani-compile-unblock-report.md` | PASS |
| Kani symbolic generators | `vb-ko29.4` | `.evidence/vb-ko29.4/kani-idempotency-generators-report.md` | PASS |
| Public tests and test review | `vb-ko29.5` | `.evidence/vb-ko29.5/public-idempotency-tests-report.md`; `.evidence/vb-ko29.5/test-review.md` | PASS / APPROVED |
| Source-proof-test bridge | `vb-ko29.6` | `.evidence/vb-ko29.6/idempotency-source-proof-test-bridge.md`; `.jsonl` | PASS |
| Loom/Miri | `vb-ko29.7` | `.evidence/vb-ko29.7/loom-miri-idempotency-report.md` | PASS |
| Final proof review | `vb-ko29.8` | `.evidence/vb-ko29.8/proof-review.md` | APPROVED |
| Black-hat ordering re-review | `vb-ko29.8` | `.evidence/vb-ko29.8/black-hat-review.md` | APPROVED |
| Latest targeted public suite | `vb-ko29.8` | `.evidence/vb-ko29.8/idempotency-suite.raw.log` | PASS: 13 passed, 0 failed |
| Latest targeted runtime admission | `vb-ko29.8` | `.evidence/vb-ko29.8/vb-runtime-admission.raw.log` | PASS: 60 passed, 0 failed admission-filtered unit tests |
| Touched Rust formatting | `vb-ko29.8` | `.evidence/vb-ko29.8/touched-rustfmt-check.raw.log`; `.evidence/vb-ko29.8/rustfmt-touched-check.exit` | PASS: exit 0 |

Raw command logs are listed in `.evidence/vb-ko29.8/assurance-bundle.md` and in the child reports above.

Refresh note: `.evidence/vb-ko29.8/black-hat-review.md` approves the digest-before-stale ordering fix; bridge status is now `PROVED: 8`, `TESTED: 12`, `BLOCKED: 0`, `WAIVED: 0`.
