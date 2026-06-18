# Tier-A-0-002 Codebase Map — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 2 (explore)
updated_at: 2026-06-17T21:00:00.000000+00:00
attempt: 1-of-7

## 1. Scope Summary (Bead Description Re-Statement)

The bead installs a fail-closed CI gate that quarantines residue of forbidden
runtime dependencies in the four hot crates
(`crates/vb_core/src/`, `crates/vb_runtime/src/`, `crates/vb_storage/src/`,
`crates/vb_ipc/src/`). The new gate is a sibling of the existing residue
gates (`check-removed-crate-residue`, `check-removed-feature-residue`,
`hot-cold-forbidden-apis`) but targets a different and additive
forbidden-token set:

- `serde_json`
- `serde_yaml`
- `hyper`
- `reqwest`
- `axum`
- `HashMap<String, _>`
- `tokio::sync::mpsc::unbounded`

The gate is wired into `moon ci` as a new `.moon/tasks/forbid-runtime-fmt.yml`
task and ships with three failing-first behavior tests:
`test_quarantine_gate_blocks_json_import`,
`test_quarantine_gate_blocks_unbounded_channel`,
`test_moon_ci_quarantine_dependency_correctly_ordered`.

The gate is grounded in master §2 (Non-Negotiable Rust Rules: "No JSON in the
runtime core", "No HTTP in the runtime core", "No `HashMap<String, Value>`
runtime state"), §12 (Forbidden Hot-Path APIs: explicit `serde_json`,
`HashMap<String, _>`, `JSON parser calls`, `HTTP server/client calls`,
`unbounded channel creation`), and §43 (AI Agent Acceptance Contract
triggers 7-10: "Allocation behavior", "Hot-path behavior", "Fjall
persistence behavior if touched", "IPC behavior if touched" — the four
behaviors most directly impacted by forbidden-deps residue).

## 2. Hot Crates Where the Gate Applies

These four paths are the explicit gate target. They are the canonical
hot-path crate set already used by sibling gates (`hot-cold-forbidden-apis`,
`hotpath-scan`, `unsafe-audit`, `source-length`).

| Path | Cargo.toml | First-party runtime deps of interest | `serde_json` / `serde_yaml` / `hyper` / `reqwest` / `axum` / `tokio` status |
|------|------------|--------------------------------------|----------------------------------------------------------------------------|
| `crates/vb_core/src/` | `crates/vb_core/Cargo.toml` | `serde`, `thiserror`, `bytes`, `chrono`, `indexmap`, `postcard` | `serde_json` present as `dev-dependencies` (line 22); `# allow-cold-adapter` marker indicates test-only usage, never linked into runtime |
| `crates/vb_runtime/src/` | `crates/vb_runtime/Cargo.toml` | `serde`, `thiserror`, `tracing`, `crossbeam-queue`, `rtrb`, `blake3`, `chrono`, `indexmap`, `postcard`, `vb_core`, `vb_storage` | No forbidden deps in `[dependencies]` or `[dev-dependencies]` |
| `crates/vb_storage/src/` | `crates/vb_storage/Cargo.toml` | `serde`, `thiserror`, `fjall`, `arrayvec`, `blake3`, `chrono`, `crc32c`, `rustix`, `postcard`, `vb_core` | No forbidden deps in `[dependencies]` or `[dev-dependencies]` |
| `crates/vb_ipc/src/` | `crates/vb_ipc/Cargo.toml` | `serde`, `thiserror`, `arrayvec`, `byteorder`, `bytes`, `crossbeam-channel`, `crossbeam-queue`, `mio` (net + os-poll), `postcard`, `vb_core`, `vb_runtime` | No forbidden deps in `[dependencies]` or `[dev-dependencies]` |

None of the four hot crates declares `serde_json` or `serde_yaml` outside
`vb_core`'s `dev-dependencies`. None declares `hyper`, `reqwest`, `axum`, or
`tokio`. The gate's primary use-case is therefore *regression prevention*:
any future commit that introduces one of these forbidden deps as a runtime
or dev-dep on a hot crate, or that introduces the banned source patterns
(`HashMap<String, _>` in non-test code, `tokio::sync::mpsc::unbounded`
anywhere in a hot crate), must be caught at the moon-ci layer before
`moon run :check` even runs.

### 2.1 Cargo.toml survey of the entire workspace

The survey is wider than the four hot crates because the gate is the
*next* layer after the four. Other crates already declare `serde_json` /
`serde_yaml`; the gate must not false-positive on those:

