# VB-J3E3S Stage 1 Repair Handoff: Sound Benchmark Evidence Freshness

## Status
Source repair complete. All targeted gates pass (40 benchmark tests, strict clippy,
fmt, source-length). Ready for root freeze + independent review. No Stage 2 measurements.

## What was wrong (draft09ef93e9)
`validate_evidence_freshness` compared only two commits (`git diff <older> <newer>`).
It never compared the measured commit against the actual working tree, so a dirty or
staged working tree with `evidence.commit == HEAD` passed. The only "dirty tree" test
asserted on `git diff --name-only HEAD` instead of on the validator.

## What changed (source)
Split the 1152-line `xtask/src/benchmark_policy.rs` into three focused modules:

- `xtask/src/benchmark_budget.rs` (116 ln) — `PerfBudget`/`BenchmarkBudget`, `parse_budget`,
  `validate_budget`, `normalized`/`normalized_set`/`has_deferred_marker`.
- `xtask/src/benchmark_freshness.rs` (238 ln) — the sound working-tree identity check plus
  commit resolution and `current_head_commit`.
- `xtask/src/benchmark_policy.rs` (277 ln) — evidence-claim validation (fields/mode/commit/
  raw-log/threshold), CLI entry `cmd_benchmark_policy`.

### Soundness fix (`benchmark_freshness.rs`)
1. `resolve_commit` resolves via `git rev-parse --verify <commit>^{commit}` and requires a
   full 40-char hex SHA. Rejects missing objects, blobs/trees, tags-to-blob/tree, and
   ambiguous prefixes.
2. Ancestor check via `git merge-base --is-ancestor` (rejects unrelated commits; error vs
   not-ancestor distinguished).
3. Working-tree identity: `git diff --no-renames -z --name-only <measured_commit>` compares
   the measured commit tree directly against the actual working tree (staged + unstaged),
   always — even when `HEAD == measured commit`. `--no-renames` prevents rename detection
   from hiding a move-out. `-z` yields NUL-separated raw paths; `String::from_utf8` is
   fail-closed (non-UTF-8 path => error, never lossy `.lines()`).
4. Untracked inputs: `git ls-files --others -z` WITHOUT `--exclude-standard`, so gitignored
   relevant inputs (`.cargo/config`, ignored `include!`d Rust) still invalidate evidence.
   Only build-output/VCS/cache dirs (`target`, `.git`, `.jj`, `.moon/cache`) are excluded
   via explicit `--exclude`. Pathspecs are limited to the relevant roots.
5. Relevant roots (conservative include): `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
   `rust-toolchain`, `.cargo/`, `crates/`, `xtask/`, `contracts/perf-budget.yaml`,
   `fixtures/`, `scripts/`, `.moon/`. Only unrelated roots (evidence/docs/.beads/reports/
   specs/schemas/design/reference/fuzz/verification) are excluded.

### Benchmark metadata neutralization
All 13 `crates/workspace_tests/benches/*.rs` had `BENCH_METADATA` containing the forbidden
markers `allocations=allocator-external` and `instructions=not-collected` (which real
Criterion raw stdout would emit and the gate rejects). Replaced only those two fields with
`measurement=criterion-timing;external-counters=separate-evidence`. Metadata-only; no
workload/code changes. Forbidden-marker CHECK in `benchmark_policy` is unchanged.

## Tests
- Unit: `xtask/src/benchmark_freshness_tests.rs` (292 ln) — 23 git-fixture tests; shared
  fixture helper moved to `xtask/src/benchmark_tests_support.rs` (96 ln, unique tempdir per
  test via `tempfile`, all helpers `Result`, no new `.expect()`/`.unwrap()`/`.panic!`).
  `xtask/src/benchmark_policy_tests.rs` (114 ln) — 4 pre-existing unit tests retained + 2 new
  exact-context tests.
- Public CLI: `xtask/tests/benchmark_policy_cli.rs` (271 ln) — 5 tests invoke the built
  `xtask` binary with `current_dir(fixture)` (no `--workspace-root` flag added; legacy CLI
  uses cwd, so child cwd achieves the public flow without global cwd mutation).

### Acceptance matrix (all actual validator cases, exact context asserted)
- clean same-HEAD passes; evidence-only descendant passes; touched-then-reverted passes.
- changed source (committed / staged / dirty same-HEAD) rejects; Cargo.lock, rust-toolchain,
  bench fixture (`fixtures/`), perf-budget, xtask policy, script mutation all reject.
- untracked relevant Rust/config rejects (incl. gitignored `.cargo/config.local.toml`);
  gitignored build output (`target/`) not flagged; unrelated dirty/untracked (`docs/`) passes.
- unrelated/non-ancestor, missing, and blob (non-commit) hashes reject; unique prefix resolves.

## Gate evidence (`.beads/vb-j3e3s/evidence/`)
- `final-test-benchmark` — `cargo test --manifest-path xtask/Cargo.toml benchmark`:
  40 passed (6 benchmark-metadata gate + 29 unit + 5 CLI), 0 failed.
- `final-clippy` — `cargo clippy --manifest-path xtask/Cargo.toml --lib --bins --all-features -- -D warnings`: exit 0.
- `final-fmt-check` — `cargo fmt --manifest-path xtask/Cargo.toml --check`: exit 0.
- source-length gate (`bash scripts/check-source-length.sh`): exit 0, `over_limit=0` all categories.

## Notes / caveats
- `xtask/src/benchmark_policy.rs` is now 277 ln (< 300); the pre-existing source-length
  exception row for it (vb-jpq7.47) is retained but is now a non-fatal stale WARN — root may
  remove it. No exception rows added.
- Running the FULL xtask test suite (`cargo test --manifest-path xtask/Cargo.toml`) has two
  pre-existing environment caveats, unrelated to this bead:
  1. `xtask/tests/integration_gates.rs` fails (4 tests) because it invokes `cargo xtask`
     (`cargo run -p xtask`), and xtask is excluded from the root workspace. My CLI tests use
     `CARGO_BIN_EXE_xtask` instead.
  2. Its cleanup test deletes the gitignored-but-tracked fixture
     `.evidence/vb-itest-fast/ai-fast.yaml`; I restored it byte-identical to HEAD
     (`git status` clean), but JJ's gitignore-filtered working-copy snapshot still reports it
     as `D`. Root to resolve during freeze (file is on disk, matches HEAD).
- No VCS commit/bookmark/push/close performed (root owns VCS). No checker/TSV edits. No
  benchmark measurements (Stage 2 runs after root freeze).
