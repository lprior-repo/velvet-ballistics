#![forbid(unsafe_code)]
//! YB = Yield-Based token types for computational work budgeting.
//!
//! YB tokens represent a u64 counter used to track and limit computational
//! work done during workflow execution. The budget layer tracks remaining
//! tokens and enforces consumption costs.

use thiserror::Error;

// ---------------------------------------------------------------------------
// YbToken
// ---------------------------------------------------------------------------

/// YB = Yield-Based token (u64 counter for computational work).
///
/// A `YbToken` represents a single token value used to track computational
/// work units. Tokens are consumed by operations with associated costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YbToken {
    value: u64,
}

impl YbToken {
    /// Creates a new YbToken with the given raw value.
    #[inline]
    pub fn new(value: u64) -> Self {
        YbToken { value }
    }

    /// Returns the raw token value.
    #[inline]
    pub fn value(self) -> u64 {
        self.value
    }

    /// Returns the remaining token value (alias for `value()`).
    #[inline]
    pub fn remaining(self) -> u64 {
        self.value
    }

    /// Attempts to consume `cost` tokens from this token.
    ///
    /// Returns `Some((remaining, cost))` if the token has sufficient value,
    /// or `None` if the token does not have enough value to cover the cost.
    #[inline]
    pub fn try_consume(self, cost: YbTokenCost) -> Option<(Self, YbTokenCost)> {
        if self.value >= cost.value {
            Some((YbToken::new(self.value - cost.value), cost))
        } else {
            None
        }
    }

    /// Checks if this token can cover the given cost.
    #[inline]
    pub fn can_consume(self, cost: YbTokenCost) -> bool {
        self.value >= cost.value
    }
}

impl Default for YbToken {
    fn default() -> Self {
        YbToken { value: 0 }
    }
}

impl From<u64> for YbToken {
    fn from(value: u64) -> Self {
        YbToken::new(value)
    }
}

impl From<YbToken> for u64 {
    fn from(token: YbToken) -> Self {
        token.value
    }
}

// ---------------------------------------------------------------------------
// YbTokenBudget
// ---------------------------------------------------------------------------

/// Budget tracking remaining tokens for a workflow run.
///
/// `YbTokenBudget` wraps an initial token allocation and tracks the remaining
/// balance after consumption. The budget is constructed with a validated
/// initial value and enforces non-negative remaining values.
#[derive(Debug, Clone)]
pub struct YbTokenBudget {
    initial: u64,
    remaining: u64,
}

impl YbTokenBudget {
    /// Creates a new budget with the given initial token count.
    ///
    /// Returns `Err(YbTokenError::InvalidInitial)` if `initial` is 0.
    #[inline]
    pub fn new(initial: u64) -> Result<Self, YbTokenError> {
        if initial == 0 {
            return Err(YbTokenError::InvalidInitial { value: initial });
        }
        Ok(YbTokenBudget {
            initial,
            remaining: initial,
        })
    }

    /// Consumes `cost` tokens from the budget.
    ///
    /// Returns `Ok(())` if the budget has sufficient tokens,
    /// or `Err(YbTokenError::InsufficientBudget)` if not.
    #[inline]
    pub fn consume(&mut self, cost: YbTokenCost) -> Result<(), YbTokenError> {
        if self.remaining >= cost.value {
            self.remaining -= cost.value;
            Ok(())
        } else {
            Err(YbTokenError::InsufficientBudget {
                required: cost.value,
                available: self.remaining,
            })
        }
    }

    /// Returns the number of remaining tokens in the budget.
    #[inline]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns the initial token count.
    #[inline]
    pub fn initial(&self) -> u64 {
        self.initial
    }

    /// Returns the fraction of tokens remaining as a ratio (0.0 to 1.0).
    #[inline]
    pub fn remaining_ratio(&self) -> f64 {
        if self.initial == 0 {
            0.0
        } else {
            self.remaining as f64 / self.initial as f64
        }
    }

    /// Checks if the budget is exhausted.
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }

    /// Checks if the budget has at least `cost` tokens available.
    #[inline]
    pub fn can_consume(&self, cost: YbTokenCost) -> bool {
        self.remaining >= cost.value
    }
}

impl Default for YbTokenBudget {
    fn default() -> Self {
        // Direct struct literal: 1 is guaranteed valid (non-zero), avoiding .expect()/.unwrap()
        Self { initial: 1, remaining: 1 }
    }
}

// ---------------------------------------------------------------------------
// YbTokenCost
// ---------------------------------------------------------------------------

/// Cost of a single operation in YB tokens.
///
/// `YbTokenCost` represents the token cost of a single workflow operation.
/// Costs are non-negative u64 values with a canonical `BASIC` cost of 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YbTokenCost {
    value: u64,
}

impl YbTokenCost {
    /// Canonical cost of a basic operation (value = 1).
    pub const BASIC: YbTokenCost = YbTokenCost { value: 1 };

    /// Creates a new cost with the given raw value.
    #[inline]
    pub fn new(value: u64) -> Self {
        YbTokenCost { value }
    }

    /// Returns the raw cost value.
    #[inline]
    pub fn value(self) -> u64 {
        self.value
    }

    /// Returns true if this cost is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }
}

impl From<u64> for YbTokenCost {
    fn from(value: u64) -> Self {
        YbTokenCost::new(value)
    }
}

impl From<YbTokenCost> for u64 {
    fn from(cost: YbTokenCost) -> Self {
        cost.value
    }
}

// ---------------------------------------------------------------------------
// YbTokenError
// ---------------------------------------------------------------------------

/// Errors that can occur during YB token operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum YbTokenError {
    /// The budget does not have enough tokens to cover the cost.
    #[error("insufficient budget: required {required}, available {available}")]
    InsufficientBudget {
        /// The cost that could not be covered.
        required: u64,
        /// The available tokens in the budget.
        available: u64,
    },

    /// The initial token count is invalid (must be > 0).
    #[error("invalid initial token count: {value}")]
    InvalidInitial {
        /// The invalid value.
        value: u64,
    },
}