- `crates/vb_core/Cargo.toml` line 22 — `serde_json.workspace = true` (dev-dep, allow-cold-adapter marker)
- `crates/vb_cli/Cargo.toml` line 18 — `serde_json = { workspace = true, features = ["alloc"] }` (CLI/cold, not in gate scope)
- `crates/vb_benchmark/Cargo.toml` line 14 — `serde_json.workspace = true` (bench, not in gate scope)
- `crates/vb_boundary_inventory/Cargo.toml` line 8 — `serde_json.workspace = true` (cold, not in gate scope)
- `crates/workspace_tests/Cargo.toml` lines 16-17 — `serde_json.workspace = true` AND `serde_yaml.workspace = true` (test/workspace harness, not in gate scope)
- No `Cargo.toml` declares `hyper`, `reqwest`, `axum`, or `tokio` (verified with `rg -n 'serde_json|serde_yaml|hyper|reqwest|axum|tokio' crates/*/Cargo.toml`).
- `Cargo.lock` does list `serde_json` (line 1694) and `serde_yaml` (line 1716) as transitive dependencies brought in by the cold crates and dev-deps; this is expected and the gate must not flag transitive lock entries — the gate scans `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/**/*.rs` and direct `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/Cargo.toml` `[dependencies]` / `[dev-dependencies]` tables only.

## 3. Existing Residue-Gate Patterns in the Repo

Five pre-existing gate families cover the residue/hot-path territory.
The new gate is modeled on the closest siblings: `check-removed-crate-residue`
and `check-removed-feature-residue`. Each is a `bash` + `rustc` pair (or
`bash` + `clippy-driver` pair for the hard-deny lint enforcement) plus a
self-test script plus positive/negative fixtures.

### 3.1 `scripts/check-removed-crate-residue.sh` + `scripts/check-removed-crate-residue.rs`

- `scripts/check-removed-crate-residue.sh` (59 lines): bash wrapper that
  compiles `scripts/check-removed-crate-residue.rs` with `clippy-driver` and
  the hard-deny clippy lints (`-D clippy::unwrap_used`, `-D clippy::expect_used`,
  `-D clippy::panic`, `-D clippy::todo`, `-D clippy::unimplemented`,
  `-D clippy::dbg_macro`, `-D unsafe_code`, `-D warnings`), then runs it
  with no args (full repo scan) or a single explicit path. Banned tokens:
  `vb_codegen`, `vb_ui_model`, `vb_ui_makepad`, `makepad-widgets`,
  `makepad-draw`, and bare `makepad` with word boundary.
- `scripts/check-removed-crate-residue.rs` (15.2 KB): Rust scanner with
  per-line allowlist; reports `REMOVED-CRATE:` file:line findings and
  exits 0 if active findings == 0.
- Self-test: `scripts/test-check-removed-crate-residue.sh` (165 lines) with
  8 assertions (positive, negative, makepad, shell-bypass, real-repo,
  missing-path fail-closed, word-boundary, allowlisted).
- Fixtures: `fixtures/removed-crate-residue/{positive.md, negative.md, negative_makepad.rs, negative_boundary.md, negative_allowlisted.md}`.
- Moon task: `.moon/tasks/all.yml::check-removed-crate-residue` (line 255)
  with `command: bash scripts/check-removed-crate-residue.sh`, inputs
  on the script + rust file + fixtures + `.moon/tasks/all.yml`,
  `runInCI: true`.

### 3.2 `scripts/check-removed-feature-residue.sh` + `scripts/check-removed-feature-residue.rs`

- `scripts/check-removed-feature-residue.sh` (45 lines): bash wrapper
  that compiles `scripts/check-removed-feature-residue.rs` with `rustc` and
  runs it. Banned tokens: `target-cpu=native`, `pgo` (in active contexts
  `pgo = `, `cargo pgo`, `pgo-data`, `RUSTC_PGO`), `maxperf` (as feature
  identifier), `generated` (as feature identifier). Per-line allowlist
  marker: `# allow-removed-feature: <reason>` or
  `// allow-removed-feature: <reason>`.
- `scripts/check-removed-feature-residue.rs` (12.1 KB): scanner.
- Self-test: `scripts/test-check-removed-feature-residue.sh` (7.3 KB).
- Fixtures: `fixtures/removed-feature-residue/{positive.toml, negative.toml, negative_cli_features.txt, negative_profile.txt, negative_profile_pgo.txt}`.
- Moon task: `.moon/tasks/all.yml::check-removed-feature-residue` (line 267)
  with `command: bash scripts/check-removed-feature-residue.sh`,
  `runInCI: true`.

