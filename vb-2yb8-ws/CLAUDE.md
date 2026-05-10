# Agent Contract

`/velvet-ballistics-MASTER.md` is the authoritative build plan, lifecycle, phase tracker, architecture contract, and implementation acceptance contract for this repository. Other docs provide goals and context only; they cannot override the master document.

## Canonical Naming

- Product, binary, and package: `velvet-ballastics`
- Crate and module: `velvet_ballastics`
- Bead rig: `velvet-ballastics`
- Bead database: `velvet_ballistics`
- Language version: `velvet-ballastics/v1`
- `velvet-ballistics` is invalid except in external migration artifacts.

## Agent Workflow

- Run `bd prime` before work to load the beads workflow.
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
