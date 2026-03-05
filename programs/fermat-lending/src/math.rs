//! Core lending math using fermat-core's panic-free fixed-point arithmetic.
//!
//! ## Health Factor
//!
//! ```text
//! health_factor = (collateral_usd × liquidation_threshold) / total_debt_usd
//! ```
//!
//! A position is healthy when `health_factor >= 1.0`. We round **down**
//! (toward −∞) so the reported health factor is always ≤ the true value —
//! this makes the protocol conservative: a position can only be liquidated
//! when it is truly undercollateralised, never due to rounding up. Uses
//! `checked_mul_div` with a U256 intermediate to prevent overflow on large
//! positions.
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
/// so the reported health factor never exceeds the true value — a position
/// is only eligible for liquidation when it is genuinely undercollateralised.
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

// ─── Interest Rate Model ───────────────────────────────────────────────────────

/// Compute the current utilisation rate: `total_borrowed / total_deposited`.
///
/// Returns `Decimal::ZERO` when `total_deposited` is zero (empty reserve).
/// Result is rounded to 6 decimal places.
pub fn utilisation_rate(
    total_borrowed: Decimal,
    total_deposited: Decimal,
) -> Result<Decimal, ArithmeticError> {
    if total_deposited.is_zero() {
        return Ok(Decimal::ZERO);
    }
    let util = total_borrowed.checked_div(total_deposited)?;
    util.round(6, RoundingMode::HalfEven)
}

/// Kinked two-slope borrow rate model.
///
/// ```text
/// if util ≤ optimal:
///     rate = base + slope1 × (util / optimal)
/// else:
///     excess = util − optimal
///     rate = base + slope1 + slope2 × (excess / (1 − optimal))
/// ```
///
/// All inputs and the return value are 6 dp fractions (e.g. `0.05 = 5% APR`).
///
/// # Errors
/// - `DivisionByZero` if `optimal == 0` or `optimal == 1` (degenerate model).
/// - Propagates overflow errors from underlying arithmetic.
pub fn kinked_borrow_rate(
    utilisation: Decimal,
    base: Decimal,
    slope1: Decimal,
    slope2: Decimal,
    optimal: Decimal,
) -> Result<Decimal, ArithmeticError> {
    if optimal.is_zero() {
        return Err(ArithmeticError::DivisionByZero);
    }
    let one = Decimal::ONE;
    let remaining = one.checked_sub(optimal)?;
    if remaining.is_zero() {
        return Err(ArithmeticError::DivisionByZero);
    }

    let rate = if utilisation <= optimal {
        // base + slope1 × (util / optimal)
        let ratio = utilisation
            .checked_div(optimal)?
            .round(6, RoundingMode::HalfEven)?;
        let term = slope1
            .checked_mul(ratio)?
            .round(6, RoundingMode::HalfEven)?;
        base.checked_add(term)?
    } else {
        // base + slope1 + slope2 × ((util − optimal) / (1 − optimal))
        let excess = utilisation.checked_sub(optimal)?;
        let ratio = excess
            .checked_div(remaining)?
            .round(6, RoundingMode::HalfEven)?;
        let term = slope2
            .checked_mul(ratio)?
            .round(6, RoundingMode::HalfEven)?;
        base.checked_add(slope1)?.checked_add(term)?
    };
    rate.round(6, RoundingMode::HalfEven)
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

    // ── Utilisation Rate ──────────────────────────────────────────────────────

    #[test]
    fn utilisation_empty_reserve_is_zero() {
        let u = utilisation_rate(Decimal::ZERO, Decimal::ZERO).unwrap();
        assert!(u.is_zero());
    }

    #[test]
    fn utilisation_fifty_percent() {
        // borrowed = 50, deposited = 100 → 0.50
        let u = utilisation_rate(d(50_000_000, 6), d(100_000_000, 6)).unwrap();
        assert_eq!(u, d(500_000, 6));
    }

    #[test]
    fn utilisation_eighty_percent() {
        let u = utilisation_rate(d(80_000_000, 6), d(100_000_000, 6)).unwrap();
        assert_eq!(u, d(800_000, 6));
    }

    // ── Kinked Borrow Rate ────────────────────────────────────────────────────

    // Model params: base=2%, slope1=4%, slope2=50%, optimal=80%
    fn base() -> Decimal {
        d(20_000, 6)
    } // 0.02
    fn slope1() -> Decimal {
        d(40_000, 6)
    } // 0.04
    fn slope2() -> Decimal {
        d(500_000, 6)
    } // 0.50
    fn optimal() -> Decimal {
        d(800_000, 6)
    } // 0.80

    #[test]
    fn kinked_rate_at_zero_util() {
        // util=0 → rate = base + 0 = 2%
        let rate =
            kinked_borrow_rate(Decimal::ZERO, base(), slope1(), slope2(), optimal()).unwrap();
        assert_eq!(rate, d(20_000, 6));
    }

    #[test]
    fn kinked_rate_at_optimal_util() {
        // util=optimal=0.80 → rate = base + slope1 = 2% + 4% = 6%
        let rate = kinked_borrow_rate(optimal(), base(), slope1(), slope2(), optimal()).unwrap();
        assert_eq!(rate, d(60_000, 6));
    }

    #[test]
    fn kinked_rate_above_optimal() {
        // util=0.90, excess=0.10, remaining=0.20
        // rate = 6% + 50% × (0.10/0.20) = 6% + 25% = 31%
        let u = d(900_000, 6); // 0.90
        let rate = kinked_borrow_rate(u, base(), slope1(), slope2(), optimal()).unwrap();
        assert_eq!(rate, d(310_000, 6));
    }

    #[test]
    fn kinked_rate_at_full_util() {
        // util=1.0, excess=0.20, remaining=0.20
        // rate = 6% + 50% × (0.20/0.20) = 6% + 50% = 56%
        let u = d(1_000_000, 6); // 1.0
        let rate = kinked_borrow_rate(u, base(), slope1(), slope2(), optimal()).unwrap();
        assert_eq!(rate, d(560_000, 6));
    }

    #[test]
    fn kinked_rate_degenerate_optimal_zero_errors() {
        assert!(
            kinked_borrow_rate(Decimal::ZERO, base(), slope1(), slope2(), Decimal::ZERO).is_err()
        );
    }

    #[test]
    fn kinked_rate_degenerate_optimal_one_errors() {
        assert!(
            kinked_borrow_rate(Decimal::ZERO, base(), slope1(), slope2(), d(1_000_000, 6)).is_err()
        );
    }
}
