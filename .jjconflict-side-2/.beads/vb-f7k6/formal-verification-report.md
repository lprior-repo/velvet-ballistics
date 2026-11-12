# Formal Verification Report

STATUS: APPROVED

## Inputs

- proof-obligations.jsonl: `.beads/vb-f7k6/proof-obligations.jsonl`
- delivery-scope.jsonl: `.beads/vb-f7k6/delivery-scope.jsonl`
- baseline-report.md: `.beads/vb-f7k6/baseline-report.md`
- tla-spec.md: `.beads/vb-f7k6/tla-spec.md`
- contract-verification-review.md: `.beads/vb-f7k6/contract-verification-review.md`

Mandatory artifact gate passed: required files exist, contract verification review has `STATUS: APPROVED`, and JSONL inputs parse.

## Startup Doctrine Cited

- `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md` were read before execution.
- Both files say the formal verifier executes existing ledgers, accounts for every obligation as `PASS`, `FAIL_LOCAL`, `FAIL_REGRESSION`, `WAIVED`, or `DEFERRED_GLOBAL`, fails closed on missing required tools, and writes `formal-verification-report.md` plus `verification-ledger.jsonl`.
- The files match; per startup rule, `/home/lewis/.agents/skills/formal-verifier/SKILL.md` remains authoritative if future conflicts appear.

## Tool Availability

- tlc / TLC: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`
- apalache-mc: `/home/lewis/.local/share/mise/installs/http-apalache/0.57.0/bin/apalache-mc`
- verus: `/home/lewis/.local/bin/verus`
- lake: `/home/lewis/.elan/bin/lake`
- aeneas / charon: missing
- hax: missing
- cargo creusot / why3: missing
- flux: missing
- prusti: missing
- rust-verification-gauntlet.sh: missing
- scripts/verify-lean.sh: not executable / not used
- cargo kani: `cargo-kani 0.67.0`
- crux-mir: missing
- cargo careful: missing
- sanitizer runtime: covered by configured `moon ci` gates
- moon: `/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon`
- cargo fuzz: `cargo-fuzz 0.13.1`
- cargo bolero: missing
- lockbud: missing
- cargo mutants: `cargo-mutants 27.0.0`
- cargo llvm-cov: `cargo-llvm-cov 0.8.7`
- cargo asm / cargo-show-asm: missing
- cargo semver-checks: `cargo-semver-checks 0.47.0`
- cargo auditable: missing
- cargo cyclonedx: missing
- crux: missing
- saw: missing
- stateright: missing
- jq: present and used in mandatory gate

## Obligation Results

- TLA-TW-001 through TLA-TW-006: PASS via `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`, exit 0, no TLC error, 4,209,522 states generated, 315,211 distinct states, depth 16, temporal properties checked.
- VERUS-TW-001 and VERUS-TW-002: WAIVED as non-required Verus waiver rows; presence command exited 0 and compensating TLA/Loom/runtime lanes ran.
- LOOM-TW-001: PASS via `cargo xtask loom --model timer_fired_cancel`, exit 0, 3 model tests passed, xtask reported `PASS`.
- TEST-TW-001: PASS via `/usr/bin/env cargo test -p vb_runtime timer`, exit 0, 77 unit timer-filtered tests and 1 integration timer-filtered test passed.
- AUTH-TW-001: PASS via `/usr/bin/env cargo test -p vb_runtime timer`, exit 0; observed coverage includes run-only fail-closed behavior, stale replacement rejection, cancelled/terminal rejection, wrong authority rejection, and generation overflow fail-closed tests.
- REVIEW-TW-001: PASS via `test -s .beads/vb-f7k6/contract-verification-review.md && test -s .beads/vb-f7k6/proof-review.md`, exit 0.

## Canonical Gates

- `/usr/bin/env cargo check --workspace --all-targets --all-features`: PASS, exit 0.
- `/usr/bin/env moon ci`: PASS, exit 0, `Tasks: 23 completed`, `Time: 36s 977ms`.

## Waivers

- VERUS-TW-001: WAIVED, non-required row with compensating TLA/runtime evidence.
- VERUS-TW-002: WAIVED, non-required row with compensating TLA/Loom/runtime evidence.

## Residual Risk

- No bead-local formal or canonical gate blocker remains in State 11.
- Optional unavailable tools were not required by `.beads/vb-f7k6/proof-obligations.jsonl`; they are not counted as passes.
