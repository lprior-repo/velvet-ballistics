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
