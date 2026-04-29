# vb-6rt Implementation

## Summary

Implemented the Phase 3 strict YAML parser foundation inside the cold `vb-compiler` boundary. Runtime crates remain independent of YAML parser crates.

Repair pass for rejected QA blockers preserves parser source spans in strict diagnostics, removes the hand-rolled indirection scanner, adds missing rejection coverage, and keeps contract-governed strict YAML functions below the 25 logical line ceiling.

## Files Changed

- `crates/vb-compiler/src/strict_yaml.rs`
  - Added cohesive event-level strict YAML profile checks using `saphyr-parser`.
  - Rejects empty/no-document sources, multiple documents, aliases/anchors, and custom tags before semantic compilation.
  - Preserves the parser-provided `Span` start/end byte offsets plus one-indexed line/column marks in `SourceMark` for alias, anchor, tag, duplicate-key, merge-key, and event-level non-string-key diagnostics where the event API exposes them.
  - Classifies anchored nodes as `AnchorForbidden` and alias events as `AliasForbidden`.
- `crates/vb-compiler/src/lib.rs`
  - Exports and invokes `strict_yaml` through `YamlCompiler::compile`.
  - Added deterministic diagnostics for empty source, anchors, and merge keys.
  - Rejects merge keys and non-string mapping keys at mapping-key validation boundaries.
  - Added tests for accepted minimal strict workflow, empty source, multiple documents, merge keys, custom tags, non-string object keys, and anchor/alias diagnostic classification.
  - Removed the duplicate line-scanner for `&`, `*`, and `!`; strict YAML profile checking now uses parser events and parser spans.
- `.github/workflows/ci.yml`
  - Named the Moon CI step to expose the existing geiger/vet/bench/fuzz gate intent required by current scaffold tests.

## Contract Adherence

- YAML parsing remains cold-path only in `vb-compiler`.
- No YAML crates were added to runtime crates.
- Unsupported strict profile features now have deterministic `CompileError` variants/paths.
- `saphyr-parser` exposes event `Span { start: Marker, end: Marker }`; `SourceMark` records start byte offset, end byte offset, one-indexed line, and one-indexed column when available. Tree-only `saphyr::Yaml` validation documents unavailable marks explicitly with `SourceMark::unavailable()` because tree nodes do not retain marks.
- Existing compiler behavior remains green while adding strict parser foundation coverage.
- This bead intentionally does not implement full AST/type validation or compiler rewrites.

## Verification Results

All requested commands passed:

```text
cargo fmt --all -- --check
exit 0
```

```text
cargo test -p vb-compiler
cargo test: 66 passed (2 suites, 0.00s)
exit 0
```

```text
cargo test --workspace --all-targets
cargo test: 153 passed (15 suites, 0.05s)
exit 0
```

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished dev profile with no warnings
exit 0
```

## Remaining Risks

- Tree-only validation paths use `SourceMark::unavailable()` because `saphyr::Yaml` does not retain source spans; event-level strict checks and duplicate-key traversal preserve parser spans.
- Duplicate-key and merge-key checks are deterministic but still split between event/tree validation until the full AST phase lands.
- The current compiler still accepts the pre-existing `velvet/v1` version used by existing tests; canonical `velvet-ballastics/v1` migration remains separate work.
