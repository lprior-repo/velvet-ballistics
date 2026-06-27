//! Fuzz target for the YAML source-span bridge (`build_source_map` and
//! `build_semantic_source_map`).
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural and determinism assertions:
//! - `build_source_map` is deterministic — two builds from the same text
//!   produce identical span mappings (asserted at target level).
//! - `build_semantic_source_map` is deterministic — two builds from the same
//!   text resolve identical JSON-pointer paths to identical spans.
//! - `SourceSpan` is identity-equal to its input coordinates (enforced inside
//!   `fuzz_lib::fuzz_span_bridge`).
//! - `SourceMap::span_for_node(index)` returns `Some` exactly when an entry
//!   for `index` was emitted by the build (enforced inside
//!   `fuzz_lib::fuzz_span_bridge`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: span bridge is deterministic. Two builds from the
    // same text MUST produce identical span mappings.
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(map_a) = vb_compile::build_source_map(text) {
            if let Ok(map_b) = vb_compile::build_source_map(text) {
                let len_a = map_a.iter().count();
                let len_b = map_b.iter().count();
                assert_eq!(
                    len_a, len_b,
                    "build_source_map is not deterministic (entry count mismatch)"
                );
                for ((idx_a, span_a), (idx_b, span_b)) in
                    map_a.iter().zip(map_b.iter())
                {
                    assert_eq!(
                        idx_a, idx_b,
                        "build_source_map is not deterministic (node index)"
                    );
                    assert_eq!(
                        span_a, span_b,
                        "build_source_map is not deterministic (span value)"
                    );
                }
            }
        }

        if let Ok(sem_a) = vb_compile::build_semantic_source_map(text) {
            if let Ok(sem_b) = vb_compile::build_semantic_source_map(text) {
                for path in ["$", "$.when.manual", "$.steps[0]"] {
                    assert_eq!(
                        sem_a.span_for_path(path),
                        sem_b.span_for_path(path),
                        "build_semantic_source_map is not deterministic for path {path:?}"
                    );
                }
            }
        }
    }

    fuzz_lib::fuzz_span_bridge(data);
});