### 3.3 `scripts/check-hot-cold-forbidden-apis.sh` + `scripts/check-hot-cold-forbidden-apis.rs`

- `scripts/check-hot-cold-forbidden-apis.sh` (13 lines): bash wrapper.
- `scripts/check-hot-cold-forbidden-apis.rs` (920 lines): scanner that
  scans the four hot crates and detects forbidden hot-path APIs
  (`HashMap<String, u8>`, `mpsc::sync_channel`, `Mutex<VecDeque<...>>`,
  etc.). Class IDs: `FORMAT-PRINT-001`, `FORMAT-DBG-001`, `FORMAT-JSON-001`,
  `FORMAT-YAML-001`, `MAP-STRING-001`, `CHANNEL-UNBOUNDED-001`,
  `CHANNEL-BOUNDED-001`, `QUEUE-MUTEX-VECDEQUE-001`.
- Allowlist: `scripts/hot-cold-forbidden-apis.allow` (13 lines header
  comment + 0 entries; format
  `crates/<crate>/src/<file>.rs|CLASS|owner=...|reviewed_by=...|test=...|reason=...`).
- Moon task: `.moon/tasks/all.yml::hot-cold-forbidden-apis` (line 633),
  declared as a `deps:` of the `check` task (line 117), `runInCI: true`.

### 3.4 `scripts/forbidden-scan.sh`

- `scripts/forbidden-scan.sh` (40 lines): wraps
  `xtask/src/forbidden_scan.rs` via a temporary `cargo run` to detect
  first-party introduction of forbidden token/pattern combos.
- Moon task: `.moon/tasks/all.yml::forbidden-scan` (line 478), `runInCI: false`
  (xtask currently excluded from the workspace; off-CI developer tool).

### 3.5 `scripts/hotpath-scan.sh` + `scripts/hotpath-scan.allow`

- `scripts/hotpath-scan.sh` (3.6 KB): hot-path scanner for allocations,
  formatting, `Vec::push` without pre-reserve, `String` construction in
  hot paths.
- `scripts/hotpath-scan.allow` (6 KB): explicit exceptions keyed by
  `crates/<crate>/src/<file>.rs|<token>|owner=...|reviewed_by=...|test=...|reason=...`.
- Moon task: `.moon/tasks/all.yml::hotpath-scan` (line 549), `runInCI: true`.

### 3.6 Pattern summary the new gate MUST follow

To be consistent with the existing residue-gate family, the new gate should:

1. Be a `bash` wrapper at `scripts/forbid-runtime-fmt.sh` that compiles a
   `scripts/forbid-runtime-fmt.rs` scanner with `rustc` (or
   `clippy-driver` if the scanner needs hard-deny lint enforcement, but
   the sibling `check-removed-feature-residue` uses `rustc`, so `rustc`
   is the lightest precedent).
2. The scanner reports `RUNTIME-FMT:` (or similar) file:line findings and
   exits 0 if active findings == 0.
3. Ship a self-test at `scripts/test-forbid-runtime-fmt.sh` with positive
   and negative fixtures and a real-repo exit-0 assertion, modeled on
   `scripts/test-check-removed-crate-residue.sh`.
4. Ship fixtures at `fixtures/forbid-runtime-fmt/{positive.rs, negative.rs,
   negative_unbounded_channel.rs}`.
5. Be wired as a moon task in a new `forbid-runtime-fmt.yml` (per the bead
   description; the existing gates live in `all.yml` but the bead
   description explicitly names `forbid-runtime-fmt.yml` so a new file
   is the expected layout — or, alternatively, a new top-level entry
   inside `all.yml` since `.moon.yml` only `includes:` `tasks/all.yml`,
   `flux.yml`, `kani.yml`, `loom.yml`, `verus.yml`, `tlc.yml`). The bead
   description's path is canonical; the implementation may either
   follow it literally or add a new entry to `all.yml` and document the
   divergence in the implementation report.
6. Be added as a `deps:` of the existing `check` task
   (mirror `hot-cold-forbidden-apis` at line 117) so the gate runs before
   `cargo check` and the moon pipeline ordering is correct.

## 4. Existing Moon Task Patterns in `.moon/tasks/all.yml`

The closest siblings and the lines to model the new task on:

