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
