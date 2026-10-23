# Cold region native qualification and preserved failures

V96 source f5d12cebba3bb316302f926e822daa421c17398a: dedicated constructor/cache31 PASS, source-length PASS. No runtime rearm enabled.

V96 combined library invocation: compiler504 PASS/1 FAIL on stale CollectPage expectation (production emits CollectNext), core/runtime not run. Proptest generated a regression seed under crates/vb_compile/proptest-regressions during execution: raw cargo exit101; pinned runner exit125 for the changed source map. The after pin32393a1a277e3b15e487bdd11aeaaef708391139 adds only that seed. Do not treat this invocation as unchanged-source acceptance.

V97 same32393 source, unchanged: core2188 PASS/5 FAIL; runtime was not run in that invocation. Separate runtime1992 PASS/2 FAIL. Failures are recorded for ownership-policy review and fixture repair; no rejection or assertion is waived.

V98 source35a998d9323b276301fbe199fe2f16b5fa6fe112: compiler505 PASS after only exact CollectPage->CollectNext property kind/message repair. All four-node and ID assertions/strategies retained, saved regression seed retained.

No full canonical CI, region hot activation, composite source body, nested execution, error recovery or whole F1/F2 acceptance is established here.
