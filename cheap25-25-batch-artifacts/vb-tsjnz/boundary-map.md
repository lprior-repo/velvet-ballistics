# Boundary Map — vb-tsjnz

- bead_id: `vb-tsjnz`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- capture: 2026-07-01

This bead's boundary map is **unusually small** because the patch is metadata-only. There is no parser, no I/O, no runtime, no async, no FFI, no unsafe surface in scope. The boundaries are TOML/Cargo boundaries and the inheritance edges into the workspace policy tables.

```
+-------------------------------------------------------+
|  Root /home/lewis/src/isoloated/velvet-ballistics-... |
+-------------------------------------------------------+
                       |
   +-------------------+-------------------+
   |                                           |
   v                                           v
+--------+                                +--------------+
| B1     |                                | B2           |
| Root   |                                | Member       |
| Cargo. |                                | Cargo.toml   | <-- EDIT
| toml   |--- inherits ------------------>| vb_queue_    |
|        |    [workspace.package].version | semantics/   |
|        |                                | Cargo.toml   |
|        |--- inherits ------------------>|              |
|        |    [workspace.lints.rust]      |              |
|        |    [workspace.lints.clippy]    |              |
+--------+                                +--------------+
                                                       |
                                                       v
                                                +--------------+
                                                | B3           |
                                                | lib.rs       |
                                                | 423 lines    |
                                                | (read-only,  |
                                                |  compile     |
                                                |  target of   |
                                                |  B2 lints)   |
                                                +--------------+
                                                       |
                                              inherited
                                                       v
                                                +--------------+
                                                | B4 / B5      |
                                                | scripts/     |
                                                | check-       |
                                                | workspace-   |
                                                | assertions.  |
                                                | rs and       |
                                                | tests/...    |
                                                | (read-only   |
                                                |  post-build) |
                                                +--------------+
```

## Boundary Inventory

### B1 — Root Workspace Manifest (read-only reference)

- **Path:** `/Cargo.toml`
- **Owner:** Workspace author.
- **Touched by this bead?** No.
- **Declares:** `[workspace.members]` (`vb_queue_semantics` already at line 7), `[workspace.package].version = "0.1.0"`, `[workspace.dependencies]`, `[workspace.lints.rust]`, `[workspace.lints.clippy]`.
- **Boundary role:** Single source of truth for inherited fields and lint policy. The bead inherits from B1.

### B2 — Member Manifest (EDIT TARGET)

- **Path:** `crates/vb_queue_semantics/Cargo.toml`
- **Owner:** Holzman-rust (this bead).
- **Touched by this bead?** Yes.
- **Edit windows:**
  - Line 3: `version = "0.1.0"` → `version.workspace = true`.
  - Line 11 to tail: append `\n[lints]\nworkspace = true`.
- **Boundary role:** Declares the inheritance markers that pull from B1.

### B3 — Crate Source (read-only, compile target)

- **Path:** `crates/vb_queue_semantics/src/lib.rs` (423 lines).
- **Owner:** Out of scope. Held invariant for vb-tsjnz; covered by `vb-2lu1` source-length exception at `.config/source-length-exceptions.txt:323`.
- **Boundary role:** **The compile target of the new lint policy**. Adding `[lints]\nworkspace = true` at B2 changes the contract surface that B3 compiles under. B3's lint-clean status is the load-bearing risk of this bead.

### B4 — Workspace Assertion Script (read-only gate)

- **Path:** `scripts/check-workspace-assertions.rs`
- **Touched by this bead?** No.
- **Boundary role:** Already enumerates `crates/vb_queue_semantics` (line 11 `EXPECTED_MEMBERS`, line 44 `EXPECTED_PACKAGE_NAMES`). Remains green either way.

### B5 — Workspace Tests (read-only gate)

- **Path:** `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs`, `crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs`.
- **Touched by this bead?** No.
- **Boundary role:** References `crates/vb_queue_semantics` (lines 12/57 and 14 respectively). Will continue to pass.

## Boundary Classifications (Wlaschin)

| Boundary | Pure core | Imperative shell | Async shell | Storage | Network | Time | FFI | Unsafe | Parser |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B1 | n/a (config) | n/a | n/a | n/a | n/a | n/a | n/a | n/a | cargo TOML parser (edge) |
| B2 | n/a (config) | n/a | n/a | n/a | n/a | n/a | n/a | n/a | cargo TOML parser (edge) |
| B3 | pure data types (read-only) | n/a | n/a | n/a | n/a | n/a | already `forbid(unsafe_code)` | n/a |
| B4 | rule script | runs in `cargo test` driver | n/a | n/a | n/a | reads filesystem | n/a | n/a | n/a |
| B5 | rule script + tests | runs in `cargo test` driver | n/a | n/a | n/a | reads filesystem | n/a | n/a | n/a |

## Inherited Boundaries (Where the Patch Resolves)

The TOML inheritance makes B2 syntactically depend on B1. Concretely:

1. B2 `[package].version.workspace = true` resolves to B1 `[workspace.package].version` (= `"0.1.0"`).
2. B2 `[lints]\nworkspace = true` resolves to B1 `[workspace.lints.rust]` + `[workspace.lints.clippy]`.

**Boundary failure mode**: any inconsistency between B1 declarations and B2 inheritance markers shows up at `cargo metadata` time. The patch MUST keep both axes self-consistent. There is no scenario in which B2 can opt in to lints without B1 having declared them.

## External Boundaries

- **`rust-toolchain.toml`** — pins a nightly new enough to support Cargo's `[lints]` feature (Cargo 1.74+). The governance pins the right version; this bead trusts that pin and does not modify the file.
- **`Cargo.lock`** — regen by cargo on build. Not edited by the agent.

## Boundary Diagram Notes

- The patch is a single-file edit at B2.
- B3 is **not** modified, but its build acceptance is materially affected.
- B4 and B5 are downstream gates; their inputs already include this member.
- No storage, no network, no async, no FFI, no unsafe surfaces were introduced.
