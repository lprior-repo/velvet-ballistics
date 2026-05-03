//! Lightweight document model for schema validation.

#![allow(unreachable_pub)]
pub struct WorkflowDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

#[derive(Clone)]
pub enum FieldValue {
    String(String),
    Sequence(Vec<StepDoc>),
    Mapping(Vec<(String, FieldValue)>),
    Empty,
}

#[derive(Clone)]
pub struct StepDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

impl WorkflowDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self { Self { fields } }
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::String(s) = v { Some(s.as_str()) } else { None } } else { None })
    }
    pub fn get_sequence(&self, field: &str) -> Option<&[StepDoc]> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::Sequence(s) = v { Some(s.as_slice()) } else { None } } else { None })
    }
    pub fn get_mapping(&self, field: &str) -> Option<&[(String, FieldValue)]> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::Mapping(e) = v { Some(e.as_slice()) } else { None } } else { None })
    }
    pub fn has_field(&self, field: &str) -> bool { self.fields.iter().any(|(n, _)| n == field) }
    pub fn field_names(&self) -> Vec<&str> { self.fields.iter().map(|(n, _)| n.as_str()).collect() }
}

impl StepDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self { Self { fields } }
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::String(s) = v { Some(s.as_str()) } else { None } } else { None })
    }
    pub fn field_names(&self) -> Vec<&str> { self.fields.iter().map(|(n, _)| n.as_str()).collect() }
}

#[cfg(test)]
mod doc_tests {
    use super::*;

    fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
        WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
        StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    // -- WorkflowDoc::from_pairs tests --

    #[test]
    fn from_pairs_creates_empty_workflow() {
        let doc = WorkflowDoc::from_pairs(vec![]);
        assert_eq!(doc.field_names(), Vec::<&str>::new());
    }

    #[test]
    fn from_pairs_preserves_field_order() {
        let doc = make_workflow(vec![
            ("z_field", FieldValue::Empty),
            ("a_field", FieldValue::Empty),
            ("m_field", FieldValue::Empty),
        ]);
        assert_eq!(doc.field_names(), vec!["z_field", "a_field", "m_field"]);
    }

    // -- WorkflowDoc::get_string tests --

    #[test]
    fn get_string_returns_value_for_string_field() {
        let doc = make_workflow(vec![("name", FieldValue::String("hello".to_owned()))]);
        assert_eq!(doc.get_string("name"), Some("hello"));
    }

    #[test]
    fn get_string_returns_none_for_missing_field() {
        let doc = make_workflow(vec![]);
        assert_eq!(doc.get_string("name"), None);
    }

    #[test]
    fn get_string_returns_none_for_non_string_field() {
        let doc = make_workflow(vec![("name", FieldValue::Empty)]);
        assert_eq!(doc.get_string("name"), None);
    }

    #[test]
    fn get_string_returns_none_for_sequence_field() {
        let doc = make_workflow(vec![("steps", FieldValue::Sequence(vec![]))]);
        assert_eq!(doc.get_string("steps"), None);
    }

    #[test]
    fn get_string_returns_none_for_mapping_field() {
        let doc = make_workflow(vec![("when", FieldValue::Mapping(vec![]))]);
        assert_eq!(doc.get_string("when"), None);
    }

    // -- WorkflowDoc::get_sequence tests --

    #[test]
    fn get_sequence_returns_steps_for_sequence_field() {
        let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
        let doc = make_workflow(vec![("steps", FieldValue::Sequence(vec![step]))]);
        let seq = doc.get_sequence("steps");
        assert!(seq.is_some());
        let Some(s) = seq else { return };
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn get_sequence_returns_none_for_missing() {
        let doc = make_workflow(vec![]);
        assert!(doc.get_sequence("steps").is_none());
    }

    #[test]
    fn get_sequence_returns_none_for_string_field() {
        let doc = make_workflow(vec![("steps", FieldValue::String("bad".to_owned()))]);
        assert!(doc.get_sequence("steps").is_none());
    }

    // -- WorkflowDoc::get_mapping tests --

    #[test]
    fn get_mapping_returns_entries_for_mapping_field() {
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        )]);
        let mapping = doc.get_mapping("when");
        assert!(mapping.is_some());
        let Some(m) = mapping else { return };
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn get_mapping_returns_none_for_missing() {
        let doc = make_workflow(vec![]);
        assert!(doc.get_mapping("when").is_none());
    }

    // -- WorkflowDoc::has_field tests --

    #[test]
    fn has_field_returns_true_for_present_field() {
        let doc = make_workflow(vec![("version", FieldValue::Empty)]);
        assert!(doc.has_field("version"));
    }

    #[test]
    fn has_field_returns_false_for_missing_field() {
        let doc = make_workflow(vec![]);
        assert!(!doc.has_field("version"));
    }

    // -- WorkflowDoc::field_names tests --

    #[test]
    fn field_names_returns_all_names() {
        let doc = make_workflow(vec![
            ("a", FieldValue::Empty),
            ("b", FieldValue::Empty),
            ("c", FieldValue::Empty),
        ]);
        assert_eq!(doc.field_names(), vec!["a", "b", "c"]);
    }

    // -- StepDoc tests --

    #[test]
    fn step_from_pairs_creates_step() {
        let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
        assert_eq!(step.get_string("id"), Some("s1"));
    }

    #[test]
    fn step_get_string_returns_none_for_missing() {
        let step = make_step(vec![("finish", FieldValue::Empty)]);
        assert_eq!(step.get_string("id"), None);
    }

    #[test]
    fn step_get_string_returns_none_for_non_string() {
        let step = make_step(vec![("id", FieldValue::Empty)]);
        assert_eq!(step.get_string("id"), None);
    }

    #[test]
    fn step_field_names_returns_all() {
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("do", FieldValue::Empty),
        ]);
        assert_eq!(step.field_names(), vec!["id", "do"]);
    }

    #[test]
    fn step_from_pairs_empty() {
        let step = StepDoc::from_pairs(vec![]);
        assert_eq!(step.field_names(), Vec::<&str>::new());
        assert_eq!(step.get_string("id"), None);
    }

    // -- FieldValue::Clone tests --

    #[test]
    fn field_value_string_is_cloneable() {
        let v = FieldValue::String("test".to_owned());
        let cloned = v.clone();
        assert!(matches!(cloned, FieldValue::String(s) if s == "test"));
    }

    #[test]
    fn field_value_sequence_is_cloneable() {
        let v = FieldValue::Sequence(vec![]);
        let cloned = v.clone();
        assert!(matches!(cloned, FieldValue::Sequence(s) if s.is_empty()));
    }

    #[test]
    fn field_value_mapping_is_cloneable() {
        let v = FieldValue::Mapping(vec![]);
        let cloned = v.clone();
        assert!(matches!(cloned, FieldValue::Mapping(m) if m.is_empty()));
    }

    #[test]
    fn field_value_empty_is_cloneable() {
        let v = FieldValue::Empty;
        let cloned = v.clone();
        assert!(matches!(cloned, FieldValue::Empty));
    }
}
