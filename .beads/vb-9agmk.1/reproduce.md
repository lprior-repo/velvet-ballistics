Review target:948f19cb7cd92afec3d95e9fb285216cc6689d41.

The archived Rust probe is intentionally a counterexample observer: its tests PASS when the unsafe admission behavior exists. It is NOT a regression suite that should remain green after repair. When the implementation is repaired, reverse the substitution assertions to expect typed rejection and retain a valid control.

To reproduce at the exact target in an isolated Git/JJ workspace, copy vb_9agmk_code_review_probes.rs into crates/workspace_tests/tests/, retain the candidate artifact_evidence_support module, then run the argv in focused-review-probes-command.json. Do not apply this evidence file as a production fix.
