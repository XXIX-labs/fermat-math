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
//! Market  ─ authority, pause flag, aggregated TVL/debt
//!   └── Reserve ─ per-token params, kinked rate model, cumulative index
//!         └── Position ─ per-user collateral + debt
//! ```
//!
//! ## Instructions
//!
//! | Instruction            | Who        | Description                          |
//! |------------------------|------------|--------------------------------------|
//! | `init_market`          | authority  | Create a new lending market          |
//! | `init_reserve`         | authority  | Add a token reserve                  |
//! | `init_position`        | user       | Open a position                      |
//! | `deposit`              | user       | Deposit collateral                   |
//! | `withdraw`             | user       | Withdraw collateral (HF check)       |
//! | `borrow_funds`         | user       | Borrow against collateral (HF check) |
//! | `repay`                | user       | Repay debt with accrued interest     |
//! | `liquidate`            | anyone     | Liquidate undercollateralised pos.   |
//! | `accrue_interest`      | keeper     | Advance index + recompute rate       |
//! | `update_reserve_params`| authority  | Update risk / rate parameters        |
//! | `set_market_paused`    | authority  | Pause / unpause the whole market     |
//! | `set_reserve_paused`   | authority  | Pause / unpause a single reserve     |

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use fermat_solana::DecimalBorsh;

pub mod instructions;
pub mod math;
pub mod state;

use instructions::LendingError;
use state::{Market, Position, Reserve};

declare_id!("FErMaT2222222222222222222222222222222222222");

#[program]
pub mod fermat_lending {
    use super::*;

    // ── Market initialisation ─────────────────────────────────────────────────

    pub fn init_market(ctx: Context<InitMarket>, bump: u8) -> Result<()> {
        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.total_deposited_value = DecimalBorsh(Decimal::ZERO);
        market.total_borrowed_value = DecimalBorsh(Decimal::ZERO);
        market.paused = false;
        market.bump = bump;
        Ok(())
    }

    // ── Reserve initialisation ────────────────────────────────────────────────

