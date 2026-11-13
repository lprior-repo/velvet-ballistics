# Shared legacy whitespace semantics

Reuse the existing hotpath whitespace predicate across all three Rust validators. U+001C through U+001F are whitespace for required fields, resolution rationale/IDs, manifest trimming, raw-log emptiness and stderr compaction. Error enum extraction retains the original line-separator and leading-whitespace behavior so variants cannot disappear. Hotpath behavior stays identical; no dependency, schema, public CLI or gate change.

Add public baseline/cause-specific negative controls and parser regressions first, observe red on the current implementation, then repair the shared consumers. Verify all existing cases, new controls, strict source checks and independent reviews. No Python execution, metadata waiver or v1 acceptance is inferred.
