---
section: 33
title: "CLI Commands"
parent: velvet-ballistics-MASTER.md
---

## 33. CLI Commands


```bash
velvet-ballistics validate <workflow.yaml>
velvet-ballistics compile <workflow.yaml> --emit ir --out <file.vbir>
velvet-ballistics run <workflow.yaml> --input-bin <input.vbin> --durability <mode>
velvet-ballistics run-compiled <workflow.vbir> --input-bin <input.vbin> --durability <mode>
velvet-ballistics ipc-serve --socket <path> --db <path>
velvet-ballistics agent-context
velvet-ballistics inspect <run_id> --db <path>
velvet-ballistics events <run_id> --db <path>
velvet-ballistics replay <run_id> --db <path>
velvet-ballistics graph <workflow.yaml> --emit yaml
velvet-ballistics system status --emit yaml
velvet-ballistics action list --emit yaml
velvet-ballistics action inspect <action-name> --emit yaml
velvet-ballistics incident <run_id> --db <path> --emit yaml
velvet-ballistics ai context <run_id> --db <path> --emit yaml
velvet-ballistics bench-run <workflow.yaml>
velvet-ballistics doctor --db <path>
```

CLI structured output is a cold-path operator/agent contract and never enters `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`. `--emit yaml` is the canonical structured text flag for v1; `--emit postcard` is the canonical binary machine-output flag where supported. JSON may be added later as a separate cold adapter. Runtime machine artifacts remain binary/Postcard.

The `ui` command and Makepad desktop application are removed from the current command surface.

---
