//! On-chain account types for the fermat-lending reference protocol.
//!
//! ## Account Hierarchy
//!
//! ```text
//! Market (1 per deployment)
//!   └── Reserve (1 per token — e.g. USDC reserve, SOL reserve)
//!         └── Position (1 per user per market)
//! ```
//!
//! All monetary values are stored as [`fermat_solana::DecimalBorsh`] fields
//! (17 bytes each). Prices are denominated in USD with 6 decimal places
//! (USDC_SCALE), matching Pyth oracle output.

use anchor_lang::prelude::*;
use fermat_solana::DecimalBorsh;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Discriminator (8) + authority (32) + total_deposited_value (17)
/// + total_borrowed_value (17) + bump (1) = 75 bytes.
pub const MARKET_SPACE: usize = 8 + 32 + 17 + 17 + 1;

/// Discriminator (8) + market (32) + mint (32) + mint_decimals (1)
/// + total_deposited (17) + total_borrowed (17) + borrow_rate (17)
/// + liquidation_threshold (17) + cumulative_borrow_index (17) + bump (1) = 159 bytes.
pub const RESERVE_SPACE: usize = 8 + 32 + 32 + 1 + 17 + 17 + 17 + 17 + 17 + 1;

/// Discriminator (8) + market (32) + owner (32) + collateral_amount (17)
/// + debt_amount (17) + entry_borrow_index (17) + bump (1) = 124 bytes.
pub const POSITION_SPACE: usize = 8 + 32 + 32 + 17 + 17 + 17 + 1;

// ─── Market ───────────────────────────────────────────────────────────────────

/// Protocol-wide state account.
///
/// One per deployment; controls which reserves are active.
#[account]
pub struct Market {
    /// Authority allowed to add reserves and update parameters.
    pub authority: Pubkey,

    /// Aggregate USD value of all deposited collateral (6 dp).
    pub total_deposited_value: DecimalBorsh,

    /// Aggregate USD value of all outstanding borrows (6 dp).
    pub total_borrowed_value: DecimalBorsh,

    /// PDA bump.
    pub bump: u8,
}

impl Market {
    pub const SPACE: usize = MARKET_SPACE;
}

// ─── Reserve ──────────────────────────────────────────────────────────────────

/// Per-token reserve holding liquidity and risk parameters.
///
/// Risk parameters (borrow_rate, liquidation_threshold) are set by the
/// market authority and bounded to sane ranges by instruction logic.
#[account]
pub struct Reserve {
    /// Parent market.
    pub market: Pubkey,

    /// SPL mint for the token held in this reserve.
    pub mint: Pubkey,

    /// Decimals of the SPL mint (e.g. 6 for USDC, 9 for SOL).
    pub mint_decimals: u8,

    /// Total tokens deposited as collateral (in mint decimals).
    pub total_deposited: DecimalBorsh,

    /// Total tokens borrowed from this reserve (in mint decimals).
    pub total_borrowed: DecimalBorsh,

    /// Annual borrow rate as a fraction (e.g. 0.05 = 5% APR, 6 dp).
    pub borrow_rate: DecimalBorsh,

    /// Maximum collateral fraction usable before liquidation (e.g. 0.80 = 80%, 6 dp).
    pub liquidation_threshold: DecimalBorsh,

    /// Cumulative borrow index; starts at 1.0, compounds with each accrual.
    pub cumulative_borrow_index: DecimalBorsh,

    /// PDA bump.
    pub bump: u8,
}

impl Reserve {
    pub const SPACE: usize = RESERVE_SPACE;
}

// ─── Position ─────────────────────────────────────────────────────────────────

/// A user's collateral and debt position within a market.
///
/// Collateral and debt are tracked in native token amounts (not USD);
/// USD conversion happens at instruction time using Pyth price feeds.
#[account]
pub struct Position {
    /// Parent market.
    pub market: Pubkey,

    /// Position owner (must sign mutating instructions).
    pub owner: Pubkey,

    /// Amount of collateral deposited (in reserve mint decimals).
    pub collateral_amount: DecimalBorsh,

    /// Amount borrowed (scaled by cumulative_borrow_index at borrow time).
    pub debt_amount: DecimalBorsh,

    /// `Reserve.cumulative_borrow_index` at the time of last borrow/repay.
    pub entry_borrow_index: DecimalBorsh,

    /// PDA bump.
    pub bump: u8,
}

impl Position {
    pub const SPACE: usize = POSITION_SPACE;
}
