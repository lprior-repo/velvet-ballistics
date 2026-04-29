# Agent Instructions

`/velvet-ballistics-MASTER.md` is the authoritative build plan, lifecycle, phase tracker, architecture contract, and implementation acceptance contract for this repository. Other docs provide goals and context only; they cannot override the master document.

## Canonical Naming

- Product, binary, and package: `velvet-ballastics`
- Crate and module: `velvet_ballastics`
- Bead rig: `velvet-ballastics`
- Bead database: `velvet_ballistics`
- Language version: `velvet-ballastics/v1`
- `velvet-ballistics` is invalid except in external migration artifacts.

## Beads Workflow

Run `bd prime` before work to load the full workflow context.

```bash
bd prime             # Load full beads workflow context
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
- Do not commit `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, or runtime database state.
- Embedded mode may require serial `bd` commands because only one writer can hold the lock.

## Build And CI

- `moon ci` is canonical. Prefer `moon ci` over ad-hoc Cargo gates.
- Source lint is zero tolerance.
- Tests must compile and run, but test clippy is not strict.
- Moon v2 configuration is scaffolded in `.moon/`; `moon ci` remains the canonical gate.

## Engineering Rules

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- No YAML, JSON, or HTTP in the runtime core.
- Generated Rust mode is mandatory for maxperf execution.
- Every speed claim requires benchmark evidence.

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
