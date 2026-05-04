#![forbid(unsafe_code)]
//! Step compilation primitives and shared types.
//!
//! Defines StepPrimitive, StepSpec, and the step_spec parser.

use saphyr::Yaml;

use super::slot_compiler::CompileError;

const RESERVED_NAMES: &[&str] = &[
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "result",
    "when",
    "item",
    "error",
    "summary",
    "cursor",
    "page",
    "event",
    "attempt",
    "attempts",
    "true",
    "false",
    "null",
    "run",
    "do",
    "set",
    "save",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPrimitive {
    Set,
    Run,
    Do,
    Save,
    Choose,
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    Wait,
    Ask,
    Finish,
}

impl StepPrimitive {
    #[must_use]
    pub fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" => Some(Self::Together),
            "collect" => Some(Self::Collect),
            "reduce" => Some(Self::Reduce),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Together => "together",
            Self::Collect => "collect",
            Self::Reduce => "reduce",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StepSpec<'a> {
    pub primitive: StepPrimitive,
    pub body: &'a Yaml<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum ChooseCondition {
    Slot(vb_core::SlotIdx),
    Literal(bool),
}

pub fn step_spec<'a>(step: &'a Yaml<'a>, index: usize) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    let mut selected = None;

    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        } else {
            validate_phase_zero_step_metadata(field, body, index)?;
        }
    }

    selected.ok_or(CompileError::MissingStepPrimitive { step: index })
}

fn validate_phase_zero_step_metadata(
    field: &str,
    body: &Yaml<'_>,
    step: usize,
) -> Result<(), CompileError> {
    match field {
        "id" => Ok(()),
        "name" => validate_step_display_name(body, step),
        "if" | "with" | "try_again" | "on_error" | "then" => {
            Err(CompileError::UnsupportedStepControlField {
                step,
                field: Box::<str>::from(field),
            })
        }
        _ => Err(CompileError::UnknownStepField {
            step,
            field: Box::<str>::from(field),
        }),
    }
}

fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    if body.as_str().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "name",
            expected: "a string",
        })
    }
}

#[allow(clippy::unnecessary_wraps)]
pub fn non_string_key_error() -> CompileError {
    CompileError::NonStringKey {
        mark: super::SourceMark::unavailable(),
    }
}

