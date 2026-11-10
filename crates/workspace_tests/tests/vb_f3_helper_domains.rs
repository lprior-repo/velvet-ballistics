#![forbid(unsafe_code)]

use vb_expr::parser::{ExprAst, ExprHelper, ExprLiteral};
use vb_expr::typecheck::{ExprType, TypeContext, typecheck_expr};
use vb_expr::{ExprError, ExprResult};

fn helper(helper: ExprHelper, args: &[ExprType]) -> ExprResult<ExprType> {
    let mut context = TypeContext::new();
    let mut references = Vec::new();
    for (index, ty) in args.iter().enumerate() {
        let name = format!("$arg{index}").into_boxed_str();
        context.add_variable(name.clone(), *ty);
        references.push(ExprAst::Reference(name));
    }
    typecheck_expr(
        &ExprAst::Helper {
            name: helper,
            args: references.into_boxed_slice(),
        },
        &context,
    )
}

#[test]
fn helper_domains_reject_known_invalid_arguments() {
    for (name, args, expected, found) in [
        (
            ExprHelper::Length,
            vec![ExprType::I64],
            "text, list, object, or null",
            "i64",
        ),
        (
            ExprHelper::Count,
            vec![ExprType::Text],
            "list or null",
            "text",
        ),
        (
            ExprHelper::Empty,
            vec![ExprType::Bool],
            "text, list, object, or null",
            "boolean",
        ),
        (
            ExprHelper::Contains,
            vec![ExprType::Bool, ExprType::I64],
            "text or list",
            "boolean",
        ),
        (
            ExprHelper::Contains,
            vec![ExprType::Text, ExprType::I64],
            "text",
            "i64",
        ),
    ] {
        assert_eq!(
            helper(name, &args),
            Err(ExprError::TypeMismatch {
                expected: expected.into(),
                found: found.into(),
            }),
            "helper {name:?} arguments {args:?}"
        );
    }
}

#[test]
fn helper_domains_accept_supported_known_and_dynamic_arguments() -> ExprResult<()> {
    for ty in [
        ExprType::Text,
        ExprType::List,
        ExprType::Object,
        ExprType::Null,
        ExprType::Unknown,
    ] {
        assert_eq!(helper(ExprHelper::Length, &[ty])?, ExprType::I64);
        assert_eq!(helper(ExprHelper::Empty, &[ty])?, ExprType::Bool);
    }
    for ty in [ExprType::List, ExprType::Null, ExprType::Unknown] {
        assert_eq!(helper(ExprHelper::Count, &[ty])?, ExprType::I64);
    }
    for ty in [
        ExprType::I64,
        ExprType::Bool,
        ExprType::Null,
        ExprType::List,
        ExprType::Object,
    ] {
        assert_eq!(
            helper(ExprHelper::Contains, &[ExprType::List, ty])?,
            ExprType::Bool
        );
        assert_eq!(
            helper(ExprHelper::Contains, &[ExprType::Unknown, ty])?,
            ExprType::Bool
        );
    }
    assert_eq!(
        helper(ExprHelper::Contains, &[ExprType::Text, ExprType::Text])?,
        ExprType::Bool
    );
    Ok(())
}

#[test]
fn helper_arity_remains_checked_before_argument_domains() {
    let ast = ExprAst::Helper {
        name: ExprHelper::Contains,
        args: Box::from([ExprAst::Literal(ExprLiteral::Bool(false))]),
    };
    assert_eq!(
        typecheck_expr(&ast, &TypeContext::new()),
        Err(ExprError::HelperArityMismatch {
            helper: "contains".into(),
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn yaml_runtime_and_expression_helpers_agree() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::{
        CapabilitySet, ObjectField, RunId, SlotValue as V, StepBudget, Taint, ValueStore,
        new_run_frame,
    };
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;

    let mut store = ValueStore::new();
    let key = store.insert_symbol("field")?;
    let text = store.insert_symbol("hello")?;
    let needle = store.insert_symbol("ell")?;
    let list = store.insert_list(Box::from([V::I64(2), V::Bool(false)]))?;
    let empty = store.insert_object(Box::from([]))?;
    let object = store.insert_object(Box::from([ObjectField::clean(key, V::I64(2))]))?;
    for (name, keyword, args, expected) in [
        (
            ExprHelper::Exists,
            "exists",
            vec![V::Bool(false)],
            V::Bool(true),
        ),
        (
            ExprHelper::Exists,
            "exists",
            vec![V::Object(empty)],
            V::Bool(true),
        ),
        (ExprHelper::Length, "length", vec![V::Null], V::I64(0)),
        (ExprHelper::Count, "count", vec![V::Null], V::I64(0)),
        (ExprHelper::Empty, "empty", vec![V::Null], V::Bool(true)),
        (
            ExprHelper::Contains,
            "contains",
            vec![V::List(list), V::I64(2)],
            V::Bool(true),
        ),
        (
            ExprHelper::Contains,
            "contains",
            vec![V::List(list), V::I64(9)],
            V::Bool(false),
        ),
        (
            ExprHelper::Contains,
            "contains",
            vec![V::Symbol(text), V::Symbol(needle)],
            V::Bool(true),
        ),
        (
            ExprHelper::Has,
            "has",
            vec![V::Object(object), V::Symbol(key)],
            V::Bool(true),
        ),
        (
            ExprHelper::Has,
            "has",
            vec![V::Object(empty), V::Symbol(key)],
            V::Bool(false),
        ),
    ] {
        assert_eq!(
            vb_expr::eval::eval_helper_with_store(name, &args, &mut store.clone())?,
            expected
        );
        let expression = if args.len() == 1 {
            format!("{keyword}($input.subject)")
        } else {
            format!("{keyword}($input.subject, $input.other)")
        };
        let source = format!(
            "version: velvet-ballistics/v1\nname: helper_parity\nwhen: {{manual: {{}}}}\ninputs: {{subject: any, other: {{is: any, optional: true}}}}\nsteps:\n  - id: value\n    set: {{output: checked, value: '{expression}'}}\n  - id: done\n    finish: {{result: checked}}\n"
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let mut frame = new_run_frame(RunId::new(1), &workflow)?;
        for (declaration, value) in workflow.input_slots().iter().zip(&args) {
            frame.write_slot_with_taint(declaration.slot, *value, Taint::Clean)?;
        }
        let result = drive_deterministic_full(
            &workflow,
            &mut frame,
            &mut StepBudget::new(32),
            &mut store.clone(),
            &[],
            RetryPolicy::NEVER,
            &mut EvidenceCollector::new(),
            &mut CollectStates::new(),
            &CapabilitySet::empty(),
            1,
        )?;
        assert_eq!(result, RuntimeSignal::Finished(expected), "helper {name:?}");
    }
    Ok(())
}
