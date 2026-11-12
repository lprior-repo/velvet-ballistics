use super::receipt::{self, ReceiptRecord};
use super::{ExecutorError, RegisterGrant, RegisterReceipt};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Instant};
use vb_core::ActionTicket;
use vb_core::action_pack::CapabilityScopeSpec;

const MAX_RECEIPTS: u32 = 4096;
const MAX_RECEIPT_BYTES: usize = 512;

#[derive(Serialize, Deserialize)]
struct Metadata {
    schema: u16,
    capacity: u32,
    count: u32,
    effects: u64,
}

/// Exclusive local adapter. Mutable ownership prevents lookup/commit races;
/// Fjall's database lock rejects another independent opener of the same path.
pub struct FjallRegister {
    db: Database,
    targets: Keyspace,
    receipts: Keyspace,
    metadata: Keyspace,
    capacity: u32,
}

impl FjallRegister {
    /// Open an exclusive bounded register. Capacity cannot silently change after reopen.
    pub fn open(path: &Path, capacity: u32) -> Result<Self, ExecutorError> {
        if capacity == 0 || capacity > MAX_RECEIPTS {
            return Err(ExecutorError::Capacity);
        }
        let db = Database::builder(path).open()?;
        let targets = db.keyspace("register_targets_v1", KeyspaceCreateOptions::default)?;
        let receipts = db.keyspace("register_receipts_v1", KeyspaceCreateOptions::default)?;
        let metadata = db.keyspace("register_metadata_v1", KeyspaceCreateOptions::default)?;
        let store = Self {
            db,
            targets,
            receipts,
            metadata,
            capacity,
        };
        store.check_metadata()?;
        Ok(store)
    }

    /// Perform a desired-state write or return the identical existing receipt.
    /// An I/O/deadline error may follow commit; use reconcile before another effect.
    pub fn execute(
        &mut self,
        grant: &RegisterGrant,
        ticket: ActionTicket,
        value: i64,
    ) -> Result<RegisterReceipt, ExecutorError> {
        let started = Instant::now();
        grant.validate(ticket)?;
        let result = match self.lookup(grant, value)? {
            Some(receipt) => receipt,
            None => self.commit_effect(grant, value)?,
        };
        self.db.persist(PersistMode::SyncAll)?;
        if started.elapsed().as_millis() > u128::from(grant.timeout_ms) {
            return Err(ExecutorError::Timeout);
        }
        Ok(result)
    }

    /// Read and sync an existing matching receipt. Never writes a production target.
    pub fn reconcile(
        &mut self,
        grant: &RegisterGrant,
        ticket: ActionTicket,
        value: i64,
    ) -> Result<RegisterReceipt, ExecutorError> {
        grant.validate(ticket)?;
        let receipt = self
            .lookup(grant, value)?
            .ok_or(ExecutorError::ReceiptMissing)?;
        self.db.persist(PersistMode::SyncAll)?;
        Ok(receipt)
    }

    /// Inspect the exact target. This never authorizes an effect.
    pub fn value(&self, scope: CapabilityScopeSpec) -> Result<Option<i64>, ExecutorError> {
        self.targets
            .get(target_key(scope)?)?
            .map(|bytes| postcard::from_bytes(&bytes).map_err(ExecutorError::from))
            .transpose()
    }

    /// Durable number of target effects, including repeated writes to one target.
    pub fn effect_count(&self) -> Result<u64, ExecutorError> {
        Ok(self.load_metadata()?.effects)
    }

    fn load_metadata(&self) -> Result<Metadata, ExecutorError> {
        let Some(bytes) = self.metadata.get(b"state")? else {
            return Ok(Metadata {
                schema: 1,
                capacity: self.capacity,
                count: 0,
                effects: 0,
            });
        };
        if bytes.len() > 64 {
            return Err(ExecutorError::Corrupt);
        }
        Ok(postcard::from_bytes(&bytes)?)
    }

    fn check_metadata(&self) -> Result<(), ExecutorError> {
        let meta = self.load_metadata()?;
        if meta.schema != 1
            || meta.capacity != self.capacity
            || meta.count > meta.capacity
            || meta.effects != u64::from(meta.count)
        {
            return Err(ExecutorError::Corrupt);
        }
        let limit = usize::try_from(self.capacity).map_err(|_| ExecutorError::Capacity)?;
        let observed = self
            .receipts
            .iter()
            .take(limit.saturating_add(1))
            .try_fold(0_u32, |count, guard| {
                guard.key()?;
                count.checked_add(1).ok_or(ExecutorError::Capacity)
            })?;
        if observed != meta.count {
            return Err(ExecutorError::Corrupt);
        }
        Ok(())
    }

    fn lookup(
        &self,
        grant: &RegisterGrant,
        value: i64,
    ) -> Result<Option<RegisterReceipt>, ExecutorError> {
        let Some(bytes) = self.receipts.get(receipt::receipt_key(grant.ticket)?)? else {
            return Ok(None);
        };
        if bytes.len() > MAX_RECEIPT_BYTES {
            return Err(ExecutorError::Corrupt);
        }
        let record: ReceiptRecord = postcard::from_bytes(&bytes)?;
        receipt::validate_record(&record)?;
        let expected = receipt::record(grant, value, record.body.effect_ordinal)?;
        if record != expected {
            return Err(ExecutorError::ReceiptConflict);
        }
        Ok(Some(RegisterReceipt(record)))
    }

    fn commit_effect(
        &mut self,
        grant: &RegisterGrant,
        value: i64,
    ) -> Result<RegisterReceipt, ExecutorError> {
        let meta = self.load_metadata()?;
        if meta.count >= self.capacity {
            return Err(ExecutorError::Capacity);
        }
        let count = meta.count.checked_add(1).ok_or(ExecutorError::Capacity)?;
        let effects = meta.effects.checked_add(1).ok_or(ExecutorError::Capacity)?;
        let record = receipt::record(grant, value, effects)?;
        let encoded = postcard::to_allocvec(&record)?;
        if encoded.len() > MAX_RECEIPT_BYTES {
            return Err(ExecutorError::Capacity);
        }
        let mut batch = self.db.batch();
        batch.insert(
            &self.targets,
            target_key(grant.scope)?,
            postcard::to_allocvec(&value)?,
        );
        batch.insert(&self.receipts, receipt::receipt_key(grant.ticket)?, encoded);
        let next = Metadata {
            schema: 1,
            capacity: self.capacity,
            count,
            effects,
        };
        batch.insert(&self.metadata, b"state", postcard::to_allocvec(&next)?);
        batch.commit()?;
        Ok(RegisterReceipt(record))
    }
}

fn target_key(scope: CapabilityScopeSpec) -> Result<Vec<u8>, ExecutorError> {
    Ok(postcard::to_allocvec(&scope)?)
}
