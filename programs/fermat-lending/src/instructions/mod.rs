//! Lending protocol instructions.
//!
//! | Instruction | Description |
//! |---|---|
//! | [`deposit`]  | Add collateral to a position |
//! | [`withdraw`] | Remove collateral (health check enforced) |
//! | [`borrow`]   | Borrow against collateral (health check enforced) |
//! | [`repay`]    | Repay outstanding debt with accrued interest |

pub mod borrow;
pub mod deposit;
pub mod repay;
pub mod withdraw;

use anchor_lang::prelude::*;

/// Shared error codes used across all lending instructions.
#[error_code]
pub enum LendingError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Position and reserve belong to different markets")]
    MarketMismatch,
    #[msg("Arithmetic error in lending math")]
    MathError,
    #[msg("Position is undercollateralised")]
    Undercollateralised,
}