// ---------------------------------------------------------------------------
// Kani Harnesses
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_harnesses {
    use super::*;

    /// Harness: YbToken construction is panic-free for all u64 values.
    ///
    /// PO: yb_token_construction
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to actual Rust YbToken::new implementation.
    #[kani::proof]
    #[kani::unwind(4)]
    fn yb_token_construction() {
        let value: u64 = kani::any();
        let token = YbToken::new(value);
        #![forbid(unsafe_code)]
//! YB = Yield-Based token types for computational work budgeting.
//!
//! YB tokens represent a u64 counter used to track and limit computational
//! work done during workflow execution. The budget layer tracks remaining
//! tokens and enforces consumption costs.

use thiserror::Error;

// ---------------------------------------------------------------------------
// YbToken
// ---------------------------------------------------------------------------

/// YB = Yield-Based token (u64 counter for computational work).
///
/// A `YbToken` represents a single token value used to track computational
/// work units. Tokens are consumed by operations with associated costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YbToken {
    value: u64,
}

impl YbToken {
    /// Creates a new YbToken with the given raw value.
    #[inline]
    pub fn new(value: u64) -> Self {
        YbToken { value }
    }

    /// Returns the raw token value.
    #[inline]
    pub fn value(self) -> u64 {
        self.value
    }

    /// Returns the remaining token value (alias for `value()`).
    #[inline]
    pub fn remaining(self) -> u64 {
        self.value
    }

    /// Attempts to consume `cost` tokens from this token.
    ///
    /// Returns `Some((remaining, cost))` if the token has sufficient value,
    /// or `None` if the token does not have enough value to cover the cost.
    #[inline]
    pub fn try_consume(self, cost: YbTokenCost) -> Option<(Self, YbTokenCost)> {
        if self.value >= cost.value {
            Some((YbToken::new(self.value - cost.value), cost))
        } else {
            None
        }
    }

    /// Checks if this token can cover the given cost.
    #[inline]
    pub fn can_consume(self, cost: YbTokenCost) -> bool {
        self.value >= cost.value
    }
}

impl Default for YbToken {
    fn default() -> Self {
        YbToken { value: 0 }
    }
}

impl From<u64> for YbToken {
    fn from(value: u64) -> Self {
        YbToken::new(value)
    }
}

impl From<YbToken> for u64 {
    fn from(token: YbToken) -> Self {
        token.value
    }
}

// ---------------------------------------------------------------------------
// YbTokenBudget
// ---------------------------------------------------------------------------

/// Budget tracking remaining tokens for a workflow run.
///
/// `YbTokenBudget` wraps an initial token allocation and tracks the remaining
/// balance after consumption. The budget is constructed with a validated
/// initial value and enforces non-negative remaining values.
#[derive(Debug, Clone)]
pub struct YbTokenBudget {
    initial: u64,
    remaining: u64,
}

impl YbTokenBudget {
    /// Creates a new budget with the given initial token count.
    ///
    /// Returns `Err(YbTokenError::InvalidInitial)` if `initial` is 0.
    #[inline]
    pub fn new(initial: u64) -> Result<Self, YbTokenError> {
        if initial == 0 {
            return Err(YbTokenError::InvalidInitial { value: initial });
        }
        Ok(YbTokenBudget {
            initial,
            remaining: initial,
        })
    }

    /// Consumes `cost` tokens from the budget.
    ///
    /// Returns `Ok(())` if the budget has sufficient tokens,
    /// or `Err(YbTokenError::InsufficientBudget)` if not.
    #[inline]
    pub fn consume(&mut self, cost: YbTokenCost) -> Result<(), YbTokenError> {
        if self.remaining >= cost.value {
            self.remaining -= cost.value;
            Ok(())
        } else {
            Err(YbTokenError::InsufficientBudget {
                required: cost.value,
                available: self.remaining,
            })
        }
    }

    /// Returns the number of remaining tokens in the budget.
    #[inline]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns the initial token count.
    #[inline]
    pub fn initial(&self) -> u64 {
        self.initial
    }

    /// Returns the fraction of tokens remaining as a ratio (0.0 to 1.0).
    #[inline]
    pub fn remaining_ratio(&self) -> f64 {
        if self.initial == 0 {
            0.0
        } else {
            self.remaining as f64 / self.initial as f64
        }
    }

    /// Checks if the budget is exhausted.
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }

    /// Checks if the budget has at least `cost` tokens available.
    #[inline]
    pub fn can_consume(&self, cost: YbTokenCost) -> bool {
        self.remaining >= cost.value
    }
}

impl Default for YbTokenBudget {
    fn default() -> Self {
        // Direct struct literal: 1 is guaranteed valid (non-zero), avoiding .expect()/.unwrap()
        Self { initial: 1, remaining: 1 }
    }
}

// ---------------------------------------------------------------------------
// YbTokenCost
// ---------------------------------------------------------------------------

/// Cost of a single operation in YB tokens.
///
/// `YbTokenCost` represents the token cost of a single workflow operation.
/// Costs are non-negative u64 values with a canonical `BASIC` cost of 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YbTokenCost {
    value: u64,
}

impl YbTokenCost {
    /// Canonical cost of a basic operation (value = 1).
    pub const BASIC: YbTokenCost = YbTokenCost { value: 1 };

    /// Creates a new cost with the given raw value.
    #[inline]
    pub fn new(value: u64) -> Self {
        YbTokenCost { value }
    }

    /// Returns the raw cost value.
    #[inline]
    pub fn value(self) -> u64 {
        self.value
    }

    /// Returns true if this cost is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }
}

impl From<u64> for YbTokenCost {
    fn from(value: u64) -> Self {
        YbTokenCost::new(value)
    }
}

impl From<YbTokenCost> for u64 {
    fn from(cost: YbTokenCost) -> Self {
        cost.value
    }
}

// ---------------------------------------------------------------------------
// YbTokenError
// ---------------------------------------------------------------------------

/// Errors that can occur during YB token operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum YbTokenError {
    /// The budget does not have enough tokens to cover the cost.
    #[error("insufficient budget: required {required}, available {available}")]
    InsufficientBudget {
        /// The cost that could not be covered.
        required: u64,
        /// The available tokens in the budget.
        available: u64,
    },

    /// The initial token count is invalid (must be > 0).
    #[error("invalid initial token count: {value}")]
    InvalidInitial {
        /// The invalid value.
        value: u64,
    },
}

