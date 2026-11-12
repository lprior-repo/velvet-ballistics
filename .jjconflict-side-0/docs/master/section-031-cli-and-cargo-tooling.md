---
section: 31
title: "CLI and Cargo Tooling"
parent: velvet-ballistics-MASTER.md
---

## 31. CLI and Cargo Tooling

There are two command surfaces:

```text
cargo velvet       compiler/developer tool
velvet-ballistics runtime/operator tool
```

### Compiler commands

```text
cargo velvet verify --package <workflow-crate> [--workflow <name>] [--profile <profile>] [--emit json|postcard]
cargo velvet explain --workflow <name> [--emit json|postcard]
cargo velvet graph --workflow <name> [--emit json|postcard]
cargo velvet simulate --workflow <name> --input-bin <file> --mocks <file> [--emit json|postcard]
cargo velvet artifact --workflow <name> --out <file.vbir>
cargo velvet lsp
cargo velvet agent-context [--emit json]
```

### Runtime commands

```text
velvet-ballistics ipc-serve --socket <path> --db <path>
velvet-ballistics install <artifact.vbir> --db <path> [--emit json|postcard]
velvet-ballistics submit <artifact-digest> --input-bin <file> --db <path> [--durability <profile>] [--emit json|postcard]
velvet-ballistics inspect <run-id> --db <path> [--emit json|postcard]
velvet-ballistics events <run-id> --db <path> --limit <n> [--cursor <cursor>] [--emit json|postcard]
velvet-ballistics replay <run-id> --db <path> [--emit json|postcard]
velvet-ballistics incident <run-id> --db <path> [--emit json|postcard]
velvet-ballistics cancel <run-id> --db <path> [--emit json|postcard]
velvet-ballistics answer <run-id> --ask <ask-id> --value-bin <file> --db <path> [--emit json|postcard]
velvet-ballistics action list --emit json
velvet-ballistics action inspect <action> --emit json
velvet-ballistics system status --emit json
velvet-ballistics doctor --db <path> --emit json
```

`--emit json` is cold-path operator/agent output and must not enter runtime core. Binary machine artifacts use `--emit postcard`.

---

