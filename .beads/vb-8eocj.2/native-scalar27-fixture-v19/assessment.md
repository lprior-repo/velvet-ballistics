# Native scalar27 V19 receipt

Exact tested candidate 7ffb75d9b9c2c49085f11b68f4a3f5cf0fb6a237. Actual strict result23 PASS /4 FAIL;0ignored/filtered; exit101;8.886s. Original20 all PASS;3of7new cases PASS;4new cases fail with exact CompileError::InvalidName { field: "step id", value: "finish" }. Their source helpers use reserved step ID finish; expectations remain typed boolean arithmetic errors / runtime12. No compiler defect inferred from these invalid source fixtures. Local correction requested to rename only new helper step IDs to done. No oracle weakening.

Root cargo metadata confirms do_action/negative/positive/type_identity helper modules are not standalone Cargo targets after subdirectory moves; canonical scalar entry target registered. Native test target compiles all27; broader package gates not run.

Moved original3helper files are byte-identical to ba8ac8da baseline; type_identity byte-identical to correctedfixture65514410 before move (55dbb3b2e897dbb68775986b3910a14c781a84bfac89d9190284557a373f6e04). Layout v2 author's report was emitted as fake XML text, not written; root independently verified actual source moves/hashes. No fabricated author report counted. FullF1, independent TEST review and main acceptance remain open.
