# vb-6rt.4 Contract

The public `vb_yaml::load_fixture_source(text: &str) -> YamlResult<WorkflowSource>` accepts YAML source text and returns the parsed workflow AST; it does not accept or read a filesystem path.

Given canonical `velvet-ballistics/v1` workflow text with a manual trigger, a `set` step, and a `finish` step, loading succeeds and independently asserted AST values include version, name, trigger, ordered step identifiers, set output/value, and finish literal.

Given empty input, loading returns exactly `YamlError::EmptySource`; given a YAML scalar instead of a workflow mapping, it returns exactly `YamlError::FieldShape { field: "workflow", expected: "mapping" }`.

Given the concrete unclosed flow-sequence source `version: [unfinished\n`, loading returns `YamlError::ParseError` at line 2 with the pinned parser reason `while parsing a flow sequence, expected comma or closing bracket` (the executable scenario asserts the exact punctuation-bearing message from saphyr-parser 0.0.6); unexpected error kinds fail through Result. Given an anchor/alias strict-profile feature, loading returns exactly `YamlError::AnchorAliasMerge`. Other malformed sources still delegate their typed parser diagnostics unchanged; no generic vendor-message stability is claimed.

Repeated calls with unchanged source return identical AST values, and caller input remains unchanged; loading performs no fixture-path or filesystem operation.

Behavior is covered by executable Given/When/Then scenarios in `crates/workspace_tests/tests/vb_eirmk_fixture_source_contract.rs` and verified only with the specified focused native nextest lane; this contract does not claim full parser correctness or compile closure.
