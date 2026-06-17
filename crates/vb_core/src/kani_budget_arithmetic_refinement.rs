#![cfg(kani)]
#![forbid(unsafe_code)]

//! VB-9WG3: refinement proof between `BudgetArithmetic.tla` four-limb
//! arithmetic and Rust `u64` checked arithmetic.

const MAX_U16_LIMB: u64 = 65_535;
const BASE: u64 = 65_536;
const ONE_PAST_U32: u64 = 4_294_967_296;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Word {
    l0: u64,
    l1: u64,
    l2: u64,
    l3: u64,
}

// Manual transcription of `BudgetArithmetic.tla` lines 57 and 60.
const TLA_MAX_U16_WORD: Word = Word {
    l0: MAX_U16_LIMB,
    l1: 0,
    l2: 0,
    l3: 0,
};

const TLA_MAX_U32_WORD: Word = Word {
    l0: MAX_U16_LIMB,
    l1: MAX_U16_LIMB,
    l2: 0,
    l3: 0,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordError {
    Overflow,
    Underflow,
}

fn word_from_u64(value: u64) -> Word {
    Word {
        l0: value & MAX_U16_LIMB,
        l1: (value >> 16) & MAX_U16_LIMB,
        l2: (value >> 32) & MAX_U16_LIMB,
        l3: (value >> 48) & MAX_U16_LIMB,
    }
}

fn word_to_u64(word: Word) -> u64 {
    word.l0 | (word.l1 << 16) | (word.l2 << 32) | (word.l3 << 48)
}

fn word_type_ok(word: Word) -> bool {
    word.l0 <= MAX_U16_LIMB
        && word.l1 <= MAX_U16_LIMB
        && word.l2 <= MAX_U16_LIMB
        && word.l3 <= MAX_U16_LIMB
}

fn word_lt(left: Word, right: Word) -> bool {
    left.l3 < right.l3
        || (left.l3 == right.l3 && left.l2 < right.l2)
        || (left.l3 == right.l3 && left.l2 == right.l2 && left.l1 < right.l1)
        || (left.l3 == right.l3 && left.l2 == right.l2 && left.l1 == right.l1 && left.l0 < right.l0)
}

fn word_le(left: Word, right: Word) -> bool {
    word_lt(left, right) || left == right
}

fn checked_add_or_overflow(left: u64, right: u64) -> Result<u64, WordError> {
    left.checked_add(right).ok_or(WordError::Overflow)
}

fn checked_sub_or_underflow(left: u64, right: u64) -> Result<u64, WordError> {
    left.checked_sub(right).ok_or(WordError::Underflow)
}

fn carry(sum: u64) -> u64 {
    if sum <= MAX_U16_LIMB { 0 } else { 1 }
}

fn limb(sum: u64) -> Result<u64, WordError> {
    if sum <= MAX_U16_LIMB {
        Ok(sum)
    } else {
        checked_sub_or_underflow(sum, BASE)
    }
}

fn add_word(left: Word, right: Word) -> Result<Word, WordError> {
    let s0 = checked_add_or_overflow(left.l0, right.l0)?;
    let r0 = limb(s0)?;
    let c0 = carry(s0);

    let l1_sum = checked_add_or_overflow(left.l1, right.l1)?;
    let s1 = checked_add_or_overflow(l1_sum, c0)?;
    let r1 = limb(s1)?;
    let c1 = carry(s1);

    let l2_sum = checked_add_or_overflow(left.l2, right.l2)?;
    let s2 = checked_add_or_overflow(l2_sum, c1)?;
    let r2 = limb(s2)?;
    let c2 = carry(s2);

    let l3_sum = checked_add_or_overflow(left.l3, right.l3)?;
    let s3 = checked_add_or_overflow(l3_sum, c2)?;
    let r3 = limb(s3)?;

    if carry(s3) == 0 {
        Ok(Word {
            l0: r0,
            l1: r1,
            l2: r2,
            l3: r3,
        })
    } else {
        Err(WordError::Overflow)
    }
}

fn sub_limb_with_borrow(
    minuend: u64,
    subtrahend: u64,
    borrow: u64,
) -> Result<(u64, u64), WordError> {
    let total_subtrahend = checked_add_or_overflow(subtrahend, borrow)?;
    if minuend >= total_subtrahend {
        Ok((checked_sub_or_underflow(minuend, total_subtrahend)?, 0))
    } else {
        let shortfall = checked_sub_or_underflow(total_subtrahend, minuend)?;
        Ok((checked_sub_or_underflow(BASE, shortfall)?, 1))
    }
}

fn sub_word(left: Word, right: Word) -> Result<Word, WordError> {
    if !word_le(right, left) {
        return Err(WordError::Underflow);
    }

    let (r0, b0) = sub_limb_with_borrow(left.l0, right.l0, 0)?;
    let (r1, b1) = sub_limb_with_borrow(left.l1, right.l1, b0)?;
    let (r2, b2) = sub_limb_with_borrow(left.l2, right.l2, b1)?;
    let (r3, b3) = sub_limb_with_borrow(left.l3, right.l3, b2)?;
    if b3 != 0 {
        return Err(WordError::Underflow);
    }

    Ok(Word {
        l0: r0,
        l1: r1,
        l2: r2,
        l3: r3,
    })
}

#[kani::proof]
fn tla_word_round_trips_all_rust_u64_values() {
    let value: u64 = kani::any();
    let word = word_from_u64(value);

    kani::assert(word_type_ok(word), "encoded word has four 16-bit limbs");
    kani::assert(word_to_u64(word) == value,
        "TLA word decodes to original u64",
    );
}

#[kani::proof]
fn tla_word_order_matches_rust_u64_order() {
    let left: u64 = kani::any();
    let right: u64 = kani::any();
    let model_order = word_le(word_from_u64(left), word_from_u64(right));

    kani::assert(model_order == (left <= right), "TLA WordLE matches Rust <=");
}

#[kani::proof]
fn tla_add_word_matches_rust_checked_add_for_all_u64() {
    let current: u64 = kani::any();
    let requested: u64 = kani::any();
    let model = add_word(word_from_u64(current), word_from_u64(requested));

    match current.checked_add(requested) {
        Some(expected) => match model {
            Ok(actual) => {
                kani::assert(word_type_ok(actual), "TLA AddWord Ok preserves WordTypeOK");
                kani::assert(word_to_u64(actual) == expected,
                    "TLA AddWord Ok value matches checked_add",
                );
            }
            Err(_) =>  == expected,
                    "TLA AddWord Ok value matches checked_add",
                );
            }
            Err(_) => kani::assert(
                false,
                "TLA AddWord must not overflow when checked_add succeeds",
            ),
        },
        None => match model {
            Ok(_) => kani::assert(
                false,
                "TLA AddWord must overflow when checked_add overflows",
            ),
            Err(error) => {
                kani::assert(error == WordError::Overflow, "TLA AddWord reports Overflow");
            }
        },
    }
}

