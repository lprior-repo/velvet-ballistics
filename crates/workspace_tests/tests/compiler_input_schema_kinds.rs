use vb_core::InputSlotKind;

fn source(schema: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: schema_kind\nwhen:\n  manual: {{}}\ninputs:\n  value: {schema}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n"
    )
}

#[test]
fn schema_kind_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("text", InputSlotKind::Symbol),
        ("number", InputSlotKind::I64),
        ("boolean", InputSlotKind::Bool),
        ("object", InputSlotKind::Object),
        ("any", InputSlotKind::Any),
        ("list<any>", InputSlotKind::List),
        ("list<text>", InputSlotKind::List),
        ("list<number>", InputSlotKind::List),
        ("list<boolean>", InputSlotKind::List),
    ];

    for (schema, expected) in cases {
        let compiled = vb_compile::compile_workflow(source(schema).as_bytes())?;
        let slots = compiled.input_slots();
        if slots.len() != 1 {
            return Err(format!("expected 1 slot for {schema}, got {}", slots.len()).into());
        }
        let slot = slots.first().ok_or(std::io::Error::other("missing slot"))?;
        if slot.kind != expected {
            return Err(format!("slot kind mismatch for {schema}").into());
        }
    }

    Ok(())
}

#[test]
fn test_schema_kind_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("{is: text}", InputSlotKind::Symbol),
        ("{is: number}", InputSlotKind::I64),
        ("{is: number, default: 42}", InputSlotKind::I64),
        ("{is: boolean}", InputSlotKind::Bool),
        ("{is: any}", InputSlotKind::Any),
        ("{is: object, fields: {}}", InputSlotKind::Object),
        ("{is: list, of: number}", InputSlotKind::List),
    ];
    for (schema, expected) in cases {
        let compiled = vb_compile::compile_workflow(source(schema).as_bytes())?;
        let slots = compiled.input_slots();
        if slots.len() != 1 {
            return Err(format!("expected 1 slot, got {}", slots.len()).into());
        }
        let slot = slots.first().ok_or("missing slot")?;
        if slot.kind != expected {
            return Err(format!("expected {:?}, got {:?}", expected, slot.kind).into());
        }
    }
    Ok(())
}

fn full_source(inputs_yaml: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: schema_full\nwhen:\n  manual: {{}}\ninputs:\n{inputs_yaml}steps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n"
    )
}

fn single_constraint(
    schema: &str,
) -> Result<vb_core::CompiledInputConstraint, Box<dyn std::error::Error>> {
    let compiled = vb_compile::compile_workflow(source(schema).as_bytes())?;
    let slot = compiled.input_slots().first().ok_or("missing slot")?;
    Ok(slot.constraint)
}

#[test]
fn shorthand_schema_is_required() -> Result<(), Box<dyn std::error::Error>> {
    let constraint = single_constraint("number")?;
    assert_eq!(constraint, vb_core::CompiledInputConstraint::required());
    Ok(())
}

#[test]
fn longform_keywords_are_retained() -> Result<(), Box<dyn std::error::Error>> {
    let constraint = single_constraint(
        "{is: number, min: 1, max: 10, optional: true, nullable: true, default: 5, secret: true}",
    )?;
    assert_eq!(
        constraint,
        vb_core::CompiledInputConstraint {
            optional: true,
            nullable: true,
            default_value: Some(vb_core::SlotValue::I64(5)),
            min: Some(1),
            max: Some(10),
            min_length: None,
            max_length: None,
            element: None,
            allow_extra: None,
            secret: true,
        }
    );
    let list = single_constraint("{is: list, of: number, min: 0, max: 4}")?;
    assert_eq!(list.element, Some(vb_core::InputSlotKind::I64));
    assert_eq!((list.min, list.max), (Some(0), Some(4)));
    assert!(list.requires_value());
    let text = single_constraint("{is: text, min_length: 1, max_length: 8}")?;
    assert_eq!((text.min_length, text.max_length), (Some(1), Some(8)));
    let object = single_constraint("{is: object, fields: {enabled: boolean}, extra: reject}")?;
    assert_eq!(object.allow_extra, Some(false));
    let boolean = single_constraint("{is: boolean, default: true, optional: true}")?;
    assert_eq!(boolean.default_value, Some(vb_core::SlotValue::Bool(true)));
    assert!(!boolean.requires_value());
    Ok(())
}

#[test]
fn digest_is_sensitive_to_each_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let base = "{is: number, min: 1, max: 10}";
    let base_digest = vb_compile::compile_workflow(source(base).as_bytes())?.digest();
    for variant in [
        "{is: number, min: 2, max: 10}",
        "{is: number, min: 1, max: 9}",
        "{is: number, min: 1, max: 10, optional: true}",
        "{is: number, min: 1, max: 10, nullable: true}",
        "{is: number, min: 1, max: 10, default: 5}",
        "{is: number, min: 1, max: 10, secret: true}",
    ] {
        let digest = vb_compile::compile_workflow(source(variant).as_bytes())?.digest();
        assert_ne!(
            digest, base_digest,
            "keyword change must alter digest: {variant}"
        );
    }
    Ok(())
}