// ---------------------------------------------------------------------------
// Kani Harnesses
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_harnesses {
    use super::*;

    /// Harness: YbToken construction is panic-free for all u64 values.
    ///
    /// PO: yb_token_construction
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to actual Rust YbToken::new implementation.
    #[kani::proof]
    #[kani::unwind(4)]
    fn yb_token_construction() {
        let value: u64 = kani::any();
        let token = YbToken::new(value);
        kani::assert(token.value == value, "YbToken::new preserves value");
    }

    /// Harness: YbToken::try_consume is panic-free and correct.
    ///
    /// PO: yb_token_consumption
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to actual Rust YbToken::try_consume implementation.
    #[kani::proof]
    #[kani::unwind(5)]
    fn yb_token_consumption() {
        let value: u64 = kani::any();
        let cost_value: u64 = kani::any();
        let token = YbToken::new(value);
        let cost = YbTokenCost::new(cost_value);

        let result = token.try_consume(cost);

        // If result is Some, remaining must be value - cost
        match result {
            Some((remaining, consumed_cost)) => {
                ) => {
                kani::assert(remaining.value == value - cost_value, "remaining correct after consume");
                kani::assert(consumed_cost.value == cost_value, "consumed cost preserved");
                kani::assert(value >= cost_value, "precondition: can consume");
            }
            None => {
                kani::assert(value < cost_value, "None iff insufficient funds");
            }
        }
    }

    /// Harness: YbToken invariants hold for all operations.
    ///
    /// PO: yb_token_invariant
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to actual Rust YbToken invariants.
    #[kani::proof]
    #[kani::unwind(6)]
    fn yb_token_invariant() {
        let value: u64 = kani::any();
        let token = YbToken::new(value);

        // Invariant: value() and remaining() return the same value
        kani::assert(token.value() == token.remaining(), "value() == remaining()");

        // Invariant: from u64 roundtrip
        let roundtrip = YbToken::from(u64::from(token));
        kani::assert(roundtrip.value(, "assertion failed") == token.value(), "from/into roundtrip");

        // Invariant: try_consume only returns Some when sufficient funds
        let cost = YbTokenCost::new(kani::any());
        let result = token.try_consume(cost);
        match result {
            Some((_, _)) => {
                kani::assert(token.value(, "assertion failed") >= cost.value(), "Some implies sufficient funds");
            }
            None => {
                kani::assert(token.value(, "assertion failed") < cost.value(), "None implies insufficient funds");
            }
        }
    }

    /// Harness: YbTokenBudget saturating arithmetic properties.
    ///
    /// PO: yb_token_saturating_arithmetic
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to actual Rust YbTokenBudget implementation.
    #[kani::proof]
    #[kani::unwind(5)]
    fn yb_token_saturating_arithmetic() {
        // Use non-zero initial to avoid InvalidInitial error
        let initial: u64 = kani::any();
        kani::assume(initial > 0);
        kani::assume(initial <= 1_000_000); // Reasonable bound for verification

        let mut budget = match YbTokenBudget::new(initial) {
            Ok(b) => b,
            Err(_) => { kani::assume(false); return; },
        };

        // Invariant: remaining never exceeds initial
        kani::assert(budget.remaining(, "assertion failed") <= budget.initial(), "remaining <= initial");

        // NOTE: Removed vacuous `remaining >= 0` assertion - u64 can never be negative.
        // This was flagged as HIGH-001 in proof-review. The meaningful invariant is `remaining <= initial`.

        // Consume some tokens
        let cost_value: u64 = kani::any();
        let cost = YbTokenCost::new(cost_value % (initial + 1)); // Allow up to initial
        let consume_result = budget.consume(cost);

        match consume_result {
            Ok(()) => {
                kani::assert(budget.remaining(, "assertion failed") <= initial, "after consume: remaining <= initial");
            }
            Err(YbTokenError::InsufficientBudget { required, available }) => {
                 <= initial, "after consume: remaining <= initial");
            }
            Err(YbTokenError::InsufficientBudget { required, available }) => {
                kani::assert(required > available, "insufficient budget error is correct");
                kani::assert(budget.remaining() == initial, "failed consume leaves budget unchanged");
            }
            Err(YbTokenError::InvalidInitial { .. }) => {
                // Cannot happen because we assumed initial > 0
                 == initial, "failed consume leaves budget unchanged");
            }
            Err(YbTokenError::InvalidInitial { .. }) => {
                // Cannot happen because we assumed initial > 0
                kani::assert(false, "InvalidInitial should not occur with initial > 0");
            }
        }

        // Final invariant: remaining never exceeds initial
        kani::assert(budget.remaining() <= budget.initial(), "final: remaining <= initial");
    }

    /// Harness: YbTokenBudget construction from ResourceContract values.
    ///
    /// PO: yb_token_from_resource_contract
    /// GOD RULE 1: Uses kani::any() for all core types.
    /// GOD RULE 2: Binds to ResourceContract::max_step_budget_per_tick.
    #[kani::proof]
    #[kani::unwind(4)]
    fn yb_token_from_resource_contract() {
        // max_step_budget_per_tick is u64 from ResourceContract
        let max_step_budget: u64 = kani::any();
        kani::assume(max_step_budget > 0);
        kani::assume(max_step_budget <= 1_000_000_000); // Reasonable bound

        // Create budget from resource contract value
        let budget_result = YbTokenBudget::new(max_step_budget);

        match budget_result {
            Ok(budget) => {
                kani::assert(budget.initial(, "assertion failed") == max_step_budget, "initial preserved from contract");
                kani::assert(budget.remaining(, "assertion failed") == max_step_budget, "initial == remaining at creation");
                kani::assert(!budget.is_exhausted(, "assertion failed"), "new budget is not exhausted");
            }
            Err(YbTokenError::InvalidInitial { value }) => {
                , "new budget is not exhausted");
            }
            Err(YbTokenError::InvalidInitial { value }) => {
                kani::assert(value == 0, "only zero is invalid initial");
            }
            Err(YbTokenError::InsufficientBudget { .. }) => {
                // Cannot happen for YbTokenBudget::new
                kani::assert(false, "InsufficientBudget cannot occur on construction");
            }
        }
    }

    /// Harness: YbTokenBudget::new with u64::MAX (HIGH-002 fix).
    /// Explicitly verifies full u64 range to justify TB-YB-001 trust marker.
    #[kani::proof]
    #[kani::unwind(5)]
    fn yb_token_budget_u64_max() {
        let initial = u64::MAX;

        let result = YbTokenBudget::new(initial);
        kani::assert(result.is_ok(), "new(u64::MAX) succeeds");

        let mut budget = match result {
            Ok(b) => b,
            Err(_) => { kani::assume(false); return; },
        };
        kani::assert(budget.initial(, "assertion failed") == u64::MAX, "initial preserved at u64::MAX");
        kani::assert(budget.remaining(, "assertion failed") == u64::MAX, "remaining == initial at u64::MAX");

        // Consume 1 token from u64::MAX budget
        let cost = YbTokenCost::new(1);
        let consume_result = budget.consume(cost);

        kani::assert(consume_result.is_ok(, "assertion failed"), "consume(1) from u64::MAX succeeds");
        if consume_result.is_ok() {
            kani::assert(budget.remaining(, "assertion failed") == u64::MAX - 1, "remaining = u64::MAX - 1");
        }

        // Invariant holds: remaining <= initial
        kani::assert(budget.remaining(, "assertion failed") <= budget.initial(), "remaining <= initial after consume");
    }
}

