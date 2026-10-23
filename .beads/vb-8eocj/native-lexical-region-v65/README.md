# Native F1 lexical and loop-region evidence

V62 lexical source `57d85d5a48e2a338e3662a3f5b62befcd8232b6e`: new lexical target 5 PASS / 22 FAIL. Existing four source targets 70 PASS, compiler library 505 PASS, and strict three-library Clippy PASS at unchanged source hash. These passes do not establish full F1 coverage.

V64 malformed-region diagnostic initially failed at construction with UnreachableNode(6): the fixture confused final Finish with the owning RepeatFinish exit. This is retained as a fixture defect, not runtime reproduction.

V65 source `07295e1638e270d136ce34786c4debd8768d45b7`: corrected diagnostic derives RepeatCheck.kind.done, constructor accepts escape to an outside Jump and reentry, then drive fails invalid_state_transition. The test expects the proposed region guard and is RED. This intentionally mutated-IR diagnostic is not source-language acceptance. Runtime region repair and cold ownership/edge closure remain open.

Each raw command has contemporaneous before/after source maps. Logs are immutable historical observations; later corrections do not rewrite failures. No full canonical CI or main landing is claimed.
