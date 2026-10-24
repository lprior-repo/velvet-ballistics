//! Pre-traversal graph validation: duplicate keys and edge work budget.

use super::super::errors::OwnedRootBundleError as Error;
use super::arena;
use crate::ids::ObjectId;
use crate::value_store::ValueStoreCheckpointV1;

/// Validate checkpoint graph shape before DFS traversal.
pub(in crate::root_bundle) fn validate(cp: &ValueStoreCheckpointV1) -> Result<(), Error> {
    check_duplicate_keys(cp)?;
    check_edge_work(cp)?;
    Ok(())
}

/// Scan for duplicate object field keys across all objects.
fn check_duplicate_keys(cp: &ValueStoreCheckpointV1) -> Result<(), Error> {
    let symbol_count = cp.symbols.len();
    let mut markers = arena::reserve(symbol_count, None)?;

    for (obj_idx, fields) in cp.objects.iter().enumerate() {
        let obj_id = ObjectId::new(u32::try_from(obj_idx).map_err(|_| arena::invariant())?);
        check_one_object(&mut markers, obj_id, fields)?;
    }

    Ok(())
}

/// Validate a single object's fields for duplicate keys.
fn check_one_object(
    markers: &mut [Option<ObjectId>],
    obj_id: ObjectId,
    fields: &[crate::value_store::ObjectField],
) -> Result<(), Error> {
    for field in fields.iter() {
        let key_idx = usize::try_from(field.key.get()).map_err(|_| arena::invariant())?;

        let slot = markers.get_mut(key_idx).ok_or(arena::invariant())?;

        if *slot == Some(obj_id) {
            return Err(Error::GraphDuplicateObjectKey {
                object: obj_id,
                key: field.key,
            });
        }
        *slot = Some(obj_id);
    }
    Ok(())
}

/// Verify total edge work is within the 1,000,000 budget.
fn check_edge_work(cp: &ValueStoreCheckpointV1) -> Result<(), Error> {
    let mut total: u64 = 0;

    for list in cp.lists.iter() {
        let len = u64::try_from(list.len()).map_err(|_| arena::invariant())?;
        total = checked_edge_work(total, len)?;
    }

    for fields in cp.objects.iter() {
        let field_count = u64::try_from(fields.len()).map_err(|_| arena::invariant())?;
        let work = field_count
            .checked_mul(2)
            .ok_or(Error::GraphEdgeWorkOverflow)?;
        total = checked_edge_work(total, work)?;
    }

    Ok(())
}

/// Add edge work with overflow and budget checks.
pub fn checked_edge_work(current: u64, additional: u64) -> Result<u64, Error> {
    let sum = current
        .checked_add(additional)
        .ok_or(Error::GraphEdgeWorkOverflow)?;

    if sum > 1_000_000 {
        return Err(Error::GraphEdgeWorkExceeded {
            edges: sum,
            limit: 1_000_000,
        });
    }

    Ok(sum)
}
