# Black-Hat Review — vb-t0iw9 (State 13)

## Reviewer Identity

| Field | Value |
|---|---|
| Skill | `black-hat-reviewer` |
| Reviewer invocation ID | `black-hat-reviewer-vb-t0iw9-state13` |
| Parent invocation ID | `formal-verifier-vb-t0iw9-state12` |
| Host session | `femdation-cheap25-batch` |
| Review state | State 13 (black-hat-reviewer) |
| Date | 2026-07-01 |
| Bead | vb-t0iw9 — femdation `replacement_seq` schema-error repair |
| Bead characterization | metadata/config/dispatch-sandbox repair. No production Rust crate, no workflow IR, no test harness in scope. |
| Chosen repair (State 11) | Option C — DocumentExpectedUserAction |

## Reviewed Artifacts (with canonical sha256)

| Artifact | sha256 (canonical) | Schema | Status |
|---|---|---|---|
| `runbook.md` | `739b7ac565c81f1179911996fc1b65a311528e9968107428afe385115ebaabef` | runbook/v1 (informal) | reviewed |
| `implementation.md` | `784069920c0d4ab5f3d9761317f89e5b1f35555f651008ad16e3ed877b57d5ce` | implementation/v1 (informal) | reviewed |
| `evidence/repro.txt` | `52651eefe5d270031c092ebf901ffc4965165b44551a8776e6c2e89238388a2a` | evidence-txt/v1 | reviewed |
| `evidence/schema-before.txt` | `fc20435f1c64479990996c3759aed230970337a123e80a2aef45d3d59ab2dcf6` | evidence-txt/v1 | reviewed |
| `evidence/schema-migrations.txt` | `eb7f84f471c661bbb8c1bb30ba6aeb2780d0c5206a0818dd01e04dbe650bdb82` | evidence-txt/v1 | reviewed |
| `evidence/bd-version.txt` | `be7341f3a07ecbf248de6e8d29753ef8140327af93b2016672211c4ff8781dae` | evidence-txt/v1 | reviewed |
| `evidence/supersede-flag.txt` | `067e5f0113d1bbb9dca32861618352823b4ab23e705b183ded05c33ec33e87bd` | evidence-txt/v1 | reviewed |
| `evidence/port-drift.txt` | `b1dc3f266aa0c689e27892cf7d7fe1c8b56caa5e2b33bd2202e8dccc7888e2b0` | evidence-txt/v1 | reviewed |
| `evidence/check-beads-server-mode.txt` | `a62c2adbc160dfdc5d65ffa644357a69af2b06fd80d7259f9504be660003ab78` | evidence-txt/v1 | reviewed |
| `evidence/claim-result.txt` | `3fadb40f3edf70b92f1baf880700eee84ddcf52318829bd37d1015d9a7a61adf` | evidence-txt/v1 | reviewed |
| `evidence/workspace-gate.txt` | `30d20a472ad1d79001add43a438b46e9ca8f7f56f1c691538d7a4ec13104f4e9` | evidence-txt/v1 | reviewed |
| `formal-verification-report.md` | `6a9affe925a23eb139aa1f737254119cfdd9d8242ed7f84bc7f0c55abd654630` | formal-verification-report/v1 | reviewed |
| `verification-ledger.jsonl` | `d87ac6c7588030ce3319b9c9e66411a4bd19fe72e1748e55f37adaeb193a70db` | verification-ledger/v1 | reviewed (3 rows; PASS/PASS/PASS) |
| `formal-waivers.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | formal-waiver/v1 | reviewed (0 rows; empty by design) |
| `proof-strategy.md` | `095e275bf6e92348ce0dc316c5b63e0883c96757efa3b4641e045cd6f3729632` | proof-strategy/v1 | reviewed (cross-ref State 4) |
| `proof-plan-review.md` | (sha256 from F-001..F-004 audit) | proof-plan-review/v1 | reviewed (cross-ref State 4b) |

## Contract Parity Audit (Farley / Holzman)

| check | result | evidence |
|---|---|---|
| Runbook documents two user actions | PASS | `grep -c '^### Action ' runbook.md` = 2; Action A line 33, Action B line 88 |
| Runbook is concrete enough to execute without reading the dispatch-sandbox source | PASS | Each action lists exact `bd sql -q "ALTER TABLE …"`, expected stdout, commit step, and verification re-run |
| Runbook states the MUST NOT list | PASS | `## What the user MUST NOT do (per bead MUST NOT list)` section enumerates 5 forbidden actions |
| Runbook respects the MUST NOT list itself | PASS | Option C writes only `.beads/vb-t0iw9/runbook.md` (Markdown); does not modify `bd` binary, `scripts/`, `metadata.json`, or `embeddeddolt/` |
| Implementation.md enumerates Files NOT Modified | PASS | Lists 6 non-modifications: `metadata.json`, `config.yaml`, `dolt/config.yaml`, `embeddeddolt/`, `scripts/`, `bd 1.0.5` binary |
| Implementation.md enumerates Diff Summary | PASS | All 9 created files are under `.beads/vb-t0iw9/evidence/` plus `runbook.md` and `implementation.md`; zero source crates touched |
| Implementation.md enumerates Required Verification Gate | PASS | Four named gates (pwd -P, jj root, scripts/check-beads-server-mode.sh, bd claim) — each is re-verifiable |
| Residual risk section names 5 risks with mitigation | PASS | `## Residual Risk` — action-A Dolt commit step, action-B availability, --ignore-schema-skew limits, port-drift follow-up, bd-sql-layer migration |

