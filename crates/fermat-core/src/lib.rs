//! # fermat-core
//!
//! 128-bit fixed-point decimal arithmetic for Solana's sBPF runtime.
//!
//! ## Design Goals
//!
//! - **`no_std`**: Works on Solana sBPF without any OS dependencies.
//! - **Zero external dependencies**: Only `proptest` appears as a dev-dependency.
//! - **Panic-free**: Every fallible operation returns `Result<_, ArithmeticError>`.
//! - **`#![forbid(unsafe_code)]`**: No unsafe blocks anywhere in the crate.
//! - **Overflow-safe `mul_div`**: Uses a 256-bit intermediate (`U256`) to prevent
//!   the class of bugs where `(a × b) / c` silently wraps when `a × b` overflows `i128`.
//!
//! ## Core Type
//!
//! ```text
//! Decimal { mantissa: i128, scale: u8 }
//! value = mantissa × 10^(-scale)
//! ```
//!
//! The `scale` is bounded to `[0, 28]` (`MAX_SCALE`). On-chain Borsh encoding is
//! exactly 17 bytes (16 bytes mantissa LE + 1 byte scale).
//!
//! ## Quick Start
//!
//! ```rust
//! use fermat_core::{Decimal, RoundingMode};
//!
//! let price  = Decimal::new(150_000_000, 6).unwrap(); // 150.000000
//! let amount = Decimal::new(2_500_000,   6).unwrap(); //   2.500000
//! let total  = price.checked_mul(amount).unwrap();    // 375.000000...
//! let result = total.round(6, RoundingMode::HalfEven).unwrap();
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]

extern crate alloc;

pub mod arithmetic;
pub mod compare;
pub mod convert;
pub mod decimal;
pub mod display;
pub mod error;
pub mod rounding;

pub use decimal::{Decimal, MAX_SCALE, SOL_SCALE, USDC_SCALE};
pub use error::ArithmeticError;
pub use rounding::RoundingMode;

#[cfg(test)]
mod tests_arithmetic;
#[cfg(test)]
mod tests_rounding;