    /// Add a token reserve with a kinked interest rate model.
    ///
    /// # Rate model parameters
    /// - `base_borrow_rate` — rate at 0% utilisation (e.g. 0.02 = 2%).
    /// - `rate_slope1`      — additional rate at `optimal_utilisation` (e.g. 0.04).
    /// - `rate_slope2`      — rate slope above the kink (e.g. 0.50 = 50%).
    /// - `optimal_utilisation` — kink point, ∈ (0, 1) exclusive (e.g. 0.80).
    pub fn init_reserve(
        ctx: Context<InitReserve>,
        mint_decimals: u8,
        liquidation_threshold: DecimalBorsh,
        liquidation_bonus: DecimalBorsh,
        optimal_utilisation: DecimalBorsh,
        base_borrow_rate: DecimalBorsh,
        rate_slope1: DecimalBorsh,
        rate_slope2: DecimalBorsh,
        bump: u8,
    ) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;
        reserve.market = ctx.accounts.market.key();
        reserve.mint = ctx.accounts.mint.key();
        reserve.mint_decimals = mint_decimals;
        reserve.total_deposited =
            DecimalBorsh::zero_with_scale(mint_decimals).map_err(|_| LendingError::MathError)?;
        reserve.total_borrowed =
            DecimalBorsh::zero_with_scale(mint_decimals).map_err(|_| LendingError::MathError)?;
        // Cached borrow rate starts at base (0% utilisation).
        reserve.borrow_rate = base_borrow_rate;
        reserve.liquidation_threshold = liquidation_threshold;
        reserve.cumulative_borrow_index =
            DecimalBorsh(Decimal::new(1_000_000, 6).map_err(|_| LendingError::MathError)?);
        reserve.paused = false;
        reserve.liquidation_bonus = liquidation_bonus;
        reserve.optimal_utilisation = optimal_utilisation;
        reserve.base_borrow_rate = base_borrow_rate;
        reserve.rate_slope1 = rate_slope1;
        reserve.rate_slope2 = rate_slope2;
        reserve.bump = bump;
        Ok(())
    }

    // ── Position initialisation ───────────────────────────────────────────────

    pub fn init_position(ctx: Context<InitPosition>, mint_decimals: u8, bump: u8) -> Result<()> {
        let position = &mut ctx.accounts.position;
        position.market = ctx.accounts.market.key();
        position.owner = ctx.accounts.owner.key();
        position.collateral_amount =
            DecimalBorsh::zero_with_scale(mint_decimals).map_err(|_| LendingError::MathError)?;
        position.debt_amount =
            DecimalBorsh::zero_with_scale(mint_decimals).map_err(|_| LendingError::MathError)?;
        position.entry_borrow_index =
            DecimalBorsh(Decimal::new(1_000_000, 6).map_err(|_| LendingError::MathError)?);
        position.bump = bump;
        Ok(())
    }

    // ── Core instructions ─────────────────────────────────────────────────────

    pub fn deposit(ctx: Context<DepositAccounts>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.market.paused, LendingError::MarketPaused);
        require!(!ctx.accounts.reserve.paused, LendingError::ReservePaused);
        instructions::deposit::handler(
            &mut ctx.accounts.reserve,
            &mut ctx.accounts.position,
            amount,
        )
    }

    pub fn withdraw(
        ctx: Context<WithdrawAccounts>,
        amount: u64,
        collateral_price_usd: DecimalBorsh,
        debt_price_usd: DecimalBorsh,
    ) -> Result<()> {
        require!(!ctx.accounts.market.paused, LendingError::MarketPaused);
        require!(!ctx.accounts.reserve.paused, LendingError::ReservePaused);
        instructions::withdraw::handler(
            &mut ctx.accounts.reserve,
            &mut ctx.accounts.position,
            amount,
            collateral_price_usd,
            debt_price_usd,
        )
    }

    pub fn borrow_funds(
        ctx: Context<BorrowAccounts>,
        amount: u64,
        collateral_price_usd: DecimalBorsh,
        borrow_price_usd: DecimalBorsh,
    ) -> Result<()> {
        require!(!ctx.accounts.market.paused, LendingError::MarketPaused);
        require!(!ctx.accounts.reserve.paused, LendingError::ReservePaused);
        instructions::borrow::handler(
            &mut ctx.accounts.reserve,
            &mut ctx.accounts.position,
            amount,
            collateral_price_usd,
            borrow_price_usd,
        )
    }

    pub fn repay(ctx: Context<RepayAccounts>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.market.paused, LendingError::MarketPaused);
        require!(!ctx.accounts.reserve.paused, LendingError::ReservePaused);
        instructions::repay::handler(
            &mut ctx.accounts.reserve,
            &mut ctx.accounts.position,
            amount,
        )
    }

    /// Liquidate an undercollateralised position.
    ///
    /// Anyone may call this when the position's health factor < 1.0.
    pub fn liquidate(
        ctx: Context<LiquidateAccounts>,
        repay_amount: u64,
        collateral_price_usd: DecimalBorsh,
        debt_price_usd: DecimalBorsh,
    ) -> Result<()> {
        require!(!ctx.accounts.market.paused, LendingError::MarketPaused);
        require!(!ctx.accounts.reserve.paused, LendingError::ReservePaused);
        instructions::liquidate::handler(
            &mut ctx.accounts.reserve,
            &mut ctx.accounts.position,
            repay_amount,
            collateral_price_usd,
            debt_price_usd,
        )
    }

    // ── Interest accrual ──────────────────────────────────────────────────────

    /// Advance the cumulative borrow index and recompute the kinked rate.
    ///
    /// Should be called by a keeper bot on every epoch / slot boundary.
    /// `dt_years` is elapsed time as a fraction of a year (pre-computed
    /// off-chain from slot timestamps).
    pub fn accrue_interest(ctx: Context<AccrueInterest>, dt_years: DecimalBorsh) -> Result<()> {
        let reserve = &mut ctx.accounts.reserve;

        // Recompute borrow rate from current utilisation.
        let util = math::utilisation_rate(reserve.total_borrowed.0, reserve.total_deposited.0)
            .map_err(|_| LendingError::MathError)?;
        let new_rate = math::kinked_borrow_rate(
            util,
            reserve.base_borrow_rate.0,
            reserve.rate_slope1.0,
            reserve.rate_slope2.0,
            reserve.optimal_utilisation.0,
        )
        .map_err(|_| LendingError::MathError)?;
        reserve.borrow_rate = DecimalBorsh(new_rate);

        // Advance the index: new_index = old_index × (1 + rate × dt)
        let new_index =
            math::accrue_interest(reserve.cumulative_borrow_index.0, new_rate, dt_years.0)
                .map_err(|_| LendingError::MathError)?;
        reserve.cumulative_borrow_index = DecimalBorsh(new_index);

        Ok(())
    }

    // ── Governance ────────────────────────────────────────────────────────────

    /// Update risk and rate model parameters for a reserve.
    ///
    /// Callable only by the market authority.
    pub fn update_reserve_params(
        ctx: Context<UpdateReserveParams>,
        liquidation_threshold: DecimalBorsh,
        liquidation_bonus: DecimalBorsh,
        optimal_utilisation: DecimalBorsh,
        base_borrow_rate: DecimalBorsh,
        rate_slope1: DecimalBorsh,
        rate_slope2: DecimalBorsh,
    ) -> Result<()> {
        // Basic sanity checks: all values must be non-negative fractions ≤ 1.0
        // (except slope2 which can exceed 1 for steep jump rates).
        let one = Decimal::ONE;
        require!(
            liquidation_threshold.0 > Decimal::ZERO && liquidation_threshold.0 <= one,
            LendingError::InvalidParameter
        );
        require!(
            liquidation_bonus.0 >= Decimal::ZERO && liquidation_bonus.0 <= one,
            LendingError::InvalidParameter
        );
        require!(
            optimal_utilisation.0 > Decimal::ZERO && optimal_utilisation.0 < one,
            LendingError::InvalidParameter
        );

        let reserve = &mut ctx.accounts.reserve;
        reserve.liquidation_threshold = liquidation_threshold;
        reserve.liquidation_bonus = liquidation_bonus;
        reserve.optimal_utilisation = optimal_utilisation;
        reserve.base_borrow_rate = base_borrow_rate;
        reserve.rate_slope1 = rate_slope1;
        reserve.rate_slope2 = rate_slope2;
        Ok(())
    }

    /// Pause or unpause all instructions for every reserve in the market.
    pub fn set_market_paused(ctx: Context<AdminMarket>, paused: bool) -> Result<()> {
        ctx.accounts.market.paused = paused;
        Ok(())
    }

    /// Pause or unpause instructions for a single reserve.
    pub fn set_reserve_paused(ctx: Context<AdminReserve>, paused: bool) -> Result<()> {
        ctx.accounts.reserve.paused = paused;
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

    pub market: Account<'info, Market>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    pub market: Account<'info, Market>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct BorrowAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    pub market: Account<'info, Market>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RepayAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market, has_one = owner)]
    pub position: Account<'info, Position>,

    pub market: Account<'info, Market>,

    pub owner: Signer<'info>,
}

