# Verification Layers

## Boundary
- **Verus-owned kernel:** Not applicable (no new pure Rust-core logic).
- **TLA+ temporal model:** Not applicable (no temporal behavior).
- **Theorem projection:** Not applicable (no theorem kernel).
- **Runtime shell:** `crates/vb_ipc/src/server/handlers.rs` and its orphaned sibling files in `crates/vb_ipc/src/server/handlers/`.
- **External systems excluded from formal proof:** N/A.

## Layer Assignment

| Clause | Layer | Rationale |
|--------|-------|-----------|
| C1 (vb_ipc compiles) | `static-scan` | `cargo check` is the canonical compilation gate. |
| C2 (workspace-tests compiles) | `static-scan` | Cross-crate compilation validation via `cargo check --tests`. |
| C3 (no unsafe/unwrap/panic) | `static-scan` + manual review | Source lint (`-D warnings`) plus `grep` inspection of the diff. |
| C4 (orphaned files safe) | `static-scan` + manual review | Confirm no `mod.rs` references them; confirm `cargo check` ignores them. |
| INV-001 (type consistency) | `static-scan` | The Rust type checker proves enum variants match struct field types. |
| INV-002 (compilation isolation) | `static-scan` | `cargo check` succeeds only if orphaned files are truly excluded. |
| INV-003 (safety preservation) | `static-scan` + `miri` (optional) | Clippy `forbid(unsafe_code)` + grep for panicking APIs. |

## Compilation Gate (Primary)

The primary verification layer for a compile fix is the **compilation gate**:

```bash
# Gate 1: vb_ipc crate compiles
cargo check -p vb_ipc

# Gate 2: Source lint (zero tolerance)
cargo clippy -p vb_ipc -- -D warnings

# Gate 3: Workspace tests compile
cargo check -p velvet-ballistics-workspace-tests --tests
```

All three gates must exit `0` with zero errors and zero warnings.

## Safety Scan (Secondary)

```bash
# Verify no unsafe introduced
grep -n 'unsafe' crates/vb_ipc/src/server/handlers.rs
# Expected: no matches

# Verify no unwrap/expect/panic/todo/unimplemented introduced
grep -n 'unwrap\|expect\|panic!\|todo!\|unimplemented!' crates/vb_ipc/src/server/handlers.rs
# Expected: no new matches beyond any pre-existing occurrences
```

Note: `handlers.rs` already contains `#![forbid(unsafe_code)]` at the top of the file.

## Orphaned File Check (Secondary)

```bash
# Confirm no mod.rs exists in the handlers/ subdirectory
ls crates/vb_ipc/src/server/handlers/mod.rs 2>/dev/null
# Expected: file not found

# Confirm handlers/ files are not referenced elsewhere
rg 'mod command;\|mod event;\|mod query;\|mod session;' crates/vb_ipc/src/
# Expected: no matches
```

## Waivers

- **Verus waiver:** No Verus obligation is assigned because no new pure Rust-core logic is introduced. The Rust type checker provides equivalent mechanical assurance for type consistency. See `lean-contract.md`.
- **TLA+ waiver:** No TLA+ obligation is assigned because no temporal behavior is modified. See `tla-spec.md`.
- **Theorem waiver:** No Lean/Aeneas/Hax obligation is assigned because no theorem-critical kernel is involved. See `lean-contract.md`.

## Non-goals

- Performance benchmarking (no hot paths modified).
- API compatibility checks (`cargo semver-checks`) -- no public API surface changed.
- Fuzzing -- no parser/codec/protocol boundary changes.
- Loom/Shuttle -- no concurrency changes.
- Coverage -- no new executable code to measure.
