# Collect budget and traversal qualification

Repair vacuous outcomes in crates/vb_core/src/workflow/proptest_collect_budget.rs and proptest_collect_traversal.rs. Preserve the original registered properties and use the production CompiledWorkflow::try_from_parts and WholeWorkflowBudget::compute APIs with a valid five-node Collect fixture.

Assert independently checked total steps 3 + 2 * max(limit, 1), gather items equal limit, and gather pages equal ceil(limit / max(page_size, 1)). Arbitrary u32 inputs with total above u32::MAX must yield the exact StepCountOverflow actual value. Representable totals use the budget computation directly because default runtime policy separately limits admitted steps.

For limit and page_size in 0..1000 and explicit zero/one cases, actual workflow validation must expose loop-region start 0, body 1, control 2 and done 3. The u32::MAX case must assert exact overflow rejection. This proves bounded cold validation and budget behavior; runtime Collect iteration termination remains outside this bead.

No swallowed errors, assumed success, unchecked arithmetic, new dependencies, production API changes or gate changes. Acceptance requires pinned native registered tests, source review, independent test review and the existing ignored-fallible-results scanner on these files.