#[kani::proof]
fn tla_sub_word_matches_rust_checked_sub_for_all_u64() {
    let current: u64 = kani::any();
    let requested: u64 = kani::any();
    let model = sub_word(word_from_u64(current), word_from_u64(requested));

    match current.checked_sub(requested) {
        Some(expected) => match model {
            Ok(actual) => {
                kani::assert(word_type_ok(actual), "TLA SubWord Ok preserves WordTypeOK");
                kani::assert(word_to_u64(actual) == expected,
                    "TLA SubWord Ok value matches checked_sub",
                );
            }
            Err(_) =>  == expected,
                    "TLA SubWord Ok value matches checked_sub",
                );
            }
            Err(_) => kani::assert(
                false,
                "TLA SubWord must not underflow when checked_sub succeeds",
            ),
        },
        None => match model {
            Ok(_) => kani::assert(
                false,
                "TLA SubWord must underflow when checked_sub underflows",
            ),
            Err(error) => kani::assert(
                error == WordError::Underflow,
                "TLA SubWord reports Underflow",
            ),
        },
    }
}

#[kani::proof]
fn tla_budget_field_widths_match_rust_domains() {
    let u16_value: u16 = kani::any();
    let u32_value: u32 = kani::any();

    kani::assert(
        word_from_u64(u64::from(u16::MAX)) == TLA_MAX_U16_WORD,
        "Rust u16::MAX encodes to the transcribed TLA MaxU16Word",
    );
    kani::assert(word_from_u64(u64::from(u32::MAX)) == TLA_MAX_U32_WORD,
        "Rust u32::MAX encodes to the transcribed TLA MaxU32Word",
    );

    kani::assert(word_le(word_from_u64(u64::from(u16_value)), TLA_MAX_U16_WORD),
        "every Rust u16 value fits the TLA U16 field max",
    );
    kani::assert(!word_le(word_from_u64(BASE), TLA_MAX_U16_WORD),
        "one past Rust u16 max exceeds the TLA U16 field max",
    );
    kani::assert(word_le(word_from_u64(u64::from(u32_value)), TLA_MAX_U32_WORD),
        "every Rust u32 value fits the TLA U32 field max",
    );
    kani::assert(!word_le(word_from_u64(ONE_PAST_U32), TLA_MAX_U32_WORD),
        "one past Rust u32 max exceeds the TLA U32 field max",
    );
}
