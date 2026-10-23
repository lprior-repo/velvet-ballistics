# Strict pre-fix Repeat counter alias reproduction V21

Exact tested candidate e35d5f074750c0102b2b211c5c666be7e2758a15; frozen local5090 source e35bbd233b01844553b4826d137762c3b6c251c6; production base V15 9820a5d9dcd73b478bfcdb653435e3c91eea1da0. No production validator fix applied before this run.

Actual strict native result **1 PASS /1 FAIL**,0ignored/filtered. Cargo exit101 captured without an output pipeline;1.987s gate walltime;RUSTFLAGS=-Dwarnings,RUSTC_WRAPPER empty.

Baseline canonical source compiles and finishes I64(42) with3body executions in64steps. Negative test compiles the SAME baseline, mutates ONLY the compiled body node outer output to owning counter SlotIdx(1), and calls CompiledWorkflow::try_from_parts. Constructor ACCEPTS. Bounded runtime returns StepBudgetExhausted,31body executions,63counter SlotWritten events. This is an executed pre-fix malformed-IR ownership defect. It is not evidence of canonical source reaching a counter alias.

Worker raw output independently showed the same1/1,31/63 but used tee/tail without pipefail and had no true Cargo exit capture. Its pipeline0 is not PASS. Full raw worker events/invocation/stderr/terminal and liaison observation preserved separately. Root strict result is the authoritative receipt.

Limitations: test currently logs counter write count, not exact counter SlotValue sequence or final frame value. Baseline unchanged-parts assertion is around a second fresh execution. Test comments invert pre/post fix terminology; those comments are wrong and do not change the executed constructor/runtime observation. A separate debug helper contains temporary expectation-style code; preserve as WIP artifact, exclude/repair before any accepted landing. This branch intentionally contains an observational RED test and must not be treated as an accepted release candidate. No error-oracle weakening, no full F1 or main acceptance.

Next repair must enforce owning Repeat counter write isolation across relevant explicit/error/implicit writers, preserve valid max1/max3/private body outputs and exhausted-state equality, and use a typed constructor rejection regression. Frozen pre-fix source and raw observations must remain available.
