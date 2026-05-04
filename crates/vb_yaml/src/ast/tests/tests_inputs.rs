//! Inputs, vars, and secrets parsing tests.

#[cfg(test)]
mod tests {
    use super::super::parse::parse_workflow_ast;
    use super::super::types::*;

    fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    macro_rules! parse_ok {
        ($yaml:expr) => {
            match parse_workflow_ast($yaml) {
                Ok(value) => value,
                Err(error) => {
                    fail_assert!("parse failed: {error}");
                    return;
                }
            }
        };
    }

    macro_rules! first_item {
        ($values:expr, $label:expr) => {
            match $values.first() {
                Some(value) => value,
                None => {
                    fail_assert!("missing {}", $label);
                    return;
                }
            }
        };
    }

    #[test]
    fn parse_inputs_vars_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: full
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
                default: \"10\"
            vars:
              - name: acc
                value: \"0\"
            secrets:
              - name: api_key
                key: vault/api_key
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.inputs.len(), 1);
        let first_input = first_item!(wf.inputs, "input");
        assert_eq!(first_input.name, "count");
        assert_eq!(first_input.field_type.as_deref(), Some("u32"));
        assert_eq!(first_input.default.as_deref(), Some("10"));

        assert_eq!(wf.vars.len(), 1);
        let first_var = first_item!(wf.vars, "var");
        assert_eq!(first_var.name, "acc");

        assert_eq!(wf.secrets.len(), 1);
        let first_secret = first_item!(wf.secrets, "secret");
        assert_eq!(first_secret.name, "api_key");
        assert_eq!(first_secret.key.as_deref(), Some("vault/api_key"));
    }

    #[test]
    fn parse_workflow_with_inputs_and_defaults() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: inputs-test
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
                default: \"10\"
              - name: name
                type: string
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.inputs.len(), 2);
        assert_eq!(
            wf.inputs.first().map(|input| input.name.as_str()),
            Some("count")
        );
        assert_eq!(
            wf.inputs
                .first()
                .and_then(|input| input.field_type.as_deref()),
            Some("u32")
        );
        assert_eq!(
            wf.inputs.first().and_then(|input| input.default.as_deref()),
            Some("10")
        );
        assert_eq!(
            wf.inputs.get(1).map(|input| input.name.as_str()),
            Some("name")
        );
        assert_eq!(
            wf.inputs
                .get(1)
                .and_then(|input| input.field_type.as_deref()),
            Some("string")
        );
        assert_eq!(
            wf.inputs.get(1).and_then(|input| input.default.as_ref()),
            None
        );
    }

    #[test]
    fn parse_workflow_with_vars() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: vars-test
            when:
              manual: {}
            vars:
              - name: acc
                value: \"0\"
              - name: buf
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.vars.len(), 2);
        assert_eq!(wf.vars.first().map(|var| var.name.as_str()), Some("acc"));
        assert_eq!(
            wf.vars.first().and_then(|var| var.value.as_deref()),
            Some("0")
        );
        assert_eq!(wf.vars.get(1).map(|var| var.name.as_str()), Some("buf"));
        assert_eq!(wf.vars.get(1).and_then(|var| var.value.as_ref()), None);
    }

    #[test]
    fn parse_workflow_with_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: secrets-test
            when:
              manual: {}
            secrets:
              - name: api_key
                key: vault/api_key
              - name: db_pass
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.secrets.len(), 2);
        assert_eq!(
            wf.secrets.first().map(|secret| secret.name.as_str()),
            Some("api_key")
        );
        assert_eq!(
            wf.secrets.first().and_then(|secret| secret.key.as_deref()),
            Some("vault/api_key")
        );
        assert_eq!(
            wf.secrets.get(1).map(|secret| secret.name.as_str()),
            Some("db_pass")
        );
        assert_eq!(
            wf.secrets.get(1).and_then(|secret| secret.key.as_ref()),
            None
        );
    }
}