- `.moon/tasks/all.yml::check-removed-crate-residue` (line 255-265):
  ```yaml
  check-removed-crate-residue:
    command: 'bash scripts/check-removed-crate-residue.sh'
    toolchains:
      - rust
    inputs:
      - 'scripts/check-removed-crate-residue.sh'
      - 'scripts/check-removed-crate-residue.rs'
      - 'fixtures/removed-crate-residue/**/*'
      - '.moon/tasks/all.yml'
    options:
      runInCI: true
  ```
- `.moon/tasks/all.yml::check-removed-feature-residue` (line 267-277):
  identical shape, fixture dir is `fixtures/removed-feature-residue/**/*`.
- `.moon/tasks/all.yml::hot-cold-forbidden-apis` (line 633-645):
  `command: 'bash scripts/check-hot-cold-forbidden-apis.sh'`, inputs
  include the four hot crate paths as `crates/vb_core/src/**/*` etc.
  Listed as a `deps:` of `check` (line 117).
- `.moon/tasks/all.yml::check` (line 105-130) has the canonical
  dependency list (lines 113-121) that the new gate must be added to,
  ordered to run before `cargo check --quiet --workspace --all-targets
  --all-features` (line 111).

The new gate should be inserted into the `check` task's `deps:` array
with the other residue/forbidden gates. The bead description's test name
`test_moon_ci_quarantine_dependency_correctly_ordered` implies a test
that asserts the gate appears in `check`'s `deps:` list *before* the
heavier compile gates (so it can fail fast on a one-line violation),
matching the existing `hot-cold-forbidden-apis` placement at line 117.

`.moon.yml` (the top-level config, line 1-7) does `includes:` the
following task files: `all.yml`, `flux.yml`, `kani.yml`, `loom.yml`,
`verus.yml`, `tlc.yml`. A new `.moon/tasks/forbid-runtime-fmt.yml` would
NOT be picked up by `moon` unless `.moon.yml` is updated to include it.
This is a discoverable risk: the bead description's path is canonical,
but a literal implementation must update `.moon.yml` to add the include,
or the implementation must place the new task in `all.yml` to keep
moon happy. The implementation must decide one path and document it.
(Marked as an open question for State 3+ implementation ownership.)

## 5. Master §43 Trigger Table — Triggers 7-10 Cross-Reference

Master §43 "AI Agent Acceptance Contract" (lines 2027-2065) defines a
14-item reporting list (1-14) and an "Automatic rejection triggers"
block. The bead description says "Master §2 + §12 + §43 triggers 7,8,9,10".
These four triggers are the §43 reporting list items 7-10:

- Trigger 7: "Allocation behavior."
- Trigger 8: "Hot-path behavior."
- Trigger 9: "Fjall persistence behavior if touched."
- Trigger 10: "IPC behavior if touched."

(Verified at `velvet-ballistics-MASTER.md` lines 2038-2041.)

The new gate does not directly check Fjall or IPC implementation code
(trigger 9 and 10 are about reporting *behaviors* of the change, not
about banning transitive deps). The gate's role relative to §43 is to
make *future* PRs mechanically fail-fast if their allocation / hot-path
behavior regresses by re-introducing `serde_json`, `serde_yaml`, `hyper`,
`reqwest`, `axum`, `HashMap<String, _>`, or `tokio::sync::mpsc::unbounded`
in the four hot crates — all of which are §2 "Non-Negotiable Rust Rules"
violations (lines 82-104) and §12 "Forbidden Hot-Path APIs" entries
(lines 405-439). The new gate therefore closes the mechanical enforcement
loop on §2 and §12, which are themselves the upstream constraints for
§43 triggers 7 and 8.

The §43 automatic-rejection list (lines 2051-2065) is the second piece
of §43 relevant to the gate: "JSON inserted into runtime core", "HTTP
inserted into runtime core", "HashMap<String, Value> runtime state",
"unbounded queue/loop/retry/fanout" are all items the new gate is
designed to mechanically catch.

The master document also references `scripts/forbid-runtime-fmt.sh` at
line 6147 as a Tier A acceptance-criteria gate (in the §78 Tier A
forbidden-construct audit gates list at lines 6136-6149). This is the
*same* script the bead is asking to install — confirming the bead is
filling a pre-existing master spec gap, not introducing a new spec.

## 6. Test Directory Layout for the New Gate

The repo uses two distinct test patterns for residue gates:

1. **Bash self-test scripts** under `scripts/test-check-*.sh` (or
   `scripts/test-forbid-runtime-fmt.sh` for the new gate). These are
   not `cargo test` targets; they are shell scripts invoked from the
   moon task `command:` line. They drive the bash wrapper against
   positive/negative/real-repo fixtures and assert exit codes and
   diagnostic content.