/// Anyone may liquidate — no `has_one = owner` constraint on position.
#[derive(Accounts)]
pub struct LiquidateAccounts<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    #[account(mut, has_one = market)]
    pub position: Account<'info, Position>,

    pub market: Account<'info, Market>,

    pub liquidator: Signer<'info>,
}

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
    #[account(mut)]
    pub reserve: Account<'info, Reserve>,

    /// CHECK: any signer may trigger accrual (permissionless keeper).
    pub caller: Signer<'info>,
}

/// Governance: update reserve parameters (authority only).
#[derive(Accounts)]
pub struct UpdateReserveParams<'info> {
    #[account(mut, has_one = market)]
    pub reserve: Account<'info, Reserve>,

    pub market: Account<'info, Market>,

    #[account(address = market.authority)]
    pub authority: Signer<'info>,
}

/// Governance: pause / unpause the whole market.
#[derive(Accounts)]
pub struct AdminMarket<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(address = market.authority)]
    pub authority: Signer<'info>,
}

/// Governance: pause / unpause a single reserve.
#[derive(Accounts)]
pub struct AdminReserve<'info> {
    #[account(mut, has_one = market)]
    pub reserve: Account<'info, Reserve>,

    pub market: Account<'info, Market>,

    #[account(address = market.authority)]
    pub authority: Signer<'info>,
}
