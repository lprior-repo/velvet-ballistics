// Obligation: PO-R07
// Claim: Migration counters and byte limits use checked bounded arithmetic
// and cannot overflow into success. Overflow returns typed limit error.
#![cfg(kani)]

const MAX_BYTES: u8 = 64;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    current_bytes: u8,
    delta_bytes: u8,
    current_count: u8,
    delta_count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccountingResult {
    Ok { total_bytes: u8, total_count: u8 },
    LimitExceeded,
}

fn adapter_checked_accounting(
    bytes: u8,
    delta_bytes: u8,
    count: u8,
    delta_count: u8,
) -> AccountingResult {
    let total_bytes = match bytes.checked_add(delta_bytes) {
        Some(t) if t <= MAX_BYTES => t,
        _ => return AccountingResult::LimitExceeded,
    };
    let total_count = match count.checked_add(delta_count) {
        Some(t) => t,
        None => return AccountingResult::LimitExceeded,
    };
    AccountingResult::Ok {
        total_bytes,
        total_count,
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_migration_accounting_checked_bounds() {
    let input: AoahInput = kani::any();
    kani::assume(input.current_bytes <= MAX_BYTES);

    let result = adapter_checked_accounting(
        input.current_bytes,
        input.delta_bytes,
        input.current_count,
        input.delta_count,
    );

    match result {
        AccountingResult::Ok {
            total_bytes,
            total_count,
        } => {
            // Claim: success path is always within bounds
            assert!(total_bytes <= MAX_BYTES);
            // checked_add did not overflow
            assert!(total_bytes >= input.current_bytes);
            assert!(total_count >= input.current_count);
        }
        AccountingResult::LimitExceeded => {
            // Claim: overflow path returns typed limit error, not wrapped success
            let bytes_overflow = input.current_bytes.checked_add(input.delta_bytes).is_none()
                || input.current_bytes.saturating_add(input.delta_bytes) > MAX_BYTES;
            let count_overflow = input.current_count.checked_add(input.delta_count).is_none();
            assert!(bytes_overflow || count_overflow);
        }
    }
}
