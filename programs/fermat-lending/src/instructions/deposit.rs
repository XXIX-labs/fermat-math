//! Deposit collateral into a reserve.
//!
//! The depositor transfers SPL tokens into the protocol's vault and their
//! `Position.collateral_amount` is increased by the deposited amount.

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

use crate::instructions::LendingError;
use crate::state::{Position, Reserve};

/// Deposit `amount` raw token units of collateral into `reserve`.
///
/// # Validation
/// - `amount > 0`
/// - `position.market == reserve.market`
///
/// # State Changes
/// - `position.collateral_amount += Decimal::new(amount, reserve.mint_decimals)`
/// - `reserve.total_deposited   += Decimal::new(amount, reserve.mint_decimals)`
/// Process a deposit of `amount` raw token units.
///
/// Takes mutable account refs so the caller (lib.rs) owns the Context
/// and the Anchor macro generates client account types at the crate root.
pub fn handler(
    reserve: &mut Reserve,
    position: &mut Position,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, LendingError::ZeroAmount);

    require_keys_eq!(
        position.market,
        reserve.market,
        LendingError::MarketMismatch
    );

    let dec_amount =
        Decimal::new(amount as i128, reserve.mint_decimals).map_err(|_| LendingError::MathError)?;

    let new_collateral = position
        .collateral_amount
        .0
        .checked_add(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    let new_total = reserve
        .total_deposited
        .0
        .checked_add(dec_amount)
        .map_err(|_| LendingError::MathError)?;

    position.collateral_amount = DecimalBorsh(new_collateral);
    reserve.total_deposited = DecimalBorsh(new_total);

    Ok(())
}
