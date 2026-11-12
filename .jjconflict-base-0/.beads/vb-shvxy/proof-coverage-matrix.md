# Proof Coverage Matrix: vb-shvxy

## Proof Seed → Obligation Mapping

| Proof Seed ID | Requirement ID | Contract Clause | Verifier | Obligation IDs |
|--------------|---------------|-----------------|----------|----------------|
| vb-shvxy-seed-001 | REQ-kani-command-availability | contract.md#kani-command; codebase-map.md | kani | PO-001, PO-002, PO-003 |
| vb-shvxy-seed-002 | REQ-flux-command-availability | contract.md#flux-command; codebase-map.md | flux-rs | PO-004, PO-005 |
| vb-shvxy-seed-003 | REQ-tla-jar-availability | contract.md#tla-command; codebase-map.md | — | NOT APPLICABLE (TLA+ globally removed) |
| vb-shvxy-seed-004 | REQ-proptest-zero-test-gate | contract.md#proptest; codebase-map.md | proptest | PO-006, PO-007 |
| vb-shvxy-seed-005 | REQ-fuzz-target-triple | contract.md#fuzz; codebase-map.md | cargo-fuzz | PO-008, PO-009 |
| vb-shvxy-seed-006 | REQ-loom-cfg-wiring | contract.md#loom; codebase-map.md | loom | PO-010, PO-011 |
| vb-shvxy-seed-007 | REQ-formal-closure-fail-closed | contract.md#closure; codebase-map.md | kani, flux-rs, proptest, cargo-fuzz, loom | PO-012K, PO-012F, PO-012P, PO-012C, PO-012L |

## Contract Clause Coverage

| Clause | Summary | Covered By | Status |
|--------|---------|-----------|--------|
| C-001 | Lane identity closure | PO-012K, PO-012F, PO-012P, PO-012C, PO-012L | covered |
| C-002 | Availability preflight | PO-001, PO-002, PO-004 | covered |
| C-003 | Non-vacuous success | PO-007, PO-012K..L | covered |
| C-004 | Evidence classification | PO-012K..L | covered |
| C-005 | Kani feature parity | PO-003 | covered |
| C-006 | Flux wrapper shape | PO-005 | covered |
| C-007 | TLC portability | — | waived (TLA+ globally removed) |
| C-008 | Proptest zero-test guard | PO-006, PO-007 | covered |
| C-009 | Fuzz target/sanitizer guard | PO-008, PO-009 | covered |
| C-010 | Loom cfg/dependency guard | PO-010, PO-011 | covered |
| C-011 | Fresh evidence boundary | PO-012K..L | covered |
| C-012 | Fail closed on unknowns | PO-012K..L | covered |

## Hazard → Obligation Coverage

| Hazard | Obligations |
|--------|------------|
| HAZ-001 (Kani inventory ≠ execution) | PO-001, PO-012K |
| HAZ-002 (Kani undeclared features) | PO-003 |
| HAZ-003 (Flux unsupported selectors) | PO-005 |
| HAZ-004 (Missing TLA jar) | N/A (TLA globally removed) |
| HAZ-005 (TLC output truncation) | N/A (TLA globally removed) |
| HAZ-006 (Proptest zero tests + exit 0) | PO-006, PO-007, PO-012P |
| HAZ-007 (Fuzz musl + ASAN) | PO-009, PO-012C |
| HAZ-008 (Fuzz missing target) | PO-008, PO-012C |
| HAZ-009 (Loom cfg dependency leak) | PO-010, PO-011, PO-012L |
| HAZ-010 (Setup/version closing obligations) | PO-012K..L |
| HAZ-011 (Ambient target drift) | PO-009, PO-012C |
| HAZ-012 (Prior evidence reused) | PO-012K..L |

## Obligation Summary

| Obligation ID | Verifier | Target | Mode | Owner State |
|--------------|----------|--------|------|-------------|
| PO-001 | kani | `scripts/kani-list.sh vb_core` | verify-tooling | 6 |
| PO-002 | kani | `scripts/kani-list.sh vb_runtime` | verify-tooling | 6 |
| PO-003 | kani | `scripts/kani-list.sh vb_runtime` with feature gate | verify-tooling | 6 |
| PO-004 | flux-rs | `scripts/flux-check-package.sh vb_core` | verify-tooling | 6 |
| PO-005 | flux-rs | `scripts/flux-check-package.sh` selector rejection | verify-tooling | 6 |
| PO-006 | proptest | Zero-test detector script creation | verify-tooling | 6 |
| PO-007 | proptest | `cargo test` with fail-closed output parse | verify-tooling | 6 |
| PO-008 | cargo-fuzz | `cargo fuzz list` target registration | verify-tooling | 6 |
| PO-009 | cargo-fuzz | `cargo fuzz build --target x86_64-unknown-linux-gnu` | verify-tooling | 6 |
| PO-010 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib` | verify-tooling | 6 |
| PO-011 | loom | Loom model list verification | verify-tooling | 6 |
| PO-012K | kani | Kani lane closure evidence | verify-formal-closure | 10 |
| PO-012F | flux-rs | Flux-rs lane closure evidence | verify-formal-closure | 10 |
| PO-012P | proptest | proptest lane closure evidence | verify-formal-closure | 10 |
| PO-012C | cargo-fuzz | cargo-fuzz lane closure evidence | verify-formal-closure | 10 |
| PO-012L | loom | Loom lane closure evidence | verify-formal-closure | 10 |

## Lane Decision Summary

| Verifier | Required | Not Applicable | Seeds |
|----------|----------|---------------|-------|
| kani | 2 (seed-001, seed-007) | 5 | All 7 |
| flux-rs | 2 (seed-002, seed-007) | 5 | All 7 |
| proptest | 2 (seed-004, seed-007) | 5 | All 7 |
| verus | 0 | 7 | All 7 (already working) |
| cargo-fuzz | 2 (seed-005, seed-007) | — | seeds 005, 007 |
| loom | 2 (seed-006, seed-007) | — | seeds 006, 007 |
| **Total** | **5 required decisions** | **27 not_applicable** | **32 decisions** |
