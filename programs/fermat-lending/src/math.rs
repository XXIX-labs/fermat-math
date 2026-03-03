//! Core lending math using fermat-core's panic-free fixed-point arithmetic.
//!
//! ## Health Factor
//!
//! ```text
//! health_factor = (collateral_usd × liquidation_threshold) / total_debt_usd
//! ```
//!
//! A position is healthy when `health_factor >= 1.0`. We round **down**
//! (conservative for the liquidator) using `checked_mul_div` with U256
//! intermediate to prevent overflow on large positions.
//!
//! ## Interest Accrual
//!
//! ```text
//! new_index = old_index × (1 + rate × Δt_years)
//! ```

use fermat_core::{ArithmeticError, Decimal, RoundingMode};

// ─── Health Factor ─────────────────────────────────────────────────────────────

/// Compute the health factor for a position.
///
/// Returns `Err(DivisionByZero)` when debt is zero (caller treats as healthy).
///
/// Uses `checked_mul_div` (256-bit intermediate) and `RoundingMode::Down`
/// so the protocol is always conservative about position safety.
pub fn health_factor(
    collateral_usd: Decimal,
    liquidation_threshold: Decimal,
    total_debt_usd: Decimal,
) -> Result<Decimal, ArithmeticError> {
    let raw = collateral_usd.checked_mul_div(liquidation_threshold, total_debt_usd)?;
    raw.round(6, RoundingMode::Down)
}

/// Returns `true` when the position is healthy (health_factor >= 1.0).
pub fn is_healthy(health: Decimal) -> bool {
    health >= Decimal::ONE
}

// ─── Interest Accrual ─────────────────────────────────────────────────────────

/// Compound the cumulative borrow index by `rate × Δt_years`.
///
/// ```text
/// new_index = old_index × (1 + rate × Δt_years)
/// ```
///
/// Rounds with `HalfEven` for long-run statistical accuracy.
pub fn accrue_interest(
    old_index: Decimal,
    borrow_rate: Decimal,
    dt_years: Decimal,
) -> Result<Decimal, ArithmeticError> {
    let interest = borrow_rate.checked_mul(dt_years)?;
    let factor = Decimal::ONE.checked_add(interest)?;
    let new_index = old_index.checked_mul(factor)?;
    new_index.round(6, RoundingMode::HalfEven)
}

/// Scale a debt amount by the index ratio `current / entry`.
///
/// ```text
/// current_debt = principal × (current_index / entry_index)
/// ```
pub fn scale_debt(
    principal: Decimal,
    current_index: Decimal,
    entry_index: Decimal,
) -> Result<Decimal, ArithmeticError> {
    principal.checked_mul_div(current_index, entry_index)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    #[test]
    fn healthy_position() {
        // collateral = $150, threshold = 80%, debt = $100 → hf = 1.2
        let hf = health_factor(d(150_000_000, 6), d(800_000, 6), d(100_000_000, 6)).unwrap();
        assert_eq!(hf.to_i128_truncated(), 1);
        assert!(is_healthy(hf));
    }

    #[test]
    fn exactly_at_threshold() {
        // collateral = $100, threshold = 80%, debt = $80 → hf = 1.0
        let hf = health_factor(d(100_000_000, 6), d(800_000, 6), d(80_000_000, 6)).unwrap();
        assert!(is_healthy(hf));
    }

    #[test]
    fn undercollateralised() {
        // collateral = $50, threshold = 80%, debt = $100 → hf = 0.4
        let hf = health_factor(d(50_000_000, 6), d(800_000, 6), d(100_000_000, 6)).unwrap();
        assert!(!is_healthy(hf));
    }

    #[test]
    fn zero_debt_returns_div_by_zero() {
        let result = health_factor(d(100_000_000, 6), d(800_000, 6), Decimal::ZERO);
        assert_eq!(result, Err(ArithmeticError::DivisionByZero));
    }

    #[test]
    fn one_year_five_percent() {
        // index = 1.0, rate = 5%, dt = 1 year → new_index = 1.05
        let idx = accrue_interest(d(1_000_000, 6), d(50_000, 6), d(1_000_000, 6)).unwrap();
        assert_eq!(idx.mantissa(), 1_050_000);
        assert_eq!(idx.scale(), 6);
    }

    #[test]
    fn zero_elapsed_time_unchanged() {
        let idx = accrue_interest(d(1_000_000, 6), d(50_000, 6), Decimal::ZERO).unwrap();
        assert_eq!(idx.mantissa(), 1_000_000);
    }

    #[test]
    fn debt_grows_with_index() {
        // principal = $100, entry = 1.0, current = 1.05 → scaled = $105
        let scaled = scale_debt(d(100_000_000, 6), d(1_050_000, 6), d(1_000_000, 6)).unwrap();
        assert_eq!(scaled.to_i128_truncated(), 105);
    }
}
