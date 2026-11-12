# Bead vb-08daa: Missing Hunk Recovery — Implementation Record

## Work Performed

Recovered 3 of 4 hunks from `.beads/vb-08daa/historical-own-patch.patch`. The 4th hunk was already present.

### Changes Applied

| File | Line(s) | Change | Historical Hunk |
|------|---------|--------|-----------------|
| `.cargo/config.toml` | 18 | `--all-targets` → `--lib --bins --examples` in `ci` alias | Hunk 1 |
| `contracts/invariants.yaml` | 400 | `--all-targets` → `--lib --bins --examples` in `no_unsafe_in_first_party` scan | Hunk 2 |
| `xtask/src/evidence/persistence.rs` | 67 | `--all-targets --all-features` → `--lib --bins --examples --all-features` in clippy repair hint | Hunk 3 |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | 57 | No change needed — already `source.inputs()` (no `&`) | Hunk 4 (already present) |

### Rationale

- **Hunk 1**: AGENTS.md states "tests compile and run, but test clippy is not strict." The `ci` alias should target source code only (`--lib --bins --examples`), not test code (`--all-targets`).
- **Hunk 2**: Same rationale for the unsafe-code scan. Target selection follows the source-lint policy; the existing independent `unsafe-audit`, `panic-surface`, and `banned-token-gates` moon tasks enforce the no-unsafe policy across all first-party code.
- **Hunk 3**: Repair hint must match the `ci` alias target selection for consistency.
- **Hunk 4**: Already present in current code — `source.inputs()` returns `&[InputField]` directly; the `&` was historical cruft removed by an earlier pass.

### Preserved Invariants

- `--workspace` flag preserved in all 3 changed commands
- `--all-features` preserved in `ci` alias and persistence repair command
- `-- -D warnings` preserved in `ci` alias
- `check-all` alias unchanged (type checking still uses `--all-targets`)
- No runtime-core behavior change; `xtask/src/evidence/persistence.rs` string-literal change to tooling repair hint only
- No test framework changes
- No new tests or test scripts created
- Master doc and broader docs unchanged per user instruction

### Verification Method

No tests executed. This is config/contract/repair-hint recovery with no runtime behavior change. Verification relies on:
- `lint-src` moon task (source gate) runs `cargo clippy --workspace --lib --bins --examples --all-features` with `-D unsafe_code`
- `cargo ci` alias for manual use targets `--lib --bins --examples` matching the source gate
- Manual `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` as smoke
- Existing independent source gates: `panic-surface`, `ignored-fallible-results`, `unsafe-audit`, `banned-token-gates`, `test-integrity`

### Risk Profile

Low. Tooling-only changes. No API, source attribute, or runtime behavior modifications.