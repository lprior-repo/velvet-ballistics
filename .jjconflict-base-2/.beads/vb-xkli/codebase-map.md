# Codebase Map — vb-xkli

STATUS: APPROVED

Relevant Kani surfaces found under:

- `scripts/rust-verification-gauntlet.sh` scripted proof lane.
- `crates/vb_compile` Kani harnesses for bytecode overflow, slot references, constants, accessors, and node ordering.
- `crates/vb_runtime` Kani harnesses for strict admission malformed artifacts, capability rejection, and valid admission.
- Additional historical harness files exist under `kani/`, `verification/kani/`, and crate-local `kani_*` modules, but the executable P0 gate is the proof gauntlet.
