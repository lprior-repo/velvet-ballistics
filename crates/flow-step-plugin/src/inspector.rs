/// Inspector field definition.
#[derive(Debug, Clone)]
pub struct InspectorField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Text,
    Number,
    Boolean,
    Select(Vec<String>),
    Expression,
    Duration,
    SecretRef,
}

/// Get inspector fields for a given state kind.
pub fn fields_for_kind(kind: &str) -> Vec<InspectorField> {
    let _ = kind;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_for_kind_returns_empty_for_known_kinds() {
        for kind in [
            "Task",
            "Choice",
            "Wait",
            "Pass",
            "Succeed",
            "Fail",
            "Parallel",
            "Map",
        ] {
            let fields = fields_for_kind(kind);
            assert!(
                fields.is_empty(),
                "expected empty fields for kind {kind:?}, got {} fields",
                fields.len()
            );
        }
    }

    #[test]
    fn fields_for_kind_returns_empty_for_unknown_kind() {
        let fields = fields_for_kind("NonExistent");
        assert!(fields.is_empty());
    }

    #[test]
    fn fields_for_kind_returns_empty_for_empty_string() {
        let fields = fields_for_kind("");
        assert!(fields.is_empty());
    }

    #[test]
    fn inspector_field_debug_format() {
        let field = InspectorField {
            id: "resource".into(),
            label: "Resource".into(),
            field_type: FieldType::Text,
            required: true,
        };
        let s = format!("{field:?}");
        assert!(s.contains("resource"), "debug output: {s}");
    }

    #[test]
    fn field_type_variants_debug_format() {
        let variants: Vec<FieldType> = vec![
            FieldType::Text,
            FieldType::Number,
            FieldType::Boolean,
            FieldType::Select(vec!["a".into(), "b".into()]),
            FieldType::Expression,
            FieldType::Duration,
            FieldType::SecretRef,
        ];
        for v in &variants {
            let s = format!("{v:?}");
            assert!(!s.is_empty(), "FieldType variant should have debug output");
        }
    }

    #[test]
    fn inspector_field_clone_is_equal() {
        let field = InspectorField {
            id: "timeout".into(),
            label: "Timeout".into(),
            field_type: FieldType::Number,
            required: false,
        };
        let cloned = field.clone();
        assert_eq!(cloned.id, field.id);
        assert_eq!(cloned.label, field.label);
        assert!(matches!(cloned.field_type, FieldType::Number));
        assert_eq!(cloned.required, field.required);
    }

    #[test]
    fn field_type_clone_is_equal() {
        let ft = FieldType::Select(vec!["x".into(), "y".into()]);
        let cloned = ft.clone();
        if let FieldType::Select(opts) = cloned {
            assert_eq!(opts.len(), 2);
        } else {
            panic!("expected Select variant");
        }
    }
}
