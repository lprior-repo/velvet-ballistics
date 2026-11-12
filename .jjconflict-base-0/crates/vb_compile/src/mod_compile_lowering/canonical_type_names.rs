pub(super) fn type_name_static(name: &str) -> &'static str {
    match name {
        "null" => "null",
        "boolean" => "boolean",
        "i64" => "i64",
        "f64" => "f64",
        "text" => "text",
        "list" => "list",
        "object" => "object",
        "number" => "number",
        "unknown" => "unknown",
        "text or list" => "text or list",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::type_name_static;

    #[test]
    fn composite_label_is_preserved() {
        assert_eq!(type_name_static("text or list"), "text or list");
    }

    #[test]
    fn existing_scalar_labels_and_unknown_fallback_preserved() {
        assert_eq!(type_name_static("boolean"), "boolean");
        assert_eq!(type_name_static("i64"), "i64");
        assert_eq!(type_name_static("f64"), "f64");
        assert_eq!(type_name_static("unrecognized"), "unknown");
    }

    #[test]
    fn public_contains_receiver_error_preserves_composite_label() -> Result<(), String> {
        let source = "version: velvet-ballistics/v1\nname: bad_contains\nwhen: {manual: {}}\nsteps:\n  - id: probe\n    set: {output: answer, value: 'contains(true,1)'}\n  - id: done\n    finish: {result: '$steps.probe'}\n";
        match crate::compile_workflow(source.as_bytes()) {
            Err(errors) => assert_eq!(
                errors.0,
                vec![crate::CompileError::TypeMismatch {
                    field: "set.value",
                    expected: "text or list",
                    found: "boolean",
                }],
            ),
            Ok(_) => return Err("invalid Contains receiver was accepted".to_owned()),
        }
        Ok(())
    }
}
