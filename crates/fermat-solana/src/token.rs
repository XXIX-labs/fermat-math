//! SPL token amount ↔ `Decimal` conversions.
//!
//! Solana SPL tokens represent amounts as raw `u64` integers with an
//! associated `decimals` field from the mint account (0–9 for most tokens).
//! This module bridges that world with `fermat_core::Decimal`.
//!
//! ## Example
//!
//! ```rust
//! use fermat_core::RoundingMode;
//! use fermat_solana::token::{token_amount_to_decimal, decimal_to_token_amount};
//!
//! // 1_000_000 raw USDC (6 decimals) == 1.000000 USDC
//! let d = token_amount_to_decimal(1_000_000, 6).unwrap();
//! assert_eq!(d.mantissa(), 1_000_000);
//! assert_eq!(d.scale(), 6);
//!
//! // Convert back: 1.5 USDC → 1_500_000 raw
//! use fermat_core::Decimal;
//! let one_five = Decimal::new(1_500_000, 6).unwrap();
//! let raw = decimal_to_token_amount(one_five, 6, RoundingMode::HalfEven).unwrap();
//! assert_eq!(raw, 1_500_000u64);
//! ```

use fermat_core::{ArithmeticError, Decimal, RoundingMode};

/// Convert a raw SPL token amount (`u64`) to a `Decimal` value.
///
/// The `mint_decimals` parameter is the `decimals` field from the SPL mint
/// account (e.g. 6 for USDC, 9 for SOL/wSOL). The resulting `Decimal` has
/// `scale == mint_decimals`, so arithmetic with other token prices works
/// without additional rescaling.
///
/// # Errors
/// Returns [`ArithmeticError::ScaleExceeded`] if `mint_decimals > MAX_SCALE`.
pub fn token_amount_to_decimal(amount: u64, mint_decimals: u8) -> Result<Decimal, ArithmeticError> {
    Decimal::new(amount as i128, mint_decimals)
}

/// Convert a `Decimal` to a raw SPL token amount (`u64`).
///
/// The `Decimal` is first rounded to `mint_decimals` decimal places using the
/// provided `mode`, then the mantissa is extracted and cast to `u64`.
///
/// # Errors
/// - [`ArithmeticError::Overflow`] if the value is negative or too large for `u64`.
/// - Any error from [`Decimal::round`].
pub fn decimal_to_token_amount(
    value: Decimal,
    mint_decimals: u8,
    mode: RoundingMode,
) -> Result<u64, ArithmeticError> {
    if value.is_negative() {
        return Err(ArithmeticError::Overflow);
    }

    // Round (or rescale up) to exactly mint_decimals.
    let rounded = if value.scale() >= mint_decimals {
        value.round(mint_decimals, mode)?
    } else {
        value.rescale_up(mint_decimals)?
    };

    // The mantissa now represents the raw token units.
    let raw = rounded.mantissa();
    u64::try_from(raw).map_err(|_| ArithmeticError::Overflow)
}

/// Rescale a `Decimal` to match `mint_decimals` for display or storage.
///
/// Unlike [`decimal_to_token_amount`] this preserves the `Decimal` type and
/// is useful when you want to align two values before comparison or arithmetic
/// without converting to `u64`.
///
/// # Errors
/// Returns an error if rescaling overflows or if `mint_decimals > MAX_SCALE`.
pub fn align_to_mint(
    value: Decimal,
    mint_decimals: u8,
    mode: RoundingMode,
) -> Result<Decimal, ArithmeticError> {
    if value.scale() >= mint_decimals {
        value.round(mint_decimals, mode)
    } else {
        value.rescale_up(mint_decimals)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use fermat_core::{Decimal, RoundingMode};

    #[test]
    fn usdc_amount_roundtrip() {
        // 1_500_000 raw = 1.500000 USDC (6 dp)
        let d = token_amount_to_decimal(1_500_000, 6).unwrap();
        assert_eq!(d.mantissa(), 1_500_000);
        assert_eq!(d.scale(), 6);
        let raw = decimal_to_token_amount(d, 6, RoundingMode::HalfEven).unwrap();
        assert_eq!(raw, 1_500_000u64);
    }

    #[test]
    fn sol_amount_roundtrip() {
        // 2_500_000_000 lamports = 2.5 SOL (9 dp)
        let d = token_amount_to_decimal(2_500_000_000, 9).unwrap();
        let raw = decimal_to_token_amount(d, 9, RoundingMode::HalfEven).unwrap();
        assert_eq!(raw, 2_500_000_000u64);
    }

    #[test]
    fn round_down_on_withdrawal() {
        // 1.9999999 (7 dp) rounded to 6 dp with Down → 1.999999 (no rounding up)
        let d = Decimal::new(19_999_999, 7).unwrap();
        let raw = decimal_to_token_amount(d, 6, RoundingMode::Down).unwrap();
        assert_eq!(raw, 1_999_999u64);
    }

    #[test]
    fn round_up_on_fee() {
        // 0.0000001 (7 dp) rounded to 6 dp with Up → 0.000001 (round up)
        let d = Decimal::new(1, 7).unwrap();
        let raw = decimal_to_token_amount(d, 6, RoundingMode::Up).unwrap();
        assert_eq!(raw, 1u64);
    }

    #[test]
    fn zero_amount() {
        let d = token_amount_to_decimal(0, 6).unwrap();
        let raw = decimal_to_token_amount(d, 6, RoundingMode::HalfEven).unwrap();
        assert_eq!(raw, 0u64);
    }

    #[test]
    fn negative_value_errors() {
        let d = Decimal::new(-1_000_000, 6).unwrap();
        let result = decimal_to_token_amount(d, 6, RoundingMode::HalfEven);
        assert_eq!(result, Err(ArithmeticError::Overflow));
    }

    #[test]
    fn rescale_up_adds_precision() {
        // 1.5 (scale=1) aligned to 6 dp → 1.500000
        let d = Decimal::new(15, 1).unwrap();
        let aligned = align_to_mint(d, 6, RoundingMode::HalfEven).unwrap();
        assert_eq!(aligned.scale(), 6);
        assert_eq!(aligned.mantissa(), 1_500_000);
    }

    #[test]
    fn invalid_mint_decimals() {
        let result = token_amount_to_decimal(100, 29); // scale > MAX_SCALE
        assert!(result.is_err());
    }
}
