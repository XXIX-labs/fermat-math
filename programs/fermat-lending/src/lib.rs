//! # fermat-lending
//!
//! Reference lending protocol built on `fermat-core` for Solana sBPF.
//!
//! Demonstrates how fixed-point arithmetic eliminates floating-point risk in
//! on-chain DeFi: all monetary values use `Decimal` with explicit scale and
//! rounding modes, health factors use the 256-bit `checked_mul_div` to prevent
//! the Balancer-class overflow bug, and Borsh serialisation validates scale
//! before constructing any `Decimal`.
//!
//! ## Account Hierarchy
//!
//! ```text
//! Market  ─ authority, aggregated TVL/debt
//!   └── Reserve ─ per-token params, rates, cumulative index
//!         └── Position ─ per-user collateral + debt
//! ```
//!
//! ## Key Math
//!
//! ```text
//! health_factor = (collateral_usd × liquidation_threshold) / total_debt_usd
//!                 ↑ uses checked_mul_div (U256) with RoundingMode::Down
//!
//! interest_index_new = index × (1 + rate × Δt_years)
//!                      ↑ uses checked_mul × 2 with RoundingMode::HalfEven
//! ```

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

pub mod instructions;
pub mod math;
pub mod state;

use state::{Market, Position, Reserve};

declare_id!("FErMaT2222222222222222222222222222222222222");

#[program]
pub mod fermat_lending {
    use super::*;

    // ── Market initialisation ─────────────────────────────────────────────────

    /// Initialise a new lending market.
    pub fn init_market(ctx: Context<InitMarket>, bump: u8) -> Result<()> {
        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.total_deposited_value = DecimalBorsh(Decimal::ZERO);
        market.total_borrowed_value = DecimalBorsh(Decimal::ZERO);
        market.bump = bump;
        Ok(())
    }

    // ── Reserve initialisation ────────────────────────────────────────────────

    /// Add a token reserve to an existing market.
    pub fn init_reserve(
        ctx: Context<InitReserve>,
        mint_decimals: u8,
        borrow_rate: DecimalBorsh,
        liquidation_threshold: DecimalBorsh,
        bump: u8,
    ) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        reserve.market = ctx.accounts.market.key();
        reserve.mint = ctx.accounts.mint.key();
        reserve.mint_decimals = mint_decimals;
        reserve.total_deposited = DecimalBorsh::zero_with_scale(mint_decimals)
            .map_err(|_| FermatLendingError::MathError)?;
        reserve.total_borrowed = DecimalBorsh::zero_with_scale(mint_decimals)
            .map_err(|_| FermatLendingError::MathError)?;
        reserve.borrow_rate = borrow_rate;
        reserve.liquidation_threshold = liquidation_threshold;
        reserve.cumulative_borrow_index =
            DecimalBorsh(Decimal::new(1_000_000, 6).map_err(|_| FermatLendingError::MathError)?);
        reserve.bump = bump;
        Ok(())
    }

    // ── Position initialisation ───────────────────────────────────────────────

    /// Open a new position for a user.
    pub fn init_position(ctx: Context<InitPosition>, mint_decimals: u8, bump: u8) -> Result<()> {
        let position = &mut ctx.accounts.position;
        position.market = ctx.accounts.market.key();
        position.owner = ctx.accounts.owner.key();
        position.collateral_amount = DecimalBorsh::zero_with_scale(mint_decimals)
            .map_err(|_| FermatLendingError::MathError)?;
        position.debt_amount = DecimalBorsh::zero_with_scale(mint_decimals)
            .map_err(|_| FermatLendingError::MathError)?;
        position.entry_borrow_index =
            DecimalBorsh(Decimal::new(1_000_000, 6).map_err(|_| FermatLendingError::MathError)?);
        position.bump = bump;
        Ok(())
    }

    // ── Core instructions ─────────────────────────────────────────────────────

    /// Deposit collateral into a reserve.
    pub fn deposit(ctx: Context<DepositAccounts>, amount: u64) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        let position = &mut ctx.accounts.position;
        instructions::deposit::handler(reserve, position, amount)
    }

    /// Withdraw collateral, enforcing health factor >= 1.0.
    pub fn withdraw(
        ctx: Context<WithdrawAccounts>,
        amount: u64,
        collateral_price_usd: DecimalBorsh,
        debt_price_usd: DecimalBorsh,
    ) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        let position = &mut ctx.accounts.position;
        instructions::withdraw::handler(
            reserve,
            position,
            amount,
            collateral_price_usd,
            debt_price_usd,
        )
    }

    /// Borrow tokens against deposited collateral.
    pub fn borrow_funds(
        ctx: Context<BorrowAccounts>,
        amount: u64,
        collateral_price_usd: DecimalBorsh,
        borrow_price_usd: DecimalBorsh,
    ) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        let position = &mut ctx.accounts.position;
        instructions::borrow::handler(
            reserve,
            position,
            amount,
            collateral_price_usd,
            borrow_price_usd,
        )
    }

    /// Repay outstanding debt (with accrued interest).
    pub fn repay(ctx: Context<RepayAccounts>, amount: u64) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        let position = &mut ctx.accounts.position;
        instructions::repay::handler(reserve, position, amount)
    }

    // ── Interest accrual ──────────────────────────────────────────────────────

    /// Accrue interest on a reserve's cumulative borrow index.
    ///
    /// `dt_years` is the elapsed time since the last accrual expressed as a
    /// fraction of a year, pre-computed off-chain from slot timestamps.
    pub fn accrue_interest(ctx: Context<AccrueInterest>, dt_years: DecimalBorsh) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        let new_index = math::accrue_interest(
            reserve.cumulative_borrow_index.0,
            reserve.borrow_rate.0,
            dt_years.0,
        )
        .map_err(|_| FermatLendingError::MathError)?;
        reserve.cumulative_borrow_index = DecimalBorsh(new_index);
        Ok(())
    }
}

// ─── Account contexts ─────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct InitMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = Market::SPACE,
        seeds = [b"market", authority.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitReserve<'info> {
    #[account(
        init,
        payer = authority,
        space = Reserve::SPACE,
        seeds = [b"reserve", market.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub reserve: Account<'info, Reserve>,

    pub market: Account<'info, Market>,

    /// CHECK: any SPL mint is valid; decimals verified by caller
    pub mint: UncheckedAccount<'info>,

    #[account(mut, address = market.authority)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitPosition<'info> {
    #[account(
        init,
        payer = owner,
        space = Position::SPACE,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    pub market: Account<'info, Market>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market)]
    pub position: Account<'info, Position>,

    /// CHECK: validated via has_one on position
    pub market: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    /// CHECK: validated via has_one
    pub market: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct BorrowAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    /// CHECK: validated via has_one
    pub market: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RepayAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    /// CHECK: validated via has_one
    pub market: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    /// CHECK: any signer may trigger accrual (keeper bot)
    pub caller: Signer<'info>,
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum FermatLendingError {
    #[msg("Arithmetic error in lending math")]
    MathError,
}
