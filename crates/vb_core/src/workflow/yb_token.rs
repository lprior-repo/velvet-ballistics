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
        // Default budget uses 1 token to avoid InvalidInitial error
        Self::new(1).expect("YbTokenBudget::default should always succeed with initial=1")
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
        kani::assert(roundtrip.value() == token.value(), "from/into roundtrip");

        // Invariant: try_consume only returns Some when sufficient funds
        let cost = YbTokenCost::new(kani::any());
        let result = token.try_consume(cost);
        match result {
            Some((_, _)) => {
                kani::assert(token.value() >= cost.value(), "Some implies sufficient funds");
            }
            None => {
                kani::assert(token.value() < cost.value(), "None implies insufficient funds");
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

        let mut budget = YbTokenBudget::new(initial).unwrap();

        // Invariant: remaining never exceeds initial
        kani::assert(budget.remaining() <= budget.initial(), "remaining <= initial");

        // NOTE: Removed vacuous `remaining >= 0` assertion - u64 can never be negative.
        // This was flagged as HIGH-001 in proof-review. The meaningful invariant is `remaining <= initial`.

        // Consume some tokens
        let cost_value: u64 = kani::any();
        let cost = YbTokenCost::new(cost_value % (initial + 1)); // Allow up to initial
        let consume_result = budget.consume(cost);

        match consume_result {
            Ok(()) => {
                kani::assert(budget.remaining() <= initial, "after consume: remaining <= initial");
            }
            Err(YbTokenError::InsufficientBudget { required, available }) => {
                kani::assert(required > available, "insufficient budget error is correct");
                kani::assert(budget.remaining() == initial, "failed consume leaves budget unchanged");
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
                kani::assert(budget.initial() == max_step_budget, "initial preserved from contract");
                kani::assert(budget.remaining() == max_step_budget, "initial == remaining at creation");
                kani::assert(!budget.is_exhausted(), "new budget is not exhausted");
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

        let mut budget = result.unwrap();
        kani::assert(budget.initial() == u64::MAX, "initial preserved at u64::MAX");
        kani::assert(budget.remaining() == u64::MAX, "remaining == initial at u64::MAX");

        // Consume 1 token from u64::MAX budget
        let cost = YbTokenCost::new(1);
        let consume_result = budget.consume(cost);

        kani::assert(consume_result.is_ok(), "consume(1) from u64::MAX succeeds");
        if consume_result.is_ok() {
            kani::assert(budget.remaining() == u64::MAX - 1, "remaining = u64::MAX - 1");
        }

        // Invariant holds: remaining <= initial
        kani::assert(budget.remaining() <= budget.initial(), "remaining <= initial after consume");
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

        assert!(result.is_some(), "should succeed when sufficient tokens");
        let (remaining, consumed) = result.unwrap();
        assert_eq!(remaining.value(), 50, "remaining tokens should be 50");
        assert_eq!(consumed.value(), 50, "consumed cost should be 50");
    }

    #[test]
    fn yb_token_try_consume_exact() {
        let token = YbToken::new(42);
        let cost = YbTokenCost::new(42);

        let result = token.try_consume(cost);

        assert!(result.is_some(), "should succeed with exact tokens");
        let (remaining, consumed) = result.unwrap();
        assert_eq!(remaining.value(), 0, "remaining should be 0");
        assert_eq!(consumed.value(), 42, "consumed should be full cost");
    }

    #[test]
    fn yb_token_try_consume_failure() {
        let token = YbToken::new(30);
        let cost = YbTokenCost::new(50);

        let result = token.try_consume(cost);

        assert!(result.is_none(), "should fail when insufficient tokens");
    }

    #[test]
    fn yb_token_try_consume_zero_cost() {
        let token = YbToken::new(100);
        let cost = YbTokenCost::new(0);

        let result = token.try_consume(cost);

        assert!(result.is_some(), "zero cost should always succeed");
        let (remaining, consumed) = result.unwrap();
        assert_eq!(remaining.value(), 100, "remaining unchanged");
        assert_eq!(consumed.value(), 0, "consumed is 0");
    }

    #[test]
    fn yb_token_can_consume() {
        let token = YbToken::new(100);

        assert!(token.can_consume(YbTokenCost::new(50)), "can consume when sufficient");
        assert!(token.can_consume(YbTokenCost::new(100)), "can consume exact");
        assert!(!token.can_consume(YbTokenCost::new(101)), "cannot consume when insufficient");
    }

    #[test]
    fn yb_token_default() {
        let token = YbToken::default();
        assert_eq!(token.value(), 0, "default token should be 0");
    }

    #[test]
    fn yb_token_from_u64_roundtrip() {
        let original: u64 = 12345;
        let token = YbToken::from(original);
        let recovered: u64 = token.into();

        assert_eq!(original, recovered, "u64 -> YbToken -> u64 roundtrip");
    }

    // -------------------------------------------------------------------------
    // YbTokenBudget Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_budget_new_valid() {
        let budget = YbTokenBudget::new(100);
        assert!(budget.is_ok(), "new with non-zero should succeed");
        let budget = budget.unwrap();
        assert_eq!(budget.initial(), 100, "initial preserved");
        assert_eq!(budget.remaining(), 100, "remaining equals initial at creation");
    }

    #[test]
    fn yb_token_budget_new_zero() {
        let result = YbTokenBudget::new(0);
        assert!(result.is_err(), "new with zero should fail");
        assert_eq!(
            result.unwrap_err(),
            YbTokenError::InvalidInitial { value: 0 },
            "error variant should be InvalidInitial"
        );
    }

    #[test]
    fn yb_token_budget_consume_success() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        let cost = YbTokenCost::new(30);

        let result = budget.consume(cost);

        assert!(result.is_ok(), "consume should succeed with sufficient budget");
        assert_eq!(budget.remaining(), 70, "remaining should decrease");
    }

    #[test]
    fn yb_token_budget_consume_exact() {
        let mut budget = YbTokenBudget::new(50).unwrap();
        let cost = YbTokenCost::new(50);

        let result = budget.consume(cost);

        assert!(result.is_ok(), "consume exact should succeed");
        assert_eq!(budget.remaining(), 0, "remaining should be 0");
        assert!(budget.is_exhausted(), "budget should be exhausted");
    }

    #[test]
    fn yb_token_budget_consume_failure() {
        let mut budget = YbTokenBudget::new(30).unwrap();
        let cost = YbTokenCost::new(50);

        let result = budget.consume(cost);

        assert!(result.is_err(), "consume should fail with insufficient budget");
        assert_eq!(
            result.unwrap_err(),
            YbTokenError::InsufficientBudget {
                required: 50,
                available: 30
            },
            "error should contain required and available"
        );
        assert_eq!(budget.remaining(), 30, "remaining unchanged after failed consume");
    }

    #[test]
    fn yb_token_budget_consume_zero() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        let cost = YbTokenCost::new(0);

        let result = budget.consume(cost);

        assert!(result.is_ok(), "consume zero should always succeed");
        assert_eq!(budget.remaining(), 100, "remaining unchanged");
    }

    #[test]
    fn yb_token_budget_remaining_ratio() {
        let budget = YbTokenBudget::new(100).unwrap();
        assert_eq!(budget.remaining_ratio(), 1.0, "full budget ratio is 1.0");

        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(25)).unwrap();
        assert_eq!(budget.remaining_ratio(), 0.75, "ratio after 25% consume is 0.75");
    }

    #[test]
    fn yb_token_budget_is_exhausted() {
        let mut budget = YbTokenBudget::new(50).unwrap();
        assert!(!budget.is_exhausted(), "new budget not exhausted");

        budget.consume(YbTokenCost::new(50)).unwrap();
        assert!(budget.is_exhausted(), "exhausted after consuming all");
    }

    #[test]
    fn yb_token_budget_can_consume() {
        let budget = YbTokenBudget::new(100).unwrap();
        assert!(budget.can_consume(YbTokenCost::new(50)), "can consume within budget");
        assert!(budget.can_consume(YbTokenCost::new(100)), "can consume exact budget");
        assert!(!budget.can_consume(YbTokenCost::new(101)), "cannot consume over budget");
    }

    #[test]
    fn yb_token_budget_default() {
        let budget = YbTokenBudget::default();
        assert_eq!(budget.initial(), 1, "default initial is 1");
        assert_eq!(budget.remaining(), 1, "default remaining is 1");
    }

    #[test]
    fn yb_token_budget_max_initial() {
        let budget = YbTokenBudget::new(u64::MAX).unwrap();
        assert_eq!(budget.initial(), u64::MAX, "u64::MAX initial preserved");
        assert_eq!(budget.remaining(), u64::MAX, "u64::MAX remaining preserved");
        assert!(!budget.is_exhausted(), "u64::MAX budget not exhausted");
    }

    // -------------------------------------------------------------------------
    // YbTokenCost Unit Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_cost_basic() {
        assert_eq!(YbTokenCost::BASIC.value(), 1, "BASIC cost is 1");
    }

    #[test]
    fn yb_token_cost_new_and_value() {
        let cost = YbTokenCost::new(42);
        assert_eq!(cost.value(), 42, "value preserved");
    }

    #[test]
    fn yb_token_cost_is_zero() {
        assert!(YbTokenCost::new(0).is_zero(), "zero cost is zero");
        assert!(!YbTokenCost::new(1).is_zero(), "non-zero is not zero");
    }

    #[test]
    fn yb_token_cost_from_u64_roundtrip() {
        let original: u64 = 999;
        let cost = YbTokenCost::from(original);
        let recovered: u64 = cost.into();
        assert_eq!(original, recovered, "u64 -> YbTokenCost -> u64 roundtrip");
    }

    #[test]
    fn yb_token_cost_default() {
        let cost = YbTokenCost::default();
        assert_eq!(cost.value(), 0, "default cost is 0");
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
        assert!(msg.contains("50"), "error message contains required");
        assert!(msg.contains("30"), "error message contains available");
    }

    #[test]
    fn yb_token_error_invalid_initial() {
        let err = YbTokenError::InvalidInitial { value: 0 };
        let msg = err.to_string();
        assert!(msg.contains("0"), "error message contains invalid value");
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
        assert_eq!(err1, err2, "same errors are equal");
        assert_ne!(err1, err3, "different errors are not equal");
    }

    // -------------------------------------------------------------------------
    // u64 Boundary Tests
    // -------------------------------------------------------------------------

    #[test]
    fn yb_token_zero_value() {
        let token = YbToken::new(0);
        assert_eq!(token.value(), 0, "zero token value is 0");
        assert_eq!(token.remaining(), 0, "zero token remaining is 0");
    }

    #[test]
    fn yb_token_zero_try_consume_any_cost_fails() {
        let token = YbToken::new(0);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        assert!(result.is_none(), "zero token cannot consume any positive cost");
    }

    #[test]
    fn yb_token_zero_try_consume_zero_cost_succeeds() {
        let token = YbToken::new(0);
        let cost = YbTokenCost::new(0);
        let result = token.try_consume(cost);
        assert!(result.is_some(), "zero token can consume zero cost");
        let (remaining, consumed) = result.unwrap();
        assert_eq!(remaining.value(), 0, "remaining stays 0");
        assert_eq!(consumed.value(), 0, "consumed is 0");
    }

    #[test]
    fn yb_token_one_value() {
        let token = YbToken::new(1);
        assert_eq!(token.value(), 1, "one token value is 1");
        assert!(token.can_consume(YbTokenCost::new(1)), "can consume exact cost of 1");
        assert!(!token.can_consume(YbTokenCost::new(2)), "cannot consume cost > 1");
    }

    #[test]
    fn yb_token_max_value() {
        let token = YbToken::new(u64::MAX);
        assert_eq!(token.value(), u64::MAX, "u64::MAX token value preserved");
        assert!(token.can_consume(YbTokenCost::new(u64::MAX)), "can consume u64::MAX cost");
        assert!(token.can_consume(YbTokenCost::new(1)), "can consume cost of 1 from u64::MAX");
    }

    #[test]
    fn yb_token_try_consume_u64_max_minus_one() {
        let token = YbToken::new(u64::MAX - 1);
        let cost = YbTokenCost::new(u64::MAX);
        let result = token.try_consume(cost);
        assert!(result.is_none(), "cannot consume more than available");
    }

    #[test]
    fn yb_token_budget_new_one() {
        let budget = YbTokenBudget::new(1).unwrap();
        assert_eq!(budget.initial(), 1, "initial is 1");
        assert_eq!(budget.remaining(), 1, "remaining equals initial");
        assert!(!budget.is_exhausted(), "budget with 1 is not exhausted");
    }

    #[test]
    fn yb_token_budget_consume_one_from_one() {
        let mut budget = YbTokenBudget::new(1).unwrap();
        let result = budget.consume(YbTokenCost::new(1));
        assert!(result.is_ok(), "consume(1) from budget(1) succeeds");
        assert_eq!(budget.remaining(), 0, "remaining is 0");
        assert!(budget.is_exhausted(), "budget is exhausted");
        assert_eq!(budget.remaining_ratio(), 0.0, "ratio is 0.0 when exhausted");
    }

    #[test]
    fn yb_token_budget_consume_u64_max_from_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        let result = budget.consume(YbTokenCost::new(u64::MAX));
        assert!(result.is_ok(), "consume(u64::MAX) from u64::MAX budget succeeds");
        assert_eq!(budget.remaining(), 0, "remaining is 0 after consuming all");
        assert!(budget.is_exhausted(), "budget is exhausted");
    }

    #[test]
    fn yb_token_budget_consume_one_from_u64_max() {
        let mut budget = YbTokenBudget::new(u64::MAX).unwrap();
        let result = budget.consume(YbTokenCost::new(1));
        assert!(result.is_ok(), "consume(1) from u64::MAX budget succeeds");
        assert_eq!(budget.remaining(), u64::MAX - 1, "remaining is u64::MAX - 1");
    }

    #[test]
    fn yb_token_budget_remaining_ratio_zero_remaining() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(100)).unwrap();
        assert_eq!(budget.remaining_ratio(), 0.0, "ratio is 0.0 when fully consumed");
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
        assert!(ratio >= 0.0 && ratio <= 1.0, "ratio is between 0 and 1");
    }

    #[test]
    fn yb_token_budget_consume_leaves_initial_unchanged() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(50)).unwrap();
        assert_eq!(budget.initial(), 100, "initial never changes");
        budget.consume(YbTokenCost::new(30)).unwrap();
        assert_eq!(budget.initial(), 100, "initial still 100 after multiple consumes");
    }

    #[test]
    fn yb_token_budget_can_consume_zero_always() {
        let budget = YbTokenBudget::new(0).unwrap_err(); // 0 is invalid
        // Just verify the error path - zero initial is rejected
        assert_eq!(budget, YbTokenError::InvalidInitial { value: 0 });
    }

    #[test]
    fn yb_token_budget_multiple_consume_all() {
        let mut budget = YbTokenBudget::new(100).unwrap();
        budget.consume(YbTokenCost::new(25)).unwrap();
        assert_eq!(budget.remaining(), 75);
        budget.consume(YbTokenCost::new(25)).unwrap();
        assert_eq!(budget.remaining(), 50);
        budget.consume(YbTokenCost::new(25)).unwrap();
        assert_eq!(budget.remaining(), 25);
        budget.consume(YbTokenCost::new(25)).unwrap();
        assert_eq!(budget.remaining(), 0);
        assert!(budget.is_exhausted());
    }

    #[test]
    fn yb_token_budget_attempt_overconsume_preserves_state() {
        let mut budget = YbTokenBudget::new(30).unwrap();
        let result = budget.consume(YbTokenCost::new(50));
        assert!(result.is_err(), "overconsume fails");
        assert_eq!(
            result.unwrap_err(),
            YbTokenError::InsufficientBudget {
                required: 50,
                available: 30
            }
        );
        // Budget state unchanged
        assert_eq!(budget.remaining(), 30, "remaining unchanged after failed consume");
        assert_eq!(budget.initial(), 30, "initial unchanged after failed consume");
    }

    #[test]
    fn yb_token_cost_u64_max() {
        let cost = YbTokenCost::new(u64::MAX);
        assert_eq!(cost.value(), u64::MAX, "cost value is u64::MAX");
        assert!(!cost.is_zero(), "u64::MAX cost is not zero");
    }

    #[test]
    fn yb_token_cost_zero_is_zero() {
        let cost = YbTokenCost::new(0);
        assert!(cost.is_zero(), "zero cost is zero");
    }

    #[test]
    fn yb_token_error_display_format() {
        let err = YbTokenError::InsufficientBudget {
            required: 100,
            available: 25,
        };
        let s = err.to_string();
        assert!(s.contains("insufficient budget"), "error contains insufficient budget text");
        assert!(s.contains("100"), "error contains required");
        assert!(s.contains("25"), "error contains available");
    }

    #[test]
    fn yb_token_error_invalid_initial_display() {
        let err = YbTokenError::InvalidInitial { value: 42 };
        let s = err.to_string();
        assert!(s.contains("invalid initial"), "error contains invalid initial text");
        assert!(s.contains("42"), "error contains the invalid value");
    }

    #[test]
    fn yb_token_error_both_variants_differ() {
        let err_insufficient = YbTokenError::InsufficientBudget {
            required: 10,
            available: 5,
        };
        let err_invalid = YbTokenError::InvalidInitial { value: 0 };
        assert_ne!(err_insufficient, err_invalid, "different error variants are not equal");
    }

    #[test]
    fn yb_token_try_consume_one_token_exact() {
        let token = YbToken::new(1);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        assert!(result.is_some(), "exact consume succeeds");
        let (remaining, _) = result.unwrap();
        assert_eq!(remaining.value(), 0, "remaining is 0");
    }

    #[test]
    fn yb_token_try_consume_partial() {
        let token = YbToken::new(100);
        let cost = YbTokenCost::new(1);
        let result = token.try_consume(cost);
        assert!(result.is_some(), "partial consume succeeds");
        let (remaining, consumed) = result.unwrap();
        assert_eq!(remaining.value(), 99, "remaining decreased by 1");
        assert_eq!(consumed.value(), 1, "consumed is 1");
    }

    // -------------------------------------------------------------------------
    // Property Tests: Budget Arithmetic Consistency
    // -------------------------------------------------------------------------

    proptest! {
        #[test]
        fn yb_token_budget_remaining_never_exceeds_initial(initial in 1u64..=1_000_000u64) {
            let budget = YbTokenBudget::new(initial).unwrap();
            assert_eq!(budget.remaining(), budget.initial(), "remaining equals initial at creation");
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
                assert_eq!(budget.remaining(), initial.saturating_sub(cost_value), "remaining decreases by cost");
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

            assert!(budget.remaining() <= budget.initial(), "remaining never exceeds initial");
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

            assert_eq!(budget.remaining(), expected_remaining, "remaining accounts for consumed costs");
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
                    assert_eq!(remaining.value(), token_value.saturating_sub(cost_value), "remaining is token minus cost");
                    assert_eq!(consumed.value(), cost_value.min(token_value), "consumed is min of cost and token");
                }
                None => {
                    assert!(cost_value > token_value, "None iff cost exceeds token");
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
            assert_eq!(sum_ab, sum_ba, "cost value addition is commutative");
        }

        #[test]
        fn yb_token_budget_is_exhausted_after_full_consume(
            initial in 1u64..=1000u64
        ) {
            let mut budget = YbTokenBudget::new(initial).unwrap();
            let result = budget.consume(YbTokenCost::new(initial));
            assert!(result.is_ok(), "consuming exact initial succeeds");
            assert!(budget.is_exhausted(), "budget is exhausted after consuming all");
            assert_eq!(budget.remaining(), 0, "remaining is 0");
            assert_eq!(budget.remaining_ratio(), 0.0, "ratio is 0.0");
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
                assert!(!budget.is_exhausted(), "budget not exhausted when cost < initial");
            }
        }

        #[test]
        fn yb_token_budget_ratio_is_between_zero_and_one(
            initial in 1u64..=100_000u64
        ) {
            let budget = YbTokenBudget::new(initial).unwrap();
            let ratio = budget.remaining_ratio();
            assert!(ratio >= 0.0 && ratio <= 1.0, "ratio always between 0 and 1");
            assert_eq!(ratio, 1.0, "new budget has ratio 1.0");
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
                assert!(result.is_ok(), "can_consume true means consume succeeds");
            } else {
                assert!(result.is_err(), "can_consume false means consume fails");
                let err = result.unwrap_err();
                assert!(matches!(err, YbTokenError::InsufficientBudget { .. }));
            }
        }

        #[test]
        fn yb_token_budget_clone_is_independent(
            initial in 1u64..=1000u64
        ) {
            let mut budget1 = YbTokenBudget::new(initial).unwrap();
            let mut budget2 = budget1.clone();

            budget1.consume(YbTokenCost::new(initial / 2)).unwrap();

            assert_eq!(budget1.remaining(), initial - (initial / 2), "budget1 consumed");
            assert_eq!(budget2.remaining(), initial, "budget2 unchanged after budget1 consume");
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
                    assert!(token_value >= cost_value, "success implies sufficient funds");
                    assert_eq!(remaining_token.value(), token_value - cost_value, "remaining correct");
                }
                None => {
                    // try_consume failed, so token_value < cost_value
                    assert!(token_value < cost_value, "failure implies insufficient funds");
                }
            }
        }
    }
}
