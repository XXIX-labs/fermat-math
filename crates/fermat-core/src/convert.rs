//! Conversions between `Decimal` and primitive types.
//!
//! ## Supported Conversions
//!
//! | From/To          | Method                       | Notes                               |
//! |------------------|------------------------------|-------------------------------------|
//! | `u64` → Decimal  | `Decimal::from_u64`          | scale = 0                           |
//! | `i64` → Decimal  | `Decimal::from_i64`          | scale = 0                           |
//! | `u128` → Decimal | `Decimal::from_u128`         | scale = 0, fails if > i128::MAX     |
//! | `i128` → Decimal | `Decimal::from_i128`         | scale = 0                           |
//! | Decimal → `i128` | `Decimal::to_i128_truncated` | truncates toward zero               |
//! | Decimal → `u64`  | `Decimal::to_u64_truncated`  | fails if negative or overflows u64  |
//! | `str` → Decimal  | `Decimal::from_str_exact`    | parses decimal notation             |

use crate::arithmetic::{pow10, POW10};
use crate::decimal::{Decimal, MAX_SCALE};
use crate::error::ArithmeticError;

// ─── From primitives ─────────────────────────────────────────────────────────

impl Decimal {
    /// Create a `Decimal` with scale 0 from a `u64`.
    #[inline]
    pub fn from_u64(v: u64) -> Self {
        Decimal::new_unchecked(v as i128, 0)
    }

    /// Create a `Decimal` with scale 0 from an `i64`.
    #[inline]
    pub fn from_i64(v: i64) -> Self {
        Decimal::new_unchecked(v as i128, 0)
    }

    /// Create a `Decimal` with scale 0 from a `u128`.
    ///
    /// Returns `Err(Overflow)` if `v > i128::MAX`.
    pub fn from_u128(v: u128) -> Result<Self, ArithmeticError> {
        let mantissa = i128::try_from(v).map_err(|_| ArithmeticError::Overflow)?;
        Decimal::new(mantissa, 0)
    }

    /// Create a `Decimal` with scale 0 from an `i128`.
    #[inline]
    pub fn from_i128(v: i128) -> Self {
        Decimal::new_unchecked(v, 0)
    }

    /// Create a `Decimal` from a raw SPL token `amount` and the mint's `decimals`.
    ///
    /// `from_token_amount(1_500_000, 6)` represents `1.500000 USDC`.
    pub fn from_token_amount(amount: u64, decimals: u8) -> Result<Self, ArithmeticError> {
        Decimal::new(amount as i128, decimals)
    }
}

// ─── To primitives ────────────────────────────────────────────────────────────

impl Decimal {
    /// Truncate toward zero and return the integer part as `i128`.
    ///
    /// `Decimal { mantissa: 157, scale: 2 }` (= `1.57`) → `1`
    pub fn to_i128_truncated(self) -> i128 {
        if self.scale == 0 {
            return self.mantissa;
        }
        let factor = POW10[self.scale as usize];
        self.mantissa / factor
    }

    /// Truncate toward zero and return the integer part as `u64`.
    ///
    /// Returns `Err(Overflow)` if the value is negative or exceeds `u64::MAX`.
    pub fn to_u64_truncated(self) -> Result<u64, ArithmeticError> {
        let v = self.to_i128_truncated();
        u64::try_from(v).map_err(|_| ArithmeticError::Overflow)
    }

    /// Convert to a raw SPL token `u64` amount with explicit rounding.
    ///
    /// Rounds to `decimals` decimal places first, then converts to an integer.
    pub fn to_token_amount(
        self,
        decimals: u8,
        mode: crate::rounding::RoundingMode,
    ) -> Result<u64, ArithmeticError> {
        let rounded = self.round(decimals, mode)?;
        let diff = decimals.saturating_sub(rounded.scale());
        let factor = pow10(diff)?;
        let raw = rounded
            .mantissa()
            .checked_mul(factor)
            .ok_or(ArithmeticError::Overflow)?;
        u64::try_from(raw).map_err(|_| ArithmeticError::Overflow)
    }
}

// ─── String parsing ───────────────────────────────────────────────────────────

