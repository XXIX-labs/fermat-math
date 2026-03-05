//! Lending protocol instructions.
//!
//! | Instruction           | Description                                     |
//! |-----------------------|-------------------------------------------------|
//! | [`deposit`]           | Add collateral to a position                    |
//! | [`withdraw`]          | Remove collateral (health check enforced)       |
//! | [`borrow`]            | Borrow against collateral (health check)        |
//! | [`repay`]             | Repay outstanding debt with accrued interest    |
//! | [`liquidate`]         | Liquidate undercollateralised position          |

pub mod borrow;
pub mod deposit;
pub mod liquidate;
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
    #[msg("Position cannot be liquidated: health factor >= 1.0")]
    NotLiquidatable,
    #[msg("Market is paused")]
    MarketPaused,
    #[msg("Reserve is paused")]
    ReservePaused,
    #[msg("Parameter out of allowed range")]
    InvalidParameter,
}
