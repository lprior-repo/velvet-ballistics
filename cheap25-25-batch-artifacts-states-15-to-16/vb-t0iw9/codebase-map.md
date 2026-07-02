# vb-t0iw9 State 2 codebase map: femdation `replacement_seq` schema error

## Bead identity and isolation
- Bead: `vb-t0iw9` — "Automation: repair femdation replacement_seq schema error"; `P1 bug`, status `in_progress`, assignee `Lewis`, owner `priorlewis43@gmail.com`.
- Source checkout (forbidden to mutate): `/home/lewis/src/velvet-ballistics`.
- Isolated workspace verified by `pwd -P`: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9` (this is the exploration workspace).
- Bead description: `bd show vb-t0iw9 --json` reports dispatch error string `no such column: replacement_seq` from femdation first-wave; femdation subagent returns only that schema error and never reaches `STATE 1` lifecycle.
- Reproduction path called out by bead: "dispatching femdation on ready P0 beads after bd status is healthy" in `/home/lewis/src/velvet-ballistics`.
- Controller: `femdation` skill at `/home/lewis/.agents/skills/femdation/SKILL.md` (orchestrator only — no source checked into velvet-ballistics).
- Shared skill mirror: `/home/lewis/.opencode/skill/femdation` (mirror exists for femdation alongside other agents per the global-readiness-report.md field `mirror_drift: SKILL.md aligned`).

## Inventory of files / paths actually relevant to the bead

### `scripts/` directory in the source checkout (bead says "femdation scripts under scripts/")
- All 41 entries under `/home/lewis/src/velvet-ballistics/scripts/` enumerated: `bench-instruction-counts.sh`, `check-agent-cli-contract.sh`, `check-beads-server-mode.sh`, `check-doc-taint-consistency.py`, `check-error-exhaustiveness.sh`, `check-hot-cold-forbidden-apis.{rs,sh}`, `check-ignored-fallible-results.sh`, `check-nightly-features.sh`, `check-no-rustc-bootstrap.sh`, `check-panic-surface.sh`, `check-production-inner-drift.sh`, `check-public-api-diff.sh`, `check-source-length.sh`, `check-spelling-gate.sh`, `check-stepstate-matrix.sh`, `check-test-integrity.{rs,sh}`, `check-vb-jpq7-closure-evidence.py`, `check-verus-production-binding.sh`, `check-workspace-assertions.{py,rs,sh}`, `flux-check-package.sh`, `forbidden-scan.sh`, `fuzz-minimization.sh`, `generate_queue_state_verus_helpers.py`, `guard-zero-tests.sh`, `hot-cold-forbidden-apis.allow`, `hotpath-scan.{allow,sh}`, `ignored-fallible-results.allow`, `kani-list.sh`, `lib-source-length.sh`, `loom-list.sh`, `run-tlc-checks.sh`, `rust-verification-gauntlet.sh`, `test-check-source-length.sh`, `test-source-length-fixture.sh`, `verify-lean.sh`, `verify-verus.sh`, `verify_no_legacy_primitives.sh`.
- **FINDING**: zero files under `scripts/` named anything-femdation/replace*/sequ*. `rtk grep -rln 'femdation\|replacement_seq' /home/lewis/src/velvet-ballistics/scripts/` returned 0 hits. The femdation-related flows live in the skill files under `~/.agents/skills/`, not as scripts in the source repo.

### `.beads/` schema artifacts in the source checkout
- `.beads/metadata.json`: declares `database: dolt`, `backend: dolt`, `dolt_mode: server`, `dolt_server_host: 127.0.0.1`, `dolt_database: velvet-ballistics`. **No** `dolt_server_port` pin (per AGENTS.md directive).
- `.beads/config.yaml`: team defaults. Last non-comment key observed: `dolt.server-port: 43643` (unrelated to the present schema-error class but worth noting as a stale-port pin that conflicts with the running server on `45645`).
- `.beads/dolt/`: server-side dolt data directory (`.dolt/`, `velvet-ballistics/.dolt/` noms tree, lock file, journal). Schema and data live here; not human-edit friendly.
- `.beads/schemas/*.cue`: cue schemas, none of which mention `replacement_seq`.
- `.beads/hooks/{post-checkout,post-merge,pre-commit,pre-push,prepare-commit-msg}`: Git hook installation only; not relevant to the schema error class.

### Beads database tables (verified through `bd sql ...` against the live shared Dolt server at 127.0.0.1:45645, database `velvet-ballistics`)
- Tables enumerated (`SHOW FULL TABLES`): `blocked_issues` (VIEW), `child_counters`, `comments`, `compaction_snapshots`, `config`, `custom_statuses`, `custom_types`, `dependencies`, `events`, `federation_peers`, `ignored_schema_migrations`, `interactions`, `issue_counter`, `issue_snapshots`, `issues`, `labels`, `local_metadata`, `metadata`, `ready_issues` (VIEW), `repo_mtimes`, `routes`, `schema_migrations`, `wisp_child_counters`, `wisp_comments`, `wisp_dependencies`, `wisp_events`, `wisp_labels`, `wisps`. Total: 28 entries (26 base tables + 2 views).
- `SHOW COLUMNS FROM issues` (57-row output) confirms full schema; no `replacement_seq` column.
- `SHOW COLUMNS FROM dependencies` (10-row output, columns: `id`, `issue_id`, `type`, `created_at`, `created_by`, `metadata`, `thread_id`, `depends_on_issue_id`, `depends_on_wisp_id`, `depends_on_external`); no `replacement_seq`.
- `SHOW COLUMNS FROM ready_issues`, `child_counters`, `wisp_child_counters`, `interactions`, `events`, `routes`, `schema_migrations`, `ignored_schema_migrations`, `metadata`, `local_metadata`, `repo_mtimes`: every table inspected; no column named `replacement_seq` exists in any.
- Migration state: `bd migrate` reports `Schema Version: 1.0.5`, `Registered Migrations: 0`, `Issue Count: 2650`. `schema_migrations` holds 49 rows; `ignored_schema_migrations` holds 8 rows (versions 1..8 applied 2026-06-24). No migration row references a `replacement_seq` column add.

### bd binary inspection
- Binary path: `/home/lewis/.local/share/mise/installs/go-github-com-steveyegge-beads-cmd-bd/1.0.5/bin/bd` (resolved via `which bd`); `bd version 1.0.5 (dev)`.
- `strings <bd-binary>` over the entire 1.74M-line string dump (saved at `/tmp/bd-strings.txt`): `rtk grep -c 'replacement_seq\|replacement-seq\|replacementSeq'` → 0 matches. The exact error phrase `no such column` is also absent from the binary.
- Replacement-related strings present in the binary: `Replacement issue ID (required)`, `replacement issue not found: %s`, `insert replacement dependency target: %w`, `failed to update dependency metadata: %w`, `FIX: Daemon zombie state after DB replacement (#1213)`, `bd supersede` wiring. None of those imply a literal `replacement_seq` column.
- `bd` is statically built (no source available in the repo); repo-wide `rtk grep -rln 'replacement_seq' /home/lewis/src /home/lewis/.agents /home/lewis/.opencode` matches only the bead creation `bd create` invocation log entry in `/home/lewis/.local/share/opencode/log/opencode.log:734441`.

### Femdation skill and `bd` command surface femdation exercises
- `/home/lewis/.agents/skills/femdation/SKILL.md` lines 109-134 enumerate the child dispatch flow (`bd ready` → `bd show <id>` → `bd update <id> --claim` → work → `bd close <id>` → `bd dolt push`) and the dispatch-manifest/routing-ledger contract.
- The replacement-related bead command is `bd supersede <id> --with <new>` (verified `--help` shows `--with string  Replacement issue ID (required)`); confirmed runtime execution (`bd supersede vb-qryp7 --with vb-t0iw9`) returned `✓ Marked vb-qryp7 as superseded by vb-t0iw9 (closed)` against the live server, so the underlying SQL that touches the `dependencies`/`issues` replacement flow is functionally OK today.
- Other `bd` commands directly exercised by femdation lifecycle ran clean: `bd ready`, `bd list`, `bd show vb-t0iw9 --json`, `bd update vb-t0iw9 --claim`, `bd update vb-t0iw9 --priority 1`, `bd supersede vb-qryp7 --with vb-t0iw9`, `bd dolt status`, `bd dolt test`, `bd migrate`, `bd sql ...`. No `no such column: replacement_seq` error reproduces against the current live data.

## Source-checkout contract anchors
- `velvet-ballistics-MASTER.md`: NOT inspected in detail for femdation-specific clauses (out of scope for this bead which targets a bd schema/migration assumption, not an application contract). Repository-level Beads/Dolt clauses are covered under `.beads/hooks/`, `scripts/check-beads-server-mode.sh`, and the `.beads/dolt/` policy enforced by `bd dolt status`.
- `AGENTS.md` lines 21-39 (Beads Dolt Remote policy): mandates `dolt_mode: server`, forbids `dolt_server_port` in `metadata.json`, forbids `.beads/embeddeddolt/`, forbids committing `.beads/dolt`, `.beads/backup`, runtime database state. All of those remain valid; the present bead does not violate them but inherits them as guardrails for any schema repair.

## Existing tests / verifications / evidence touched by this bead
- `scripts/check-beads-server-mode.sh`: hard-fail guard for beads backend/dolt_mode. Indirectly relevant: any schema-repair must keep `dolt_mode=server` and not introduce `.beads/embeddeddolt/`.
- `scripts/check-source-length.sh`, `scripts/check-spelling-gate.sh`, `scripts/check-panic-surface.sh`, etc.: unrelated to bd schema; should be unaffected by a schema-repair commit.
- `bd info`, `bd dolt status`, `bd dolt test`, `bd migrate`: all green at capture time.

## Open questions / risks / unknowns
- **The exact query that fails**: the bead description (`no such column: replacement_seq`) does not show up under `bd strings` and is not present in any of the inspected Dolt tables. This rules out (a) a column missing in a current live table and (b) a literal `replacement_seq` substring in `bd v1.0.5`. Possible remaining hypotheses (none yet confirmed) are:
  - The error originates from an older `bd` binary that did reference a `replacement_seq`-named column on `dependencies`/`issues` (e.g., a column it expected to be present for the `bd supersede`/`bd duplicate` replacement flow) and was later renamed (`depends_on_id` is a STORED generated column as of v1.0.5 per `bd info --whats-new`).
  - The femdation dispatch environment is pinning an older `bd` binary or shadowing `.beads/`. To verify, the repair contract must capture `bd version` output inside the dispatch sandbox.
  - The error string `replacement_seq` is a placeholder pointing at whichever column femdation actually queries during the `claim/replace` flow (likely `dependencies.depends_on_id` or `wisps`/replacement-tracking column); the schema-error class is real but the exact column name has drifted and needs to be re-derived by replaying the failing bd trace.
- **Port pin in `config.yaml`**: `dolt.server-port: 43643` differs from the live `bd dolt status`-reported port `45645`. Not directly the `replacement_seq` cause but indicates stale state; flagged for the contract agent.
- **Ignored schema migrations**: 8 ignored migrations persisted on `ignored_schema_migrations` (versions 1..8, applied 2026-06-24). These are independent of the `replacement_seq` schema error but may complicate fresh-clone bootstrap; surface them for the contract agent.
- **`bd supersede --with <new>` flag reuses the literal token `Replacement issue ID (required)` from `bd --help`** and updates `dependencies` to point at the replacement target. If femdation's `bd update --claim` runs first and locks the bead, the later `bd supersede` flow would touch `dependencies` with replacement-target semantics; the column it expects on `dependencies` may be the historical `depends_on_id` (now a generated STORED column in 1.0.5). This is the most plausible site of the `no such column: replacement_seq` family of errors and should be the first thing the contract agent asserts.

## Likely touched files / globs for repair (State 3+)
- `.beads/dolt/` (only via `bd` writes, never direct edits)
- `.beads/metadata.json` if a `dolt_database` rename or project-id bump is required for schema-skew reset
- `.beads/config.yaml` to drop the stale `dolt.server-port: 43643` pin and let bd auto-discover
- `scripts/check-beads-server-mode.sh` may need a follow-up assertion if the schema-error class warrants a CI gate
- New evidence files: `.beads/vb-t0iw9/{schema-diff.md,bd-version-capture.md,reproduction-trace.txt}`
- UNKNOWN: any explicit femdation source controlled by this repo (scripts/ has none).

## Excluded paths
- `crates/**`, `verification/**`, `tests/**`, `fuzz/**`, `xtask/**`, `kani/**`, `fixtures/**`: out of scope. This bead targets the beads-tracker (bd) and femdation orchestrator surface, not application code.
- `bd/vb-y4pa/` (old dispatch test data): out of scope.
- `.worktrees/`, `.jj/`, `.dolt/`, `.doltcfg/`: read-only inspection only; never written from the isolated workspace.
