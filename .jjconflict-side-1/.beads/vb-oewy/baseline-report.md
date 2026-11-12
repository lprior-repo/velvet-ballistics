---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 1
updated_at: 2026-05-20T00:00:00Z
attempt: 1
---

# Baseline Report — vb-oewy

## Purpose

This baseline captures the pre-edit state of the repository to detect regressions and global blockers during bead execution. It is not an excuse for existing failures.

## Repository State

| Field | Value |
|-------|-------|
| repo_path | /home/lewis/src/velvet-ballistics |
| current_HEAD | 374c4f7b (origin/main, main) |
| HEAD_message | chore(vb-c1s0): add bead artifacts after force-close |
| branch | main |

## Workspace Structure

```
/home/lewis/src/velvet-ballistics/
├── crates/
│   ├── workspace_tests/     # Cross-crate integration tests and benchmarks
│   ├── vb_core/             # Core runtime
│   ├── vb_compile/          # Compiler
│   ├── vb_runtime/          # Runtime
│   ├── vb_storage/          # Storage (Fjall)
│   ├── vb_cli/              # CLI
│   ├── vb_codegen/          # Code generation
│   └── ...
├── fuzz/                   # Fuzzing targets
├── xtask/                  # Automation
└── .beads/                 # Bead metadata and artifacts
```

## Key Files for BDD Suite

- `crates/workspace_tests/tests/` — Integration tests
- `crates/vb_cli/tests/` — CLI tests
- `crates/vb_compile/tests/` — Compiler tests
- `crates/vb_runtime/src/` — Runtime source
- `crates/vb_storage/src/` — Storage source

## Existing BDD/Test Infrastructure

The repository has existing BDD test infrastructure that vb-oewy will extend or integrate with.

## Bead Dependencies

vb-oewy has open dependencies that are blocking it:
- vb-0sps (closed)
- vb-6o73 (closed)
- vb-82ah (closed)
- vb-c1s0 (closed)
- vb-e4mt (closed)
- vb-fwhp (unknown status)
- vb-hjvq (parent - status unknown)
- vb-hs9m (status unknown)
- vb-kyyf (status unknown)
- vb-lp2v (status unknown)
- vb-m214 (status unknown)
- vb-njju (status unknown)
- vb-rpch (closed)
- vb-te1i (closed)

## Git Status (Source Checkout)

```
$ git status
(Will be captured from isolated workspace referencing source checkout)
```

## Baseline Evidence

Captured from jj workspace `go-skill-vb-oewy` at `/home/lewis/src/vb-oewy-workspace`.

## Notes

- This baseline is captured before any bead implementation work begins
- Any pre-existing failures are classified as BLOCK_GLOBAL and must be resolved before bead advancement
- The BDD suite runner and evidence artifact contract bead (vb-oewy) is meant to provide a full suite runner and evidence artifact contract for BDD scenarios
