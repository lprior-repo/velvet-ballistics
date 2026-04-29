STATUS: APPROVED

# Black Hat QA Review — vb-6rt

## Phase 1: Contract & Bead Parity

PASS. The previous bead-parity blockers are fixed.

- Source marks are no longer silently dropped: `crates/vb-compiler/src/strict_yaml.rs:36-41` carries `(event, mark)` into validation, `crates/vb-compiler/src/strict_yaml.rs:44-58` attaches `SourceMark::from_parser_span(mark)` to alias/anchor/tag diagnostics, and `crates/vb-compiler/src/lib.rs:491-572` preserves parser spans for duplicate-key, merge-key, alias, and event-level non-string-key failures.
- The source-location type is explicit: `crates/vb-compiler/src/lib.rs:62-103` records parser byte offset, end byte offset, line, column, and availability. Tree-only fallback paths are marked unavailable instead of pretending to have source data.
- Explicit rejection tests now exist for custom tags and non-string object keys: `crates/vb-compiler/src/lib.rs:2813-2821` and `crates/vb-compiler/src/lib.rs:2823-2831`.
- Anchor and alias classification is distinct: `crates/vb-compiler/src/strict_yaml.rs:46-56` emits `AliasForbidden` for alias events and `AnchorForbidden` for anchored nodes, with direct tests at `crates/vb-compiler/src/strict_yaml.rs:96-118`.

## Phase 2: Farley Engineering Rigor

PASS. The strict YAML profile functions are under the 25 logical-line ceiling. The previous oversized scanner is gone; the implementation now uses parser events instead of the lazy half-parser hack.

- `crates/vb-compiler/src/strict_yaml.rs:10-89` is split into small parser-event functions.
- `crates/vb-compiler/src/lib.rs:491-572` keeps the duplicate-key/profile traversal split into bounded helpers.
- Tests assert externally visible behavior and diagnostic variants, not private parser mechanics.

## Phase 3: NASA-Level Functional Rust

PASS. Unsupported YAML states are represented as typed `CompileError` variants with source marks where the parser exposes them. Custom tags, aliases, anchors, merge keys, duplicate keys, non-string keys, empty sources, and multiple documents all have deterministic rejection paths.

## Phase 4: Ruthless Simplicity & DDD

PASS. No panic-vector hit was found in the reviewed compiler sources. The repaired code is boring parser-event validation, which is what it should have been the first time.

## Phase 5: Bitter Truth

APPROVED. The repair addresses the actual failures instead of sanding around them. Source marks are preserved where available, the missing tests exist, the strict YAML functions are small, and anchors/aliases are no longer blurred into the same diagnostic bucket.

## Commands Executed

- `cargo fmt --all -- --check` — PASS
- `cargo test -p vb-compiler` — PASS, 66 tests
- `cargo test --workspace --all-targets` — PASS, 153 tests
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS

## Verdict

APPROVED. The previous blockers are fixed. Do not regress this by reintroducing hand-rolled YAML scanning or silently unavailable source marks.
