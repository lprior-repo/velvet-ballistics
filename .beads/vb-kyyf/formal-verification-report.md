# vb-kyyf State 11 Formal Verification Report

STATUS: APPROVED

## Startup doctrine cited

- `/home/lewis/.claude/skills/formal-verifier/SKILL.md` lines 14 and 21-31 require executing existing approved obligations, accounting every obligation, classifying failures by scope/baseline, and never inventing evidence.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md` lines 14 and 21-31 contain the same rules and win on conflict; no conflict observed.

## Input gate

- Workdir: `/home/lewis/src/bd-vb-kyyf-bdd` only.
- Manifest: `.beads/vb-kyyf/dispatch-state11-cap-canonical-aggregate.json`.
- Preflight command passed: required `test -s` checks for base and planned obligations, traceability, delivery scope, baseline, TLA spec, Lean contract, and contract verification review; approved contract status found; `jq -c .` passed for base obligations, planned obligations, traceability, and delivery scope.

## Tool availability evidence

- `tlc`: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `apalache-mc`: `/home/lewis/.local/share/mise/installs/http-apalache/0.57.0/bin/apalache-mc`.
- `verus`: `/home/lewis/.local/bin/verus`.
- `lake`: `/home/lewis/.elan/bin/lake`.
- `moon`: `/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon`.
- `jq`: `/usr/bin/jq`.
- Optional unavailable tools not required by planned obligations: `aeneas`, `charon`, `hax`, `cargo-careful`, `cargo-fuzz`, `cargo-asm`, `cargo-semver-checks`, `cargo-auditable`, `cargo-cyclonedx`, `cargo-vet`.

## Obligation results

| ID | Result | Command evidence |
|---|---|---|
| PO-001 | PASS | `rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism -- --test-threads=1` exit 0; `cargo test: 16 passed (1 suite, 1.14s)`. |
| PO-002 | PASS | `rtk cargo test -p vb_storage --test replay_resume` exit 0; `cargo test: 3 passed (1 suite, 0.02s)`. |
| PO-003 | PASS | `rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism` exit 0; `cargo test: 16 passed (1 suite, 1.15s)`. |
| PO-004 | PASS | `rtk cargo test -p vb_storage --test recovery_bdd_tests` exit 0; `cargo test: 29 passed, 2 ignored (1 suite, 0.11s)`. |
| PO-005 | PASS | `rtk cargo test -p vb_codegen` exit 0; `cargo test: 367 passed (4 suites, 7.76s)`. |
| PO-006 | PASS | Same exact command as PO-003; generated-subset fail-closed scenario included in passing suite. |
| PO-007 | PASS | `rtk cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog` exit 0; `cargo test: 6 passed (1 suite, 0.00s)`. |
| PO-008 | PASS | `JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp' tlc -workers 32 -metadir /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla` exit 0; TLC reported no errors; `42,907,696` states generated, `16,483,704` distinct, depth `9`, finished in `07min 46s`. |
| PO-009 | PASS | `verus verification/verus/vb_kyyf_normalization.rs` exit 0; `verification results:: 43 verified, 0 errors`. |
| PO-010 | DEFERRED_GLOBAL | `moon ci` exit 1 after all scoped vb-kyyf obligations passed. Failures: two out-of-scope `vb_cli` storage-error exit-code tests and `velvet-ballastics:mutants-smoke` disk quota copying `.tlc-metadir`. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e3cd7fbff001gu1aeCytJUYiDo`. |

## Evidence artifacts

- `test -s` passed for all required `.evidence/vb-kyyf/*.md` artifacts: `bdd-cross-run-determinism.md`, `storage-replay-resume.md`, `non-replay-safe-actions.md`, `recovery-bdd-errors.md`, `generated-ir-parity.md`, `generated-subset-fail-closed.md`, and `acceptance-catalog-traceability.md`.

## Waivers

- None used for PO-001..PO-010.

## Final classification

- Required bead-local/touched-crate/protocol obligations PO-001..PO-009: PASS.
- Workspace gate PO-010: DEFERRED_GLOBAL only after scoped obligations passed; failures are outside planned vb-kyyf artifacts and/or environment quota debt.
- No `.beads/vb-kyyf/ci-failure-category.txt` emitted because no bead-local blocking command failed.
