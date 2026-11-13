use vb_core::workflow::CompiledNodeKind;

#[test]
fn appended_wire_literals() -> Result<(), Box<dyn std::error::Error>> {
    check(&[34, 0, 1, 3, 2, 1, 4], "ForEachConcurrentStart")?;
    check(&[35, 2, 1, 3, 5], "TogetherConcurrentStart")?;
    check(&[36, 2, 1, 3, 5], "FirstSuccessStart")?;
    check(&[37, 0, 0], "ConcurrentLaneEnd")?;
    check(&[37, 0, 1, 2], "ConcurrentLaneEnd")?;
    check(&[38, 0], "ConcurrentGroupJoin")?;
    Ok(())
}

fn check(literal: &[u8], expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let kind = postcard::from_bytes::<CompiledNodeKind>(literal)?;
    let name = format!("{kind:?}");
    if !name.starts_with(expected) {
        return Err(format!("expected {expected}, got {name}").into());
    }
    let re = postcard::to_allocvec(&kind)?;
    if re.as_slice() != literal {
        return Err("roundtrip changed".into());
    }
    Ok(())
}