#[test]
fn roundtrip_preserves_constraints() -> Result<(), Box<dyn std::error::Error>> {
    let compiled = vb_compile::compile_workflow(
        source("{is: number, min: 1, max: 10, optional: true, default: 5}").as_bytes(),
    )?;
    let parts = compiled.to_parts();
    let bytes = postcard::to_allocvec(&parts)?;
    let restored_parts = postcard::from_bytes::<vb_core::WorkflowParts>(&bytes)?;
    let restored = vb_core::CompiledWorkflow::try_from_parts(restored_parts)?;
    assert_eq!(restored.to_parts(), parts);
    let slot = restored.input_slots().first().ok_or("missing slot")?;
    assert_eq!(slot.constraint.min, Some(1));
    assert_eq!(slot.constraint.max, Some(10));
    assert_eq!(
        slot.constraint.default_value,
        Some(vb_core::SlotValue::I64(5))
    );
    Ok(())
}

#[test]
fn dangling_from_reference_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = full_source("  b: {is: number, from: ghost}\n");
    let errors = vb_compile::compile_workflow(yaml.as_bytes()).expect_err("must reject");
    assert!(
        errors
            .0
            .iter()
            .any(|e| e.to_string().contains("inputs.from")),
        "unexpected errors: {errors:?}"
    );
    Ok(())
}

#[test]
fn declared_from_reference_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = full_source("  a: number\n  b: {is: number, from: a}\n");
    let compiled = vb_compile::compile_workflow(yaml.as_bytes())?;
    assert_eq!(compiled.input_slots().len(), 2);
    let with_var = format!(
        "version: velvet-ballistics/v1\nname: schema_from_var\nwhen:\n  manual: {{}}\nvars:\n  v: 1\ninputs:\n  b: {{is: number, from: v}}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n"
    );
    let compiled = vb_compile::compile_workflow(with_var.as_bytes())?;
    assert_eq!(compiled.input_slots().len(), 1);
    Ok(())
}

#[test]
fn invalid_constraint_combinations_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for schema in [
        "{is: text, min: 1}",
        "{is: number, of: number}",
        "{is: list, of: number, extra: allow}",
        "{is: number, min_length: 1}",
        "{is: number, min: 10, max: 1}",
        "{is: list, of: number, max: -1}",
        "{is: number, min: 1, max: 10, default: 99}",
        "{is: number, pattern: x}",
        "{is: list}",
        "{is: number, optional: 1}",
    ] {
        assert!(
            vb_compile::compile_workflow(source(schema).as_bytes()).is_err(),
            "schema must be rejected: {schema}"
        );
    }
    Ok(())
}

#[test]
fn direct_api_parity_with_compiled_constraints() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::{RunId, SlotIdx, SlotValue, WorkflowDigest};
    let run = RunId::new(41);
    let artifact = WorkflowDigest::from_bytes([3; 32]);
    let source_digest = WorkflowDigest::from_bytes([4; 32]);
    let build = |schema: &str| vb_compile::compile_workflow(source(schema).as_bytes());
    let port = |compiled: &vb_core::CompiledWorkflow| {
        compiled
            .input_slots()
            .first()
            .map(|slot| slot.slot)
            .ok_or("missing slot")
    };

    // Required: missing rejected, supplied accepted.
    let required = build("number")?;
    let slot = port(&required)?;
    assert!(matches!(
        vb_storage::RunInputsRecord::new(run, artifact, source_digest, &required, Box::new([])),
        Err(vb_storage::JournalError::InputSchemaMismatch)
    ));
    let retained = vb_storage::RunInputsRecord::new(
        run,
        artifact,
        source_digest,
        &required,
        Box::from([(slot, SlotValue::I64(2))]),
    )?;
    assert_eq!(retained.inputs(), &[(slot, SlotValue::I64(2))]);

    // Optional: missing accepted.
    let optional = build("{is: number, optional: true}")?;
    let retained =
        vb_storage::RunInputsRecord::new(run, artifact, source_digest, &optional, Box::new([]))?;
    assert!(retained.inputs().is_empty());

    // Default: missing filled.
    let defaulted = build("{is: number, min: 1, max: 10, default: 5}")?;
    let slot = port(&defaulted)?;
    let retained =
        vb_storage::RunInputsRecord::new(run, artifact, source_digest, &defaulted, Box::new([]))?;
    assert_eq!(retained.inputs(), &[(slot, SlotValue::I64(5))]);

    // Null only when nullable; range enforced on supplied values.
    let nullable = build("{is: number, nullable: true}")?;
    let null_slot = port(&nullable)?;
    let retained = vb_storage::RunInputsRecord::new(
        run,
        artifact,
        source_digest,
        &nullable,
        Box::from([(null_slot, SlotValue::Null)]),
    )?;
    assert_eq!(retained.inputs(), &[(null_slot, SlotValue::Null)]);
    let range_slot = port(&defaulted)?;
    assert!(matches!(
        vb_storage::RunInputsRecord::new(
            run,
            artifact,
            source_digest,
            &defaulted,
            Box::from([(range_slot, SlotValue::I64(99))]),
        ),
        Err(vb_storage::JournalError::InputSchemaMismatch)
    ));
    Ok(())
}

