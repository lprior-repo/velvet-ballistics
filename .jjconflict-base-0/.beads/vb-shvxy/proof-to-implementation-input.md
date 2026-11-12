# Proof-to-Implementation Bridge Input: vb-shvxy

## Purpose

This document provides the input needed by `proof-to-implementation` to map proof claims to Rust source, test, and harness obligations. Since this is a tooling infrastructure bead, the "implementation" targets are scripts, configuration files, and verifier wrapper infrastructure — not production Rust behavior.

## Proof Obligation → Implementation Mapping

### Kani Lane (PO-001, PO-002, PO-003)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-001 | `scripts/kani-list.sh` | `scripts/kani-list.sh:1-66` | Script (already exists) |
| PO-002 | `scripts/kani-list.sh` | `scripts/kani-list.sh:1-66` | Script (already exists) |
| PO-003 | Feature gate validation in kani-list.sh | `scripts/kani-list.sh:53-55` (KANI_FEATURES passthrough) | Script + `crates/vb_runtime/Cargo.toml` features |

**Open decisions for bridge:**
- Should `scripts/kani-list.sh` be extended with `--harness` pass-through for execution evidence, or remain inventory-only?
- Should missing feature `vb_runtime/kani-artifact-version-barrier` be restored in `crates/vb_runtime/Cargo.toml`, or should PO-003 use an existing declared feature?
- Should `.moon/tasks/kani.yml` be updated to invoke `scripts/kani-list.sh` instead of direct `cargo kani`?

### Flux-rs Lane (PO-004, PO-005)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-004 | `scripts/flux-check-package.sh` | `scripts/flux-check-package.sh:1-21` | Script (already exists) |
| PO-005 | Unsupported selector guard | `scripts/flux-check-package.sh:12-18` | Script (already exists) |

**Open decisions for bridge:**
- Should a Flux proof registry (analogous to `contracts/proof_obligations.yaml` for Verus) be created to enumerate required Flux artifacts per crate?
- Package-level Flux pass is setup smoke only; how will named Flux artifact wiring be documented for downstream behavior obligations?

### Proptest Lane (PO-006, PO-007)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-006 | `scripts/guard-zero-tests.sh` | **TO BE CREATED** | New script |
| PO-007 | Proptest execution with zero-test guard | `scripts/guard-zero-tests.sh` + existing proptest tests | Script + tests |

**Open decisions for bridge:**
- Where should `guard-zero-tests.sh` live? Candidate: `scripts/guard-zero-tests.sh`.
- Should the guard be a standalone script, integrated into `formal-verifier` evidence parser, or added as a cargo-test helper?
- What output parsing strategy? Suggested: grep for `running 0 tests` and exit non-zero; accept only `running [1-9][0-9]* tests`.
- Should the guard also detect `0 passed; 0 failed` patterns?

### cargo-fuzz Lane (PO-008, PO-009)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-008 | `cargo fuzz list` | `fuzz/Cargo.toml` | Existing tool invocation |
| PO-009 | Fuzz build with GNU target | `.moon/tasks/all.yml:452-470`, `.cargo/config.toml` | Config |

**Open decisions for bridge:**
- Should `.cargo/config.toml` set `build.target = "x86_64-unknown-linux-gnu"` to prevent ambient musl default, or should proof commands always specify `--target`?
- Should a wrapper script ensure `--target x86_64-unknown-linux-gnu` is always used for fuzz commands, similar to moon fuzz-smoke?

### Loom Lane (PO-010, PO-011)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-010 | Loom model compilation/execution | `crates/vb_runtime/src/models/loom/`, `crates/vb_runtime/Cargo.toml` | Models + dependency config |
| PO-011 | Loom model listing | `xtask/src/loom.rs:17-26` | Existing xtask |

**Open decisions for bridge:**
- Should `loom` be promoted from dev-dependency to optional dependency (`optional = true` with a feature flag) so that `cfg(loom)` resolves in the library build graph?
- Alternatively, should loom models be restricted to package-level tests only (not exposed in library modules)?
- Should integration tests referencing loom models be rewritten to use library-internal tests or a feature-gated re-export?

### Formal Closure Lane (PO-012)

| Obligation | Implementation Target | Source Ref | Kind |
|-----------|----------------------|-----------|------|
| PO-012 | Evidence classification + non-vacuous enforcement | `formal-verifier` evidence parser (state 10) | Parser logic |

**Open decisions for bridge:**
- The formal-verifier evidence parser must enforce `applicable_count > 0` for behavior-affecting classifications.
- Setup health and inventory evidence must be tagged and excluded from obligation-closing ledger rows.

## Source Files Potentially Affected by Implementation

### Existing Files (may be edited by proof-writer/functional-rust)
- `scripts/kani-list.sh` — may need `--harness` pass-through or improved feature validation
- `scripts/flux-check-package.sh` — already complete; may need documentation
- `crates/vb_runtime/Cargo.toml` — may need feature restoration or loom dependency promotion
- `.cargo/config.toml` — may need default target triple
- `xtask/src/loom.rs` — may need cfg/dependency parity validation

### New Files (to be created by proof-writer)
- `scripts/guard-zero-tests.sh` — fail-closed zero-test detector

### NOT Affected
- No production Rust behavior (per contract non-goal)
- No new verifier harnesses, models, or specs (per contract non-goal)
- TLA+ infrastructure (`tools/`, `scripts/run-tlc-checks.sh`, `verification/tla/`) — globally removed

## Bridge Preconditions

Before `proof-to-implementation` can map these claims:
1. All proof obligations must be planned (done — this document)
2. Proof plan must pass `proof-plan-reviewer` gate (state 5)
3. Open decisions above must be resolved or delegated to implementation owners

## Dependency on Trusted Base

All obligations reference the trusted base plan (`trusted-base-plan.md`):
- TB-001: Verus script pattern → guides new script design
- TB-002: Cargo metadata → validates feature resolution
- TB-003: Xtask loom enumeration → validates model inventory
- TB-004: Prior blocker evidence → context for negative test cases
- TB-005: Moon fuzz-smoke config → canonical target triple
