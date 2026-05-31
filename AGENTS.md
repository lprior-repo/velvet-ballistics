# Agent Instructions

`/velvet-ballistics-MASTER.md` is the authoritative build plan, lifecycle, phase tracker, architecture contract, and implementation acceptance contract for this repository. Other docs provide goals and context only; they cannot override the master document.

## Canonical Naming

- Product, binary, and package: `velvet-ballistics`
- Crate and module: `velvet_ballistics`
- Bead rig: `velvet-ballistics`
- Bead database: `velvet_ballistics`
- Language version: `velvet-ballistics/v1`
- `velvet-ballistics` is invalid except in external migration artifacts.

## Workspace Structure

This repository uses a pure virtual workspace pattern.

## Absolute Workspace Rule

- All repository work MUST be executed from the Git root `/home/lewis/src/velvet-ballistics`; do not work from any sibling clone, parent checkout, or other `~/src/*` path.
- Before editing, committing, pushing, or running broad verification, verify `git rev-parse --show-toplevel` resolves to `/home/lewis/src/velvet-ballistics`.
- If `jj root` resolves above this repository, treat that as an enclosing workspace only; never commit, describe, rebase, or push from the parent JJ workspace for this repo.
- If the Git root, JJ root, command working directory, or requested path disagree, stop and ask for clarification before modifying files.

- `crates/`: Contains all production code crates (e.g., `vb_core`, `vb_boundary_inventory`).
- `crates/workspace_tests/`: Contains all cross-crate integration tests and benchmarks. Do not place `tests/` or `benches/` at the repository root.
- `fuzz/`: Fuzzing targets.
- `xtask/`: Automation and tooling.

Never place production code, tests, or benchmarks at the repository root.

## Beads Workflow



```bash

bd ready             # Find available work
bd show <id>         # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>        # Complete work
bd dolt push         # Push beads data to remote
```

- Use beads for all task tracking. Never use markdown TODOs or separate task lists.
- Create or claim a bead before implementation.
- Close or update beads after completion, including any follow-up work.
- Use `bd remember` for persistent knowledge. Do not create memory files.
- Use the `beads` skill for issue workflow and the `dolt` skill when Dolt or beads storage problems are relevant.

## Beads Dolt Remote

- Active remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`
- Branch: `main`
- Active backend is server mode only: `.beads/metadata.json` must keep `dolt_mode` as `server` and point at the bd-managed Dolt SQL server.
- Never use embedded mode in this repository. `.beads/embeddeddolt/` is a trap directory; if it exists, remove it before running `bd`.
- Run `bash scripts/check-beads-server-mode.sh` if `bd context`, `bd where`, or any `bd dolt` command reports embedded mode.
- Do not commit `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, or runtime database state.

## Build And CI

- `moon ci` is canonical. Prefer `moon ci` over ad-hoc Cargo gates.
- Rust governance is defined in `docs/rust-governance.md`; future agents must preserve the pinned nightly and feature whitelist there.
- Source lint is zero tolerance.
- Tests must compile and run, but test clippy is not strict.
- Moon v2 configuration is scaffolded in `.moon/`; `moon ci` remains the canonical gate.

## Engineering Rules

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- No YAML, JSON, or HTTP in the runtime core.
- Generated Rust mode is mandatory for maxperf execution.
- Every speed claim requires real baseline/result benchmark evidence; compileable Criterion scaffold placeholders are not performance evidence.
- Do not add unstable Rust features outside normal `try_blocks`/`portable_simd` use and perf-only `allocator_api`/`generic_const_exprs`. Perf-only features may appear only in `crates/*/src/perf/**`, `crates/*/src/generated/**`, `benches/**`, or marker-approved files if `scripts/check-nightly-features.sh` implements `velvet-allow-perf-nightly-feature`. Use `moon run :nightly-feature-gate` for first-party feature-scope checking. Use `moon run :nightly-feature-cargo-probe` where transitive dependencies do not require extra nightly internals.

## Formal Verification Mandates (GOD RULES)

**CRITICAL DIRECTIVE:** ALL AI AGENTS AND VERIFICATION HARNESSES MUST OBEY THESE RULES UNDER PENALTY OF REJECTION. YOU MAY NOT "CHEAT" THE MATH.

1. **No Hardcoded Kani Shapes:** Kani verification harnesses MUST NOT hardcode structural inputs (like `WorkflowParts` or `RunFrame`) with fixed dummy data. You MUST implement and use `kani::Arbitrary` for core structures, or write safe, exhaustive generator harnesses using `kani::any()`. Proving that a function doesn't panic on one hardcoded data structure proves nothing.
2. **No Vacuum Verus Proofs:** Verus `proof fn` and `spec fn` models MUST mathematically bind to the actual Rust implementations (`exec fn`) inside the production codebase. You cannot define an enum in `verification/verus/`, prove its properties `by(compute)`, and call it a day. The implementation functions must use `requires` and `ensures` to guarantee they satisfy the model.
3. **No Unbounded TLA+ Math:** TLA+ specifications MUST model the exact bounded hardware limits of the target architecture (e.g., integer overflows at `MAX_U64`). You cannot use unbounded `Nat` to assume away arithmetic failures. The specs must model the `Err` state transitions and prove that the workflow engine gracefully suspends or fails rather than deadlocking or panicking on overflow.
4. **No Loop Oscillations:** Follow strict Proof-Driven Development. If a Kani/Verus harness exposes a flaw in the implementation, **FIX THE IMPLEMENTATION**. You are strictly forbidden from altering the mathematical contract or proof harness just to make the test turn green.
5. **No Blind Verification Mutations:** Differential verification only. Do not blindly trigger `cargo-mutants` or `kani` across the entire fleet for simple changes. Trim your verification scope to the call-graph blast radius of your specific bead to avoid melting the CI cluster.

## Verifier Tooling Runbook

- **Verus:** Use `bash scripts/verify-verus.sh` for registry-driven obligations. For one-off checks use `verus --crate-type=lib <file>` from the documented workdir. A standalone Verus model pass is not proof closure unless it is explicitly bound to production Rust behavior and cited in the proof/test/source bridge.
- **Flux:** Use `bash scripts/flux-check-package.sh <package>` or `cargo flux -p <package> --message-format human`. The installed `cargo-flux` does not accept `--lib` or `--test`; do not write obligations with those flags. A package-level Flux pass is only a crate smoke check unless the named `verification/flux/*.rs` artifact is wired into that crate or checked by an explicit single-file Flux command.
- **Kani inventory:** Do not run root `cargo kani list --format json` as proof evidence. Use `bash scripts/kani-list.sh <package> [...]` so package directories are selected explicitly and generated `kani-list.json` files are moved under `.evidence/kani-list/`. Use `KANI_FEATURES=<features>` for feature-gated harness groups.
- **Kani harness isolation:** Bulky or stale harness groups must be behind package features, for example `vb_core/kani-diagnostic-codes` and `vb_runtime/kani-shard-command-queue`. Do not make every `#[cfg(kani)]` module compile for unrelated package lanes.
- **Tooling evidence is not proof evidence:** Verus/Flux/Kani version checks, list/codegen, and package smoke commands prove setup health only. They do not close behavior-affecting proof obligations without the approved proof artifacts and raw verifier success logs.

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**

```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r directory
```

**Other commands that may prompt:**

- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists

- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
