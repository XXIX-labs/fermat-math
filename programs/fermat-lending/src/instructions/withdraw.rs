//! Withdraw collateral from a reserve.
//!
//! After reducing collateral, the health factor is checked — withdrawal is
//! rejected if it would leave the position undercollateralised.

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

use crate::instructions::LendingError;
use crate::math::{health_factor, is_healthy};
use crate::state::{Position, Reserve};

/// Withdraw `amount` raw token units of collateral from `reserve`.
///
/// `price_usd` is the on-chain Pyth price (6 dp USD) for this reserve's token.
/// Collateral and debt are the same token in this single-asset design, so one
/// price is sufficient for the health factor check.
///
/// # Validation
/// - `amount > 0`
/// - Resulting health factor >= 1.0 (if any debt outstanding)
pub fn handler(
    reserve: &mut Reserve,
    position: &mut Position,
    amount: u64,
    price_usd: Decimal,
) -> Result<()> {
    require!(amount > 0, LendingError::ZeroAmount);

    let dec_amount =
        Decimal::new(amount as i128, reserve.mint_decimals).map_err(|_| LendingError::MathError)?;

    let new_collateral = position
        .collateral_amount
        .0
        .checked_sub(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    if !position.debt_amount.0.is_zero() {
        let coll_usd = new_collateral
            .checked_mul(price_usd)
            .map_err(|_| LendingError::MathError)?;
        let debt_usd = position
            .debt_amount
            .0
            .checked_mul(price_usd)
            .map_err(|_| LendingError::MathError)?;
        let hf = health_factor(coll_usd, reserve.liquidation_threshold.0, debt_usd)
            .map_err(|_| LendingError::MathError)?;
        require!(is_healthy(hf), LendingError::Undercollateralised);
    }

    let new_total = reserve
        .total_deposited
        .0
        .checked_sub(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    position.collateral_amount = DecimalBorsh(new_collateral);
    reserve.total_deposited = DecimalBorsh(new_total);

    Ok(())
}
