#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Generated workflow - DO NOT EDIT
//! Produced by vb_codegen emit_rust_workflow

use std::convert::TryFrom;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotValue { Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(u32), Object(u32), Blob(u64) }

impl SlotValue {
    pub const fn is_true(&self) -> bool { matches!(self, Self::Bool(true)) }
    pub const fn type_name(&self) -> &'static str { match self { Self::Null => "null", Self::Bool(_) => "boolean", Self::I64(_) | Self::F64(_) => "number", Self::Symbol(_) => "symbol", Self::List(_) => "list", Self::Object(_) => "object", Self::Blob(_) => "blob" } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taint { Clean, DerivedFromSecret, Secret }
const fn join_taint(left: Taint, right: Taint) -> Taint { match (left, right) { (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret, (Taint::DerivedFromSecret, _) | (_, Taint::DerivedFromSecret) => Taint::DerivedFromSecret, (Taint::Clean, Taint::Clean) => Taint::Clean } }
fn join_taints(values: &[Taint]) -> Taint { values.iter().copied().fold(Taint::Clean, join_taint) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveError {
    InvalidProgramCounter,
    MissingNextStep,
    MissingOutputSlot { step: u16 },
    SlotOutOfBounds { slot: u16 },
    ExprOutOfBounds { expr: u16 },
    StepBudgetExhausted,
    TaintViolation { step: u16 },
    JournalOverflow,
    InvalidResume { step: u16 },
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
    ObjectStoreOverflow,
    InvalidObjectHandle,
    ObjectFieldOutOfBounds,
    ObjectFieldOffsetOverflow,
    MissingField { field: u32 },
    ListIndexOutOfBounds { index: u32 },
    AccessorPathTooDeep { depth: u16, max: u16 },
    InvalidRetryState,
    InvalidRetryPolicy,
    ActionSuspend { step: u16, action_id: u16, input_slot: u16, resume_pc: u16 },
    WaitUntilSuspend { step: u16, deadline_slot: u16, resume_pc: u16 },
    WaitEventSuspend { step: u16, event_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 },
    AskSuspend { step: u16, prompt_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 },
    UnknownAction,
    UnsupportedPrimitive { primitive: &'static str },
    UnsupportedExpressionOp { op: &'static str },
    InvalidCompiledWorkflow { reason: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedSuspension { ActionPending { step: u16, action_id: u16, input_slot: u16, resume_pc: u16 }, WaitUntil { step: u16, deadline_slot: u16, resume_pc: u16 }, WaitEvent { step: u16, event_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 }, AskPending { step: u16, prompt_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 } }
type SuspensionOutcome = GeneratedSuspension;
impl SuspensionOutcome { fn into_drive_error(self) -> DriveError { match self { Self::ActionPending { step, action_id, input_slot, resume_pc } => DriveError::ActionSuspend { step, action_id, input_slot, resume_pc }, Self::WaitUntil { step, deadline_slot, resume_pc } => DriveError::WaitUntilSuspend { step, deadline_slot, resume_pc }, Self::WaitEvent { step, event_slot, timeout_slot, resume_pc } => DriveError::WaitEventSuspend { step, event_slot, timeout_slot, resume_pc }, Self::AskPending { step, prompt_slot, timeout_slot, resume_pc } => DriveError::AskSuspend { step, prompt_slot, timeout_slot, resume_pc }, } } }
enum StepOutcome { Continue(u16), Finished(SlotValue) }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JournalEvent {
    SlotWritten { slot: u16, value: Option<SlotValue>, taint: Taint },
    ActionScheduled { step: u16, action_id: u16, input_slot: u16, resume_pc: u16 },
    ActionCompleted { step: u16, action_id: u16, output_slot: u16, value: SlotValue, taint: Taint },
    AskAnswered { ask_step: u16, resume_step: u16, answer_slot: u16, value: SlotValue, taint: Taint },
    RunFinished { step: u16, value: SlotValue, taint: Taint },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journal { events: [Option<JournalEvent>; GENERATED_JOURNAL_CAPACITY], len: u16 }
impl Journal {
    pub const fn new() -> Self { Self { events: [None; GENERATED_JOURNAL_CAPACITY], len: 0 } }
    pub const fn len(&self) -> u16 { self.len }
    fn ensure_capacity(&self, needed: usize) -> Result<(), DriveError> {
        let used = usize::from(self.len);
        let available = GENERATED_JOURNAL_CAPACITY.checked_sub(used).ok_or(DriveError::JournalOverflow)?;
        if available < needed { return Err(DriveError::JournalOverflow); }
        Ok(())
    }
    pub fn event(&self, index: u16) -> Option<JournalEvent> {
        if index >= self.len { return None; }
        self.events.get(usize::from(index)).copied().flatten()
    }
    fn push(&mut self, event: JournalEvent) -> Result<(), DriveError> {
        self.ensure_capacity(1)?;
        let index = usize::from(self.len);
        match self.events.get_mut(index) {
            Some(slot) => *slot = Some(event),
            None => return Err(DriveError::JournalOverflow),
        }
        self.len = self.len.checked_add(1).ok_or(DriveError::JournalOverflow)?;
        Ok(())
    }
}

impl Default for Journal {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveOutput { pub value: SlotValue, pub taint: Taint, pub journal: Journal }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuspendedRun { pub suspension: GeneratedSuspension, pub journal: Journal }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneratedRunStatus { Finished(DriveOutput), Suspended(SuspendedRun) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingResume {
    Action { step: u16, action_id: u16, resume_pc: u16 },
    Ask { ask_step: u16, resume_pc: u16 },
}

impl PendingResume {
    const fn step(self) -> u16 {
        match self {
            Self::Action { step, .. } => step,
            Self::Ask { ask_step, .. } => ask_step,
        }
    }
}

pub struct GeneratedRunState {
    slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT],
    slot_taints: [Taint; WORKFLOW_SLOT_COUNT],
    pc: u16,
    step_budget_remaining: u64,
    list_store: ListStore,
    object_store: ObjectStore,
    journal: Journal,
    pending: Option<PendingResume>,
}

impl GeneratedRunState {
    fn record_slot_changes(
        &mut self,
        before_slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT],
        before_taints: &[Taint; WORKFLOW_SLOT_COUNT],
    ) -> Result<(), DriveError> {
        self.journal.ensure_capacity(WORKFLOW_SLOT_COUNT)?;
        let mut slot = 0u16;
        while usize::from(slot) < WORKFLOW_SLOT_COUNT {
            let index = usize::from(slot);
            let before_value = before_slots.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
            let after_value = self.slots.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
            let before_taint = before_taints.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
            let after_taint = self.slot_taints.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
            if before_value != after_value || before_taint != after_taint {
                self.journal.push(JournalEvent::SlotWritten { slot, value: after_value, taint: after_taint })?;
            }
            slot = slot.checked_add(1).ok_or(DriveError::SlotOutOfBounds { slot })?;
        }
        Ok(())
    }

    fn write_slot_with_journal(&mut self, slot: u16, value: Option<SlotValue>, taint: Taint) -> Result<(), DriveError> {
        self.journal.ensure_capacity(1)?;
        write_slot_with_taint(&mut self.slots, &mut self.slot_taints, slot, value, taint)?;
        self.journal.push(JournalEvent::SlotWritten { slot, value, taint })
    }

    fn suspend_from_error(&mut self, error: DriveError) -> Result<GeneratedRunStatus, DriveError> {
        match error {
            DriveError::ActionSuspend { step, action_id, input_slot, resume_pc } => {
                if self.pending.is_some() { return Err(DriveError::InvalidResume { step }); }
                self.journal.ensure_capacity(1)?;
                let suspension = GeneratedSuspension::ActionPending { step, action_id, input_slot, resume_pc };
                self.journal.push(JournalEvent::ActionScheduled { step, action_id, input_slot, resume_pc })?;
                self.pending = Some(PendingResume::Action { step, action_id, resume_pc });
                Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension, journal: self.journal }))
            }
            DriveError::WaitUntilSuspend { step, deadline_slot, resume_pc } => Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::WaitUntil { step, deadline_slot, resume_pc }, journal: self.journal })),
            DriveError::WaitEventSuspend { step, event_slot, timeout_slot, resume_pc } => Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::WaitEvent { step, event_slot, timeout_slot, resume_pc }, journal: self.journal })),
            DriveError::AskSuspend { step, prompt_slot, timeout_slot, resume_pc } => {
                if self.pending.is_some() { return Err(DriveError::InvalidResume { step }); }
                self.pending = Some(PendingResume::Ask { ask_step: step, resume_pc });
                Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::AskPending { step, prompt_slot, timeout_slot, resume_pc }, journal: self.journal }))
            }
            other => Err(other),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryState { current_attempt: u16, remaining: u16, current_delay_ms: u32 }
