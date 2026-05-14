# Formal Verification Report: vb-qi37.3

STATUS: APPROVED

## Inputs / Startup Citations

- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: lines 14, 21-24, 30-31, 100-114, and 182 require executing approved proof obligations, accounting every line, classifying by scope, fail-closed missing tools unless waived, and no invented evidence.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same content/version observed; no conflict. Agents copy would win on conflicts.
- Required bead artifacts read: `STATE.md`, `contract.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `contract-verification-review.md`, `delivery-scope.jsonl`, `baseline-report.md`, `moon-report.md`, `regression-diff.md`, `manual-qa-smoke.md`, `qa-report.md`, `qa-review.md`, `test-suite-review.md`, `red-queen-report.md`, and `black-hat-review.md`.

## Prerequisite Checks

- Mandatory gate command exited 0: required files non-empty, `contract-verification-review.md` contains `STATUS: APPROVED`, and `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl` parse as valid JSONL with `jq`.
- Contract review approval evidence: `contract-verification-review.md` line 3 says `STATUS: APPROVED`.
- Delivery-scope basis: `delivery-scope.jsonl` scope_version 2; actual changed files are limited to `vb_core`, `vb_runtime`, and `vb_storage` collect/error surfaces.

## Tool Availability

- `tlc`: present.
- `apalache-mc`: present.
- `verus`: missing; required Verus obligations are approved waivers.
- `lake`: present.
- `aeneas` / `charon`: missing; no theorem-owned obligation required.
- `hax`: missing; no theorem-owned obligation required.
- `cargo creusot` / `why3` / `flux` / `prusti`: missing; not required by approved obligations.
- `cargo kani`: present; no Kani obligation in ledger.
- `crux-mir`, `cargo careful`, `cargo fuzz`, `cargo bolero`, `lockbud`: missing; affected lanes are waived or not required.
- `cargo mutants`: present; collect-specific mutation obligation is approved waiver.
- `cargo llvm-cov`: present.
- `cargo asm` / `cargo-show-asm`, `cargo semver-checks`, `cargo auditable`, `cargo cyclonedx`, `crux`, `saw`, `stateright`: missing; not required by approved obligations.
- `moon`: present.
- `jq`: present.
- `scripts/rust-verification-gauntlet.sh`: present.
- `scripts/verify-lean.sh`: present.

## Command Evidence Summary

- Exact `cargo nextest` obligations: 15/15 executed and passed with exit code 0.
- Waiver obligations: 14/14 validated as approved/metadata-backed in `proof-obligations.jsonl`, `verification-layers.md`, `tla-spec.md`, `contract.md`, and `contract-verification-review.md`; recorded as `WAIVED`.
- `bash scripts/rust-verification-gauntlet.sh deep`: executed, exit 1 at unrelated global rustfmt debt before bead-local miri completion; classified `DEFERRED_GLOBAL` with `vb-bkgo` follow-up. `moon-report.md` records miri completed during `moon ci --stdin`.
- `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`: executed, exit 1 at unrelated global rustfmt debt; classified `DEFERRED_GLOBAL` with `vb-bkgo` follow-up.

## Obligation Results

- Total obligations accounted: 31/31.
- `PASS`: 15 exact nextest obligations.
- `WAIVED`: 14 approved waiver obligations.
- `DEFERRED_GLOBAL`: 2 gauntlet obligations blocked only by pre-existing unrelated global FORMAT debt.
- `FAIL_LOCAL`: 0.
- `FAIL_REGRESSION`: 0.

See `verification-ledger.jsonl` for one JSONL result per `proof-obligations.jsonl` line.

## Deferred Global Classification

- Failing gauntlet output is rustfmt/format debt in unmodified global files (for example `xtask/src/proof.rs`), not the bead's actual changed files.
- `moon-report.md` and `regression-diff.md` already classify FORMAT/CLIPPY/`vb_ui_model` failures as `DEFERRED_GLOBAL`, reproduced on clean main, and outside `vb-qi37.3` delivery scope.
- Follow-up bead: `vb-bkgo`.

## Waivers

- TLA waiver: `TLA-WAIVER-COLLECT-001`, owner `State 6 implementer`, approval owner `State 4 contract-verification reviewer`, expiry `2026-05-18 or before release-critical acceptance`, limitation: no collect-specific temporal model; compensating exact tests executed and passed.
- Verus waiver: `VERUS-WAIVER-COLLECT-001`, owner `State 6 implementer`, approval owner `State 4 contract-verification reviewer`, expiry `2026-05-18 or before release-critical acceptance`, limitation: no collect Verus proof target; compensating exact tests executed and passed.
- Fuzz/proptest/static/mutation/API waivers: approved by `contract-verification-review.md`; compensating exact tests and downstream State 9-11 evidence pass.

## Residual Risk

- Formal proof/model gap remains intentionally waiver-backed until release-critical waiver expiry: no collect-specific TLA+ model and no collect Verus target exist.
- Global FORMAT/CLIPPY/`vb_ui_model` debt remains outside bead scope under `vb-bkgo`.

## Decision

State 12 formal ledger is approved for bead scope: every bead-local required obligation is `PASS` or valid `WAIVED`; the only non-green command results are unrelated `DEFERRED_GLOBAL` gauntlet blockers with existing follow-up evidence.