pub fn is_reserved_name(value: &str) -> bool {
    RESERVED_NAMES.contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::LoadableYamlNode;

    fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    // -- StepPrimitive::from_field round-trip coverage --

    #[test]
    fn from_field_maps_all_known_primitives() -> Result<(), String> {
        let cases: &[(&str, StepPrimitive)] = &[
            ("set", StepPrimitive::Set),
            ("run", StepPrimitive::Run),
            ("do", StepPrimitive::Do),
            ("save", StepPrimitive::Save),
            ("choose", StepPrimitive::Choose),
            ("for_each", StepPrimitive::ForEach),
            ("together", StepPrimitive::Together),
            ("collect", StepPrimitive::Collect),
            ("reduce", StepPrimitive::Reduce),
            ("repeat", StepPrimitive::Repeat),
            ("wait", StepPrimitive::Wait),
            ("ask", StepPrimitive::Ask),
            ("finish", StepPrimitive::Finish),
        ];
        for (field, expected) in cases {
            let result = StepPrimitive::from_field(field).ok_or_else(|| {
                format!("from_field({field:?}) returned None")
            })?;
            ensure(
                result == *expected,
                "from_field mapping mismatch",
            )?;
            ensure(
                result.as_str() == *field,
                "as_str round-trip mismatch",
            )?;
        }
        Ok(())
    }

    #[test]
    fn from_field_returns_none_for_unknown() -> Result<(), String> {
        for unknown in &["unknown", "collect ", "for each", "Collect", "CHOOSE", ""] {
            ensure(
                StepPrimitive::from_field(unknown).is_none(),
                "unknown field should map to None",
            )?;
        }
        Ok(())
    }

    // -- is_reserved_name --

    #[test]
    fn reserved_names_cover_core_keywords() -> Result<(), String> {
        let reserved = &[
            "input", "inputs", "vars", "secrets", "steps", "result",
            "when", "item", "error", "summary", "cursor", "page",
            "event", "attempt", "attempts", "true", "false", "null",
            "run", "do", "set", "save", "choose", "for_each",
            "together", "collect", "reduce", "repeat", "wait", "ask",
            "try_again", "on_error", "then", "finish",
        ];
        for name in reserved {
            ensure(
                is_reserved_name(name),
                "expected reserved",
            )?;
        }
        ensure(!is_reserved_name("my_step"), "user step should not be reserved")?;
        ensure(!is_reserved_name("step_1"), "user step should not be reserved")?;
        ensure(!is_reserved_name("a"), "short name should not be reserved")
    }

    // -- step_spec parsing from YAML --

    fn parse_step(yaml: &str) -> Result<StepSpec<'_>, String> {
        let docs = Yaml::load_from_str(yaml).map_err(|e| format!("yaml load: {e:?}"))?;
        let doc = docs.first().ok_or("empty yaml document")?;
        step_spec(doc, 0).map_err(|e| format!("step_spec: {e:?}"))
    }

    fn parse_step_err(yaml: &str) -> Result<CompileError, String> {
        let docs = Yaml::load_from_str(yaml).map_err(|e| format!("yaml load: {e:?}"))?;
        let doc = docs.first().ok_or("empty yaml document")?;
        match step_spec(doc, 0) {
            Ok(spec) => Err(format!("step_spec unexpectedly succeeded: {spec:?}")),
            Err(error) => Ok(error),
        }
    }

    #[test]
    fn step_spec_parses_do_primitive() -> Result<(), String> {
        let spec = parse_step("do:\n  action: 1\n  input: 0")?;
        ensure(spec.primitive == StepPrimitive::Do, "expected Do primitive")
    }

    #[test]
    fn step_spec_parses_finish_primitive() -> Result<(), String> {
        let spec = parse_step("finish:\n  result: 0")?;
        ensure(spec.primitive == StepPrimitive::Finish, "expected Finish primitive")
    }

    #[test]
    fn step_spec_parses_save_primitive() -> Result<(), String> {
        let spec = parse_step("save:\n  value: 42")?;
        ensure(spec.primitive == StepPrimitive::Save, "expected Save primitive")
    }

    #[test]
    fn step_spec_parses_choose_primitive() -> Result<(), String> {
        let spec = parse_step("choose:\n  condition: true\n  on_true: 1\n  on_false: 2")?;
        ensure(spec.primitive == StepPrimitive::Choose, "expected Choose primitive")
    }

    #[test]
    fn step_spec_parses_for_each_primitive() -> Result<(), String> {
        let spec = parse_step("for_each:\n  input: 0\n  item: 1\n  limit: 10")?;
        ensure(spec.primitive == StepPrimitive::ForEach, "expected ForEach primitive")
    }

    #[test]
    fn step_spec_parses_together_primitive() -> Result<(), String> {
        let spec = parse_step("together:\n  branches: [1, 2]")?;
        ensure(spec.primitive == StepPrimitive::Together, "expected Together primitive")
    }

    #[test]
    fn step_spec_parses_collect_primitive() -> Result<(), String> {
        let spec = parse_step("collect:\n  source: 0\n  limit: 10\n  page_size: 5")?;
        ensure(spec.primitive == StepPrimitive::Collect, "expected Collect primitive")
    }

    #[test]
    fn step_spec_parses_reduce_primitive() -> Result<(), String> {
        let spec = parse_step("reduce:\n  input: 0\n  accumulator: 1\n  initial: 0")?;
        ensure(spec.primitive == StepPrimitive::Reduce, "expected Reduce primitive")
    }

    #[test]
    fn step_spec_parses_repeat_primitive() -> Result<(), String> {
        let spec = parse_step("repeat:\n  max_attempts: 3")?;
        ensure(spec.primitive == StepPrimitive::Repeat, "expected Repeat primitive")
    }

    #[test]
    fn step_spec_parses_wait_primitive() -> Result<(), String> {
        let spec = parse_step("wait:\n  until: 5")?;
        ensure(spec.primitive == StepPrimitive::Wait, "expected Wait primitive")
    }

    #[test]
    fn step_spec_parses_ask_primitive() -> Result<(), String> {
        let spec = parse_step("ask:\n  prompt: 0\n  answer: 1")?;
        ensure(spec.primitive == StepPrimitive::Ask, "expected Ask primitive")
    }

    #[test]
    fn step_spec_rejects_scalar_step() -> Result<(), String> {
        let error = parse_step_err("\"just a string\"")?;
        ensure(
            matches!(error, CompileError::StepShape { step: 0 }),
            "scalar should produce StepShape error",
        )
    }

    #[test]
    fn step_spec_rejects_sequence_step() -> Result<(), String> {
        let error = parse_step_err("[1, 2, 3]")?;
        ensure(
            matches!(error, CompileError::StepShape { step: 0 }),
            "sequence should produce StepShape error",
        )
    }

    #[test]
    fn step_spec_rejects_missing_primitive() -> Result<(), String> {
        let error = parse_step_err("id: my_step\nname: My Step")?;
        ensure(
            matches!(error, CompileError::MissingStepPrimitive { step: 0 }),
            "metadata-only step should produce MissingStepPrimitive",
        )
    }

    #[test]
    fn step_spec_rejects_multiple_primitives() -> Result<(), String> {
        let error = parse_step_err("do:\n  action: 1\nsave:\n  value: 2")?;
        ensure(
            matches!(error, CompileError::MultipleStepPrimitives { step: 0 }),
            "two primitives should produce MultipleStepPrimitives",
        )
    }

    #[test]
    fn step_spec_allows_id_metadata_field() -> Result<(), String> {
        let spec = parse_step("id: my_step\ndo:\n  action: 1\n  input: 0")?;
        ensure(spec.primitive == StepPrimitive::Do, "id should be accepted as metadata")
    }

    #[test]
    fn step_spec_allows_name_metadata_field() -> Result<(), String> {
        let spec = parse_step("name: My Step\nfinish:\n  result: 0")?;
        ensure(spec.primitive == StepPrimitive::Finish, "name should be accepted as metadata")
    }

    #[test]
    fn step_spec_rejects_non_string_name_value() -> Result<(), String> {
        let error = parse_step_err("name: 42\nfinish:\n  result: 0")?;
        ensure(
            matches!(error, CompileError::StepFieldShape { step: 0, field: "name", .. }),
            "non-string name should produce StepFieldShape",
        )
    }

    #[test]
    fn step_spec_rejects_unsupported_control_fields() -> Result<(), String> {
        for control_field in &["if", "with", "try_again", "on_error", "then"] {
            let yaml = format!("{control_field}: something\nfinish:\n  result: 0");
            let error = parse_step_err(&yaml)?;
            ensure(
                matches!(error, CompileError::UnsupportedStepControlField { step: 0, .. }),
                &format!("{control_field} should produce UnsupportedStepControlField"),
            )?;
        }
        Ok(())
    }

    #[test]
    fn step_spec_rejects_unknown_metadata_field() -> Result<(), String> {
        let error = parse_step_err("bogus: true\nfinish:\n  result: 0")?;
        ensure(
            matches!(error, CompileError::UnknownStepField { step: 0, .. }),
            "unknown field should produce UnknownStepField",
        )
    }

    #[test]
    fn step_spec_body_points_to_primitive_value() -> Result<(), String> {
        let spec = parse_step("save:\n  value: 99")?;
        let body_mapping = spec.body.as_mapping();
        ensure(body_mapping.is_some(), "body should be a mapping for save")
    }

    // ========================================================================
    // Additional boundary and edge-case tests
    // ========================================================================

    // -- StepPrimitive::from_field: case sensitivity and whitespace --

    #[test]
    fn from_field_rejects_uppercase_variants() -> Result<(), String> {
        for field in &["Set", "RUN", "Do", "CHOOSE", "FINISH", "FOR_EACH"] {
            ensure(
                StepPrimitive::from_field(field).is_none(),
                "uppercase should not match",
            )?;
        }
        Ok(())
    }

    #[test]
    fn from_field_rejects_whitespace_padded() -> Result<(), String> {
        ensure(StepPrimitive::from_field(" set").is_none(), "leading space")?;
        ensure(StepPrimitive::from_field("set ").is_none(), "trailing space")?;
        ensure(StepPrimitive::from_field(" set ").is_none(), "both spaces")
    }

    // -- StepPrimitive::as_str: all variants produce correct strings --

    #[test]
    fn as_str_covers_every_variant() -> Result<(), String> {
        let cases: &[(StepPrimitive, &str)] = &[
            (StepPrimitive::Set, "set"),
            (StepPrimitive::Run, "run"),
            (StepPrimitive::Do, "do"),
            (StepPrimitive::Save, "save"),
            (StepPrimitive::Choose, "choose"),
            (StepPrimitive::ForEach, "for_each"),
            (StepPrimitive::Together, "together"),
            (StepPrimitive::Collect, "collect"),
            (StepPrimitive::Reduce, "reduce"),
            (StepPrimitive::Repeat, "repeat"),
            (StepPrimitive::Wait, "wait"),
            (StepPrimitive::Ask, "ask"),
            (StepPrimitive::Finish, "finish"),
        ];
        for (prim, expected) in cases {
            ensure(
                prim.as_str() == *expected,
                "as_str mismatch",
            )?;
        }
        Ok(())
    }

    // -- is_reserved_name: boundary checks --

    #[test]
    fn reserved_names_case_sensitive() -> Result<(), String> {
        ensure(!is_reserved_name("Do"), "Do should not be reserved")?;
        ensure(!is_reserved_name("SET"), "SET should not be reserved")?;
        ensure(!is_reserved_name("Input"), "Input should not be reserved")
    }

    #[test]
    fn reserved_names_empty_string() -> Result<(), String> {
        ensure(!is_reserved_name(""), "empty string should not be reserved")
    }

    #[test]
    fn reserved_names_no_partial_match() -> Result<(), String> {
        ensure(!is_reserved_name("set_value"), "set_value should not be reserved")?;
        ensure(!is_reserved_name("for_each_item"), "for_each_item should not be reserved")?;
        ensure(!is_reserved_name("run_step"), "run_step should not be reserved")
    }

    // -- step_spec: additional error paths --

    #[test]
    fn step_spec_rejects_null_step() -> Result<(), String> {
        let error = parse_step_err("null")?;
        ensure(
            matches!(error, CompileError::StepShape { step: 0 }),
            "null should produce StepShape",
        )
    }

    #[test]
    fn step_spec_rejects_empty_mapping() -> Result<(), String> {
        let error = parse_step_err("{}")?;
        ensure(
            matches!(error, CompileError::MissingStepPrimitive { step: 0 }),
            "empty mapping should produce MissingStepPrimitive",
        )
    }

    #[test]
    fn step_spec_rejects_non_string_key_in_mapping() -> Result<(), String> {
        let error = parse_step_err("1: value")?;
        ensure(
            matches!(error, CompileError::StepShape { step: 0 }),
            "non-string key should produce StepShape",
        )
    }

    #[test]
    fn step_spec_preserves_step_index_in_errors() -> Result<(), String> {
        let docs = Yaml::load_from_str("\"scalar\"").map_err(|e| format!("yaml: {e:?}"))?;
        let doc = docs.first().ok_or("empty doc")?;
        match step_spec(doc, 99) {
            Err(CompileError::StepShape { step: 99 }) => Ok(()),
            other => Err(format!("expected StepShape with step=99, got {other:?}")),
        }
    }

    #[test]
    fn step_spec_id_and_name_metadata_with_primitive() -> Result<(), String> {
        let spec = parse_step("id: my_step\nname: My Step\nset:\n  output: 0\n  value: 42")?;
        ensure(spec.primitive == StepPrimitive::Set, "set should be recognized with metadata")
    }

    #[test]
    fn step_spec_rejects_id_with_wrong_shape() -> Result<(), String> {
        // id accepts any value (it's just checked for presence), so this should work
        let spec = parse_step("id: 123\ndo:\n  action: 1\n  input: 0")?;
        ensure(spec.primitive == StepPrimitive::Do, "id with non-string value should still work")
    }

    #[test]
    fn step_spec_allows_boolean_name() -> Result<(), String> {
        // name must be a string
        let error = parse_step_err("name: true\ndo:\n  action: 1\n  input: 0")?;
        ensure(
            matches!(error, CompileError::StepFieldShape { step: 0, field: "name", .. }),
            "boolean name should produce StepFieldShape",
        )
    }

    #[test]
    fn step_spec_allows_null_name() -> Result<(), String> {
        let error = parse_step_err("name:\ndo:\n  action: 1\n  input: 0")?;
        ensure(
            matches!(error, CompileError::StepFieldShape { step: 0, field: "name", .. }),
            "null name should produce StepFieldShape",
        )
    }

    // -- step_spec: all primitive types produce correct StepSpec --

    #[test]
    fn step_spec_parses_set_primitive() -> Result<(), String> {
        let spec = parse_step("set:\n  output: 0\n  value: 42")?;
        ensure(spec.primitive == StepPrimitive::Set, "expected Set primitive")
    }

    #[test]
    fn step_spec_parses_run_primitive() -> Result<(), String> {
        let spec = parse_step("run:\n  action: 1")?;
        ensure(spec.primitive == StepPrimitive::Run, "expected Run primitive")
    }

    // -- step_spec: multiple primitives with metadata ordering --

    #[test]
    fn step_spec_rejects_multiple_primitives_even_with_metadata() -> Result<(), String> {
        let error = parse_step_err("id: s1\nset:\n  output: 0\ndo:\n  action: 1")?;
        ensure(
            matches!(error, CompileError::MultipleStepPrimitives { step: 0 }),
            "metadata should not suppress multiple primitives error",
        )
    }

    // -- non_string_key_error --

    #[test]
    fn non_string_key_error_returns_correct_variant() -> Result<(), String> {
        let error = non_string_key_error();
        ensure(
            matches!(error, CompileError::NonStringKey { .. }),
            "should return NonStringKey variant",
        )
    }

    #[test]
    fn non_string_key_error_mark_has_zero_fields() -> Result<(), String> {
        let error = non_string_key_error();
        if let CompileError::NonStringKey { mark } = error {
            ensure(mark.index == 0, "index should be 0")?;
            ensure(mark.end_index == 0, "end_index should be 0")?;
            ensure(mark.line == 0, "line should be 0")?;
            ensure(mark.column == 0, "column should be 0")?;
            ensure(!mark.available, "available should be false")
        } else {
            Err("expected NonStringKey".to_owned())
        }
    }

    // -- ChooseCondition: no direct construction test needed, but verify Debug/Clone/Copy --

    #[test]
    fn choose_condition_slot_construction() -> Result<(), String> {
        let slot = vb_core::SlotIdx::new(7);
        let cond = ChooseCondition::Slot(slot);
        ensure(
            matches!(cond, ChooseCondition::Slot(s) if s.as_u16() == 7),
            "Slot variant should hold the slot index",
        )
    }

    #[test]
    fn choose_condition_literal_construction() -> Result<(), String> {
        let cond_true = ChooseCondition::Literal(true);
        let cond_false = ChooseCondition::Literal(false);
        ensure(
            matches!(cond_true, ChooseCondition::Literal(true)),
            "Literal(true) should match",
        )?;
        ensure(
            matches!(cond_false, ChooseCondition::Literal(false)),
            "Literal(false) should match",
        )
    }

    // -- step_spec: body reference correctness --

    #[test]
    fn step_spec_body_contains_correct_primitive_data() -> Result<(), String> {
        let spec = parse_step("do:\n  action: 5\n  input: 3")?;
        let mapping = spec.body.as_mapping().ok_or("body should be mapping")?;
        let action_node = mapping.get(&Yaml::Value(saphyr::Scalar::String(Box::from("action"))));
        ensure(action_node.is_some(), "body should contain action field")
    }

    // -- step_spec: unsupported control field names carry field name --

    #[test]
    fn step_spec_unsupported_control_field_carries_name() -> Result<(), String> {
        for control_field in &["if", "with", "try_again", "on_error", "then"] {
            let yaml = format!("{control_field}: x\nset:\n  output: 0");
            let error = parse_step_err(&yaml)?;
            match error {
                CompileError::UnsupportedStepControlField { step: 0, field } => {
                    ensure(
                        field.as_ref() == *control_field,
                        "field name should be preserved in error",
                    )?;
                }
                other => return Err(format!("expected UnsupportedStepControlField, got {other:?}")),
            }
        }
        Ok(())
    }

    // -- step_spec: unknown field carries field name --

    #[test]
    fn step_spec_unknown_field_carries_name() -> Result<(), String> {
        let error = parse_step_err("xyz: 1\nset:\n  output: 0")?;
        match error {
            CompileError::UnknownStepField { step: 0, field } => {
                ensure(field.as_ref() == "xyz", "field name should be xyz")
            }
            other => Err(format!("expected UnknownStepField, got {other:?}")),
        }
    }
}
