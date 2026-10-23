# Rust contract: distinct Reduce branch targets

Reduce branch body and done targets must be distinct. WorkflowError::ReduceBranchTargetAlias is raised when a ReduceStart or ReduceNext node has identical body and done step indices. This is a compatibility tightening that rejects previously admitted equal-target IR at the compiled-workflow constructor boundary; canonical compiler output with distinct targets is unchanged. The actual V70 historical RED receipt (3 writes, 1 evidence) is preserved with no runtime classifier change.

Contract persisted before guard application. Numeric kind references remain validated before alias detection. Positive distinct-target execution retains exact slot values, Clean taints, iterator tokens and ordered three-write evidence. New negatives assert the typed constructor error for both Start and Next. They are adversarial IR cases, not source acceptance. Independent CODE and TEST review required.
