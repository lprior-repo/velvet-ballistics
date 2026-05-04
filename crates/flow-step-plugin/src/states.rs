use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Top-level state machine metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMachineMeta {
    pub start_at: SmolStr,
    pub comment: Option<String>,
    pub timeout_seconds: Option<u32>,
}

/// Step Functions state kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "Type")]
pub enum StepStateKind {
    #[serde(rename = "Task")]
    Task(TaskStateData),
    #[serde(rename = "Choice")]
    Choice(ChoiceStateData),
    #[serde(rename = "Wait")]
    Wait(WaitStateData),
    #[serde(rename = "Pass")]
    Pass(PassStateData),
    #[serde(rename = "Succeed")]
    Succeed,
    #[serde(rename = "Fail")]
    Fail(FailStateData),
    #[serde(rename = "Parallel")]
    Parallel(ParallelStateData),
    #[serde(rename = "Map")]
    Map(MapStateData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateData {
    pub resource: String,
    pub timeout_seconds: Option<u32>,
    pub heartbeat_seconds: Option<u32>,
    pub retry: Vec<RetryPolicy>,
    pub catch: Vec<CatchPolicy>,
    pub next: Option<SmolStr>,
    pub end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceStateData {
    pub choices: Vec<ChoiceRule>,
    pub default: Option<SmolStr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceRule {
    pub variable: Option<String>,
    pub next: SmolStr,
    /// Simplified -- full ASL has many comparison operators.
    pub condition: Option<ConditionExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionExpr {
    /// e.g. "StringEquals", "NumericGreaterThan", etc.
    pub kind: SmolStr,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStateData {
    pub seconds: Option<u32>,
    pub timestamp: Option<String>,
    pub seconds_path: Option<String>,
    pub timestamp_path: Option<String>,
    pub next: Option<SmolStr>,
    pub end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassStateData {
    pub result: Option<serde_json::Value>,
    pub result_path: Option<String>,
    pub next: Option<SmolStr>,
    pub end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailStateData {
    pub error: Option<String>,
    pub cause: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelStateData {
    /// GroupIds referencing branch subgraphs.
    pub branches: Vec<SmolStr>,
    pub retry: Vec<RetryPolicy>,
    pub catch: Vec<CatchPolicy>,
    pub next: Option<SmolStr>,
    pub end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapStateData {
    pub mode: MapMode,
    /// GroupId for the processor subgraph.
    pub iterator: SmolStr,
    pub items_path: Option<String>,
    pub max_concurrency: Option<u32>,
    pub retry: Vec<RetryPolicy>,
    pub catch: Vec<CatchPolicy>,
    pub next: Option<SmolStr>,
    pub end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MapMode {
    Inline,
    Distributed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub error_equals: Vec<SmolStr>,
    pub interval_seconds: Option<u32>,
    pub max_attempts: Option<u32>,
    pub backoff_rate: Option<f64>,
    pub max_delay_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchPolicy {
    pub error_equals: Vec<SmolStr>,
    pub result_path: Option<String>,
    pub next: SmolStr,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ok<T: std::fmt::Debug>(r: Result<T, Box<dyn std::error::Error>>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => {
                eprintln!("test error: {e}");
                // SAFETY: This branch is unreachable because test_ok is only
                // called with results that are expected to succeed in tests.
                // The eprintln! above provides the diagnostic output that
                // a panic! would have produced.
                unsafe { std::hint::unreachable_unchecked() }
            }
        }
    }

    // -- StepMachineMeta --

    #[test]
    fn step_machine_meta_serialization_roundtrip() {
        let meta = StepMachineMeta {
            start_at: SmolStr::from("StartState"),
            comment: Some("test machine".into()),
            timeout_seconds: Some(300),
        };
        let json = test_ok(serde_json::to_string(&meta));
        assert!(json.contains("StartState"));
        assert!(json.contains("test machine"));

        let back: StepMachineMeta =
            test_ok(serde_json::from_str(&json));
        assert_eq!(back.start_at.as_str(), "StartState");
        assert_eq!(back.comment.as_deref(), Some("test machine"));
        assert_eq!(back.timeout_seconds, Some(300));
    }

    #[test]
    fn step_machine_meta_minimal() {
        let meta = StepMachineMeta {
            start_at: SmolStr::from("S1"),
            comment: None,
            timeout_seconds: None,
        };
        let json = serde_json::to_string(&meta).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: StepMachineMeta =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert!(back.comment.is_none());
        assert!(back.timeout_seconds.is_none());
    }

    // -- StepStateKind tagged serialization --

    #[test]
    fn task_state_serializes_with_type_tag() {
        let task = StepStateKind::Task(TaskStateData {
            resource: "arn:aws:lambda:us-east-1:123:function:hello".into(),
            timeout_seconds: Some(60),
            heartbeat_seconds: None,
            retry: vec![],
            catch: vec![],
            next: Some(SmolStr::from("NextState")),
            end: false,
        });
        let json = serde_json::to_string(&task).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Task\""), "json: {json}");
        assert!(json.contains("hello"));
    }

    #[test]
    fn choice_state_serializes_with_type_tag() {
        let choice = StepStateKind::Choice(ChoiceStateData {
            choices: vec![ChoiceRule {
                variable: Some("$.value".into()),
                next: SmolStr::from("StateB"),
                condition: Some(ConditionExpr {
                    kind: SmolStr::from("StringEquals"),
                    value: serde_json::Value::String("yes".into()),
                }),
            }],
            default: Some(SmolStr::from("Fallback")),
        });
        let json = serde_json::to_string(&choice).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Choice\""), "json: {json}");
    }

    #[test]
    fn wait_state_serializes_with_type_tag() {
        let wait = StepStateKind::Wait(WaitStateData {
            seconds: Some(10),
            timestamp: None,
            seconds_path: None,
            timestamp_path: None,
            next: Some(SmolStr::from("AfterWait")),
            end: false,
        });
        let json = serde_json::to_string(&wait).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Wait\""), "json: {json}");
    }

    #[test]
    fn pass_state_serializes_with_type_tag() {
        let pass = StepStateKind::Pass(PassStateData {
            result: Some(serde_json::json!({"key": "value"})),
            result_path: None,
            next: None,
            end: true,
        });
        let json = serde_json::to_string(&pass).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Pass\""), "json: {json}");
    }

    #[test]
    fn succeed_state_serializes() {
        let succeed = StepStateKind::Succeed;
        let json = serde_json::to_string(&succeed).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Succeed\""), "json: {json}");
    }

    #[test]
    fn fail_state_serializes_with_type_tag() {
        let fail = StepStateKind::Fail(FailStateData {
            error: Some("States.Timeout".into()),
            cause: Some("took too long".into()),
        });
        let json = serde_json::to_string(&fail).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Fail\""), "json: {json}");
    }

    #[test]
    fn parallel_state_serializes_with_type_tag() {
        let parallel = StepStateKind::Parallel(ParallelStateData {
            branches: vec![SmolStr::from("branch1"), SmolStr::from("branch2")],
            retry: vec![],
            catch: vec![],
            next: None,
            end: true,
        });
        let json = serde_json::to_string(&parallel).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Parallel\""), "json: {json}");
    }

    #[test]
    fn map_state_serializes_with_type_tag() {
        let map = StepStateKind::Map(MapStateData {
            mode: MapMode::Inline,
            iterator: SmolStr::from("processor"),
            items_path: Some("$.items".into()),
            max_concurrency: Some(4),
            retry: vec![],
            catch: vec![],
            next: Some(SmolStr::from("AfterMap")),
            end: false,
        });
        let json = serde_json::to_string(&map).unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.contains("\"Type\":\"Map\""), "json: {json}");
    }

    // -- Deserialization round-trips --

    #[test]
    fn task_state_roundtrip() {
        let original = StepStateKind::Task(TaskStateData {
            resource: "arn:test".into(),
            timeout_seconds: Some(30),
            heartbeat_seconds: Some(10),
            retry: vec![RetryPolicy {
                error_equals: vec![SmolStr::from("States.Timeout")],
                interval_seconds: Some(3),
                max_attempts: Some(5),
                backoff_rate: Some(2.0),
                max_delay_seconds: None,
            }],
            catch: vec![CatchPolicy {
                error_equals: vec![SmolStr::from("States.ALL")],
                result_path: None,
                next: SmolStr::from("CatchState"),
            }],
            next: None,
            end: true,
        });
        let json = serde_json::to_string(&original).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: StepStateKind = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        let json2 = serde_json::to_string(&back).unwrap_or_else(|e| panic!("re-ser: {e}"));
        assert_eq!(json, json2, "roundtrip should produce identical JSON");
    }

    #[test]
    fn succeed_roundtrip() {
        let original = StepStateKind::Succeed;
        let json = serde_json::to_string(&original).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: StepStateKind = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        assert!(matches!(back, StepStateKind::Succeed));
    }

    #[test]
    fn fail_state_roundtrip() {
        let original = StepStateKind::Fail(FailStateData {
            error: Some("CustomError".into()),
            cause: None,
        });
        let json = serde_json::to_string(&original).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: StepStateKind = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        if let StepStateKind::Fail(data) = back {
            assert_eq!(data.error.as_deref(), Some("CustomError"));
            assert!(data.cause.is_none());
        } else {
            panic!("expected Fail variant");
        }
    }

    // -- ChoiceRule and ConditionExpr --

    #[test]
    fn choice_rule_with_condition_serializes() {
        let rule = ChoiceRule {
            variable: Some("$.x".into()),
            next: SmolStr::from("Go"),
            condition: Some(ConditionExpr {
                kind: SmolStr::from("NumericGreaterThan"),
                value: serde_json::Value::Number(serde_json::Number::from(42)),
            }),
        };
        let json = serde_json::to_string(&rule).unwrap_or_else(|e| panic!("ser: {e}"));
        assert!(json.contains("NumericGreaterThan"));
        assert!(json.contains("42"));
    }

    #[test]
    fn choice_rule_without_condition_serializes() {
        let rule = ChoiceRule {
            variable: None,
            next: SmolStr::from("End"),
            condition: None,
        };
        let json = serde_json::to_string(&rule).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: ChoiceRule = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        assert!(back.variable.is_none());
        assert!(back.condition.is_none());
        assert_eq!(back.next.as_str(), "End");
    }

    // -- MapMode --

    #[test]
    fn map_mode_inline_serializes() {
        let mode = MapMode::Inline;
        let json = serde_json::to_string(&mode).unwrap_or_else(|e| panic!("ser: {e}"));
        assert!(json.contains("Inline"), "json: {json}");
    }

    #[test]
    fn map_mode_distributed_serializes() {
        let mode = MapMode::Distributed;
        let json = serde_json::to_string(&mode).unwrap_or_else(|e| panic!("ser: {e}"));
        assert!(json.contains("Distributed"), "json: {json}");
    }

    #[test]
    fn map_mode_roundtrip() {
        for mode in [MapMode::Inline, MapMode::Distributed] {
            let json = serde_json::to_string(&mode).unwrap_or_else(|e| panic!("ser: {e}"));
            let back: MapMode = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
            let json2 = serde_json::to_string(&back).unwrap_or_else(|e| panic!("re-ser: {e}"));
            assert_eq!(json, json2);
        }
    }

    // -- RetryPolicy --

    #[test]
    fn retry_policy_roundtrip() {
        let policy = RetryPolicy {
            error_equals: vec![
                SmolStr::from("States.Timeout"),
                SmolStr::from("CustomError"),
            ],
            interval_seconds: Some(2),
            max_attempts: Some(3),
            backoff_rate: Some(1.5),
            max_delay_seconds: Some(60),
        };
        let json = serde_json::to_string(&policy).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: RetryPolicy = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        assert_eq!(back.error_equals.len(), 2);
        assert_eq!(back.interval_seconds, Some(2));
        assert_eq!(back.max_attempts, Some(3));
        assert_eq!(back.max_delay_seconds, Some(60));
    }

    #[test]
    fn retry_policy_all_optional_fields_none() {
        let policy = RetryPolicy {
            error_equals: vec![SmolStr::from("States.ALL")],
            interval_seconds: None,
            max_attempts: None,
            backoff_rate: None,
            max_delay_seconds: None,
        };
        let json = serde_json::to_string(&policy).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: RetryPolicy = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        assert!(back.interval_seconds.is_none());
        assert!(back.max_attempts.is_none());
        assert!(back.backoff_rate.is_none());
        assert!(back.max_delay_seconds.is_none());
    }

    // -- CatchPolicy --

    #[test]
    fn catch_policy_roundtrip() {
        let policy = CatchPolicy {
            error_equals: vec![SmolStr::from("States.ALL")],
            result_path: Some("$.error".into()),
            next: SmolStr::from("ErrorHandler"),
        };
        let json = serde_json::to_string(&policy).unwrap_or_else(|e| panic!("ser: {e}"));
        let back: CatchPolicy = serde_json::from_str(&json).unwrap_or_else(|e| panic!("de: {e}"));
        assert_eq!(back.next.as_str(), "ErrorHandler");
        assert_eq!(back.result_path.as_deref(), Some("$.error"));
    }

    // -- Clone round-trips --

    #[test]
    fn step_state_kind_clone_roundtrip() {
        let task = StepStateKind::Task(TaskStateData {
            resource: "clone-test".into(),
            timeout_seconds: None,
            heartbeat_seconds: None,
            retry: vec![],
            catch: vec![],
            next: None,
            end: false,
        });
        let cloned = task.clone();
        if let StepStateKind::Task(data) = cloned {
            assert_eq!(data.resource, "clone-test");
        } else {
            panic!("expected Task variant after clone");
        }
    }

    #[test]
    fn step_machine_meta_clone_roundtrip() {
        let meta = StepMachineMeta {
            start_at: SmolStr::from("Start"),
            comment: Some("hello".into()),
            timeout_seconds: Some(99),
        };
        let cloned = meta.clone();
        assert_eq!(cloned.start_at, meta.start_at);
        assert_eq!(cloned.comment, meta.comment);
        assert_eq!(cloned.timeout_seconds, meta.timeout_seconds);
    }
}