impl RetryState {
    const fn from_parts(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> Self {
        Self { current_attempt, remaining, current_delay_ms }
    }

    pub fn new(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> Result<Self, DriveError> {
        if retry_state_is_legal(current_attempt, remaining, current_delay_ms) {
            Ok(Self::from_parts(current_attempt, remaining, current_delay_ms))
        } else {
            Err(DriveError::InvalidRetryState)
        }
    }

    pub const fn current_attempt(&self) -> u16 { self.current_attempt }
    pub const fn remaining(&self) -> u16 { self.remaining }
    pub const fn current_delay_ms(&self) -> u32 { self.current_delay_ms }

    pub fn decode(packed: i64, max_attempts: u16) -> Result<Self, DriveError> {
        let unsigned = retry_unsigned_bits(packed)?;
        Self::from_decoded_parts(
            retry_attempt_bits(unsigned)?,
            retry_remaining_bits(unsigned)?,
            retry_delay_bits(unsigned)?,
            max_attempts,
        )
    }

    fn from_decoded_parts(current_attempt: u16, remaining: u16, current_delay_ms: u32, max_attempts: u16) -> Result<Self, DriveError> {
        if retry_decoded_state_is_legal(current_attempt, remaining, current_delay_ms, max_attempts) {
            Ok(Self::from_parts(current_attempt, remaining, current_delay_ms))
        } else {
            Err(DriveError::InvalidRetryState)
        }
    }
}

fn retry_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> bool {
    retry_zero_state_is_legal(current_attempt, remaining, current_delay_ms)
        || (current_attempt > 0 && current_delay_ms == 0 && remaining == 0)
}

fn retry_decoded_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32, max_attempts: u16) -> bool {
    retry_zero_state_is_legal(current_attempt, remaining, current_delay_ms)
        || retry_active_state_is_legal(current_attempt, remaining, max_attempts)
}

fn retry_zero_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> bool {
    current_attempt == 0 && remaining == 0 && current_delay_ms == 0
}

fn retry_active_state_is_legal(current_attempt: u16, remaining: u16, max_attempts: u16) -> bool {
    let Some(total_attempts) = current_attempt.checked_add(remaining) else { return false; };
    let Some(max_live_attempts) = max_attempts.checked_add(1) else { return false; };
    max_attempts > 0 && current_attempt > 0 && current_attempt <= max_attempts && remaining <= max_attempts && total_attempts <= max_live_attempts
}

fn retry_unsigned_bits(packed: i64) -> Result<u64, DriveError> {
    u64::try_from(packed).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_delay_bits(unsigned: u64) -> Result<u32, DriveError> {
    u32::try_from((unsigned >> 32) & 4_294_967_295_u64).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_attempt_bits(unsigned: u64) -> Result<u16, DriveError> {
    u16::try_from((unsigned >> 16) & 65_535_u64).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_remaining_bits(unsigned: u64) -> Result<u16, DriveError> {
    u16::try_from(unsigned & 65_535_u64).map_err(|_| DriveError::InvalidRetryState)
}

const MAX_EXPRESSION_STACK: usize = 64;
const ACCESSOR_MAX_PATH_DEPTH: u16 = 16;
struct ExprStack { values: [SlotValue; MAX_EXPRESSION_STACK], taints: [Taint; MAX_EXPRESSION_STACK], len: u8, capacity: u8 }
impl ExprStack {
    fn new(capacity: u8) -> Result<Self, DriveError> {
        if usize::from(capacity) <= MAX_EXPRESSION_STACK {
            Ok(Self { values: [SlotValue::Null; MAX_EXPRESSION_STACK], taints: [Taint::Clean; MAX_EXPRESSION_STACK], len: 0, capacity })
        } else {
            Err(DriveError::ExpressionStackOverflow { max: capacity })
        }
    }

    fn push(&mut self, value: SlotValue) -> Result<(), DriveError> {
        self.push_tainted(value, Taint::Clean)
    }

    fn push_tainted(&mut self, value: SlotValue, taint: Taint) -> Result<(), DriveError> {
        if self.len >= self.capacity {
            return Err(DriveError::ExpressionStackOverflow { max: self.capacity });
        }
        let index = usize::from(self.len);
        match (self.values.get_mut(index), self.taints.get_mut(index)) {
            (Some(value_slot), Some(taint_slot)) => {
                *value_slot = value;
                *taint_slot = taint;
            }
            (_, _) => return Err(DriveError::ExpressionStackOverflow { max: self.capacity }),
        }
        self.len = self.len.checked_add(1).ok_or(DriveError::ExpressionStackOverflow { max: self.capacity })?;
        Ok(())
    }

    fn pop(&mut self) -> Option<SlotValue> {
        self.pop_tainted().map(|entry| entry.0)
    }

    fn pop_tainted(&mut self) -> Option<(SlotValue, Taint)> {
        if self.len == 0 {
            return None;
        }
        self.len = self.len.checked_sub(1)?;
        let index = usize::from(self.len);
        match (self.values.get(index).copied(), self.taints.get(index).copied()) {
            (Some(value), Some(taint)) => Some((value, taint)),
            (_, _) => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ListRecord { start: u32, len: u32 }
struct ListStore {
    records: [Option<ListRecord>; LIST_STORE_RECORD_CAPACITY],
    values: [SlotValue; LIST_STORE_VALUE_CAPACITY],
    taints: [Taint; LIST_STORE_VALUE_CAPACITY],
    record_len: u32,
    value_len: u32,
}
impl ListStore {
    fn new() -> Self {
        Self {
            records: [None; LIST_STORE_RECORD_CAPACITY],
            values: [SlotValue::Null; LIST_STORE_VALUE_CAPACITY],
            taints: [Taint::Clean; LIST_STORE_VALUE_CAPACITY],
            record_len: 0,
            value_len: 0,
        }
    }

    fn insert_items_with_taints(
        &mut self,
        items: &[SlotValue],
        taints: &[Taint],
    ) -> Result<u32, DriveError> {
        if items.len() != taints.len() {
            return Err(DriveError::InvalidCompiledWorkflow {
                reason: "list value/taint length mismatch",
            });
        }
        let start = self.value_len;
        let item_count = u32::try_from(items.len()).map_err(|_| DriveError::ListStoreOverflow)?;
        self.ensure_value_capacity(start, item_count)?;
        self.copy_items(start, items, taints)?;
        self.value_len = checked_add_u32(start, item_count, DriveError::ListStoreOverflow)?;
        self.insert_record(start, item_count)
    }

    fn insert_items_prefix(
        &mut self,
        items: &[SlotValue; LIST_STORE_VALUE_CAPACITY],
        taints: &[Taint; LIST_STORE_VALUE_CAPACITY],
        count: usize,
    ) -> Result<u32, DriveError> {
        let start = self.value_len;
        let item_count = u32::try_from(count).map_err(|_| DriveError::ListStoreOverflow)?;
        self.ensure_value_capacity(start, item_count)?;
        let mut cursor = 0usize;
        while cursor < count {
            self.copy_item(start, cursor, items, taints)?;
            cursor = cursor.checked_add(1).ok_or(DriveError::ListStoreOverflow)?;
        }
        self.value_len = checked_add_u32(start, item_count, DriveError::ListStoreOverflow)?;
        self.insert_record(start, item_count)
    }

    fn ensure_value_capacity(&self, start: u32, count: u32) -> Result<(), DriveError> {
        let end = checked_add_u32(start, count, DriveError::ListStoreOverflow)?;
        let end_index = usize::try_from(end).map_err(|_| DriveError::ListStoreOverflow)?;
        if end_index > LIST_STORE_VALUE_CAPACITY {
            return Err(DriveError::ListStoreOverflow);
        }
        Ok(())
    }

    fn copy_items(
        &mut self,
        start: u32,
        items: &[SlotValue],
        taints: &[Taint],
    ) -> Result<(), DriveError> {
        let mut cursor = 0usize;
        while cursor < items.len() {
            self.copy_item(start, cursor, items, taints)?;
            cursor = cursor.checked_add(1).ok_or(DriveError::ListStoreOverflow)?;
        }
        Ok(())
    }

    fn copy_item(
        &mut self,
        start: u32,
        cursor: usize,
        items: &[SlotValue],
        taints: &[Taint],
    ) -> Result<(), DriveError> {
        let target_index = checked_offset_index(start, cursor, DriveError::ListStoreOverflow)?;
        let value = items.get(cursor).copied().ok_or(DriveError::ListStoreOverflow)?;
        let taint = taints.get(cursor).copied().ok_or(DriveError::ListStoreOverflow)?;
        match (self.values.get_mut(target_index), self.taints.get_mut(target_index)) {
            (Some(target), Some(target_taint)) => {
                *target = value;
                *target_taint = taint;
                Ok(())
            }
            _ => Err(DriveError::ListStoreOverflow),
        }
    }

    fn insert_record(&mut self, start: u32, len: u32) -> Result<u32, DriveError> {
        let handle = self.record_len;
        let index = usize::try_from(handle).map_err(|_| DriveError::ListStoreOverflow)?;
        match self.records.get_mut(index) {
            Some(slot) => *slot = Some(ListRecord { start, len }),
            None => return Err(DriveError::ListStoreOverflow),
        }
        self.record_len = checked_add_u32(self.record_len, 1, DriveError::ListStoreOverflow)?;
        Ok(handle)
    }

    fn record(&self, handle: u32) -> Result<Option<ListRecord>, DriveError> {
        if handle >= self.record_len {
            return Ok(None);
        }
        let index = usize::try_from(handle).map_err(|_| DriveError::InvalidListHandle)?;
        match self.records.get(index).copied() {
            Some(Some(record)) => Ok(Some(record)),
            Some(None) | None => Err(DriveError::InvalidListHandle),
        }
    }

    fn len(&self, handle: u32) -> Result<Option<u32>, DriveError> {
        self.record(handle).map(|record| record.map(|value| value.len))
    }

    fn first(&self, handle: u32) -> Result<Option<SlotValue>, DriveError> {
        let Some(record) = self.record(handle)? else {
            return Ok(None);
        };
        if record.len == 0 {
            return Ok(None);
        }
        let index = usize::try_from(record.start).map_err(|_| DriveError::InvalidListHandle)?;
        self.values
            .get(index)
            .copied()
            .map(Some)
            .ok_or(DriveError::InvalidListHandle)
    }

    fn tail(&mut self, handle: u32) -> Result<Option<u32>, DriveError> {
        let Some(record) = self.record(handle)? else {
            return Ok(None);
        };
        let (start, len) = if record.len == 0 {
            (record.start, 0)
        } else {
            (
                checked_add_u32(record.start, 1, DriveError::ListStoreOverflow)?,
                record.len.checked_sub(1).ok_or(DriveError::ListStoreOverflow)?,
            )
        };
        self.insert_record(start, len).map(Some)
    }

    fn value_at(&self, handle: u32, index: u32) -> Result<(SlotValue, Taint), DriveError> {
        let Some(record) = self.record(handle)? else {
            return Err(DriveError::InvalidListHandle);
        };
        if index >= record.len {
            return Err(DriveError::ListIndexOutOfBounds { index });
        }
        let offset = checked_add_u32(record.start, index, DriveError::ListStoreOverflow)?;
        let value_index = usize::try_from(offset).map_err(|_| DriveError::InvalidListHandle)?;
        match (
            self.values.get(value_index).copied(),
            self.taints.get(value_index).copied(),
        ) {
            (Some(value), Some(taint)) => Ok((value, taint)),
            (_, _) => Err(DriveError::InvalidListHandle),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ObjectField { key: u32, value: SlotValue, taint: Taint }
#[derive(Debug, Clone, Copy)]
struct ObjectRecord { start: u32, len: u32 }
struct ObjectStore {
    records: [Option<ObjectRecord>; OBJECT_STORE_RECORD_CAPACITY],
    fields: [Option<ObjectField>; OBJECT_STORE_FIELD_CAPACITY],
    record_len: u32,
    field_len: u32,
}
impl ObjectStore {
    fn new() -> Self {
        Self {
            records: [None; OBJECT_STORE_RECORD_CAPACITY],
            fields: [None; OBJECT_STORE_FIELD_CAPACITY],
            record_len: 0,
            field_len: 0,
        }
    }

    fn insert_fields(&mut self, fields: &[ObjectField]) -> Result<u32, DriveError> {
        let start = self.field_len;
        let field_count = u32::try_from(fields.len()).map_err(|_| DriveError::ObjectStoreOverflow)?;
        self.ensure_field_capacity(start, field_count)?;
        self.copy_fields(start, fields)?;
        self.field_len = checked_add_u32(start, field_count, DriveError::ObjectStoreOverflow)?;
        self.insert_record(start, field_count)
    }

    fn insert_fields_prefix(
        &mut self,
        fields: &[ObjectField; OBJECT_STORE_FIELD_CAPACITY],
        count: usize,
    ) -> Result<u32, DriveError> {
        let start = self.field_len;
        let field_count = u32::try_from(count).map_err(|_| DriveError::ObjectStoreOverflow)?;
        self.ensure_field_capacity(start, field_count)?;
        let mut cursor = 0usize;
        while cursor < count {
            self.copy_field(start, cursor, fields)?;
            cursor = cursor.checked_add(1).ok_or(DriveError::ObjectStoreOverflow)?;
        }
        self.field_len = checked_add_u32(start, field_count, DriveError::ObjectStoreOverflow)?;
        self.insert_record(start, field_count)
    }

    fn ensure_field_capacity(&self, start: u32, count: u32) -> Result<(), DriveError> {
        let end = checked_add_u32(start, count, DriveError::ObjectStoreOverflow)?;
        let end_index = usize::try_from(end).map_err(|_| DriveError::ObjectStoreOverflow)?;
        if end_index > OBJECT_STORE_FIELD_CAPACITY {
            return Err(DriveError::ObjectStoreOverflow);
        }
        Ok(())
    }

    fn copy_fields(&mut self, start: u32, fields: &[ObjectField]) -> Result<(), DriveError> {
        let mut cursor = 0usize;
        while cursor < fields.len() {
            self.copy_field(start, cursor, fields)?;
            cursor = cursor.checked_add(1).ok_or(DriveError::ObjectStoreOverflow)?;
        }
        Ok(())
    }

    fn copy_field(
        &mut self,
        start: u32,
        cursor: usize,
        fields: &[ObjectField],
    ) -> Result<(), DriveError> {
        let target_index = checked_offset_index(start, cursor, DriveError::ObjectStoreOverflow)?;
        let value = fields
            .get(cursor)
            .copied()
            .ok_or(DriveError::ObjectStoreOverflow)?;
        match self.fields.get_mut(target_index) {
            Some(target) => {
                *target = Some(value);
                Ok(())
            }
            None => Err(DriveError::ObjectStoreOverflow),
        }
    }

    fn insert_record(&mut self, start: u32, len: u32) -> Result<u32, DriveError> {
        let handle = self.record_len;
        let index = usize::try_from(handle).map_err(|_| DriveError::ObjectStoreOverflow)?;
        match self.records.get_mut(index) {
            Some(slot) => *slot = Some(ObjectRecord { start, len }),
            None => return Err(DriveError::ObjectStoreOverflow),
        }
        self.record_len = checked_add_u32(self.record_len, 1, DriveError::ObjectStoreOverflow)?;
        Ok(handle)
    }

    fn record(&self, handle: u32) -> Result<ObjectRecord, DriveError> {
        if handle >= self.record_len {
            return Err(DriveError::InvalidObjectHandle);
        }
        let index = usize::try_from(handle).map_err(|_| DriveError::InvalidObjectHandle)?;
        self.records
            .get(index)
            .copied()
            .flatten()
            .ok_or(DriveError::InvalidObjectHandle)
    }

    fn field(&self, handle: u32, key: u32) -> Result<(SlotValue, Taint), DriveError> {
        let record = self.record(handle)?;
        object_field_scan(self, record, key)
    }
}

fn list_contains_item(list_store: &ListStore, handle: u32, item: SlotValue) -> Result<bool, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    let mut cursor = 0u32;
    while cursor < record.len {
        let (value, _) = list_store.value_at(handle, cursor)?;
        if value == item {
            return Ok(true);
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    Ok(false)
}

fn clone_list_items(list_store: &mut ListStore, handle: u32) -> Result<u32, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    let mut values = [SlotValue::Null; LIST_STORE_VALUE_CAPACITY];
    let mut taints = [Taint::Clean; LIST_STORE_VALUE_CAPACITY];
    let count = usize::try_from(record.len).map_err(|_| DriveError::ListStoreOverflow)?;
    let mut cursor = 0u32;
    while cursor < record.len {
        let (value, taint) = list_store.value_at(handle, cursor)?;
        let index = usize::try_from(cursor).map_err(|_| DriveError::ListStoreOverflow)?;
        match (values.get_mut(index), taints.get_mut(index)) {
            (Some(target_value), Some(target_taint)) => {
                *target_value = value;
                *target_taint = taint;
            }
            (_, _) => return Err(DriveError::ListStoreOverflow),
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    list_store.insert_items_prefix(&values, &taints, count)
}

fn append_list_item(
    list_store: &mut ListStore,
    handle: u32,
    item: SlotValue,
    item_taint: Taint,
) -> Result<u32, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    let base_count = usize::try_from(record.len).map_err(|_| DriveError::ListStoreOverflow)?;
    let count = base_count.checked_add(1).ok_or(DriveError::ListStoreOverflow)?;
    if count > LIST_STORE_VALUE_CAPACITY {
        return Err(DriveError::ListStoreOverflow);
    }
    let mut values = [SlotValue::Null; LIST_STORE_VALUE_CAPACITY];
    let mut taints = [Taint::Clean; LIST_STORE_VALUE_CAPACITY];
    let mut cursor = 0u32;
    while cursor < record.len {
        let (value, taint) = list_store.value_at(handle, cursor)?;
        let index = usize::try_from(cursor).map_err(|_| DriveError::ListStoreOverflow)?;
        match (values.get_mut(index), taints.get_mut(index)) {
            (Some(target_value), Some(target_taint)) => {
                *target_value = value;
                *target_taint = taint;
            }
            (_, _) => return Err(DriveError::ListStoreOverflow),
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    match (values.get_mut(base_count), taints.get_mut(base_count)) {
        (Some(target_value), Some(target_taint)) => {
            *target_value = item;
            *target_taint = item_taint;
        }
        (_, _) => return Err(DriveError::ListStoreOverflow),
    }
    list_store.insert_items_prefix(&values, &taints, count)
}

fn unique_list_items(list_store: &mut ListStore, handle: u32) -> Result<u32, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    let mut values = [SlotValue::Null; LIST_STORE_VALUE_CAPACITY];
    let mut taints = [Taint::Clean; LIST_STORE_VALUE_CAPACITY];
    let mut unique_len = 0usize;
    let mut cursor = 0u32;
    while cursor < record.len {
        let (value, taint) = list_store.value_at(handle, cursor)?;
        let seen = values
            .get(..unique_len)
            .ok_or(DriveError::ListStoreOverflow)?
            .iter()
            .copied()
            .any(|item| item == value);
        if !seen {
            match (values.get_mut(unique_len), taints.get_mut(unique_len)) {
                (Some(target_value), Some(target_taint)) => {
                    *target_value = value;
                    *target_taint = taint;
                }
                (_, _) => return Err(DriveError::ListStoreOverflow),
            }
            unique_len = unique_len.checked_add(1).ok_or(DriveError::ListStoreOverflow)?;
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    list_store.insert_items_prefix(&values, &taints, unique_len)
}

fn sum_list_items(list_store: &ListStore, handle: u32) -> Result<i64, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    let mut total = 0i64;
    let mut cursor = 0u32;
    while cursor < record.len {
        let (value, _) = list_store.value_at(handle, cursor)?;
        total = total
            .checked_add(expect_i64_value(value)?)
            .ok_or(DriveError::IntegerOverflow)?;
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    Ok(total)
}

fn collect_page_handle(
    list_store: &mut ListStore,
    handle: u32,
    start: u32,
    page_size: u32,
) -> Result<u32, DriveError> {
    let Some(record) = list_store.record(handle)? else {
        return Err(DriveError::InvalidListHandle);
    };
    if start > record.len {
        return Err(DriveError::ListStoreOverflow);
    }
    let remaining = record.len.checked_sub(start).ok_or(DriveError::ListStoreOverflow)?;
    let page_len = remaining.min(page_size);
    let count = usize::try_from(page_len).map_err(|_| DriveError::ListStoreOverflow)?;
    let mut values = [SlotValue::Null; LIST_STORE_VALUE_CAPACITY];
    let mut taints = [Taint::Clean; LIST_STORE_VALUE_CAPACITY];
    let mut cursor = 0u32;
    while cursor < page_len {
        let source_index = checked_add_u32(start, cursor, DriveError::ListStoreOverflow)?;
        let (value, taint) = list_store.value_at(handle, source_index)?;
        let target_index = usize::try_from(cursor).map_err(|_| DriveError::ListStoreOverflow)?;
        match (values.get_mut(target_index), taints.get_mut(target_index)) {
            (Some(target_value), Some(target_taint)) => {
                *target_value = value;
                *target_taint = taint;
            }
            (_, _) => return Err(DriveError::ListStoreOverflow),
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ListStoreOverflow)?;
    }
    list_store.insert_items_prefix(&values, &taints, count)
}

fn object_field_count(object_store: &ObjectStore, handle: u32) -> Result<u32, DriveError> {
    object_store.record(handle).map(|record| record.len)
}

fn merge_object_records(
    object_store: &mut ObjectStore,
    left: u32,
    right: u32,
) -> Result<u32, DriveError> {
    let left_record = object_store.record(left)?;
    let right_record = object_store.record(right)?;
    let mut fields = [None; OBJECT_STORE_FIELD_CAPACITY];
    let mut field_len = copy_object_fields_to_buffer(object_store, left_record, &mut fields, 0)?;
    let mut cursor = 0u32;
    while cursor < right_record.len {
        let field = read_object_field(object_store, right_record.start, cursor)?;
        field_len = upsert_object_field(&mut fields, field_len, field)?;
        cursor = checked_add_u32(cursor, 1, DriveError::ObjectStoreOverflow)?;
    }
    let count = usize::try_from(field_len).map_err(|_| DriveError::ObjectStoreOverflow)?;
    let mut compact = [ObjectField { key: 0, value: SlotValue::Null, taint: Taint::Clean }; OBJECT_STORE_FIELD_CAPACITY];
    let mut index = 0usize;
    while index < count {
        let field = fields.get(index).copied().flatten().ok_or(DriveError::ObjectStoreOverflow)?;
        match compact.get_mut(index) {
            Some(target) => *target = field,
            None => return Err(DriveError::ObjectStoreOverflow),
        }
        index = index.checked_add(1).ok_or(DriveError::ObjectStoreOverflow)?;
    }
    object_store.insert_fields_prefix(&compact, count)
}

fn copy_object_fields_to_buffer(
    object_store: &ObjectStore,
    record: ObjectRecord,
    fields: &mut [Option<ObjectField>; OBJECT_STORE_FIELD_CAPACITY],
    start_len: u32,
) -> Result<u32, DriveError> {
    let mut len = start_len;
    let mut cursor = 0u32;
    while cursor < record.len {
        let field = read_object_field(object_store, record.start, cursor)?;
        let index = usize::try_from(len).map_err(|_| DriveError::ObjectStoreOverflow)?;
        match fields.get_mut(index) {
            Some(target) => *target = Some(field),
            None => return Err(DriveError::ObjectStoreOverflow),
        }
        len = checked_add_u32(len, 1, DriveError::ObjectStoreOverflow)?;
        cursor = checked_add_u32(cursor, 1, DriveError::ObjectStoreOverflow)?;
    }
    Ok(len)
}

fn upsert_object_field(
    fields: &mut [Option<ObjectField>; OBJECT_STORE_FIELD_CAPACITY],
    len: u32,
    field: ObjectField,
) -> Result<u32, DriveError> {
    let mut cursor = 0u32;
    while cursor < len {
        let index = usize::try_from(cursor).map_err(|_| DriveError::ObjectStoreOverflow)?;
        match fields.get_mut(index) {
            Some(Some(existing)) if existing.key == field.key => {
                *existing = field;
                return Ok(len);
            }
            Some(_) => {}
            None => return Err(DriveError::ObjectStoreOverflow),
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ObjectStoreOverflow)?;
    }
    let index = usize::try_from(len).map_err(|_| DriveError::ObjectStoreOverflow)?;
    match fields.get_mut(index) {
        Some(target) => {
            *target = Some(field);
            checked_add_u32(len, 1, DriveError::ObjectStoreOverflow)
        }
        None => Err(DriveError::ObjectStoreOverflow),
    }
}

fn read_object_field(
    object_store: &ObjectStore,
    start: u32,
    cursor: u32,
) -> Result<ObjectField, DriveError> {
    let offset = checked_add_u32(start, cursor, DriveError::ObjectFieldOffsetOverflow)?;
    let index = usize::try_from(offset).map_err(|_| DriveError::ObjectFieldOffsetOverflow)?;
    object_store
        .fields
        .get(index)
        .copied()
        .flatten()
        .ok_or(DriveError::ObjectFieldOutOfBounds)
}

fn object_field_scan(
    object_store: &ObjectStore,
    record: ObjectRecord,
    key: u32,
) -> Result<(SlotValue, Taint), DriveError> {
    let mut cursor = 0u32;
    while cursor < record.len {
        let field = read_object_field(object_store, record.start, cursor)?;
        if field.key == key {
            return Ok((field.value, field.taint));
        }
        cursor = checked_add_u32(cursor, 1, DriveError::ObjectStoreOverflow)?;
    }
    Err(DriveError::MissingField { field: key })
}

fn checked_add_u32(left: u32, right: u32, error: DriveError) -> Result<u32, DriveError> {
    left.checked_add(right).ok_or(error)
}

fn checked_offset_index(start: u32, cursor: usize, error: DriveError) -> Result<usize, DriveError> {
    let cursor_u32 = u32::try_from(cursor).map_err(|_| error)?;
    let target_offset = checked_add_u32(start, cursor_u32, error)?;
    usize::try_from(target_offset).map_err(|_| error)
}

fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> {
    match read_slot_optional(slots, slot)? {
        Some(value) => Ok(value),
        None => Err(DriveError::SlotNull),
    }
}

fn read_slot_optional(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<Option<SlotValue>, DriveError> {
    slots
        .get(usize::from(slot))
        .copied()
        .ok_or(DriveError::SlotOutOfBounds { slot })
}

fn read_taint(slot_taints: &[Taint; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<Taint, DriveError> {
    slot_taints
        .get(usize::from(slot))
        .copied()
        .ok_or(DriveError::SlotOutOfBounds { slot })
}

fn write_slot(
    slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT],
    slot: u16,
    value: Option<SlotValue>,
) -> Result<(), DriveError> {
    match slots.get_mut(usize::from(slot)) {
        Some(target) => {
            *target = value;
            Ok(())
        }
        None => Err(DriveError::SlotOutOfBounds { slot }),
    }
}

fn write_slot_with_taint(
    slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT],
    slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT],
    slot: u16,
    value: Option<SlotValue>,
    taint: Taint,
) -> Result<(), DriveError> {
    match (
        slots.get_mut(usize::from(slot)),
        slot_taints.get_mut(usize::from(slot)),
    ) {
        (Some(target), Some(target_taint)) => {
            *target = value;
            *target_taint = taint;
            Ok(())
        }
        (_, _) => Err(DriveError::SlotOutOfBounds { slot }),
    }
}

fn read_const(index: u16) -> Result<SlotValue, DriveError> { CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow { reason: "constant index out of bounds" }) }
fn expect_list_value(value: SlotValue) -> Result<u32, DriveError> { match value { SlotValue::List(handle) => Ok(handle), other => Err(DriveError::TypeMismatch { expected: "list", found: other.type_name() }), } }
fn expect_object_value(value: SlotValue) -> Result<u32, DriveError> { match value { SlotValue::Object(handle) => Ok(handle), other => Err(DriveError::TypeMismatch { expected: "object", found: other.type_name() }), } }
fn expect_bool_value(value: SlotValue) -> Result<bool, DriveError> { match value { SlotValue::Bool(value) => Ok(value), other => Err(DriveError::TypeMismatch { expected: "boolean", found: other.type_name() }), } }
fn expect_i64_value(value: SlotValue) -> Result<i64, DriveError> { match value { SlotValue::I64(value) => Ok(value), other => Err(DriveError::TypeMismatch { expected: "number", found: other.type_name() }), } }
fn read_retry_state_from_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, max_attempts: u16) -> Result<RetryState, DriveError> { match read_slot(slots, slot)? { SlotValue::I64(raw) => RetryState::decode(raw, max_attempts), other => Err(DriveError::TypeMismatch { expected: "number", found: other.type_name() }), } }
fn retry_check_target(current_attempt: u16, max_attempts: u16, body: u16, exhausted: u16) -> Result<StepOutcome, DriveError> { if max_attempts == 0 { return Err(DriveError::InvalidRetryPolicy); } if current_attempt < max_attempts { Ok(StepOutcome::Continue(body)) } else { Ok(StepOutcome::Continue(exhausted)) } }
fn list_item_count(list_store: &ListStore, handle: u32) -> Result<u32, DriveError> { match list_store.len(handle)? { Some(len) => Ok(len), None => Err(DriveError::InvalidListHandle), } }
fn first_list_item(list_store: &ListStore, handle: u32, count: u32) -> Result<SlotValue, DriveError> { if count == 0 { return Err(DriveError::InvalidListHandle); } match list_store.first(handle)? { Some(value) => Ok(value), None => Err(DriveError::InvalidListHandle), } }
fn tail_list_handle(list_store: &mut ListStore, handle: u32) -> Result<u32, DriveError> { match list_store.tail(handle)? { Some(tail) => Ok(tail), None => Err(DriveError::InvalidListHandle), } }

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

// --- Generated value arena contract ---
const LIST_STORE_RECORD_CAPACITY: usize = 1;
const LIST_STORE_VALUE_CAPACITY: usize = 1;
const OBJECT_STORE_RECORD_CAPACITY: usize = 1;
const OBJECT_STORE_FIELD_CAPACITY: usize = 1;

// --- Generated journal contract ---
const GENERATED_JOURNAL_CAPACITY: usize = 10;

// --- Constant pool ---
const CONSTANTS: [SlotValue; 1] = [
    SlotValue::I64(42),
];

// --- Main drive function ---
pub fn drive(mut slots: [Option<SlotValue>; 1]) -> Result<SlotValue, DriveError> {
    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];
    let mut pc: u16 = 0;
    let mut step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK;
    let mut list_store = ListStore::new();
    let mut object_store = ObjectStore::new();
    loop {
        if step_budget_remaining == 0 {
            return Err(DriveError::StepBudgetExhausted);
        }
        step_budget_remaining = step_budget_remaining.checked_sub(1).ok_or(DriveError::StepBudgetExhausted)?;
        let outcome = match pc {
            0 => step_0(&mut slots, &mut slot_taints, &mut list_store, &mut object_store)?,
            1 => step_1(&mut slots, &mut slot_taints, &mut list_store, &mut object_store)?,
            _ => return Err(DriveError::InvalidProgramCounter),
        };
        match outcome {
            StepOutcome::Continue(next) => pc = next,
            StepOutcome::Finished(value) => return Ok(value),
        }
    }
}

// --- Rich generated runtime API ---
pub fn drive_with_journal(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<GeneratedRunStatus, DriveError> { let mut state = GeneratedRunState::new(slots); state.run_until_blocked() }
impl GeneratedRunState {
    pub fn new(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Self { Self { slots, slot_taints: [Taint::Clean; WORKFLOW_SLOT_COUNT], pc: 0, step_budget_remaining: CONTRACT_MAX_STEP_BUDGET_PER_TICK, list_store: ListStore::new(), object_store: ObjectStore::new(), journal: Journal::new(), pending: None } }
    pub fn new_with_taints(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: [Taint; WORKFLOW_SLOT_COUNT]) -> Self { Self { slots, slot_taints, pc: 0, step_budget_remaining: CONTRACT_MAX_STEP_BUDGET_PER_TICK, list_store: ListStore::new(), object_store: ObjectStore::new(), journal: Journal::new(), pending: None } }
    pub fn run_until_blocked(&mut self) -> Result<GeneratedRunStatus, DriveError> {
        if let Some(pending) = self.pending { return Err(DriveError::InvalidResume { step: pending.step() }); }
        loop {
            if self.step_budget_remaining == 0 { return Err(DriveError::StepBudgetExhausted); }
            self.journal.ensure_capacity(WORKFLOW_SLOT_COUNT)?;
            self.step_budget_remaining = self.step_budget_remaining.checked_sub(1).ok_or(DriveError::StepBudgetExhausted)?;
            let before_slots = self.slots;
            let before_taints = self.slot_taints;
            let current_pc = self.pc;
            let outcome = match current_pc {
                0 => step_0(&mut self.slots, &mut self.slot_taints, &mut self.list_store, &mut self.object_store),
                1 => step_1(&mut self.slots, &mut self.slot_taints, &mut self.list_store, &mut self.object_store),
                _ => Err(DriveError::InvalidProgramCounter),
            };
            self.record_slot_changes(&before_slots, &before_taints)?;
            match outcome {
                Ok(StepOutcome::Continue(next)) => self.pc = next,
                Ok(StepOutcome::Finished(value)) => { let taint = read_taint(&self.slot_taints, finish_result_slot(current_pc)?)?; self.journal.ensure_capacity(1)?; self.journal.push(JournalEvent::RunFinished { step: current_pc, value, taint })?; return Ok(GeneratedRunStatus::Finished(DriveOutput { value, taint, journal: self.journal })); }
                Err(error) => return self.suspend_from_error(error),
            }
        }
    }
    pub fn complete_action(&mut self, step: u16, action_id: u16, output_slot: u16, value: SlotValue, taint: Taint) -> Result<GeneratedRunStatus, DriveError> {
        let next = action_completion_next(step, action_id, output_slot)?;
        match self.pending { Some(PendingResume::Action { step: pending_step, action_id: pending_action_id, resume_pc }) if pending_step == step && pending_action_id == action_id && resume_pc == next => {}, _ => return Err(DriveError::InvalidResume { step }), }
        self.journal.ensure_capacity(2)?;
        self.write_slot_with_journal(output_slot, Some(value), taint)?;
        self.journal.push(JournalEvent::ActionCompleted { step, action_id, output_slot, value, taint })?;
        self.pending = None;
        self.pc = next;
        self.run_until_blocked()
    }
    pub fn answer_ask(&mut self, ask_step: u16, resume_step: u16, value: SlotValue, taint: Taint) -> Result<GeneratedRunStatus, DriveError> {
        let (answer_slot, next) = ask_answer_spec(ask_step, resume_step)?;
        match self.pending { Some(PendingResume::Ask { ask_step: pending_ask_step, resume_pc }) if pending_ask_step == ask_step && resume_pc == resume_step => {}, _ => return Err(DriveError::InvalidResume { step: ask_step }), }
        self.journal.ensure_capacity(2)?;
        self.write_slot_with_journal(answer_slot, Some(value), taint)?;
        self.journal.push(JournalEvent::AskAnswered { ask_step, resume_step, answer_slot, value, taint })?;
        self.pending = None;
        self.pc = next;
        self.run_until_blocked()
    }
}

fn action_completion_next(step: u16, action_id: u16, output_slot: u16) -> Result<u16, DriveError> {
    match (step, action_id, output_slot) {
        (step, _, _) => Err(DriveError::InvalidResume { step }),
    }
}

fn ask_answer_spec(ask_step: u16, resume_step: u16) -> Result<(u16, u16), DriveError> {
    match (ask_step, resume_step) {
        (step, _) => Err(DriveError::InvalidResume { step }),
    }
}

fn finish_result_slot(step: u16) -> Result<u16, DriveError> {
    match step {
        1 => Ok(0),
        step => Err(DriveError::InvalidResume { step }),
    }
}

fn step_0(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore) -> Result<StepOutcome, DriveError> {
    // write_slot(slots, 0, Some(read_const(0)?))
    write_slot_with_taint(slots, slot_taints, 0, Some(read_const(0)?), Taint::Clean)?;
    Ok(StepOutcome::Continue(1))
}

fn step_1(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], _slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore) -> Result<StepOutcome, DriveError> {
    let value = read_slot(slots, 0)?;
    Ok(StepOutcome::Finished(value))
}

fn eval_expr_0(_slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], _slot_taints: &[Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore) -> Result<(SlotValue, Taint), DriveError> {
    let mut stack = ExprStack::new(1)?;
    stack.push(read_const(0)?)?;
    stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)
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
