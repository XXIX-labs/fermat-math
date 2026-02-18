//! Anchor account helpers for embedding `Decimal` in on-chain account structs.
//!
//! ## Usage
//!
//! Use [`DecimalBorsh`] for any `Decimal`-valued field inside an Anchor
//! `#[account]` struct. The 17-byte size is exposed via the `DECIMAL_SPACE`
//! constant so you can derive the correct `space` in `init` constraints:
//!
//! ```ignore
//! use anchor_lang::prelude::*;
//! use fermat_solana::{DecimalBorsh, account::DECIMAL_SPACE};
//!
//! #[account]
//! pub struct PriceOracle {
//!     pub authority: Pubkey,   // 32 bytes
//!     pub price:     DecimalBorsh, // 17 bytes
//!     pub confidence: DecimalBorsh, // 17 bytes
//! }
//!
//! impl PriceOracle {
//!     pub const SPACE: usize = 8 + 32 + DECIMAL_SPACE + DECIMAL_SPACE;
//! }
//! ```

use crate::borsh_impl::DecimalBorsh;
use fermat_core::Decimal;

/// On-chain byte size of a Borsh-serialised `Decimal` (mantissa + scale).
///
/// Use this constant in `init` constraint `space` calculations to ensure
/// accounts are allocated the correct number of bytes.
///
/// ```text
/// 16 bytes — mantissa (i128, little-endian)
///  1 byte  — scale    (u8)
/// ──────────
/// 17 bytes total
/// ```
pub const DECIMAL_SPACE: usize = 17;

/// Extension helpers on `DecimalBorsh` for common Anchor patterns.
impl DecimalBorsh {
    /// Create a `DecimalBorsh` representing zero with the given scale.
    ///
    /// Useful for initialising account fields in `init` instructions:
    /// ```rust
    /// use fermat_solana::account::DECIMAL_SPACE;
    /// use fermat_solana::DecimalBorsh;
    ///
    /// let field = DecimalBorsh::zero_with_scale(6).unwrap();
    /// assert_eq!(field.0.mantissa(), 0);
    /// assert_eq!(field.0.scale(), 6);
    /// ```
    pub fn zero_with_scale(scale: u8) -> Result<Self, fermat_core::ArithmeticError> {
        Ok(DecimalBorsh(Decimal::new(0, scale)?))
    }

    /// Return `true` if the wrapped `Decimal` equals zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_space_is_17() {
        assert_eq!(DECIMAL_SPACE, 17);
    }

    #[test]
    fn zero_with_scale() {
        let d = DecimalBorsh::zero_with_scale(6).unwrap();
        assert!(d.is_zero());
        assert_eq!(d.0.scale(), 6);
    }

    #[test]
    fn zero_with_invalid_scale_errors() {
        assert!(DecimalBorsh::zero_with_scale(29).is_err());
    }
}