impl Decimal {
    /// Parse a decimal string into a `Decimal`.
    ///
    /// Accepted formats:
    /// - `"123"`    → `{ mantissa: 123, scale: 0 }`
    /// - `"1.23"`   → `{ mantissa: 123, scale: 2 }`
    /// - `"-1.23"`  → `{ mantissa: -123, scale: 2 }`
    /// - `"+1.23"`  → `{ mantissa: 123, scale: 2 }`
    ///
    /// Returns `Err(ScaleExceeded)` if there are more than 28 fractional digits,
    /// or `Err(InvalidInput)` for malformed strings.
    pub fn from_str_exact(s: &str) -> Result<Self, ArithmeticError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ArithmeticError::InvalidInput);
        }

        let (negative, rest) = if let Some(stripped) = s.strip_prefix('-') {
            (true, stripped)
        } else if let Some(stripped) = s.strip_prefix('+') {
            (false, stripped)
        } else {
            (false, s)
        };

        if rest.is_empty() {
            return Err(ArithmeticError::InvalidInput);
        }

        let (int_part, frac_part, scale) = if let Some(dot) = rest.find('.') {
            let frac = &rest[dot + 1..];
            if frac.len() > MAX_SCALE as usize {
                return Err(ArithmeticError::ScaleExceeded);
            }
            (&rest[..dot], frac, frac.len() as u8)
        } else {
            (rest, "", 0u8)
        };

        if int_part.is_empty() && frac_part.is_empty() {
            return Err(ArithmeticError::InvalidInput);
        }

        let mut mantissa: i128 = 0;

        for ch in int_part.bytes() {
            let digit = ch.wrapping_sub(b'0');
            if digit > 9 {
                return Err(ArithmeticError::InvalidInput);
            }
            mantissa = mantissa
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit as i128))
                .ok_or(ArithmeticError::Overflow)?;
        }

        for ch in frac_part.bytes() {
            let digit = ch.wrapping_sub(b'0');
            if digit > 9 {
                return Err(ArithmeticError::InvalidInput);
            }
            mantissa = mantissa
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit as i128))
                .ok_or(ArithmeticError::Overflow)?;
        }

        if negative {
            mantissa = mantissa.checked_neg().ok_or(ArithmeticError::Overflow)?;
        }

        Decimal::new(mantissa, scale)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    #[test]
    fn from_u64_basic() {
        let x = Decimal::from_u64(1_000_000);
        assert_eq!(x.mantissa(), 1_000_000);
        assert_eq!(x.scale(), 0);
    }

    #[test]
    fn from_i64_negative() {
        let x = Decimal::from_i64(-42);
        assert_eq!(x.mantissa(), -42);
        assert_eq!(x.scale(), 0);
    }

    #[test]
    fn from_u128_fits() {
        assert!(Decimal::from_u128(u128::from(u64::MAX)).is_ok());
    }

    #[test]
    fn from_u128_overflow() {
        assert!(Decimal::from_u128((i128::MAX as u128) + 1).is_err());
    }

    #[test]
    fn from_token_amount() {
        let x = Decimal::from_token_amount(1_500_000, 6).unwrap();
        assert_eq!(x.mantissa(), 1_500_000);
        assert_eq!(x.scale(), 6);
    }

    #[test]
    fn to_i128_truncated_no_scale() {
        assert_eq!(d(42, 0).to_i128_truncated(), 42);
    }

    #[test]
    fn to_i128_truncated_rounds_toward_zero() {
        assert_eq!(d(157, 2).to_i128_truncated(), 1);
        assert_eq!(d(-157, 2).to_i128_truncated(), -1);
    }

    #[test]
    fn to_u64_truncated_positive() {
        assert_eq!(d(157, 2).to_u64_truncated().unwrap(), 1u64);
    }

    #[test]
    fn to_u64_truncated_negative_fails() {
        assert!(d(-1, 0).to_u64_truncated().is_err());
    }

    #[test]
    fn parse_integer() {
        assert_eq!(Decimal::from_str_exact("42").unwrap(), d(42, 0));
    }

    #[test]
    fn parse_decimal_two_places() {
        assert_eq!(Decimal::from_str_exact("1.23").unwrap(), d(123, 2));
    }

    #[test]
    fn parse_negative_decimal() {
        assert_eq!(Decimal::from_str_exact("-1.23").unwrap(), d(-123, 2));
    }

    #[test]
    fn parse_positive_sign() {
        assert_eq!(Decimal::from_str_exact("+1.23").unwrap(), d(123, 2));
    }

    #[test]
    fn parse_zero_int_part() {
        assert_eq!(Decimal::from_str_exact("0.001").unwrap(), d(1, 3));
    }

    #[test]
    fn parse_trailing_zeros_set_scale() {
        assert_eq!(Decimal::from_str_exact("1.00").unwrap().scale(), 2);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(Decimal::from_str_exact("").is_err());
    }

    #[test]
    fn parse_alpha_fails() {
        assert!(Decimal::from_str_exact("abc").is_err());
    }

    #[test]
    fn parse_too_many_decimals_fails() {
        let s = "0.00000000000000000000000000001"; // 29 dp
        assert!(matches!(
            Decimal::from_str_exact(s),
            Err(ArithmeticError::ScaleExceeded)
        ));
    }

    #[test]
    fn roundtrip_token_amount() {
        use crate::rounding::RoundingMode;
        let x = Decimal::from_token_amount(1_234_567, 6).unwrap();
        let back = x.to_token_amount(6, RoundingMode::HalfEven).unwrap();
        assert_eq!(back, 1_234_567u64);
    }
}
