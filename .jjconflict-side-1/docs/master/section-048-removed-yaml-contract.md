---
section: 48
title: "Removed YAML Contract"
parent: velvet-ballistics-MASTER.md
---

## 48. Removed YAML Contract

YAML workflow authoring is removed.

Active workflow commands must not accept `.yaml` or `.yml` workflow source.

Removed or quarantined:

```text
YAML parser crate
YAML validator crate
YAML source maps
YAML examples
YAML fixtures
YAML fuzz targets
validate <workflow.yaml>
compile <workflow.yaml>
simulate <workflow.yaml>
run <workflow.yaml>
run-compiled from loose YAML compile path
```

Allowed legacy support only:

```text
cargo velvet migrate-yaml <legacy.yaml> --out workflows/<name>.rs
```

Migration output must be reviewed and verified through the SDK compiler before artifact emission.

---

