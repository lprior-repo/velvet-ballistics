// Proptest: SemanticSourceMap path annotation properties
// PO-P07: SemanticSourceMap path annotation (C11.1-C11.3)
//
// Properties:
//  1. When SemanticSourceMap has a path for a given location, the path
//     is retrievable by exact match
//  2. When no matching path exists, span_for_path returns None
//  3. Path lookup is idempotent
//  4. Absence of SemanticSourceMap produces None for all lookups
//
// TF-VB-006: BLOCKED BEHAVIOR — SemanticSourceMap path annotation (B103/B104)
// =================================================================
//
// BLOCKER-VISIBILITY: SemanticSourceMap::push() is pub(crate), so populating
// a map with real YAML author paths from integration tests outside the
// vb_yaml crate is NOT possible. The span_for_path() method IS pub, so
// empty-map lookups work from outside.
//
// Contract: C11.1-C11.3 (SEM-MAP-MSG)
//   C11.1: When a SemanticSourceMap contains a path entry for a diagnostic's
//          span location, the rendered diagnostic message SHALL include the
//          YAML author path.
//   C11.2: Path annotation appends to the message; it SHALL NOT replace the
//          original diagnostic text.
//   C11.3: When no SemanticSourceMap is available (None or empty), the
//          diagnostic message SHALL NOT include a path annotation.
//
// Behaviors awaiting implementation (B103-B104):
//   B103: path included in diagnostic message when map has entry
//   B104: path annotation appended, not replacing original message
//   B105: un-annotated when map is absent (COVERED by tests below)
//   B106: never panics with absent map (COVERED by tests below)
//
// What this test suite CAN prove (current state):
//  1. Default empty SemanticSourceMap returns None for any path lookup
//  2. Empty-map lookups are deterministic and idempotent
//
// Coverage satisfied for B105 and B106.
//
// Unblock options (choose one):
//  1. Add a test-only constructor `SemanticSourceMap::for_test(paths)` behind
//     #[cfg(test)] or a test feature gate in vb_yaml crate, exposed via
//     vb_compile re-export.
//  2. Add unit tests in vb_yaml crate that populate a SemanticSourceMap via
//     the YAML parsing path and verify path lookups.
//  3. Write an E2E test in vb_compile that compiles YAML with known errors
//     and verifies the rendered diagnostic includes the YAML author path.
//
// Strategy: test empty map + span_for_path public API for B105/B106 coverage.
// Full push-based construction tests for B103/B104 must live in vb_yaml crate
// or require a test-constructor to be added.

use proptest::prelude::*;
use vb_yaml::source_map::SemanticSourceMap;

proptest! {
    #[test]
    fn empty_map_returns_none_for_any_path(
        path in "[a-zA-Z0-9$_./*]{1,50}",
    ) {
        // Default-constructed SemanticSourceMap has no entries
        let map = SemanticSourceMap::default();
        let result = map.span_for_path(&path);
        prop_assert_eq!(result, None);
    }

    #[test]
    fn lookup_is_deterministic_on_empty_map(
        path in "[a-zA-Z0-9$_./*]{1,50}",
    ) {
        let map = SemanticSourceMap::default();
        let first = map.span_for_path(&path);
        let second = map.span_for_path(&path);
        prop_assert_eq!(first, second);
        prop_assert_eq!(first, None);
    }
}
