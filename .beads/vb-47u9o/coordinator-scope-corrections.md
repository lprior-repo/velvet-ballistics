# Corrections to the independent scope report

The independent report correctly identifies unchanged taint harness, constructor and slot/taint operations. Its explanatory details require these corrections:

- IDs are defined in crates/vb_core/src/ids/mod.rs and its included parts, not a current ids.rs file. Full old/current blob IDs are recorded in coordinator-exact-scope-comparison.json.
- RunFrame::new allocates states from step_count (fixed1 in this harness), and slots plus taint arrays from symbolic slot_count. The source uses vec! and into_boxed_slice; the report's Vec<StepState>::with_capacity(slot_count) description is inaccurate.
- u16 is bounded at65535. The timeout is observed verifier cost over this bounded symbolic domain; no unbounded mathematical domain is claimed. The exact cause of solver runtime is an inference, not a demonstrated implementation failure.
- GOD RULE5 mandates differential verification. GOD RULE4 prohibits weakening contracts to make proofs pass. vb-heuiz asks for a sound tractability repair preserving the full domain; it does not request a smaller bound.
- Avoiding a blind unchanged whole-domain rerun does not prove that any future rerun necessarily times out. The actual recorded command remains timeout124; no proof success or unrestricted CI pass is inferred.

The file/hash comparison supports narrow differential classification. It does not resolve vb-heuiz or waive required proofs for newly changed code.