2. **Rust integration tests** under `crates/workspace_tests/tests/`
   (e.g., `cli_matrix_conformance.rs`, `proptest_compile_error_codes.rs`).
   These are `cargo nextest` targets and are wired via `[[test]]` entries
   in `crates/workspace_tests/Cargo.toml`. The new gate's behavior tests
   use *path 1* (bash self-test) because the gate is a bash + rustc
   pipeline, not a library function. The three tests in the bead
   description — `test_quarantine_gate_blocks_json_import`,
   `test_quarantine_gate_blocks_unbounded_channel`,
   `test_moon_ci_quarantine_dependency_correctly_ordered` — map to:
   - The first two are bash assertions that the scanner exits 1 with
     a file:line finding when run against a fixture containing
     `use serde_json;` and `tokio::sync::mpsc::unbounded_channel()`
     respectively.
   - The third is a bash assertion that `.moon/tasks/all.yml` (or
     `.moon/tasks/forbid-runtime-fmt.yml` + `.moon.yml`) has the gate
     declared as a `deps:` of the `check` task before the heavier
     compile gates, and that the script path resolves.

No new `[[test]]` entry in `crates/workspace_tests/Cargo.toml` is
required for the bead.

## 7. Unknowns / Open Questions for Downstream Owners

- `OQ-001` — Should the new moon task live in a new
  `.moon/tasks/forbid-runtime-fmt.yml` (matching the bead description
  literally) or be added as a new entry inside the existing
  `.moon/tasks/all.yml`? The bead description says the former, but
  `.moon.yml` does not currently include a `forbid-runtime-fmt.yml` file
  and would need a `.moon.yml` include line. Marked
  `PICK_LITERAL_PATH` — owner is the holzman-rust / State 11
  implementation agent; the rust-contract and proof-planner agents
  do not need to decide this. The decision affects the third test
  (`test_moon_ci_quarantine_dependency_correctly_ordered`).
- `OQ-002` — Should the scanner use `rustc` (light precedent:
  `check-removed-feature-residue`) or `clippy-driver` (heavy precedent:
  `check-removed-crate-residue`)? Both patterns exist; the new gate
  does not have `unsafe` or `unwrap`-style hard-deny concerns, so
  `rustc` is sufficient and matches the lighter sibling.
- `OQ-003` — Should the scanner be path-scoped to the four hot
  crates only (matching `hot-cold-forbidden-apis`) or also report on
  the whole repo? The bead description says "in
  `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/**/*.rs`" so the
  scope is the four crates, not the full repo. Other crates that
  legitimately use `serde_json` (vb_cli, vb_benchmark,
  vb_boundary_inventory, workspace_tests) are out of scope and
  must NOT be flagged.
- `OQ-004` — Should the scanner also check the four
  `crates/*/Cargo.toml` `[dependencies]` and `[dev-dependencies]`
  tables for the banned crate names (`serde_json`, `serde_yaml`,
  `hyper`, `reqwest`, `axum`, `tokio`), or is it source-pattern only?
  The bead description says "Fails on
  serde_json|serde_yaml|hyper|reqwest|axum|HashMap<String,_>|tokio::sync::mpsc::unbounded
  in `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/**/*.rs`" — the
  trailing `/*.rs` qualifier implies source-pattern only. The
  `Cargo.toml` dep check is *out of scope* for the scanner proper
  but could be a separate gate. Implementation should follow the
  bead description literally (source-only).
- `OQ-005` — `vb_core/Cargo.toml` already declares `serde_json.workspace = true`
  as a dev-dependency (line 22) and `vb_core/tests/` uses
  `serde_json::to_string` and `serde_json::from_str` heavily (e.g.,
  `tests/proptest_serde_roundtrip.rs`, `src/action/tests.rs`,
  `src/diagnostic/tests_and_verification.rs`). The scanner must
  distinguish test code (allowed) from production code (forbidden).
  The sibling `check-hot-cold-forbidden-apis.rs` has a `COLD_MARKERS`
  list (line 7-23: `"diagnostic"`, `"fixture"`, `"kani"`, `"loom"`,
  `"proof"`, `"proptest"`, `"test_util"`, `"tests"`, `"verification"`)
  that exempts paths containing these markers. The new scanner
  should follow the same convention.
