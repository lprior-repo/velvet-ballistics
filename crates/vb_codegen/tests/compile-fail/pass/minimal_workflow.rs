#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Generated workflow - DO NOT EDIT
//! Produced by vb_codegen emit_rust_workflow

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotValue { Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(u32), Object(u32), Blob(u64) }

impl SlotValue {
    pub const fn is_true(&self) -> bool { matches!(self, Self::Bool(true)) }
    pub const fn type_name(&self) -> &'static str { match self { Self::Null => "null", Self::Bool(_) => "boolean", Self::I64(_) | Self::F64(_) => "number", Self::Symbol(_) => "symbol", Self::List(_) => "list", Self::Object(_) => "object", Self::Blob(_) => "blob" } }
}

#[derive(Debug)]
pub enum DriveError {
    InvalidProgramCounter,
    MissingNextStep,
    MissingOutputSlot,
    SlotNull,
    NoBranchMatched,
    ExpressionStackOverflow { max: u8 },
    TypeMismatch { expected: &'static str, found: &'static str },
    DivisionByZero,
    IntegerOverflow,
    ExpressionStackUnderflow,
    IterationLimitExceeded { resource: &'static str },
    ListStoreOverflow,
    InvalidListHandle,
    ActionSuspend { action_id: u16, input_slot: u16 },
    UnknownAction,
    UnsupportedPrimitive { primitive: &'static str },
    UnsupportedExpressionOp { op: &'static str },
    InvalidCompiledWorkflow { reason: &'static str },
}

enum StepOutcome { Continue(u16), Finished(SlotValue) }

const MAX_EXPRESSION_STACK: usize = 64;
struct ExprStack { values: [SlotValue; MAX_EXPRESSION_STACK], len: u8, capacity: u8 }
impl ExprStack {
    fn new(capacity: u8) -> Result<Self, DriveError> { if usize::from(capacity) <= MAX_EXPRESSION_STACK { Ok(Self { values: [SlotValue::Null; MAX_EXPRESSION_STACK], len: 0, capacity }) } else { Err(DriveError::ExpressionStackOverflow { max: capacity }) } }
    fn push(&mut self, value: SlotValue) -> Result<(), DriveError> { if self.len >= self.capacity { return Err(DriveError::ExpressionStackOverflow { max: self.capacity }); } let index = usize::from(self.len); match self.values.get_mut(index) { Some(slot) => *slot = value, None => return Err(DriveError::ExpressionStackOverflow { max: self.capacity }), } self.len = self.len.checked_add(1).ok_or(DriveError::ExpressionStackOverflow { max: self.capacity })?; Ok(()) }
    fn pop(&mut self) -> Option<SlotValue> { if self.len == 0 { return None; } self.len = self.len.checked_sub(1)?; self.values.get(usize::from(self.len)).copied() }
}

#[derive(Debug, Clone, Copy)]
struct ListRecord { start: u32, len: u32 }
struct ListStore { records: [Option<ListRecord>; LIST_STORE_RECORD_CAPACITY], values: [SlotValue; LIST_STORE_VALUE_CAPACITY], record_len: u32, value_len: u32 }
impl ListStore {
    fn new() -> Self { Self { records: [None; LIST_STORE_RECORD_CAPACITY], values: [SlotValue::Null; LIST_STORE_VALUE_CAPACITY], record_len: 0, value_len: 0 } }
    fn insert_items(&mut self, items: &[SlotValue]) -> Result<u32, DriveError> { let start = self.value_len; let item_count = u32::try_from(items.len()).map_err(|_| DriveError::ListStoreOverflow)?; let end = start.checked_add(item_count).ok_or(DriveError::ListStoreOverflow)?; let end_index = usize::try_from(end).map_err(|_| DriveError::ListStoreOverflow)?; if end_index > LIST_STORE_VALUE_CAPACITY { return Err(DriveError::ListStoreOverflow); } self.copy_items(start, items)?; self.value_len = end; self.insert_record(start, item_count) }
    fn copy_items(&mut self, start: u32, items: &[SlotValue]) -> Result<(), DriveError> { let mut cursor = 0usize; while cursor < items.len() { let cursor_u32 = u32::try_from(cursor).map_err(|_| DriveError::ListStoreOverflow)?; let target_offset = start.checked_add(cursor_u32).ok_or(DriveError::ListStoreOverflow)?; let target_index = usize::try_from(target_offset).map_err(|_| DriveError::ListStoreOverflow)?; let value = items.get(cursor).copied().ok_or(DriveError::ListStoreOverflow)?; match self.values.get_mut(target_index) { Some(target) => *target = value, None => return Err(DriveError::ListStoreOverflow), } cursor = cursor.checked_add(1).ok_or(DriveError::ListStoreOverflow)?; } Ok(()) }
    fn insert_record(&mut self, start: u32, len: u32) -> Result<u32, DriveError> { let handle = self.record_len; let index = usize::try_from(handle).map_err(|_| DriveError::ListStoreOverflow)?; match self.records.get_mut(index) { Some(slot) => *slot = Some(ListRecord { start, len }), None => return Err(DriveError::ListStoreOverflow), } self.record_len = self.record_len.checked_add(1).ok_or(DriveError::ListStoreOverflow)?; Ok(handle) }
    fn record(&self, handle: u32) -> Result<Option<ListRecord>, DriveError> { if handle >= self.record_len { return Ok(None); } let index = usize::try_from(handle).map_err(|_| DriveError::InvalidListHandle)?; match self.records.get(index).copied() { Some(Some(record)) => Ok(Some(record)), Some(None) | None => Err(DriveError::InvalidListHandle), } }
    fn len(&self, handle: u32) -> Result<Option<u32>, DriveError> { match self.record(handle)? { Some(record) => Ok(Some(record.len)), None => Ok(None), } }
    fn first(&self, handle: u32) -> Result<Option<SlotValue>, DriveError> { let Some(record) = self.record(handle)? else { return Ok(None); }; if record.len == 0 { return Ok(None); } let index = usize::try_from(record.start).map_err(|_| DriveError::InvalidListHandle)?; self.values.get(index).copied().map(Some).ok_or(DriveError::InvalidListHandle) }
    fn tail(&mut self, handle: u32) -> Result<Option<u32>, DriveError> { let Some(record) = self.record(handle)? else { return Ok(None); }; let (start, len) = if record.len == 0 { (record.start, 0) } else { let next_start = record.start.checked_add(1).ok_or(DriveError::ListStoreOverflow)?; let next_len = record.len.checked_sub(1).ok_or(DriveError::ListStoreOverflow)?; (next_start, next_len) }; self.insert_record(start, len).map(Some) }
}

fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> { read_slot_optional(slots, slot).ok_or(DriveError::SlotNull) }
fn read_slot_optional(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Option<SlotValue> { slots.get(usize::from(slot)).copied().flatten() }
fn write_slot(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, value: Option<SlotValue>) -> Result<(), DriveError> { match slots.get_mut(usize::from(slot)) { Some(target) => { *target = value; Ok(()) }, None => Err(DriveError::InvalidCompiledWorkflow { reason: "slot index out of bounds" }), } }
fn read_const(index: u16) -> Result<SlotValue, DriveError> { CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow { reason: "constant index out of bounds" }) }
fn expect_list_value(value: SlotValue) -> Result<u32, DriveError> { match value { SlotValue::List(handle) => Ok(handle), other => Err(DriveError::TypeMismatch { expected: "list", found: other.type_name() }), } }
fn list_item_count(list_store: &ListStore, handle: u32) -> Result<u32, DriveError> { match list_store.len(handle)? { Some(len) => Ok(len), None => Err(DriveError::InvalidListHandle), } }
fn first_list_item(list_store: &ListStore, handle: u32, count: u32) -> Result<SlotValue, DriveError> { if count == 0 { return Err(DriveError::InvalidListHandle); } match list_store.first(handle)? { Some(value) => Ok(value), None => Err(DriveError::InvalidListHandle), } }
fn tail_list_handle(list_store: &mut ListStore, handle: u32) -> Result<u32, DriveError> { match list_store.tail(handle)? { Some(tail) => Ok(tail), None => Err(DriveError::InvalidListHandle), } }

fn symbol_contains(_haystack: u32, _needle: u32) -> bool { _haystack == _needle }
fn symbol_starts_with(_haystack: u32, _prefix: u32) -> bool { _haystack == _prefix }
fn symbol_ends_with(_haystack: u32, _suffix: u32) -> bool { _haystack == _suffix }

// --- Typed ID constants ---
const WORKFLOW_SLOT_COUNT: usize = 1;
const WORKFLOW_NODE_COUNT: u16 = 2;

// --- Resource contract ---
const CONTRACT_MAX_STEPS: u16 = 10000;
const CONTRACT_MAX_SLOTS: u16 = 1024;
const CONTRACT_MAX_CONSTANTS: u16 = 65535;
const CONTRACT_MAX_ACCESSORS: u16 = 8192;
const CONTRACT_MAX_EXPRESSIONS: u16 = 4096;
const CONTRACT_MAX_EXPR_STACK: u8 = 64;
const CONTRACT_MAX_INPUT_BYTES: u32 = 1048576;
const CONTRACT_MAX_OUTPUT_BYTES: u32 = 262144;
const CONTRACT_MAX_STEP_BUDGET_PER_TICK: u64 = 10000;
const CONTRACT_MAX_BLOB_BYTES: u64 = 16777216;
const CONTRACT_MAX_IPC_PAYLOAD_BYTES: u32 = 1048576;
const CONTRACT_MAX_RETRY_ATTEMPTS: u16 = 3;
const CONTRACT_MAX_FANOUT: u16 = 64;
const CONTRACT_MAX_COLLECT_ITEMS: u32 = 1024;
const CONTRACT_MAX_QUEUE_DEPTH: u32 = 1024;
const CONTRACT_MAX_JOURNAL_BATCH_BYTES: u32 = 1048576;

// --- Generated list arena contract ---
const LIST_STORE_RECORD_CAPACITY: usize = 1;
const LIST_STORE_VALUE_CAPACITY: usize = 1;

// --- Constant pool ---
const CONSTANTS: [SlotValue; 1] = [
    SlotValue::I64(42),
];

// --- Main drive function ---
pub fn drive(mut slots: [Option<SlotValue>; 1]) -> Result<SlotValue, DriveError> {
    let mut pc: u16 = 0;
    let mut list_store = ListStore::new();
    loop {
        let outcome = match pc {
            0 => step_0(&mut slots, &mut list_store)?,
            1 => step_1(&mut slots, &mut list_store)?,
            _ => return Err(DriveError::InvalidProgramCounter),
        };
        match outcome {
            StepOutcome::Continue(next) => pc = next,
            StepOutcome::Finished(value) => return Ok(value),
        }
    }
}

fn step_0(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore) -> Result<StepOutcome, DriveError> {
    write_slot(slots, 0, Some(read_const(0)?))?;
    Ok(StepOutcome::Continue(1))
}

fn step_1(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore) -> Result<StepOutcome, DriveError> {
    let value = read_slot(slots, 0)?;
    Ok(StepOutcome::Finished(value))
}

fn eval_expr_0(_slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], _list_store: &ListStore) -> Result<SlotValue, DriveError> {
    let mut stack = ExprStack::new(1)?;
    stack.push(read_const(0)?)?;
    stack.pop().ok_or(DriveError::ExpressionStackUnderflow)
}

// --- Action match dispatch ---
pub fn dispatch_action(action_id: u16) -> Result<(), DriveError> {
    match action_id {
        _ => Err(DriveError::UnknownAction),
    }
}

// --- Result extraction ---


fn main() {
    let slots = [None; WORKFLOW_SLOT_COUNT];
    if let Err(error) = drive(slots) {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}
