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
