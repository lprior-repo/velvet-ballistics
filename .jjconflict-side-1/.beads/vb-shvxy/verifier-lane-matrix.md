# Verifier Lane Matrix: vb-shvxy

## Lane Status Overview

| Verifier | Applicability | Script/Wrapper | Tool Available | Script Exists | Non-Vacuous Guard | Blockers |
|----------|--------------|----------------|----------------|---------------|-------------------|----------|
| Kani | required | `scripts/kani-list.sh` | `cargo-kani 0.67.0` | Yes | Inventory ≠ execution | Feature `vb_runtime/kani-artifact-version-barrier` missing |
| Flux-rs | required | `scripts/flux-check-package.sh` | `cargo-flux 4d329f2` | Yes | Unsupported selector rejection | Package-only; Flux artifact wiring needed |
| proptest | required | None (to be created) | `cargo test` | N/A | Zero-test detector needed | `running 0 tests` + exit 0 = vacuous pass |
| cargo-fuzz | required | `.moon/tasks/all.yml` fuzz-smoke | `cargo-fuzz 0.13.1` | N/A | Target registration preflight | Musl+sanitizer incompat; no default target set |
| Loom | required | `xtask/src/loom.rs` | `loom 0.7` (dev-dep) | N/A | cfg/dependency parity | Dev-dep only; integration test fails |
| TLA+ | not_applicable | N/A (globally removed) | N/A | N/A | N/A | Globally removed from repo |
| Miri | not_applicable | N/A | `cargo-miri` | N/A | N/A | No unsafe/UB risk in tooling bead |
| Verus | not_applicable | `scripts/verify-verus.sh` | `verus` | Yes | Registry-driven | Already working (template only) |

## Lane × Proof Seed Coverage

| Proof Seed | Kani | Flux | proptest | fuzz | Loom | TLA |
|-----------|------|------|----------|------|------|-----|
| vb-shvxy-seed-001 (Kani cmd) | ✅ required | — | — | — | — | — |
| vb-shvxy-seed-002 (Flux cmd) | — | ✅ required | — | — | — | — |
| vb-shvxy-seed-003 (TLA jar) | — | — | — | — | — | ❌ not_applicable |
| vb-shvxy-seed-004 (proptest zero) | — | — | ✅ required | — | — | — |
| vb-shvxy-seed-005 (fuzz triple) | — | — | — | ✅ required | — | — |
| vb-shvxy-seed-006 (loom cfg) | — | — | — | — | ✅ required | — |
| vb-shvxy-seed-007 (closure fail-closed) | ✅ required | ✅ required | ✅ required | ✅ required | ✅ required | ❌ not_applicable |

## Lane × Requirement Coverage

| Requirement | Kani | Flux | proptest | fuzz | Loom | TLA |
|------------|------|------|----------|------|------|-----|
| REQ-SHVXY-001 (closed lane identity) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| REQ-SHVXY-002 (missing tool fail-closed) | ✅ | ✅ | — | — | — | ❌ |
| REQ-SHVXY-003 (zero applicable gate) | ✅ | — | ✅ | ✅ | — | — |
| REQ-SHVXY-004 (Kani feature parity) | ✅ | — | — | — | — | — |
| REQ-SHVXY-005 (Flux selector guard) | — | ✅ | — | — | — | — |
| REQ-SHVXY-006 (TLC portability) | — | — | — | — | — | ❌ |
| REQ-SHVXY-007 (fuzz target guard) | — | — | — | ✅ | — | — |
| REQ-SHVXY-008 (loom cfg parity) | — | — | — | — | ✅ | — |
| REQ-SHVXY-009 (prior evidence only) | ✅ | ✅ | ✅ | ✅ | ✅ | — |

## Verus as Trusted-Base Template

The working Verus infrastructure (`scripts/verify-verus.sh`) demonstrates the pattern:
- Registry-driven target enumeration (no silent pass)
- Per-target evidence capture with raw output preservation
- Non-vacuous guard (`required.verus targets` count > 0)
- Trust boundary scan (no unapproved `assume`/`external_body`/`axiom`)
- Summary with version, target count, and per-target PASS/FAIL

Each new lane script should follow this template: availability check → target enumeration → per-target execution → evidence capture → non-vacuous guard → summary.
