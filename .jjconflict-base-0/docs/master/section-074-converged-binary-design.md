---
section: 74
title: "Converged Binary Design"
parent: velvet-ballistics-MASTER.md
---

## 74. Converged Binary Design


`velvet-ballistics` ships as a single binary that operates in different modes depending on the command invoked. This converged single-binary design is adapted for single-server operation.

### Modes

| Command | Binary Role | Components Active |
|---------|-------------|-------------------|
| `run` | Executor | Compiler + Engine + Storage |
| `run-compiled` | Executor | Engine + Storage |
| `validate` | Validator | YAML Parser + Validator |
| `compile` | Compiler | YAML Parser + Validator + Compiler + IR artifact writer |
| `explain` | Analyzer | YAML Parser + Validator + Compiler |
| `diff` | Analyzer | Compiler + Digest comparison |
| `inspect` | Observer | Storage reader |
| `events` | Observer | Storage reader |
| `trace` | Observer | Storage reader |
| `replay` | Observer | Storage reader + Recovery |
| `ipc-serve` | Server | Engine + Storage + IPC server loop |
| `cancel`/`resume`/`retry`/`answer` | Controller | Storage reader + Engine + Storage writer |
| `bench-run` | Benchmarker | Compiler + Engine + Timer |
| `doctor` | Diagnostics | Storage reader + Health checks |

No mode starts components it doesn't need. The `validate` command never opens Fjall. The `inspect` command never compiles YAML. The `ipc-serve` command is the only mode that runs the full stack persistently.

### Future Extension

If `velvet-ballistics` ever supports distributed operation (v2+), the binary gains additional roles (log-server, controller, ingress) but the converged model persists: a single binary, configured by role, no separate services to deploy.

---
