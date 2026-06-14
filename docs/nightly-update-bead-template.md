# Nightly Update Bead Template

Use this template for every Rust nightly update bead. The bead is incomplete
until every field below contains concrete evidence or a named blocker.

## Required Fields

- Current nightly: `nightly-YYYY-MM-DD`
- Target nightly: `nightly-YYYY-MM-DD`
- Motivation: why the update is needed now.
- Changed compiler behavior: release notes, diagnostics, feature-gate changes,
  verifier/tooling behavior, or `none observed` with evidence.
- Rollback plan: exact command or revert path, owner, and stop condition.

## Required Gate Evidence

- Full CI: `moon ci` command, exit status, raw log path, and blocker class.
- Miri: command, exit status, raw log path, or unavailable-tool blocker.
- Fuzz smoke: command, target(s), duration/iteration budget, exit status, and
  raw log path.
- Recovery tests: command, covered recovery path, exit status, and raw log path.

## Required Benchmark Evidence

- Before-update benchmark command and raw result artifact.
- After-update benchmark command and raw result artifact.
- Delta summary with allowed variance and pass/fail decision.
- Regression owner and rollback threshold.

## Closure Checklist

- [ ] Current nightly recorded.
- [ ] Target nightly recorded.
- [ ] Motivation recorded.
- [ ] Changed compiler behavior recorded.
- [ ] Rollback plan recorded.
- [ ] Full CI evidence recorded.
- [ ] Miri evidence recorded.
- [ ] Fuzz smoke evidence recorded.
- [ ] Recovery test evidence recorded.
- [ ] Before benchmark evidence recorded.
- [ ] After benchmark evidence recorded.
- [ ] Benchmark delta and variance decision recorded.
