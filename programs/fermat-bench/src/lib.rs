//! # fermat-bench
//!
//! Anchor program for measuring the compute-unit (CU) cost of each
//! `fermat-core` operation on Solana's sBPF runtime.
//!
//! ## How It Works
//!
//! Each instruction runs a specific arithmetic operation **1 000 times** in a
//! tight loop, then emits a `BenchResult` event with the operation name.
//! Off-chain code captures the CU consumed per instruction call and divides by
//! 1 000 to get the amortised per-operation cost.
//!
//! ## Running Benchmarks
//!
//! ```bash
//! anchor test -- --features bench
//! ```
//!
//! The test harness calls each instruction, reads `sol_log_compute_units` via
//! the program log, and prints a summary table.

use anchor_lang::prelude::*;
use fermat_core::{Decimal, RoundingMode};

declare_id!("FErMaT1111111111111111111111111111111111111");

/// Number of iterations per benchmark instruction.
const BENCH_ITERS: u32 = 1_000;

// ─── Events ──────────────────────────────────────────────────────────────────

/// Emitted at the end of each benchmark instruction.
#[event]
pub struct BenchResult {
    /// Name of the operation being benchmarked.
    pub op: String,
    /// Number of iterations performed.
    pub iters: u32,
    /// Last computed mantissa — prevents dead-code elimination.
    pub last_mantissa: i128,
}

// ─── Program ──────────────────────────────────────────────────────────────────

#[program]
pub mod fermat_bench {
    use super::*;

    /// Benchmark `Decimal::checked_add` — 1 000 iterations.
    pub fn bench_add(_ctx: Context<Bench>) -> Result<()> {
        let a = Decimal::new(150_000_000, 6).unwrap(); // 150.000000
        let b = Decimal::new(2_500_000, 6).unwrap(); //   2.500000
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = a.checked_add(b).unwrap();
        }
        emit!(BenchResult {
            op: "checked_add".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark `Decimal::checked_sub` — 1 000 iterations.
    pub fn bench_sub(_ctx: Context<Bench>) -> Result<()> {
        let a = Decimal::new(150_000_000, 6).unwrap();
        let b = Decimal::new(2_500_000, 6).unwrap();
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = a.checked_sub(b).unwrap();
        }
        emit!(BenchResult {
            op: "checked_sub".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark `Decimal::checked_mul` — 1 000 iterations.
    pub fn bench_mul(_ctx: Context<Bench>) -> Result<()> {
        let a = Decimal::new(150_000_000, 6).unwrap(); // 150.000000
        let b = Decimal::new(2_500_000, 6).unwrap(); //   2.500000
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = a.checked_mul(b).unwrap();
        }
        emit!(BenchResult {
            op: "checked_mul".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark `Decimal::checked_div` — 1 000 iterations.
    pub fn bench_div(_ctx: Context<Bench>) -> Result<()> {
        let a = Decimal::new(150_000_000, 6).unwrap(); // 150.000000
        let b = Decimal::new(2_500_000, 6).unwrap(); //   2.500000
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = a.checked_div(b).unwrap();
        }
        emit!(BenchResult {
            op: "checked_div".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark `Decimal::checked_mul_div` with U256 intermediate — 1 000 iterations.
    ///
    /// Uses large operands to exercise the full 256-bit path, which is the most
    /// expensive single arithmetic operation in the library.
    pub fn bench_mul_div(_ctx: Context<Bench>) -> Result<()> {
        let base = Decimal::new(i128::MAX / 4, 0).unwrap();
        let num = Decimal::new(3, 0).unwrap();
        let den = Decimal::new(4, 0).unwrap();
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = base.checked_mul_div(num, den).unwrap();
        }
        emit!(BenchResult {
            op: "checked_mul_div".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark `Decimal::round` with HalfEven mode — 1 000 iterations.
    pub fn bench_round(_ctx: Context<Bench>) -> Result<()> {
        let a = Decimal::new(1_234_567_890, 9).unwrap(); // 1.234567890
        let mut acc = Decimal::ZERO;
        for _ in 0..BENCH_ITERS {
            acc = a.round(6, RoundingMode::HalfEven).unwrap();
        }
        emit!(BenchResult {
            op: "round_half_even".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: acc.mantissa(),
        });
        Ok(())
    }

    /// Benchmark Borsh serialization of `DecimalBorsh` — 1 000 iterations.
    ///
    /// Measures the overhead of encoding the 17-byte on-chain format.
    pub fn bench_borsh_serialize(_ctx: Context<Bench>) -> Result<()> {
        use borsh::BorshSerialize;
        use fermat_solana::DecimalBorsh;

        let d = Decimal::new(1_500_000, 6).unwrap();
        let wrapped = DecimalBorsh(d);
        let mut last = 0i128;
        for _ in 0..BENCH_ITERS {
            let mut buf = Vec::with_capacity(17);
            wrapped.serialize(&mut buf).unwrap();
            last = buf.len() as i128;
        }
        emit!(BenchResult {
            op: "borsh_serialize".to_string(),
            iters: BENCH_ITERS,
            last_mantissa: last,
        });
        Ok(())
    }
}

// ─── Accounts ─────────────────────────────────────────────────────────────────

/// Empty context — benchmarks are pure compute, no accounts needed.
#[derive(Accounts)]
pub struct Bench<'info> {
    pub signer: Signer<'info>,
}