- `OQ-006` — `Cargo.lock` is in the workspace root and contains
  transitive entries for `serde_json` and `serde_yaml`. The gate
  scans `**/*.rs` only, so `Cargo.lock` is not in scope. Marked
  `EXCLUDED` for the implementation agent.

## 8. Risk Tags

- `residue-quarantine` — the gate's primary identity.
- `hot-crate-gate` — the gate targets only the four hot crates.
- `ci-gate-installation` — the gate is a moon ci wiring change.
- `master-§2-§12-§43-linkage` — the gate is grounded in three master
  sections; any drift between the gate and the master text is a
  blocker.
- `sibling-pattern-conformance` — the gate must follow the existing
  residue-gate conventions (bash + rustc + self-test + fixtures +
  moon task + deps: hookup). Drift is a maintenance risk.

## 9. Required Verifier Lanes

`[]` — no formal-verifier, Kani, Flux, Loom, Miri, proptest, fuzz, or
cargo-mutants lane is required. The gate is a pure shell + rustc
string-match scanner. It is *adjacent* to the verifier tooling (it
protects the verification surface) but is not itself a verifier
artifact. This is consistent with the `global-readiness-report.md`
finding that this bead is a `touched_production_rust: false`,
`new_production_rust: false`, `new_proof_obligations: false` CI-gate
installation only.

## 10. Downstream Owner Recommendations

- **rust-contract (State 3)**: not strictly required (bead is
  ci-gate-installation only). The contract for the new scanner
  belongs in the implementation report, not in
  `domain-model.md`/`type-contracts.md`/etc.
- **proof-planner (State 4)**: not required. The gate is a
  string-match scanner, not a proof obligation.
- **test-planner / test-writer / test-reviewer (States 8-10)**: the
  three test names from the bead description map to the bash
  self-test pattern. The test-writer will write
  `scripts/test-forbid-runtime-fmt.sh` and the three positive/
  negative fixtures.
- **holzman-rust (State 11)**: owns the scanner implementation
  (`scripts/forbid-runtime-fmt.rs` + `scripts/forbid-runtime-fmt.sh`),
  the moon task wiring, and the `.moon.yml` include if a separate
  `forbid-runtime-fmt.yml` is created.
- **formal-verifier (State 12)**: not applicable. No
  machine-gate-report.md, no refinement-verification-report.md,
  no formal-verification-report.md, no verification-ledger.jsonl
  are required for this bead.
- **black-hat-reviewer (State 13)**: the bead produces a
  `black-hat-review.md STATUS: APPROVED` that confirms the gate
  is wired, the three tests pass, and the scanner does not
  false-positive on legitimate test code (`#[cfg(test)]` modules,
  `dev-dependencies`, `Cargo.lock`).
- **evidence-packaging (State 14)**: produces
  `assurance-bundle.md`, `truth-serum-report.md`,
  `final-evidence-decision.md STATUS: APPROVED` mapping each
  master §2 / §12 / §43 trigger to a piece of evidence (scanner
  test output, moon-ci run output, scanner source link).
- **landing-skill (State 15)**: standard landing-report.md
  proving main integration, remote reachability, bead close/sync.

## 11. Excluded Paths (Out of Scope for the Bead)

- All `crates/workspace_tests/**` — these are workspace-harness
  tests, not part of the four hot crates.
- All `vb_cli`, `vb_benchmark`, `vb_boundary_inventory`,
  `vb_validate`, `vb_compile`, `vb_doc`, `vb_expr`, `vb_yaml`,
  `vb_queue_semantics`, `vb_proof_kernels`, `vb_test_util`,
  `vb_ajc40_flux`, `vb_verification` — not in the four hot
  crates, and not in the bead scope. These crates are *cold*
  crates that legitimately use `serde_json` and similar
  codecs; the gate must not false-positive on them.
- `Cargo.lock` — out of scope per `OQ-006`.
- `.beads/**`, `bd/**`, `docs/**`, `evidence/**`,
  `verification/**`, `verification/tla/**`,
  `verification/lean/**`, `proofs/**`, `kani/**`,
  `kani-target/**`, `fuzz/**`, `xtask/**`,
  `transcripts/**`, `to-fix/**`, `supply-chain/**`,
  `arch-drift-reports/**`, `arch-drift-reports/**`,
  `arch-drift-reports/**`, `reference/**`, `specs/**`,
  `design/**`, `contracts/**`, `schemas/**`,
  `tests/**`, `config.yaml` — none are scanned by the
  gate. The gate's input glob is the four `crates/*/src/**/*.rs`
  paths.