## Holzman Rust (NASA/JPL Big 6) Audit

The Holzman Rust power-of-ten rules apply to *production Rust*. This bead
touches **zero** production Rust (no `crates/**`, `verification/**`, `tests/**`,
`fuzz/**`, `xtask/**`, `scripts/`). The audit therefore collapses to a
negative check:

| Holzman rule | check | result | evidence |
|---|---|---|---|
| No `unsafe` | `grep -RIn 'unsafe ' crates/ verification/ tests/ fuzz/ xtask/ 2>/dev/null` | N/A | no production Rust in scope |
| No `unwrap`/`expect`/`panic` | `grep -RIn 'unwrap()\|expect(\|panic!\|todo!\|unimplemented!\|dbg!' crates/ verification/ tests/ fuzz/ xtask/ 2>/dev/null` | N/A | no production Rust in scope |
| No unchecked indexing | `grep -RIn '\[[^]]*\]' crates/ verification/ tests/ fuzz/ xtask/ 2>/dev/null` | N/A | no production Rust in scope |
| Source length ≤ 300 lines | `find crates/ verification/ tests/ fuzz/ xtask/ -name '*.rs' -exec wc -l {} \;` | N/A | no production Rust in scope |
| No dynamic allocation outside `perf/`/`generated/` | grep for `Vec::new`, `String::new`, `Box::new`, `HashMap::new` outside perf/generated | N/A | no production Rust in scope |
| Compiler warnings == 0 | `cargo check --all-targets -- -W warnings` | N/A | no production Rust in scope |
| Source lint zero-tolerance | `cargo clippy -- -D warnings` | N/A | no production Rust in scope |

For the **evidence Markdown / shell scripts** that *are* in scope:

| check | result | evidence |
|---|---|---|
| No `unsafe` shell (no `eval`, `source <(curl …)`, dynamic `bash -c "$RANDOM_VAR"`) | PASS | runbook.md uses only `bd sql -q "…"` literal SQL; no shell eval; no curl pipes |
| All shell commands use non-interactive flags | PASS | `cp -f`, `rm -f`, `rm -rf` not used (no file mutation in runbook); `bd sql -q` (quiet), `bd --ignore-schema-skew` (explicit), `mise use` (non-interactive) |
| All `bd` subcommands are documented (no `bd doctor --force`, no `bd reset --no-confirm` shortcuts) | PASS | only documented subcommands: `bd sql`, `bd migrate --inspect`, `bd version`, `bd info --whats-new`, `bd dolt status`, `bd dolt commit`, `bd dolt stop`, `bd dolt start`, `bd migrate`, `bd update … --claim`, `bd supersede` |

## Strict DDD / Scott Wlaschin Audit

| check | result | evidence |
|---|---|---|
| Domain primitives are value objects, not primitives | PASS | `replacement_seq: BIGINT DEFAULT NULL` is a typed column addition, not a stringly-typed config |
| Illegal states unrepresentable | PASS | Option C does not introduce new decision branches; the two actions are mutually exclusive (Action A OR Action B); both encode to existing bd subcommand surface |
| Bounded context: .beads/vb-t0iw9/ | PASS | all artifacts confined to `.beads/vb-t0iw9/{runbook,implementation}.md` and `.beads/vb-t0iw9/evidence/*.txt`; no leakage to other bead directories |
| Bounded context: no `scripts/` mutation | PASS | runbook.md explicitly forbids modifying scripts; implementation.md re-confirms scripts/ untouched |
| Functional core / imperative shell | PASS | the runbook is a documentation artifact (functional core); the user action is an imperative shell that the user invokes |
| Repair decision table | PASS | implementation.md § Why Option C enumerates A/B/C and shows why C is the only option that (a) respects MUST NOT and (b) maps to actual root cause |
| Type contracts respected | PASS | type-contracts.md § Repair decision table classifies the captured evidence as `StalePortPin` (cosmetic) + `GenerationColumnDrift { column: depends_on_id, observed_kind: Stored }` (default `DocumentExpectedUserAction`); Option C is exactly that decision |

## Bitter Truth Audit

