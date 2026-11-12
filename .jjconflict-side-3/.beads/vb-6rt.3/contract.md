# vb-6rt.3 Contract

## Scope
Qualify the public `vb_yaml::build_source_map`, `SourceMap::span_for_node`, and `vb_yaml::span_for_node` behavior against YAML parser event spans with focused integration scenarios; repair the shared builder only if a baseline-red scenario proves a contract violation.

## Contract
- Given valid flat YAML, building a source map returns parser event nodes in order and each asserted node has the independently expected `SourceSpan` byte offsets and one-indexed coordinates, with exclusive end offsets.
- Given nested mapping and sequence YAML, root, key, and sequence-value nodes resolve to exact expected spans at their event-order indexes.
- Given UTF-8 YAML before an asserted node, offsets count UTF-8 bytes while columns follow the parser's documented one-indexed source coordinates.
- Given a mapped node, the method and free lookup function return that exact span; root and last mapped nodes are available, while the first unmapped index returns `None`.
- Given malformed YAML, source-map construction returns the parser's typed error.

## Existing Coverage and Gaps
`yaml_source_map_tests.rs` covers empty construction, sequential iteration, loose range/order checks, out-of-range lookup, nested/sequence nonempty maps, malformed rejection, and several profile edges. `vb_eyij7_migration_regression.rs` only checks that building succeeds. Neither independently asserts exact public builder coordinates for flat/nested/sequence/UTF-8 inputs; existing out-of-range coverage does not qualify the last valid index or wrapper against a golden span. New integration scenarios target those uncovered obligations and avoid schema-validator policy.

## Acceptance
Run the native pinned nextest command for `velvet-ballistics-workspace-tests` and `vb_eirmk_source_map_contract`, using the prescribed build environment and evidence log. Keep edits limited to this contract and the named integration test unless a golden scenario proves a shared-builder defect; report the command's actual exit code and remaining unverified work.