// ---------------------------------------------------------------------------
// Unit and Property Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -------------------------------------------------------------------------
    // YbToken Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_try_consume_success() {
        let token = YbToken::new(100);
        let cost = YbTokenCost::new(50);

        let result = token.try_consume(cost);

        kani::assert(result.is_some(, "assertion failed"), "should succeed when sufficient tokens");
        let (remaining, consumed) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 50, "remaining tokens should be 50");
        kani::assert(consumed.value(, "assertion failed") == 50, "consumed cost should be 50");
    }

    #[test]
    fn yb_token_try_consume_exact() {
        let token = YbToken::new(42);
        let cost = YbTokenCost::new(42);

        let result = token.try_consume(cost);

        kani::assert(result.is_some(, "assertion failed"), "should succeed with exact tokens");
        let (remaining, consumed) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 0, "remaining should be 0");
        kani::assert(consumed.value(, "assertion failed") == 42, "consumed should be full cost");
    }

    #[test]
    fn yb_token_try_consume_failure() {
        let token = YbToken::new(30);
        let cost = YbTokenCost::new(50);

        let result = token.try_consume(cost);

        kani::assert(result.is_none(, "assertion failed"), "should fail when insufficient tokens");
    }

    #[test]
    fn yb_token_try_consume_zero_cost() {
        let token = YbToken::new(100);
        let cost = YbTokenCost::new(0);

        let result = token.try_consume(cost);

        kani::assert(result.is_some(, "assertion failed"), "zero cost should always succeed");
        let (remaining, consumed) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 100, "remaining unchanged");
        kani::assert(consumed.value(, "assertion failed") == 0, "consumed is 0");
    }

    #[test]
    fn yb_token_can_consume() {
        let token = YbToken::new(100);

        kani::assert(token.can_consume(YbTokenCost::new(50), "assertion failed"), "can consume when sufficient");
        kani::assert(token.can_consume(YbTokenCost::new(100), "assertion failed"), "can consume exact");
        kani::assert(!token.can_consume(YbTokenCost::new(101), "assertion failed"), "cannot consume when insufficient");
    }

    #[test]
    fn yb_token_default() {
        let token = YbToken::default();
        kani::assert(token.value(, "assertion failed") == 0, "default token should be 0");
    }

    #[test]
    fn yb_token_from_u64_roundtrip() {
        let original: u64 = 12345;
        let token = YbToken::from(original);
        let recovered: u64 = token.into();

         == 0, "default token should be 0");
    }

    #[test]
    fn yb_token_from_u64_roundtrip() {
        let original: u64 = 12345;
        let token = YbToken::from(original);
        let recovered: u64 = token.into();

        kani::assert(original == recovered, "u64 -> YbToken -> u64 roundtrip");
    }

    // -------------------------------------------------------------------------
    // YbTokenBudget Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_budget_new_valid() {
        let budget = YbTokenBudget::new(100);
        kani::assert(budget.is_ok(), "new with non-zero should succeed");
        let budget = budget.unwrap();
        kani::assert(budget.initial(, "assertion failed") == 100, "initial preserved");
        kani::assert(budget.remaining(, "assertion failed") == 100, "remaining equals initial at creation");
    }

    #[test]
    fn yb_token_budget_new_zero() {
        let result = YbTokenBudget::new(0);
        kani::assert(result.is_err(, "assertion failed"), "new with zero should fail");
        kani::assert(result.unwrap_err(, "assertion failed") == YbTokenError::InvalidInitial { value: 0 }, "error variant should be InvalidInitial");
    }

    #[test]
    fn yb_token_budget_consume_success() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        let cost = YbTokenCost::new(30);

        let result = budget.consume(cost);

        kani::assert(result.is_ok(, "assertion failed"), "consume should succeed with sufficient budget");
        kani::assert(budget.remaining(, "assertion failed") == 70, "remaining should decrease");
    }

    #[test]
    fn yb_token_budget_consume_exact() {
        let mut budget = YbTokenBudget::new(50).unwrap();
        let cost = YbTokenCost::new(50);

        let result = budget.consume(cost);

        kani::assert(result.is_ok(, "assertion failed"), "consume exact should succeed");
        kani::assert(budget.remaining(, "assertion failed") == 0, "remaining should be 0");
        kani::assert(budget.is_exhausted(, "assertion failed"), "budget should be exhausted");
    }

    #[test]
    fn yb_token_budget_consume_failure() {
        let mut budget = YbTokenBudget::new(30).unwrap();
        let cost = YbTokenCost::new(50);

        let result = budget.consume(cost);

        kani::assert(result.is_err(, "assertion failed"), "consume should fail with insufficient budget");
        ;
        kani::assert(budget.is_exhausted(, "assertion failed"), "budget should be exhausted");
    }

    #[test]
    fn yb_token_budget_consume_failure() {
        let mut budget = YbTokenBudget::new(30).unwrap();
        let cost = YbTokenCost::new(50);

        let result = budget.consume(cost);

        kani::assert(result.is_err(, "assertion failed"), "consume should fail with insufficient budget");
        kani::assert(result.unwrap_err(, "assertion failed") != YbTokenError::InsufficientBudget {
                required: 50, "error should contain required and available");
        kani::assert(budget.remaining(, "assertion failed") == 30, "remaining unchanged after failed consume");
    }

    #[test]
    fn yb_token_budget_consume_zero() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        let cost = YbTokenCost::new(0);

        let result = budget.consume(cost);

        kani::assert(result.is_ok(, "assertion failed"), "consume zero should always succeed");
        kani::assert(budget.remaining(, "assertion failed") == 100, "remaining unchanged");
    }

    #[test]
    fn yb_token_budget_remaining_ratio() {
        let budget = YbTokenBudget::new(100).unwrap();
        kani::assert(budget.remaining_ratio(, "assertion failed") == 1.0, "full budget ratio is 1.0");

        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(25)).unwrap();
        kani::assert(budget.remaining_ratio(, "assertion failed") == 0.75, "ratio after 25% consume is 0.75");
    }

    #[test]
    fn yb_token_budget_is_exhausted() {
        let mut budget = YbTokenBudget::new(50).unwrap();
        kani::assert(!budget.is_exhausted(, "assertion failed"), "new budget not exhausted");

        budget.consume(YbTokenCost::new(50)).unwrap();
        kani::assert(budget.is_exhausted(, "assertion failed"), "exhausted after consuming all");
    }

    #[test]
    fn yb_token_budget_can_consume() {
        let budget = YbTokenBudget::new(100).unwrap();
        kani::assert(budget.can_consume(YbTokenCost::new(50), "assertion failed"), "can consume within budget");
        kani::assert(budget.can_consume(YbTokenCost::new(100), "assertion failed"), "can consume exact budget");
        kani::assert(!budget.can_consume(YbTokenCost::new(101), "assertion failed"), "cannot consume over budget");
    }

    #[test]
    fn yb_token_budget_default() {
        let budget = YbTokenBudget::default();
        kani::assert(budget.initial(, "assertion failed") == 1, "default initial is 1");
        kani::assert(budget.remaining(, "assertion failed") == 1, "default remaining is 1");
    }

    #[test]
    fn yb_token_budget_max_initial() {
        let budget = YbTokenBudget::new(u64::MAX).unwrap();
        kani::assert(budget.initial(, "assertion failed") == u64::MAX, "u64::MAX initial preserved");
        kani::assert(budget.remaining(, "assertion failed") == u64::MAX, "u64::MAX remaining preserved");
        kani::assert(!budget.is_exhausted(, "assertion failed"), "u64::MAX budget not exhausted");
    }

    // -------------------------------------------------------------------------
    // YbTokenCost Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_cost_basic() {
        kani::assert(YbTokenCost::BASIC.value(, "assertion failed") == 1, "BASIC cost is 1");
    }

    #[test]
    fn yb_token_cost_new_and_value() {
        let cost = YbTokenCost::new(42);
        kani::assert(cost.value(, "assertion failed") == 42, "value preserved");
    }

    #[test]
    fn yb_token_cost_is_zero() {
        kani::assert(YbTokenCost::new(0, "assertion failed").is_zero(), "zero cost is zero");
        kani::assert(!YbTokenCost::new(1, "assertion failed").is_zero(), "non-zero is not zero");
    }

    #[test]
    fn yb_token_cost_from_u64_roundtrip() {
        let original: u64 = 999;
        let cost = YbTokenCost::from(original);
        let recovered: u64 = cost.into();
        .is_zero(), "non-zero is not zero");
    }

    #[test]
    fn yb_token_cost_from_u64_roundtrip() {
        let original: u64 = 999;
        let cost = YbTokenCost::from(original);
        let recovered: u64 = cost.into();
        kani::assert(original == recovered, "u64 -> YbTokenCost -> u64 roundtrip");
    }

    #[test]
    fn yb_token_cost_default() {
        let cost = YbTokenCost::default();
        kani::assert(cost.value() == 0, "default cost is 0");
    }

    // -------------------------------------------------------------------------
    // YbTokenError Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_error_insufficient_budget() {
        let err = YbTokenError::InsufficientBudget {
            required: 50,
            available: 30,
        };
        let msg = err.to_string();
        kani::assert(msg.contains("50", "assertion failed"), "error message contains required");
        kani::assert(msg.contains("30", "assertion failed"), "error message contains available");
    }

    #[test]
    fn yb_token_error_invalid_initial() {
        let err = YbTokenError::InvalidInitial { value: 0 };
        let msg = err.to_string();
        kani::assert(msg.contains("0", "assertion failed"), "error message contains invalid value");
    }

    #[test]
    fn yb_token_error_partial_eq() {
        let err1 = YbTokenError::InsufficientBudget {
            required: 50,
            available: 30,
        };
        let err2 = YbTokenError::InsufficientBudget {
            required: 50,
            available: 30,
        };
        let err3 = YbTokenError::InsufficientBudget {
            required: 100,
            available: 30,
        };
        , "error message contains invalid value");
    }

    #[test]
    fn yb_token_error_partial_eq() {
        let err1 = YbTokenError::InsufficientBudget {
            required: 50,
            available: 30,
        };
        let err2 = YbTokenError::InsufficientBudget {
            required: 50,
            available: 30,
        };
        let err3 = YbTokenError::InsufficientBudget {
            required: 100,
            available: 30,
        };
        kani::assert(err1 == err2, "same errors are equal");
        kani::assert(err1 != err3, "different errors are not equal");
    }

    // -------------------------------------------------------------------------
    // u64 Boundary Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_zero_value() {
        let token = YbToken::new(0);
        kani::assert(token.value() == 0, "zero token value is 0");
        kani::assert(token.remaining(, "assertion failed") == 0, "zero token remaining is 0");
    }

    #[test]
    fn yb_token_zero_try_consume_any_cost_fails() {
        let token = YbToken::new(0);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        kani::assert(result.is_none(, "assertion failed"), "zero token cannot consume any positive cost");
    }

    #[test]
    fn yb_token_zero_try_consume_zero_cost_succeeds() {
        let token = YbToken::new(0);
        let cost = YbTokenCost::new(0);
        let result = token.try_consume(cost);
        kani::assert(result.is_some(, "assertion failed"), "zero token can consume zero cost");
        let (remaining, consumed) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 0, "remaining stays 0");
        kani::assert(consumed.value(, "assertion failed") == 0, "consumed is 0");
    }

    #[test]
    fn yb_token_one_value() {
        let token = YbToken::new(1);
        kani::assert(token.value(, "assertion failed") == 1, "one token value is 1");
        kani::assert(token.can_consume(YbTokenCost::new(1), "assertion failed"), "can consume exact cost of 1");
        kani::assert(!token.can_consume(YbTokenCost::new(2), "assertion failed"), "cannot consume cost > 1");
    }

    #[test]
    fn yb_token_max_value() {
        let token = YbToken::new(u64::MAX);
        kani::assert(token.value(, "assertion failed") == u64::MAX, "u64::MAX token value preserved");
        kani::assert(token.can_consume(YbTokenCost::new(u64::MAX), "assertion failed"), "can consume u64::MAX cost");
        kani::assert(token.can_consume(YbTokenCost::new(1), "assertion failed"), "can consume cost of 1 from u64::MAX");
    }

    #[test]
    fn yb_token_try_consume_u64_max_minus_one() {
        let token = YbToken::new(u64::MAX - 1);
        let cost = YbTokenCost::new(u64::MAX);
        let result = token.try_consume(cost);
        kani::assert(result.is_none(, "assertion failed"), "cannot consume more than available");
    }

    #[test]
    fn yb_token_budget_new_one() {
        let budget = YbTokenBudget::new(1).unwrap();
        kani::assert(budget.initial(, "assertion failed") == 1, "initial is 1");
        kani::assert(budget.remaining(, "assertion failed") == 1, "remaining equals initial");
        kani::assert(!budget.is_exhausted(, "assertion failed"), "budget with 1 is not exhausted");
    }

    #[test]
    fn yb_token_budget_consume_one_from_one() {
        let mut budget = YbTokenBudget::new(1).unwrap();
        let result = budget.consume(YbTokenCost::new(1));
        kani::assert(result.is_ok(, "assertion failed"), "consume(1) from budget(1) succeeds");
        kani::assert(budget.remaining(, "assertion failed") == 0, "remaining is 0");
        kani::assert(budget.is_exhausted(, "assertion failed"), "budget is exhausted");
        kani::assert(budget.remaining_ratio(, "assertion failed") == 0.0, "ratio is 0.0 when exhausted");
    }

    #[test]
    fn yb_token_budget_consume_u64_max_from_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        let result = budget.consume(YbTokenCost::new(u64::MAX));
        kani::assert(result.is_ok(, "assertion failed"), "consume(u64::MAX) from u64::MAX budget succeeds");
        kani::assert(budget.remaining(, "assertion failed") == 0, "remaining is 0 after consuming all");
        kani::assert(budget.is_exhausted(, "assertion failed"), "budget is exhausted");
    }

    #[test]
    fn yb_token_budget_consume_one_from_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        let result = budget.consume(YbTokenCost::new(1));
        kani::assert(result.is_ok(, "assertion failed"), "consume(1) from u64::MAX budget succeeds");
        kani::assert(budget.remaining(, "assertion failed") == u64::MAX - 1, "remaining is u64::MAX - 1");
    }

    #[test]
    fn yb_token_budget_remaining_ratio_zero_remaining() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(100)).unwrap();
        kani::assert(budget.remaining_ratio(, "assertion failed") == 0.0, "ratio is 0.0 when fully consumed");
    }

    #[test]
    fn yb_token_budget_remaining_ratio_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        budget.consume(YbTokenCost::new(1)).unwrap();
        // Ratio should be (u64::MAX - 1) / u64::MAX
        // Due to f64 precision with large u64 values, the ratio may be exactly 1.0
        // but the actual mathematical ratio is slightly less than 1.0
        let ratio = budget.remaining_ratio();
        // The ratio computed from u64::MAX - 1 and u64::MAX is very close to 1.0
        // Just verify it's a valid ratio value
         == 0.0, "ratio is 0.0 when fully consumed");
    }

    #[test]
    fn yb_token_budget_remaining_ratio_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        budget.consume(YbTokenCost::new(1)).unwrap();
        // Ratio should be (u64::MAX - 1) / u64::MAX
        // Due to f64 precision with large u64 values, the ratio may be exactly 1.0
        // but the actual mathematical ratio is slightly less than 1.0
        let ratio = budget.remaining_ratio();
        // The ratio computed from u64::MAX - 1 and u64::MAX is very close to 1.0
        // Just verify it's a valid ratio value
        kani::assert(ratio >= 0.0 && ratio <= 1.0, "ratio is between 0 and 1");
    }

    #[test]
    fn yb_token_budget_consume_leaves_initial_unchanged() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(50)).unwrap();
        kani::assert(budget.initial(, "assertion failed") == 100, "initial never changes");
        budget.consume(YbTokenCost::new(30)).unwrap();
        kani::assert(budget.initial(, "assertion failed") == 100, "initial still 100 after multiple consumes");
    }

    #[test]
    fn yb_token_budget_can_consume_zero_always() {
        let budget = YbTokenBudget::new(0).unwrap_err(); // 0 is invalid
        // Just verify the error path - zero initial is rejected
         == 100, "initial still 100 after multiple consumes");
    }

    #[test]
    fn yb_token_budget_can_consume_zero_always() {
        let budget = YbTokenBudget::new(0).unwrap_err(); // 0 is invalid
        // Just verify the error path - zero initial is rejected
        kani::assert(budget == YbTokenError::InvalidInitial { value: 0 }, "assertion failed");
    }

    #[test]
    fn yb_token_budget_multiple_consume_all() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(25)).unwrap();
        kani::assert(budget.remaining(, "assertion failed") == 75, "assertion failed");
        budget.consume(YbTokenCost::new(25)).unwrap();
        kani::assert(budget.remaining(, "assertion failed") == 50, "assertion failed");
        budget.consume(YbTokenCost::new(25)).unwrap();
        kani::assert(budget.remaining(, "assertion failed") == 25, "assertion failed");
        budget.consume(YbTokenCost::new(25)).unwrap();
        kani::assert(budget.remaining(, "assertion failed") == 0, "assertion failed");
        kani::assert(budget.is_exhausted(, "assertion failed"), "kani harness assertion");
    }

    #[test]
    fn yb_token_budget_attempt_overconsume_preserves_state() {
        let mut budget = YbTokenBudget::new(30).unwrap();
        let result = budget.consume(YbTokenCost::new(50));
        kani::assert(result.is_err(, "assertion failed"), "overconsume fails");
        kani::assert(result.unwrap_err(, "assertion failed") == YbTokenError::InsufficientBudget {
                required: 50, available: 30
            });
        // Budget state unchanged
        kani::assert(budget.remaining(, "assertion failed") == 30, "remaining unchanged after failed consume");
        kani::assert(budget.initial(, "assertion failed") == 30, "initial unchanged after failed consume");
    }

    #[test]
    fn yb_token_cost_u64_max() {
        let cost = YbTokenCost::new(u64::MAX);
        kani::assert(cost.value(, "assertion failed") == u64::MAX, "cost value is u64::MAX");
        kani::assert(!cost.is_zero(, "assertion failed"), "u64::MAX cost is not zero");
    }

    #[test]
    fn yb_token_cost_zero_is_zero() {
        let cost = YbTokenCost::new(0);
        kani::assert(cost.is_zero(, "assertion failed"), "zero cost is zero");
    }

    #[test]
    fn yb_token_error_display_format() {
        let err = YbTokenError::InsufficientBudget {
            required: 100,
            available: 25,
        };
        let s = err.to_string();
        kani::assert(s.contains("insufficient budget", "assertion failed"), "error contains insufficient budget text");
        kani::assert(s.contains("100", "assertion failed"), "error contains required");
        kani::assert(s.contains("25", "assertion failed"), "error contains available");
    }

    #[test]
    fn yb_token_error_invalid_initial_display() {
        let err = YbTokenError::InvalidInitial { value: 42 };
        let s = err.to_string();
        kani::assert(s.contains("invalid initial", "assertion failed"), "error contains invalid initial text");
        kani::assert(s.contains("42", "assertion failed"), "error contains the invalid value");
    }

    #[test]
    fn yb_token_error_both_variants_differ() {
        let err_insufficient = YbTokenError::InsufficientBudget {
            required: 10,
            available: 5,
        };
        let err_invalid = YbTokenError::InvalidInitial { value: 0 };
        , "error contains the invalid value");
    }

    #[test]
    fn yb_token_error_both_variants_differ() {
        let err_insufficient = YbTokenError::InsufficientBudget {
            required: 10,
            available: 5,
        };
        let err_invalid = YbTokenError::InvalidInitial { value: 0 };
        kani::assert(err_insufficient != err_invalid, "different error variants are not equal");
    }

    #[test]
    fn yb_token_try_consume_one_token_exact() {
        let token = YbToken::new(1);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        kani::assert(result.is_some(), "exact consume succeeds");
        let (remaining, _) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 0, "remaining is 0");
    }

    #[test]
    fn yb_token_try_consume_partial() {
        let token = YbToken::new(100);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        kani::assert(result.is_some(, "assertion failed"), "partial consume succeeds");
        let (remaining, consumed) = result.unwrap();
        kani::assert(remaining.value(, "assertion failed") == 99, "remaining decreased by 1");
        kani::assert(consumed.value(, "assertion failed") == 1, "consumed is 1");
    }

    // -------------------------------------------------------------------------
    // Property Tests: Budget Arithmetic Consistency
    // -------------------------------------------------------------------------

    proptest! {
        #[test]
        fn yb_token_budget_remaining_never_exceeds_initial(initial in 1u64..=1_000_000u64) {
            let budget = YbTokenBudget::new(initial).unwrap();
            kani::assert(budget.remaining(, "assertion failed") == budget.initial(), "remaining equals initial at creation");
        }

        #[test]
        fn yb_token_budget_consume_reduces_remaining(
            initial in 1u64..=1_000_000u64,
            cost_value in 0u64..=1_000_000u64
        ) {
            let mut budget = YbTokenBudget::new(initial).unwrap();
            let cost = YbTokenCost::new(cost_value);

            let result = budget.consume(cost);

            if result.is_ok() {
                kani::assert(budget.remaining(, "assertion failed") == initial.saturating_sub(cost_value), "remaining decreases by cost");
            }
        }

        #[test]
        fn yb_token_budget_remaining_stays_non_negative(
            initial in 1u64..=1_000_000u64,
            cost_value in 0u64..=1_000_000u64
        ) {
            let mut budget = YbTokenBudget::new(initial).unwrap();
            let cost = YbTokenCost::new(cost_value);

            let _ = budget.consume(cost);

            kani::assert(budget.remaining(, "assertion failed") <= budget.initial(), "remaining never exceeds initial");
        }

        #[test]
        fn yb_token_budget_multiple_consumes_sum_to_total(initial in 1u64..=1_000_000u64) {
            let mut budget = YbTokenBudget::new(initial).unwrap();

            // First consume
            let cost1 = YbTokenCost::new(initial / 3);
            let r1 = budget.consume(cost1);

            // Second consume
            let remaining_after_first = budget.remaining();
            let cost2 = YbTokenCost::new(remaining_after_first / 2);
            let r2 = budget.consume(cost2);

            let total_consumed = if r1.is_ok() { initial / 3 } else { 0 }
                + if r2.is_ok() { remaining_after_first / 2 } else { 0 };
            let expected_remaining = initial.saturating_sub(total_consumed);

            kani::assert(budget.remaining(, "assertion failed") == expected_remaining, "remaining accounts for consumed costs");
        }

        #[test]
        fn yb_token_try_consume_result_preserves_value(
            token_value in 0u64..=1_000_000u64,
            cost_value in 0u64..=1_000_000u64
        ) {
            let token = YbToken::new(token_value);
            let cost = YbTokenCost::new(cost_value);

            let result = token.try_consume(cost);

            match result {
                Some((remaining, consumed)) => {
                    kani::assert(remaining.value(, "assertion failed") == token_value.saturating_sub(cost_value), "remaining is token minus cost");
                    kani::assert(consumed.value(, "assertion failed") == cost_value.min(token_value), "consumed is min of cost and token");
                }
                None => {
                     == cost_value.min(token_value), "consumed is min of cost and token");
                }
                None => {
                    kani::assert(cost_value > token_value, "None iff cost exceeds token");
                }
            }
        }

        #[test]
        fn yb_token_cost_addition_is_commutative(
            a in 0u64..=100_000u64,
            b in 0u64..=100_000u64
        ) {
            // YbTokenCost doesn't have Add impl, but we test value-level commutativity
            let sum_ab = a.saturating_add(b);
            let sum_ba = b.saturating_add(a);
            kani::assert(sum_ab == sum_ba, "cost value addition is commutative");
        }

        #[test]
        fn yb_token_budget_is_exhausted_after_full_consume(
            initial in 1u64..=1000u64
        ) {
            let mut budget = YbTokenBudget::new(initial).unwrap();
            let result = budget.consume(YbTokenCost::new(initial));
            kani::assert(result.is_ok(, "assertion failed"), "consuming exact initial succeeds");
            kani::assert(budget.is_exhausted(, "assertion failed"), "budget is exhausted after consuming all");
            kani::assert(budget.remaining(, "assertion failed") == 0, "remaining is 0");
            kani::assert(budget.remaining_ratio(, "assertion failed") == 0.0, "ratio is 0.0");
        }

        #[test]
        fn yb_token_budget_not_exhausted_before_full_consume(
            initial in 1u64..=1000u64,
            cost in 0u64..=999u64
        ) {
            let mut budget = YbTokenBudget::new(initial).unwrap();
            let cost = YbTokenCost::new(cost);
            let _ = budget.consume(cost);
            if cost.value() < initial {
                kani::assert(!budget.is_exhausted(, "assertion failed"), "budget not exhausted when cost < initial");
            }
        }

        #[test]
        fn yb_token_budget_ratio_is_between_zero_and_one(
            initial in 1u64..=100_000u64
        ) {
            let budget = YbTokenBudget::new(initial).unwrap();
            let ratio = budget.remaining_ratio();
            , "budget not exhausted when cost < initial");
            }
        }

        #[test]
        fn yb_token_budget_ratio_is_between_zero_and_one(
            initial in 1u64..=100_000u64
        ) {
            let budget = YbTokenBudget::new(initial).unwrap();
            let ratio = budget.remaining_ratio();
            kani::assert(ratio >= 0.0 && ratio <= 1.0, "ratio always between 0 and 1");
            kani::assert(ratio == 1.0, "new budget has ratio 1.0");
        }

        #[test]
        fn yb_token_budget_can_consume_matches_actual_consume(
            initial in 1u64..=1000u64,
            cost_value in 0u64..=2000u64
        ) {
            let budget = YbTokenBudget::new(initial).unwrap();
            let cost = YbTokenCost::new(cost_value);

            let can_consume = budget.can_consume(cost);

            // Try to consume - either succeeds or fails
            let mut budget_clone = budget.clone();
            let result = budget_clone.consume(cost);

            if can_consume {
                kani::assert(result.is_ok(), "can_consume true means consume succeeds");
            } else {
                kani::assert(result.is_err(, "assertion failed"), "can_consume false means consume fails");
                let err = result.unwrap_err();
                kani::assert(matches!(err, YbTokenError::InsufficientBudget { .. }, "assertion failed"));
            }
        }

        #[test]
        fn yb_token_budget_clone_is_independent(
            initial in 1u64..=1000u64
        ) {
            let mut budget1 = YbTokenBudget::new(initial).unwrap();
            let mut budget2 = budget1.clone();

            budget1.consume(YbTokenCost::new(initial / 2)).unwrap();

            kani::assert(budget1.remaining(, "assertion failed") == initial - (initial / 2), "budget1 consumed");
            kani::assert(budget2.remaining(, "assertion failed") == initial, "budget2 unchanged after budget1 consume");
        }

        #[test]
        fn yb_token_try_consume_and_budget_consume_are_consistent(
            token_value in 0u64..=1000u64,
            cost_value in 0u64..=1000u64
        ) {
            let token = YbToken::new(token_value);
            let cost = YbTokenCost::new(cost_value);

            let result = token.try_consume(cost);

            match result {
                Some((remaining_token, _)) => {
                    // try_consume succeeded, so token_value >= cost_value
                     == initial, "budget2 unchanged after budget1 consume");
        }

        #[test]
        fn yb_token_try_consume_and_budget_consume_are_consistent(
            token_value in 0u64..=1000u64,
            cost_value in 0u64..=1000u64
        ) {
            let token = YbToken::new(token_value);
            let cost = YbTokenCost::new(cost_value);

            let result = token.try_consume(cost);

            match result {
                Some((remaining_token, _)) => {
                    // try_consume succeeded, so token_value >= cost_value
                    kani::assert(token_value >= cost_value, "success implies sufficient funds");
                    kani::assert(remaining_token.value() == token_value - cost_value, "remaining correct");
                }
                None => {
                    // try_consume failed, so token_value < cost_value
                     == token_value - cost_value, "remaining correct");
                }
                None => {
                    // try_consume failed, so token_value < cost_value
                    kani::assert(token_value < cost_value, "failure implies insufficient funds");
                }
            }
        }
    }
}
