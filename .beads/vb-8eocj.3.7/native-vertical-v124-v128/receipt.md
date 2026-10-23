# First public Repeat vertical source qualification

Source713e414e06c599521453bffbac0c4d940f352187, digest76a6ba18675eb0b7a7a3ff77af7e627fe06d7f50492aec96fc3a35fe4da50d1c. Compiler six-file patch SHA b24c1d3d0ce8a464183c12597ce820869e1d5776bc1f081d0a1b3f9ea8985303 applied exactly.

V127 original public compile_workflow -> full runtime regression2tests PASS exit0 .714s. TWO Set values7/11 perbody/max1/max3, sentinels42/99Clean, exact traces/Finished42/unchanged compiled IR. Test artifact143a6f1706bfe8fba30664944a97754840404c1e4b75fc684d93b9676cf29ea2 unchanged before/after application. No hand-built IR substituted for source output.

V127 compiler filter canonical_repeat_tests incorrectly selected ZERO tests/513filtered, exit0 2.524s; preserve raw but award no test credit. Correct V128 canonical_repeat::tests selected actual8tests PASS exit0 .073s. Covers actual-width flat Set/Do layout, exact expression ops/input/vars/prior lexical references, scope errors/type errors, unsupported nested/control fields.

V128 full compiler513/core2205/runtime2007=4725PASS9.907s, zeroignored/filtered. Existing100mixedsource/constructor regressions PASS1.055s. Strict3library ClippyPASS.468s and source-lengthPASS3.904s. All map digests unchanged before/after.

Preceding ForEach alias repair: V124 constructor admitted forward-only body==done; actualitem9/cursor2Secret passed before independent4event expected comparison failed, actualonlyStepStarted/StepSucceeded. Published20c5dce1 exact failure retained. V12572baf2c1 typedForEachNext-only alias rejection3tests PASS6.934s incl numeric bounds precedence and distinct-forward same4event oracle. Full4717PASS19.559s/ClippyPASS2.85s/lengthPASS3.89s. Existing mixed100 had72PASS1FAIL/rest27NOTRUN because one counterprobe still used alias. V126abda7b4a changed ONLY ForEachNext.done to already checked distinctdone_target; ALL7foreign-counter-write oracles retained,100PASS.428s. No production guard weakened.

This establishes a bounded flat multi-step Repeat slice. General nested activation, arbitrary control/metadata, complete primitive source reachability, secrets, results/examples and deploy/smoke/rollback remain F1 work. Canonical moon ci and main landing are not established by these focused results.
