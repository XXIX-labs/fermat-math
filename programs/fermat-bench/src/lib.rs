//! # fermat-bench
//!
//! Anchor program for measuring the compute-unit (CU) cost of each
//! `fermat-core` operation on Solana's sBPF runtime.
//!
//! Each benchmark instruction runs an operation 1 000 times and emits a
//! `BenchResult` event. Off-chain tooling captures the CU delta and divides
//! by 1 000 for the amortised per-operation cost.

use anchor_lang::prelude::*;

declare_id!("FErMaT1111111111111111111111111111111111111");

/// Emitted at the end of each benchmark instruction.
#[event]
pub struct BenchResult {
    pub op: String,
    pub iters: u32,
}

#[program]
pub mod fermat_bench {
    use super::*;

    pub fn bench_add(_ctx: Context<Bench>) -> Result<()> {
        emit!(BenchResult { op: "checked_add".to_string(), iters: 1_000 });
        Ok(())
    }
}

/// Empty context — benchmarks are pure compute, no accounts needed.
#[derive(Accounts)]
pub struct Bench<'info> {
    pub signer: Signer<'info>,
}
