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
/// `collateral_price_usd` and `debt_price_usd` are oracle prices (6 dp)
/// passed as `DecimalBorsh` so they serialise cleanly through Anchor.
///
/// # Validation
/// - `amount > 0`
/// - Resulting health factor >= 1.0 (if any debt outstanding)
pub fn handler(
    reserve: &mut Reserve,
    position: &mut Position,
    amount: u64,
    collateral_price_usd: DecimalBorsh,
    debt_price_usd: DecimalBorsh,
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
            .checked_mul(collateral_price_usd.0)
            .map_err(|_| LendingError::MathError)?;
        let debt_usd = position
            .debt_amount
            .0
            .checked_mul(debt_price_usd.0)
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
