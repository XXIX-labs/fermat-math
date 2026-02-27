//! Borrow tokens from a reserve against deposited collateral.
//!
//! Health factor is checked after increasing debt — borrow is rejected if it
//! would put the position below the liquidation threshold.

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

use crate::instructions::LendingError;
use crate::math::{health_factor, is_healthy};
use crate::state::{Position, Reserve};

/// Borrow `amount` raw token units from `reserve`.
///
/// Oracle prices passed as `DecimalBorsh` (Borsh-serialisable, 6 dp).
///
/// # Validation
/// - `amount > 0`
/// - Resulting health factor >= 1.0
pub fn handler(
    reserve: &mut Reserve,
    position: &mut Position,
    amount: u64,
    collateral_price_usd: DecimalBorsh,
    borrow_price_usd: DecimalBorsh,
) -> Result<()> {
    require!(amount > 0, LendingError::ZeroAmount);

    let dec_amount =
        Decimal::new(amount as i128, reserve.mint_decimals).map_err(|_| LendingError::MathError)?;

    let new_debt = position
        .debt_amount
        .0
        .checked_add(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    let coll_usd = position
        .collateral_amount
        .0
        .checked_mul(collateral_price_usd.0)
        .map_err(|_| LendingError::MathError)?;
    let debt_usd = new_debt
        .checked_mul(borrow_price_usd.0)
        .map_err(|_| LendingError::MathError)?;
    let hf = health_factor(coll_usd, reserve.liquidation_threshold.0, debt_usd)
        .map_err(|_| LendingError::MathError)?;
    require!(is_healthy(hf), LendingError::Undercollateralised);

    position.entry_borrow_index = reserve.cumulative_borrow_index;

    let new_total_borrowed = reserve
        .total_borrowed
        .0
        .checked_add(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    position.debt_amount = DecimalBorsh(new_debt);
    reserve.total_borrowed = DecimalBorsh(new_total_borrowed);

    Ok(())
}
