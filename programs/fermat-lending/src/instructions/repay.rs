//! Repay borrowed tokens to a reserve.
//!
//! The repaid amount is capped at the current outstanding debt so callers
//! can pass `u64::MAX` to repay in full without off-chain debt tracking.

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

use crate::instructions::LendingError;
use crate::math::scale_debt;
use crate::state::{Position, Reserve};

/// Repay up to `amount` raw token units to `reserve`.
///
/// Adjusts for accrued interest by scaling the stored debt via the index ratio.
pub fn handler(reserve: &mut Reserve, position: &mut Position, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::ZeroAmount);

    let current_index = reserve.cumulative_borrow_index.0;
    let entry_index = position.entry_borrow_index.0;

    let current_debt = if entry_index.is_zero() {
        position.debt_amount.0
    } else {
        scale_debt(position.debt_amount.0, current_index, entry_index)
            .map_err(|_| LendingError::MathError)?
    };

    let dec_amount = Decimal::new(
        (amount as i128).min(current_debt.mantissa()),
        reserve.mint_decimals,
    )
    .map_err(|_| LendingError::MathError)?;

    let new_debt = current_debt
        .checked_sub(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    let new_total_borrowed = reserve
        .total_borrowed
        .0
        .checked_sub(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    position.debt_amount = DecimalBorsh(new_debt);
    position.entry_borrow_index = reserve.cumulative_borrow_index;
    reserve.total_borrowed = DecimalBorsh(new_total_borrowed);

    Ok(())
}
