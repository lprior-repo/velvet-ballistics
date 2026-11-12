# Contract Specification - vb-j3e3s

## Context
- **Feature**: Sound benchmark evidence staleness policy (repair unsound draft09ef93e9)
- **Domain terms**: measured commit, current HEAD, working tree, staged change, untracked input, perf-relevant path
- **Assumptions**: git is available; evidence commit resolves to a commit object in the repo; working tree is the authoritative "what was actually measured" identity
- **Open questions**: none — Stage 2 measurement collection runs after this repair lands

## Problem
Prior `validate_evidence_freshness` compared only two commits via `git diff <older> <newer>`.
It never compared against the actual working tree, so a dirty working tree (or staged
change) with `evidence.commit == HEAD` passed, and the only "dirty tree" test asserted on
`git diff --name-only HEAD` rather than on the validator.

## Postconditions
- POST-001: `evidence.commit` resolves to an actual commit object via `<commit>^{commit}`; tags-to-blob/tree, blobs, trees, and missing/ambiguous refs are rejected.
- POST-002: resolved commit must be an ancestor of current HEAD, else rejected (unrelated/noncommit/missing).
- POST-003: The measured commit's tree is compared directly against the actual working tree (staged + unstaged) via `git diff --no-renames -z --name-only <commit>`; any perf-relevant differing path is rejected even when `HEAD == measured commit`.
- POST-004: Untracked perf-relevant files (Rust/config under relevant roots) are rejected.
- POST-005: Evidence-only descendant commits pass when all measured input bytes are unchanged; touched-then-reverted source passes because bytes match.

## Invariants
- INV-001: Path parsing uses NUL-separated raw paths (fail-closed on non-UTF-8), never lossy `.lines()`.
- INV-002: Relevant-path matching is strict on path components (exact prefix including trailing separator).
- INV-003: `--no-renames` prevents rename detection from hiding a move-out of a relevant path.
- INV-004: Only unrelated roots (`evidence/`, `docs/`, `.beads/`, reports, specs, schemas, design, reference, fuzz, verification, etc.) are excluded; source, manifests, tests, benches, xtask, fixtures, scripts, and root build files are conservatively included.

## Perf-Relevant Paths (conservative include)
`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/`, `crates/`, `xtask/`,
`contracts/perf-budget.yaml`, `fixtures/`, `scripts/`.

## Error Taxonomy (exact context required in tests)
- `... is not a valid commit: <git stderr>` — resolve failure (missing/non-commit/ambiguous).
- `... is not an ancestor of current HEAD ...` — unrelated commit.
- `... is stale: performance-relevant files differ from measured commit ...: <paths>` — working-tree/staged/dirty divergence.
- `... is stale: untracked performance-relevant files exist: <paths>` — untracked relevant input.

## Contract Signatures
- `benchmark_budget::parse_budget(&str) -> anyhow::Result<PerfBudget>`
- `benchmark_budget::validate_budget(&PerfBudget) -> anyhow::Result<()>`
- `benchmark_freshness::resolve_commit(&Path, &str, usize) -> anyhow::Result<String>`
- `benchmark_freshness::validate_evidence_freshness(&Path, &str, &str, usize) -> anyhow::Result<()>`
- `benchmark_policy::cmd_benchmark_policy(&Path, &str, &str) -> anyhow::Result<()>`

## CLI
- `xtask benchmark-regression-policy --budget <p> --evidence <e> [--workspace-root <dir>]`
- `--workspace-root` defaults to the process working directory (preserves the existing moon task invocation).

## Non-goals
- No Stage 2 measurement collection (runs after root freeze).
- No new benchmark fixture; existing legacy bench must keep compiling.
- No change to existing threshold/metadata fields or forbidden markers.
- No new source-length exception ledger rows; existing `benchmark_policy.rs` exception is retained and not enlarged.
