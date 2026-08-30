# Flux Evidence Status

## MISSING — See VB-CN2ZY Gap Analysis

Flux smoke check execution logs have not been captured for vb-06f0.

The prior waiver claimed "Flux: ✓ Smoke passes all packages" but no
`.evidence/**/*flux*` files were found. This claim is UNSUPPORTED.

## How to Remediate

Run the following for each package and attach raw output:

```bash
bash scripts/flux-check-package.sh vb_core > .evidence/flux/vb_core-smoke.log
bash scripts/flux-check-package.sh vb_runtime > .evidence/flux/vb_runtime-smoke.log
bash scripts/flux-check-package.sh vb_storage > .evidence/flux/vb_storage-smoke.log
bash scripts/flux-check-package.sh vb_validate > .evidence/flux/vb_validate-smoke.log
bash scripts/flux-check-package.sh vb_ui_model > .evidence/flux/vb_ui_model-smoke.log
```

Or use the cargo-flux target directly:

```bash
cargo flux -p vb_core --message-format human > .evidence/flux/vb_core-smoke.log
```
