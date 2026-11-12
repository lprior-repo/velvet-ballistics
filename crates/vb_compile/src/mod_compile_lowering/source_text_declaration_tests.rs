use super::*;
use crate::mod_compile_lowering::{CanonicalScope, compile_canonical_expression};
use vb_core::SymbolId;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Declarations {
    builder: SlotCompiler,
    outputs: HashMap<String, SlotIdx>,
    types: HashMap<SlotIdx, ExprType>,
}

fn declarations(vars: &str) -> Result<Declarations, Box<dyn std::error::Error>> {
    let source = crate::parse_workflow_source(&format!(
        "version: velvet-ballistics/v1\nname: variables\nwhen:\n  manual: {{}}\nvars:\n{vars}\nsteps:\n  - id: done\n    finish: {{ result: 0 }}\n"
    ))?;
    let mut state = Declarations {
        builder: SlotCompiler::new(),
        outputs: HashMap::new(),
        types: HashMap::new(),
    };
    prepare_source_declarations(
        &source,
        &mut state.outputs,
        &mut state.types,
        &mut state.builder,
        &mut ForEachInputs::new(),
    )?;
    Ok(state)
}

fn compile_step(state: &mut Declarations, text: &str) -> Result<(), crate::CompileErrors> {
    let steps = HashMap::new();
    let scope = CanonicalScope {
        outputs: &state.outputs,
        step_outputs: &steps,
        slot_types: &state.types,
        parent: None,
    };
    compile_canonical_expression(text, scope, &mut state.builder).map(|_| ())
}

#[test]
fn text_declarations_preserve_written_order_and_deduplicate() -> TestResult {
    let state = declarations("  z: key\n  a: other\n  repeated: key")?;
    assert_eq!(
        state.builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(
        state.builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("key"), Box::from("other"),])
    );
    Ok(())
}

#[test]
fn declarations_and_steps_share_the_same_prefix() -> TestResult {
    let mut state = declarations("  prefix: first\n  accent: é")?;
    compile_step(&mut state, "\"é\" == \"later\"")?;
    assert_eq!(
        state.builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(2)),
        ]
    );
    assert_eq!(
        state.builder.source_symbols.finish()?,
        Box::from([
            Box::<str>::from("first"),
            Box::from("é"),
            Box::from("later"),
        ])
    );
    Ok(())
}

#[test]
fn empty_text_variable_is_legal_and_reused_by_the_step() -> TestResult {
    let mut state = declarations("  empty: ''")?;
    compile_step(&mut state, "$vars.empty == \"\"")?;
    assert_eq!(
        state.builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(
        state.builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("")])
    );
    Ok(())
}

#[test]
fn scalar_legacy_variables_do_not_create_symbol_payloads() -> TestResult {
    let state = declarations("  empty: null\n  enabled: true\n  number: 7")?;
    assert_eq!(
        state.builder.constants,
        vec![ConstValue::Null, ConstValue::Bool(true), ConstValue::I64(7)]
    );
    assert_eq!(
        state.builder.source_symbols.finish()?,
        Box::<[Box<str>]>::default()
    );
    Ok(())
}

#[test]
fn unsupported_variable_container_retains_typed_rejection() {
    let mut builder = SlotCompiler::new();
    assert_eq!(
        scalar_variable("items", &AuthorValue::Sequence(vec![]), &mut builder),
        Err(CompileError::ExpressionLoweringUnsupported {
            feature:
                "vars.items currently requires a null, boolean, integer, or text static literal"
                    .into(),
        }),
    );
    assert!(builder.source_symbols.is_empty());
}
