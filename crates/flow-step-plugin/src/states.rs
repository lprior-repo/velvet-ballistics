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

    // -- StepMachineMeta --

    #[test]
    fn step_machine_meta_serialization_roundtrip() -> Result<(), serde_json::Error> {
        let meta = StepMachineMeta {
            start_at: SmolStr::from("StartState"),
            comment: Some("test machine".into()),
            timeout_seconds: Some(300),
        };
        let json = serde_json::to_string(&meta)?;
        assert!(json.contains("StartState"));
        assert!(json.contains("test machine"));

        let back: StepMachineMeta = serde_json::from_str(&json)?;
        assert_eq!(back.start_at.as_str(), "StartState");
        assert_eq!(back.comment.as_deref(), Some("test machine"));
        assert_eq!(back.timeout_seconds, Some(300));
        Ok(())
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

    // ========================================================================
    // BLACKHAT security review tests
    // ========================================================================

    /// BH-STATES-01 (HIGH): RetryPolicy.backoff_rate is f64 with no
    /// validation. NaN, Infinity, and negative values are constructible
    /// in Rust code without any validation. A NaN or Infinity backoff_rate
    /// causes exponential delay computation to produce nonsensical or
    /// unbounded retry delays, enabling denial-of-service through resource
    /// exhaustion or indefinite retry loops. While serde_json rejects
    /// NaN/Infinity in serialization, the in-memory type has no guard.
    /// Additionally, negative and zero rates pass through serde_json
    /// roundtrip without any issue.
    #[test]
    fn blackhat_retry_policy_accepts_dangerous_backoff_rates() {
        // Test rates that can survive serde_json roundtrip (non-NaN/Inf)
        let dangerous_rates: Vec<f64> = vec![-1.0, -100.0, 0.0, f64::MIN_POSITIVE];
        for rate in dangerous_rates {
            let policy = RetryPolicy {
                error_equals: vec![SmolStr::from("States.ALL")],
                interval_seconds: Some(1),
                max_attempts: Some(3),
                backoff_rate: Some(rate),
                max_delay_seconds: None,
            };
            let json = serde_json::to_string(&policy)
                .unwrap_or_else(|e| panic!("serialize at rate {rate}: {e}"));
            let back: RetryPolicy = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize at rate {rate}: {e}"));
            // BUG: Negative and zero backoff rates are accepted. Negative
            // rates cause shrinking delays, bypassing exponential backoff.
            // Zero rate means delay never increases regardless of attempts.
            assert!(
                back.backoff_rate.is_some(),
                "malicious backoff_rate {rate} was accepted without validation"
            );
        }

        // Verify that NaN and Infinity can be set in-memory (no validation).
        // serde_json won't serialize them, but the Rust type allows construction.
        let nan_policy = RetryPolicy {
            error_equals: vec![SmolStr::from("States.ALL")],
            interval_seconds: Some(1),
            max_attempts: Some(3),
            backoff_rate: Some(f64::NAN),
            max_delay_seconds: None,
        };
        // BUG: NaN can be set in-memory without any constructor validation.
        // If this value is used in arithmetic without serialization, it
        // propagates NaN through all delay calculations.
        assert!(nan_policy.backoff_rate.is_some_and(|r| r.is_nan()));

        let inf_policy = RetryPolicy {
            error_equals: vec![SmolStr::from("States.ALL")],
            interval_seconds: Some(1),
            max_attempts: Some(3),
            backoff_rate: Some(f64::INFINITY),
            max_delay_seconds: None,
        };
        // BUG: Infinity can be set in-memory. Used in delay computation,
        // this produces infinite retry delays.
        assert!(inf_policy.backoff_rate.is_some_and(|r| r.is_infinite()));
    }

    /// BH-STATES-02 (HIGH): timeout_seconds, heartbeat_seconds,
    /// interval_seconds, max_delay_seconds are u32. A value of u32::MAX
    /// (4,294,967,295 seconds ~ 136 years) is accepted. When combined with
    /// backoff_rate in retry delay computation, multiplying interval_seconds
    /// by backoff_rate easily overflows, wrapping to a small value and
    /// bypassing the intended delay. Additionally, u32::MAX timeout makes a
    /// state machine effectively hang forever.
    #[test]
    fn blackhat_timeout_fields_accept_u32_max_causing_effective_hang() {
        let extreme_values = [u32::MAX, u32::MAX - 1];
        for &val in &extreme_values {
            let meta = StepMachineMeta {
                start_at: SmolStr::from("S"),
                comment: None,
                timeout_seconds: Some(val),
            };
            let json = serde_json::to_string(&meta).unwrap_or_else(|e| panic!("serialize: {e}"));
            let back: StepMachineMeta =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
            // BUG: u32::MAX seconds is ~136 years. This is accepted as a
            // valid timeout, effectively disabling the timeout safeguard.
            assert_eq!(back.timeout_seconds, Some(val));

            let task = TaskStateData {
                resource: "test".into(),
                timeout_seconds: Some(val),
                heartbeat_seconds: Some(val),
                retry: vec![],
                catch: vec![],
                next: None,
                end: true,
            };
            let task_json =
                serde_json::to_string(&task).unwrap_or_else(|e| panic!("serialize task: {e}"));
            let task_back: TaskStateData = serde_json::from_str(&task_json)
                .unwrap_or_else(|e| panic!("deserialize task: {e}"));
            assert_eq!(task_back.timeout_seconds, Some(val));
            assert_eq!(task_back.heartbeat_seconds, Some(val));
        }
    }

    /// BH-STATES-03 (HIGH): TaskStateData allows both `next` to be set AND
    /// `end` to be true simultaneously, or neither to be set. In ASL
    /// semantics, exactly one of `next` or `end` must be specified (except
    /// for terminal states). Having both set creates ambiguous control flow;
    /// having neither means the state is a dead end that silently drops
    /// execution. There is no validation on deserialization.
    #[test]
    fn blackhat_task_state_allows_inconsistent_next_and_end() {
        // Case 1: Both next AND end = true (ambiguous flow)
        let ambiguous = TaskStateData {
            resource: "arn:test".into(),
            timeout_seconds: None,
            heartbeat_seconds: None,
            retry: vec![],
            catch: vec![],
            next: Some(SmolStr::from("NextState")),
            end: true,
        };
        let json = serde_json::to_string(&ambiguous).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: TaskStateData =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: This deserializes successfully, but semantically the state
        // both terminates AND transitions -- ambiguous control flow.
        assert!(back.next.is_some());
        assert!(back.end);

        // Case 2: Neither next nor end (dead end, silent execution drop)
        let dead_end = TaskStateData {
            resource: "arn:test".into(),
            timeout_seconds: None,
            heartbeat_seconds: None,
            retry: vec![],
            catch: vec![],
            next: None,
            end: false,
        };
        let json2 = serde_json::to_string(&dead_end).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back2: TaskStateData =
            serde_json::from_str(&json2).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: No validation error. State has no exit path.
        assert!(back2.next.is_none());
        assert!(!back2.end);
    }

    /// BH-STATES-04 (MEDIUM): MapStateData.max_concurrency is Option<u32>
    /// with no upper bound. u32::MAX means ~4 billion concurrent executions,
    /// enabling resource exhaustion denial-of-service. ASL spec recommends
    /// practical limits.
    #[test]
    fn blackhat_map_state_accepts_extreme_max_concurrency() {
        let map = MapStateData {
            mode: MapMode::Inline,
            iterator: SmolStr::from("processor"),
            items_path: None,
            max_concurrency: Some(u32::MAX),
            retry: vec![],
            catch: vec![],
            next: None,
            end: true,
        };
        let json = serde_json::to_string(&map).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: MapStateData =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: u32::MAX concurrent executions accepted without validation.
        // This enables denial-of-service through resource exhaustion.
        assert_eq!(back.max_concurrency, Some(u32::MAX));
    }

    /// BH-STATES-05 (MEDIUM): TaskStateData.resource is an unvalidated
    /// String. Arbitrary strings including path traversal ("../../etc/passwd"),
    /// script injection, and empty strings are accepted. When used to resolve
    /// an ARN or endpoint, this enables server-side request forgery (SSRF)
    /// or local file inclusion.
    #[test]
    fn blackhat_task_resource_accepts_arbitrary_strings() {
        let malicious_resources: Vec<String> = vec![
            String::new(),
            "../../etc/passwd".to_string(),
            "file:///etc/shadow".to_string(),
            "http://169.254.169.254/latest/meta-data/".to_string(),
            "javascript:alert(1)".to_string(),
            "\x00null\x00bytes".to_string(),
            "a".repeat(1_000_000),
        ];
        for resource in &malicious_resources {
            let task = TaskStateData {
                resource: resource.clone(),
                timeout_seconds: None,
                heartbeat_seconds: None,
                retry: vec![],
                catch: vec![],
                next: None,
                end: true,
            };
            let json = serde_json::to_string(&task).unwrap_or_else(|e| panic!("serialize: {e}"));
            let back: TaskStateData =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
            // BUG: All malicious resource strings are accepted without
            // ARN format validation, enabling SSRF and injection attacks.
            assert_eq!(back.resource, *resource);
        }
    }

    /// BH-STATES-06 (MEDIUM): ChoiceRule.variable is Option<String>. A
    /// ChoiceRule with variable=None and condition=None is accepted, but
    /// cannot be evaluated at runtime. This creates a dead code path in
    /// the choice state that always falls through to default, potentially
    /// bypassing intended branching logic.
    #[test]
    fn blackhat_choice_rule_accepts_unevaluable_rules() {
        let rule = ChoiceRule {
            variable: None,
            next: SmolStr::from("Target"),
            condition: None,
        };
        let json = serde_json::to_string(&rule).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: ChoiceRule =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: A choice rule without a variable and without a condition
        // cannot be evaluated. This should fail validation.
        assert!(back.variable.is_none());
        assert!(back.condition.is_none());
    }

    /// BH-STATES-07 (MEDIUM): FailStateData.error and cause are
    /// Option<String> with no length or content validation. Extremely long
    /// strings or strings containing control characters are accepted. When
    /// these are logged or transmitted, they can cause log forging, denial
    /// of service through oversized log entries, or header injection.
    #[test]
    fn blackhat_fail_state_accepts_oversized_error_strings() {
        let oversized = "A".repeat(1_000_000);
        let with_control = "error\r\n200 OK\r\nInjected-Header: malicious".to_string();
        for error_str in [&oversized, &with_control] {
            let fail = FailStateData {
                error: Some(error_str.clone()),
                cause: Some(error_str.clone()),
            };
            let json = serde_json::to_string(&fail).unwrap_or_else(|e| panic!("serialize: {e}"));
            let back: FailStateData =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
            // BUG: No length or content validation on error/cause fields.
            assert_eq!(back.error.as_deref(), Some(error_str.as_str()));
        }
    }

    /// BH-STATES-08 (LOW): ParallelStateData.branches and other Vec fields
    /// have no size limit. A Vec with millions of entries is accepted,
    /// enabling memory exhaustion attacks through deserialization of a
    /// crafted JSON document.
    #[test]
    fn blackhat_parallel_state_accepts_large_branches_vec() {
        let many_branches: Vec<SmolStr> = (0..100)
            .map(|i| SmolStr::from(format!("branch_{i}")))
            .collect();
        let parallel = ParallelStateData {
            branches: many_branches,
            retry: vec![],
            catch: vec![],
            next: None,
            end: true,
        };
        let json = serde_json::to_string(&parallel).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: ParallelStateData =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: No maximum size enforced on branches vec.
        assert_eq!(back.branches.len(), 100);
    }

    /// BH-STATES-09 (LOW): WaitStateData accepts both `seconds` and
    /// `timestamp`, or `seconds_path` and `timestamp_path`, simultaneously.
    /// ASL semantics require exactly one wait condition. Multiple
    /// conflicting wait conditions create ambiguous behavior.
    #[test]
    fn blackhat_wait_state_accepts_conflicting_wait_conditions() {
        let conflicting = WaitStateData {
            seconds: Some(10),
            timestamp: Some("2024-01-01T00:00:00Z".into()),
            seconds_path: None,
            timestamp_path: None,
            next: None,
            end: true,
        };
        let json = serde_json::to_string(&conflicting).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: WaitStateData =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: Both seconds and timestamp are set -- which one takes effect?
        assert!(back.seconds.is_some());
        assert!(back.timestamp.is_some());
    }

    /// BH-STATES-10 (LOW): StepMachineMeta.start_at is SmolStr with no
    /// validation. An empty start_at or a start_at referencing a
    /// non-existent state is accepted, leading to a state machine that
    /// cannot begin execution.
    #[test]
    fn blackhat_step_machine_meta_accepts_empty_start_at() {
        let meta = StepMachineMeta {
            start_at: SmolStr::from(""),
            comment: None,
            timeout_seconds: None,
        };
        let json = serde_json::to_string(&meta).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: StepMachineMeta =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        // BUG: Empty start_at accepted -- state machine has no entry point.
        assert_eq!(back.start_at.as_str(), "");
    }

    /// BH-STATES-11 (INFO): ConditionExpr.value is serde_json::Value which
    /// accepts any JSON value including deeply nested objects and arrays.
    /// When used in comparison operations, deeply nested values can cause
    /// stack overflow during recursive comparison.
    #[test]
    fn blackhat_condition_expr_accepts_deeply_nested_json_value() {
        let deeply_nested: serde_json::Value = {
            let mut val = serde_json::json!("leaf");
            for _ in 0..100 {
                val = serde_json::json!({ "nested": val });
            }
            val
        };
        let expr = ConditionExpr {
            kind: SmolStr::from("StringEquals"),
            value: deeply_nested,
        };
        let rule = ChoiceRule {
            variable: Some("$.x".into()),
            next: SmolStr::from("Go"),
            condition: Some(expr),
        };
        let json = serde_json::to_string(&rule).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: ChoiceRule =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert!(back.condition.is_some());
    }
}
