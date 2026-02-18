//! # fermat-solana
//!
//! Solana / Anchor integration helpers for [`fermat_core::Decimal`].
//!
//! ## Modules
//!
//! - [`borsh_impl`]: `DecimalBorsh` wrapper — 17-byte on-chain Borsh encoding
//!   with adversarial scale validation.
//! - [`token`]: SPL mint ↔ `Decimal` conversions with explicit rounding.
//! - [`account`]: Anchor account helpers including `DECIMAL_SPACE` constant
//!   and `zero_with_scale` initialiser.

pub mod account;
pub mod borsh_impl;
pub mod token;

pub use account::DECIMAL_SPACE;
pub use borsh_impl::DecimalBorsh;