#[test]
fn declared_handles_stay_rejected_for_scalar_wire() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::{RunId, SlotIdx, SlotValue, WorkflowDigest};
    let compiled = vb_compile::compile_workflow(source("{is: list, of: any, max: 4}").as_bytes())?;
    let slot = compiled.input_slots().first().ok_or("missing slot")?;
    assert_eq!(slot.kind, vb_core::InputSlotKind::List);
    assert_eq!(slot.constraint.element, Some(vb_core::InputSlotKind::Any));
    let port = slot.slot;
    for value in [
        SlotValue::List(vb_core::ListId::new(0)),
        SlotValue::Object(vb_core::ObjectId::new(0)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ] {
        assert!(matches!(
            vb_storage::RunInputsRecord::new(
                RunId::new(43),
                WorkflowDigest::from_bytes([5; 32]),
                WorkflowDigest::from_bytes([6; 32]),
                &compiled,
                Box::from([(port, value)]),
            ),
            Err(vb_storage::JournalError::InputSchemaMismatch)
        ));
    }
    Ok(())
}

#[test]
fn test_digest_stability_and_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let src = source("{is: number}");
    let c1 = vb_compile::compile_workflow(src.as_bytes())?;
    let c2 = vb_compile::compile_workflow(src.as_bytes())?;
    if c1.digest() != c2.digest() {
        return Err("digest not stable".into());
    }
    let c3 = vb_compile::compile_workflow(source("{is: boolean}").as_bytes())?;
    if c1.digest() == c3.digest() {
        return Err("different schemas should have different digests".into());
    }
    let parts = c1.to_parts();
    let bytes = postcard::to_allocvec(&parts)?;
    let restored_parts = postcard::from_bytes::<vb_core::WorkflowParts>(&bytes)?;
    let restored = vb_core::CompiledWorkflow::try_from_parts(restored_parts)?;
    let slots = restored.input_slots();
    if slots.len() != 1 {
        return Err("roundtrip lost input slot".into());
    }
    let slot = slots.first().ok_or("missing slot after roundtrip")?;
    if slot.kind != InputSlotKind::I64 {
        return Err("roundtrip changed slot kind".into());
    }
    Ok(())
}

#[test]
fn test_postcard_ordinals() -> Result<(), Box<dyn std::error::Error>> {
    let table = [
        (InputSlotKind::Null, 0u8),
        (InputSlotKind::Bool, 1),
        (InputSlotKind::I64, 2),
        (InputSlotKind::F64, 3),
        (InputSlotKind::Symbol, 4),
        (InputSlotKind::List, 5),
        (InputSlotKind::Object, 6),
        (InputSlotKind::Blob, 7),
        (InputSlotKind::Any, 8),
    ];
    for (kind, expected) in table {
        let bytes = postcard::to_allocvec(&kind)?;
        assert_eq!(bytes, vec![expected], "ordinal mismatch for {kind:?}");
    }
    Ok(())
}

#[test]
fn test_deterministic_compile_and_digest() -> Result<(), Box<dyn std::error::Error>> {
    let src = source("number");
    let compiled1 = vb_compile::compile_workflow(src.as_bytes())?;
    let compiled2 = vb_compile::compile_workflow(src.as_bytes())?;
    assert_eq!(compiled1.digest(), compiled2.digest());

    let parts = compiled1.to_parts();
    let zero = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    assert_ne!(parts.digest, zero);

    let mut parts_for_recompute = parts.clone();
    parts_for_recompute.digest = zero;
    let computed = vb_core::WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&parts_for_recompute)?).into(),
    );
    assert_eq!(computed, parts.digest);
    Ok(())
}

#[test]
fn test_constant_change_alters_digest() -> Result<(), Box<dyn std::error::Error>> {
    let src_a = source("number");
    let src_b = src_a.replace("value: '42'", "value: '43'");
    let compiled_a = vb_compile::compile_workflow(src_a.as_bytes())?;
    let compiled_b = vb_compile::compile_workflow(src_b.as_bytes())?;
    assert_ne!(compiled_a.digest(), compiled_b.digest());

    let parts_a = compiled_a.to_parts();
    let parts_b = compiled_b.to_parts();
    assert_ne!(parts_a.constants, parts_b.constants);
    Ok(())
}

