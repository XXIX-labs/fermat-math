//! Liquidation instruction.
//!
//! Anyone may liquidate a position whose health factor has dropped below 1.0.
//! The liquidator repays part or all of the position's debt and receives
//! collateral at a discount equal to `liquidation_bonus` (e.g. 5%).
//!
//! ## Formula
//!
//! ```text
//! current_debt      = principal × (current_index / entry_index)
//! seize_usd         = repay_amount × debt_price × (1 + liquidation_bonus)
//! collateral_seized = seize_usd / collateral_price
//! ```
//!
//! Both `repay_amount` and `collateral_seized` are capped so the liquidator
//! cannot take more than the position holds.

use anchor_lang::prelude::*;
use fermat_core::{Decimal, RoundingMode};
use fermat_solana::DecimalBorsh;

use crate::instructions::LendingError;
use crate::math::{health_factor, is_healthy, scale_debt};
use crate::state::{Position, Reserve};

/// Liquidate up to `repay_amount` of debt in `position`.
///
/// # Parameters
/// - `repay_amount`        — raw token units the liquidator repays (capped at debt).
/// - `collateral_price_usd` — oracle price of the collateral token (6 dp USD).
/// - `debt_price_usd`      — oracle price of the debt token (6 dp USD).
///
/// # Validation
/// - `repay_amount > 0`
/// - Position health factor must be < 1.0 before liquidation.
pub fn handler(
    reserve: &mut Reserve,
    position: &mut Position,
    repay_amount: u64,
    collateral_price_usd: DecimalBorsh,
    debt_price_usd: DecimalBorsh,
) -> Result<()> {
    require!(repay_amount > 0, LendingError::ZeroAmount);

    // ── 1. Current debt with accrued interest ─────────────────────────────────

    let current_index = reserve.cumulative_borrow_index.0;
    let entry_index = position.entry_borrow_index.0;

    let current_debt = if entry_index.is_zero() {
        position.debt_amount.0
    } else {
        scale_debt(position.debt_amount.0, current_index, entry_index)
            .map_err(|_| LendingError::MathError)?
    };

    // ── 2. Verify position is undercollateralised ─────────────────────────────

    let coll_usd = position
        .collateral_amount
        .0
        .checked_mul(collateral_price_usd.0)
        .map_err(|_| LendingError::MathError)?;
    let debt_usd = current_debt
        .checked_mul(debt_price_usd.0)
        .map_err(|_| LendingError::MathError)?;
    let hf = health_factor(coll_usd, reserve.liquidation_threshold.0, debt_usd)
        .map_err(|_| LendingError::MathError)?;
    require!(!is_healthy(hf), LendingError::NotLiquidatable);

    // ── 3. Cap repay at outstanding debt ──────────────────────────────────────

    let repay_dec = Decimal::new(
        (repay_amount as i128).min(current_debt.mantissa()),
        reserve.mint_decimals,
    )
    .map_err(|_| LendingError::MathError)?;

    // ── 4. Collateral to seize = repay × debt_price × (1 + bonus) / coll_price

    let bonus_factor = Decimal::ONE
        .checked_add(reserve.liquidation_bonus.0)
        .map_err(|_| LendingError::MathError)?;
    let repay_usd = repay_dec
        .checked_mul(debt_price_usd.0)
        .map_err(|_| LendingError::MathError)?;
    let seize_usd = repay_usd
        .checked_mul(bonus_factor)
        .map_err(|_| LendingError::MathError)?;
    let collateral_seized = seize_usd
        .checked_div(collateral_price_usd.0)
        .map_err(|_| LendingError::MathError)?;

    // Round down — protocol never overpays the liquidator.
    let collateral_seized = collateral_seized
        .round(reserve.mint_decimals, RoundingMode::Down)
        .map_err(|_| LendingError::MathError)?;

    // Cap at available collateral (partial liquidation).
    let collateral_seized = if collateral_seized > position.collateral_amount.0 {
        position.collateral_amount.0
    } else {
        collateral_seized
    };

    // ── 5. Update position and reserve ───────────────────────────────────────

    let new_debt = current_debt
        .checked_sub(repay_dec)
        .map_err(|_| LendingError::MathError)?;
    let new_collateral = position
        .collateral_amount
        .0
        .checked_sub(collateral_seized)
        .map_err(|_| LendingError::MathError)?;
    let new_total_borrowed = reserve
        .total_borrowed
        .0
        .checked_sub(repay_dec)
        .map_err(|_| LendingError::MathError)?;
    let new_total_deposited = reserve
        .total_deposited
        .0
        .checked_sub(collateral_seized)
        .map_err(|_| LendingError::MathError)?;

    position.debt_amount = DecimalBorsh(new_debt);
    position.collateral_amount = DecimalBorsh(new_collateral);
    position.entry_borrow_index = reserve.cumulative_borrow_index;
    reserve.total_borrowed = DecimalBorsh(new_total_borrowed);
    reserve.total_deposited = DecimalBorsh(new_total_deposited);

    Ok(())
}
