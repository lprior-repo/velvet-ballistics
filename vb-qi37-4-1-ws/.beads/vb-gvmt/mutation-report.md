# Mutation Evidence: vb-gvmt

## Command

```bash
cargo mutants -p vb_codegen -f "crates/vb_codegen/src/lib.rs" -F 'emit_(journal_contract|generated_runtime_api|run_until_blocked|action_resume_api|ask_resume_api|action_completion_spec|ask_answer_spec)' --in-place --timeout 60 --baseline skip -- post_
```

## Result

- Status: FAIL_UNVIABLE / DEFERRED
- Observed evidence: `35 mutants tested in 34s: 35 unviable`
- Representative log: `mutants.out/log/crates__vb_codegen__src__lib.rs_line_350_col_5.log` showed rustc `E0599` after a mutant replaced a `Result`-returning function with `CodegenResult::new()`.

## Interpretation

This is not mutation adequacy evidence. The scoped emitter mutation slice did not produce viable semantic mutants, so `MUTATION-PARITY-001` remains deferred rather than passed.
