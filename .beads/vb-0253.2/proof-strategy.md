# Proof Strategy: vb-0253.2 — vb_ipc Facade Conversion

## Bead & Workspace

- **bead**: vb-0253.2
- **isolated workspace**: /tmp/vb-ws/vb-0253.2
- **state transition**: 3 (Contract complete) → 4 (Proof Planning)

---

## Verification Philosophy

This bead is a **pure facade refactor** — structural reorganization only, zero behavioral change. The existing test suite (60+ Fowler tests) already exercises all behavioral contracts. No new proof obligations are generated; all 16 obligations are inherited from the State 3 contract artifact.

The single verification lane is **verify-standard** (cargo test + clippy), supplemented by static-scan obligations that confirm canonical-definition uniqueness post-dedupe.

---

## Lane: verify-standard

### Obligation Set

| ID | Layer | Checker | Target | Claim | Risk |
|---|---|---|---|---|---|
| SRC-001 | static-scan | rg | ingress.rs | Only one MemoryIngress definition | medium |
| SRC-002 | static-scan | rg | ingress.rs | Only one IngressFrame definition | medium |
| SRC-003 | static-scan | rg | bounded.rs | Only one QueueCapacity definition | medium |
| SRC-004 | static-scan | rg | bounded.rs | Only one MaxPayloadBytes definition | medium |
| SRC-005 | static-scan | rg | bounded.rs | Only one BoundedPayload definition | medium |
| SRC-006 | static-scan | rg | error.rs | Only one IpcError definition | medium |
| SRC-007 | static-scan | rg | lib.rs | map_try_send removed | low |
| SRC-008 | static-scan | rg | lib.rs | u32_to_usize duplicate removed | low |
| SRC-009 | static-scan | rg | lib.rs | pub mod bounded/ingress/error added | low |
| BUILD-001 | compile-check | cargo build | vb_ipc | Crate compiles | high |
| BUILD-002 | compile-check | cargo build | velvet_ballastics | Downstream crate compiles | high |
| BUILD-003 | compile-check | cargo build | workspace_tests | Downstream crate compiles | medium |
| TEST-001 | compile-check | cargo test | vb_ipc | All tests pass | high |
| LINT-001 | static-scan | rg | vb_ipc/*.rs | No unsafe code | high |
| MOON-001 | gauntlet-standard | moon run | workspace | Full moon ci lane passes | medium |
| WAIVER-FORMAL-001 | waiver | contract.md | N/A | Formal proof waived | low |

**Total obligations**: 16 (14 required, 2 advisory/waiver)

### Lane Ordering

All obligations are independent and may run in parallel. Recommended execution order:

1. **Static scans first** (SRC-001–SRC-009, LINT-001) — fast, catch structural issues immediately
2. **Compile checks** (BUILD-001, BUILD-002, BUILD-003) — catch import/re-export breakage
3. **Test execution** (TEST-001) — confirm behavioral invariants unchanged
4. **Moon ci gauntlet** (MOON-001) — workspace-wide gate
5. **Waiver recorded** (WAIVER-FORMAL-001) — contract.md Non-goals section is the evidence

### Failure Classification

| Obligation | FAIL_LOCAL triggers | FAIL_REGRESSION triggers | BLOCK_RELEASE? |
|---|---|---|---|
| SRC-001–SRC-009 | Duplicate definition found in lib.rs | Duplicate definition found in any other file | YES |
| BUILD-001 | vb_ipc fails to compile | Any downstream compile failure | YES |
| BUILD-002 | velvet_ballastics fails | — | YES |
| BUILD-003 | workspace_tests fails | — | NO |
| TEST-001 | Any vb_ipc test fails | Any downstream test fails | YES |
| LINT-001 | unsafe block found | — | YES |
| MOON-001 | moon ci non-zero exit | — | YES |
| WAIVER-FORMAL-001 | N/A | N/A | NO |

---

## Formal Waiver Rationale

**WAIVER-FORMAL-001** is granted because:

1. This is a facade conversion — canonical sources are unchanged, only re-exports added
2. The behavioral surface (60+ tests in vb_ipc, cross_crate_adversarial, cli_integration) is fully exercised by TEST-001
3. No temporal properties, protocol state machines, or concurrency patterns are introduced or changed
4. crossbeam_channel is the trusted runtime component; no new channel usage patterns added
5. INV-007 (bounded-memory) and INV-008 (payload-validation) are exercised by existing Fowler tests, not new formal proofs

---

## No Additional Verifier Lanes

- **TLA+**: Not applicable — no temporal/protocol/scheduler changes
- **Verus**: Not applicable — no ghost/exec refinement needed for facade reorganization
- **Kani**: Not applicable — no unsafe code, no arithmetic/index proofs needed
- **Loom**: Not applicable — crossbeam_channel usage is unchanged
- **Miri**: Not applicable — no unsafe code introduced

---

## Risk Summary

| Risk Tag | Mitigation | Evidence |
|---|---|---|
| public_api | BUILD-001, BUILD-002 confirm all re-exports compile | build-report.txt |
| migration | BUILD-003 confirms downstream crates unaffected | build-report.txt |
| behavioral-drift | TEST-001 confirms all 60+ tests pass | test-report.txt |
| duplicate-definitions | SRC-001–SRC-006 confirm single canonical source | source-audit-report.md |
| unsafe-introduction | LINT-001 confirms no unsafe blocks | lint-report.txt |