#[test]
fn test_journal_rejects_mutated_payload() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let journal = vb_storage::FjallJournal::open(dir.path(), None)?;
    let src = source("number");
    let compiled = vb_compile::compile_workflow(src.as_bytes())?;
    let accepted =
        vb_storage::submit_artifact(&journal, &compiled, vb_core::RuntimePolicy::Strict)?;
    assert_eq!(accepted.ir, postcard::to_allocvec(&compiled.to_parts())?);

    let parts = compiled.to_parts();
    let mut parts_mut = parts.clone();
    let slot = parts_mut.input_slots.first_mut().ok_or("missing port")?;
    slot.kind = vb_core::InputSlotKind::Bool;
    let rebuilt = vb_core::CompiledWorkflow::try_from_parts(parts_mut)?;
    assert!(matches!(
        vb_storage::submit_artifact(&journal, &rebuilt, vb_core::RuntimePolicy::Strict),
        Err(vb_storage::JournalError::ArtifactChecksumMismatch)
    ));
    Ok(())
}

#[test]
fn test_input_slot_ordering() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = source("number").replace("  value: number", "  zulu: boolean\n  alpha: number");
    let compiled = vb_compile::compile_workflow(yaml.as_bytes())?;
    let kinds: Vec<vb_core::InputSlotKind> =
        compiled.input_slots().iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        vec![vb_core::InputSlotKind::Bool, vb_core::InputSlotKind::I64]
    );
    let slots = compiled.input_slots();
    for pair in slots.windows(2) {
        let [a, b] = pair else {
            return Err("pair".into());
        };
        assert!(a.slot.get() < b.slot.get());
    }
    Ok(())
}

#[test]
fn test_invalid_input_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let base = vb_compile::parse_workflow_source(&source("number"))?;
    let invalid_schemas = vec![
        vb_compile::AuthorValue::Text("list<garbage>".into()),
        vb_compile::AuthorValue::Text("unknown".into()),
        vb_compile::AuthorValue::Mapping(vec![]),
        vb_compile::AuthorValue::Mapping(vec![vb_compile::AuthorEntry {
            key: "is".into(),
            value: vb_compile::AuthorValue::Text("unknown".into()),
        }]),
    ];
    for invalid in invalid_schemas {
        let modified = vb_compile::WorkflowSource::new(vb_compile::WorkflowSourceParts {
            version: base.version().into(),
            name: base.name().into(),
            trigger: base.trigger().clone(),
            inputs: vec![vb_compile::InputField {
                key: "value".into(),
                value: invalid,
            }],
            vars: vec![],
            secrets: vec![],
            steps: base.steps().to_vec(),
            result: None,
            examples: vec![],
        });
        let result = vb_compile::compile_source(&modified);
        assert_eq!(
            result,
            Err(vb_compile::CompileErrors(vec![
                vb_compile::CompileError::InvalidInputSchema {
                    field: "inputs".into(),
                    expected: "an allowed input schema".into(),
                }
            ]))
        );
    }
    Ok(())
}

#[test]
fn any_schema_digest_and_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let any_src = "{is: any}";
    let num_src = "{is: number}";
    let obj_src = "{is: object, fields: {enabled: boolean}}";
    let any1 = vb_compile::compile_workflow(source(any_src).as_bytes())?;
    let any2 = vb_compile::compile_workflow(source(any_src).as_bytes())?;
    assert_eq!(any1.digest(), any2.digest());
    let num = vb_compile::compile_workflow(source(num_src).as_bytes())?;
    assert_ne!(any1.digest(), num.digest());
    assert_ne!(any1.to_parts().input_slots, num.to_parts().input_slots);
    let obj = vb_compile::compile_workflow(source(obj_src).as_bytes())?;
    assert_eq!(
        num.input_slots().first().ok_or("missing input port")?.kind,
        vb_core::InputSlotKind::I64
    );
    assert_eq!(
        obj.input_slots().first().ok_or("missing input port")?.kind,
        vb_core::InputSlotKind::Object
    );
    let parts = any1.to_parts();
    let enc = postcard::to_allocvec(&parts)?;
    let dec: vb_core::WorkflowParts = postcard::from_bytes(&enc)?;
    let recon = vb_core::CompiledWorkflow::try_from_parts(dec)?;
    assert_eq!(recon.to_parts(), parts);
    assert_eq!(
        recon
            .input_slots()
            .first()
            .ok_or("missing input port")?
            .kind,
        vb_core::InputSlotKind::Any
    );
    Ok(())
}
