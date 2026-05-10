#![forbid(unsafe_code)]

    use super::{CompileError, CompileErrors, SlotCompiler, SourceMark, YamlCompiler, YamlLimits};
    use super::{
        compile_to_generated_rust, compute_compiled_digest, lower_ask, lower_do, lower_finish,
        lower_set,
    };
    use vb_core::ConstValue;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprProgram, WorkflowParts};
    use vb_core::{CompiledWorkflow, ResourceContract};

    macro_rules! compile_test_fail {
        ($($arg:tt)*) => {{
            let failed = false;
            assert!(failed, $($arg)*);
            return;
        }};
    }

    const NESTED_SAVE_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: nested_save
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
      tags:
        - demo
        - fast
      metadata:
        attempts: 1
        active: true
        note: null
  - id: done
    finish:
      result: 0
"#;

    const OPTIONAL_TOP_LEVEL_FIELDS_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
inputs:
  value: text
vars:
  label: 1
secrets:
  api_key: API_KEY
result: {}
examples:
  - name: fixture
    input:
      value: 1
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    fn compile_with_inputs(inputs: &str) -> Result<CompiledWorkflow, CompileErrors> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        YamlCompiler::default().compile(source.as_bytes())
    }

    fn compile_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().compile(source) {
            Ok(_) => "compile unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn parse_ast_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().parse_ast(source) {
            Ok(_) => "parse_ast unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn assert_compile_parse_first_error(source: &[u8]) {
        assert_eq!(compile_error_text(source), parse_ast_error_text(source));
    }

    fn compile_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn parse_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn assert_compile_code(source: &[u8], expected: &'static str) -> Result<(), String> {
        let error = compile_first_error(source)?;
        ensure_equal(error.code(), expected)?;
        ensure_equal(error.diagnostic_code(), expected)
    }

