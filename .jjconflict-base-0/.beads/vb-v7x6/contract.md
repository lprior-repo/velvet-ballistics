bead_id: vb-v7x6
phase: 3
attempt: 1-of-7

REQ-1: `xtask::ui_release_gates::ai_release_includes_ui_release_gates` must execute `ai-release --bead vb-nf2u` under cargo test and nextest.

REQ-2: The test must validate the same required UI release subgates and evidence file; no gate weakening or fixture relaxation.

REQ-3: `moon run :doc` must pass from the isolated workspace.

REQ-4: Canonical product naming remains `velvet-ballistics`; no master-doc deferred UI semantics are changed.
