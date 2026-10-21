Only two new input_slots fields inserted in test fixtures. Policy42/0failed/0ignored, IPC531/0failed/0ignored. Actual exit statuses0; raw logs retained. Legacy expects/arithmetic predate this change, not rewritten or claimed resolved.

code-review
APPROVE

The change is a minimal, targeted fix adding the required `input_slots` field to two test fixture constructors. All 573 tests pass (42 policy + 531 ipc-lib). No production code, behavior, or assertions changed. The empty `Box::new([])` initialization is appropriate for test fixtures.
test-review
APPROVE

The change adds the required `input_slots` field to two test fixture constructors with empty `Box::new([])` values. All 573 tests pass (42 policy + 531 ipc-lib). No production code, behavior, or assertions were modified. The empty input slots are appropriate for these minimal test fixtures.