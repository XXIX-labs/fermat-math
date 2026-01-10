//! Arithmetic operations: add, sub, mul, div, mul_div, neg, abs.
//! Placeholder — full implementation added in subsequent commits.

use crate::decimal::{Decimal, MAX_SCALE};
use crate::error::ArithmeticError;

impl Decimal {
    /// Placeholder for checked_add — implemented in a later commit.
    pub fn checked_add(self, _rhs: Decimal) -> Result<Decimal, ArithmeticError> {
        Err(ArithmeticError::InvalidInput)
    }
}
