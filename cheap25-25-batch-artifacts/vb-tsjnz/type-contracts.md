# Type Contracts — vb-tsjnz

- bead_id: `vb-tsjnz`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- scope: cargo TOML type contracts (not Rust types)
- capture: 2026-07-01

This bead's "type system" is TOML + cargo manifest semantics. There is no Rust source delta. We model the manifest fragment that the patch introduces as a value-level type contract.

## TC-1 `VersionField` (TOML string-or-inheritance marker)

Canonical form for this crate (post-patch):

```toml
version.workspace = true
```

Forbidden shapes (illegal states):

- `version = "0.1.0"` — literal hardcode; produces drift (F-1).
- `version = ""` — empty literal; rejected by cargo.
- `version.workspace = false` — disallowed by cargo when `[workspace.package]` exists.
- `version = 0.1.0` (no quotes) — wrong TOML type; rejected by cargo.

The **only** legal shape at this site, post-patch, is the inheritance marker.

## TC-2 `EditionField` (held invariant)

```toml
edition.workspace = true
```

Already correct in the target file (line 4). Held for review; not edited by this bead.

## TC-3 `LicenseField` (held invariant)

```toml
license.workspace = true
```

Already correct in the target file (line 5). Held for review; not edited by this bead.

## TC-4 `PublishFlag` (held invariant)

```toml
publish = false
```

Already correct in the target file (line 6). Held for review; not edited by this bead. Required for `xtask/src/forbidden_scan.rs` allow-list parity (line 18).

## TC-5 `LintsBlock` (TOML table)

Canonical shape (matches `vb_cli:38`, `vb_compile:26`, `vb_core:34`, `vb_ipc:23`, `vb_runtime:44`, `vb_storage:43`, `vb_validate:20`):

```toml
[lints]
workspace = true
```

Forbidden shapes:

- `[lints.rust]` / `[lints.clippy]` sub-tables — selective opt-out not allowed; sibling pattern forbids it.
- `[lints]` with multiple keys (e.g. `workspace = true\nrust = "..."`) — sibling pattern forbids it.
- A non-table token (e.g. `lints = true`) — wrong TOML type; cargo rejects.
- `[lints]\nworkspace = "true"` (quoted) — wrong TOML type for boolean field; cargo rejects.

## TC-6 `DependenciesBlock` (held invariant — no edit)

The existing `[dependencies]` table footer is preserved verbatim. No new entry; no reshuffle.

## TC-7 `LeadingHeader` (held invariant — no edit)

Lines 8-10 (the stub comment) are preserved verbatim. They are descriptive, not load-bearing; touch is out of scope.

## TC-8 `FileEndPolicy`

The file MUST end after the `[lints]` block. No trailing whitespace, no comment after `workspace = true` (matches sibling convention).

## Type-Level Equivalence Table (Sibling Reference)

| Field / Block | Position | Sibling shape | Target shape (post-patch) |
| --- | --- | --- | --- |
| `name` | line 2 | literal string | unchanged (`"vb_queue_semantics"`) |
| `version` | line 3 | `version.workspace = true` | `version.workspace = true` |
| `edition` | line 4 | `edition.workspace = true` | unchanged |
| `license` | line 5 | `license.workspace = true` | unchanged |
| `publish` | only in `vb_cli` etc. if present | varies | held: `publish = false` (already present) |
| `[lints]` | final block | `[lints]\nworkspace = true` | appended as final block |

## "Smart Constructor"

Conceptually, this bead's edit is equivalent to:

```text
let pre  = read("crates/vb_queue_semantics/Cargo.toml");
let post = MemberManifest::patch(pre, PatchPlan::InheritVersion + PatchPlan::AppendLintsBlock)
    .expect("incoming file conforms to documented shape");
write("crates/vb_queue_semantics/Cargo.toml", post);
```

Where `MemberManifest::patch` validates:

1. The input contains exactly one `[package]` table.
2. The input `[package]` currently has a literal `version`.
3. The output `[package]` MUST have `version.workspace = true`.
4. The output file MUST end with `[lints]\nworkspace = true\n`.
5. No field outside the listed scope is touched.

The actual implementation is performed by holzman-rust as a two-step `Edit` on lines 3 + 11→13 (effectively). The contract is what the diff satisfies, not the algorithm.

## External Parsing Boundary (B-import)

The manifest is parsed by cargo. Any malformed TOML or invalid `version.workspace` marker is rejected at `cargo metadata` time, BEFORE any source code is compiled. This is the outermost parser boundary for the patch.

```
input: TOML text
        |
        v
+-----------------------------------+
|  cargo manifest parser            |
|  (B1, B2 boundary)                |
+-----------------------------------+
        |
        v
resolution: manifest graph node  (---> cargo build pipeline)
```

The patch MUST pass this boundary cleanly. Failure paths live in `error-taxonomy.md`.