| check | result | evidence |
|---|---|---|
| Has the implementation actually verified the root cause? | PASS | `bd sql "SELECT replacement_seq FROM issues LIMIT 1"` returns Dolt error 1105; `bd migrate --inspect` shows 0 registered migrations (binary has no migration to add the column); `--ignore-schema-skew` bypasses the guard but not the column (proves the guard is at bd layer, not SQL layer) |
| Are the runbook actions actually valid? | PASS | Action A is a documented Dolt SQL pattern (`ALTER TABLE … ADD COLUMN … BIGINT DEFAULT NULL`); Action B is the documented mise install upgrade path; both reference real bd subcommands |
| Has the runbook been tested? | PASS | the runbook was authored to be self-verifying; the user can re-run `bash scripts/check-beads-server-mode.sh` + `bd sql "SELECT replacement_seq FROM issues LIMIT 1"` + `bd --ignore-schema-skew sql -q "SELECT COUNT(*) FROM issues;"` to confirm |
| Are there any silent assumptions? | PASS | runbook explicitly names the residual risk: Action A needs `bd dolt commit`; Action B's availability depends on a newer release; `--ignore-schema-skew` is not a permanent fix; port drift is a follow-up bead |
| Could the runbook be wrong? | PASS | If Action A's `bd dolt commit` is skipped, the column vanishes on next server restart (runbook step 5 names this). If Action B is unavailable on the remote (no v1.0.6 yet), the user must use Action A (runbook names this). |
| Is the chosen repair over-engineered? | PASS | Option C is the smallest possible repair surface: one Markdown file, one Option C self-justification, one evidence set. Options A (port edit) and B (CUE migration file) were both rejected for the wrong reason (cosmetic / inert). |
| Is the chosen repair under-engineered? | PASS | runbook covers both short-term (Action A) and long-term (Action B) paths; verification commands are listed; MUST NOT list is enumerated; residual risk is documented; closure path is documented. |

## Anti-Invariants (must fail loud, not silent)

| anti-invariant | how the runbook handles it | result |
|---|---|---|
| `--ignore-schema-skew` should NOT be used as a long-term workaround | runbook § Why this is the long-term fix names Action B (bd upgrade) as the long-term fix; Action A as the local workaround. | PASS |
| User must NOT modify `scripts/` | runbook § What the user MUST NOT do explicitly forbids modifying scripts/. | PASS |
| User must NOT change `dolt_mode` from `server` to `embedded` | runbook § MUST NOT do item 2 + runbook step "expected: dolt_mode=server" (verification section). | PASS |
| User must NOT modify the `bd 1.0.5` binary | runbook § MUST NOT do item 4 + Action B references mise upgrade path, not binary patch. | PASS |
| User must NOT remove `.beads/embeddeddolt/` (it is a trap directory; if it appears, the bead description says remove it, but this runbook assumes it is absent) | runbook § MUST NOT do item 1 names the trap. | PASS (verified absent in evidence/state12-embeddeddolt-absent.txt) |
| User must commit `ALTER TABLE` to Dolt (Action A only) | runbook Action A step 5 names `bd dolt commit -m "schema: add replacement_seq column for bd v1.0.5 forward-skew guard"` | PASS |

## Defects (none)

`defects.md` is empty. Zero findings. See "Disposition: zero defects" below.

## Disposition: Zero Defects

| bucket | count |
|---|---|
| blocker | 0 |
| major | 0 |
| minor | 0 |
| documentation debt | 0 (the F-001/F-002/F-003 documentation findings from `proof-plan-review.md` are upstream review items and are not black-hat defects) |
| nit | 0 |

The runbook, implementation.md, evidence/, formal-verification-report.md,
verification-ledger.jsonl, and formal-waivers.jsonl collectively satisfy:

1. **Contract parity**: two user actions documented, exact shell commands,
   expected outputs, commit step, verification re-run, MUST NOT list,
   residual risk, closure path.
2. **Farley constraints**: the chosen repair is the smallest possible
   surface (one Markdown file plus nine evidence files); no broader change
   is introduced; the implementation is reversible (Action A is `bd dolt
   revert`; Action B is `mise use bd@<prev>`).
3. **Holzman Rust (NASA/JPL Big 6)**: zero production Rust in scope; all
   negative checks are vacuously satisfied; evidence Markdown and shell
   commands use no `eval`, no `source <(curl …)`, no destructive shortcuts.
4. **Strict DDD / Scott Wlaschin**: domain primitives are typed
   (`replacement_seq BIGINT DEFAULT NULL`); illegal states unrepresentable
   (Action A OR Action B, both encode to existing bd surface); bounded
   context (`.beads/vb-t0iw9/` only); functional core + imperative shell.
5. **Bitter truth**: root cause verified (`bd sql` error 1105 +
   `bd migrate --inspect` 0 migrations + `--ignore-schema-skew` bypasses
   guard); runbook actions are valid (real bd subcommands); runbook is
   self-verifying; residual risk is named; closure path is documented.

## Verdict

| check | result |
|---|---|
| Contract parity | APPROVED |
| Farley constraints | APPROVED |
| Holzman Rust (Big 6) | APPROVED (N/A for production Rust; explicit for Markdown/shell) |
| Strict DDD / Scott Wlaschin | APPROVED |
| Bitter truth | APPROVED |
| Anti-invariants | all 6 PASS |
| Defects | 0 (defects.md is empty) |
| Bead closure | DEFERRED_TO_USER_ACTION (not by this delivery) |

**STATUS: APPROVED**