//! Core `Decimal` type definition and constants.

use crate::error::ArithmeticError;

/// Maximum allowed scale (decimal places). Fixed at 28 to stay within `i128` range.
pub const MAX_SCALE: u8 = 28;

/// Conventional scale for USDC (6 decimal places).
pub const USDC_SCALE: u8 = 6;

/// Conventional scale for SOL (9 decimal places: lamports).
pub const SOL_SCALE: u8 = 9;

/// A 128-bit signed fixed-point decimal.
///
/// ```text
/// value = mantissa × 10^(-scale)
/// ```
///
/// - `mantissa`: signed coefficient stored as `i128`
/// - `scale`: number of decimal places in `[0, MAX_SCALE]`
///
/// On-chain Borsh encoding: 17 bytes (16-byte LE mantissa + 1-byte scale).
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Decimal {
    pub(crate) mantissa: i128,
    pub(crate) scale: u8,
}

impl Decimal {
    /// Additive identity: `0 × 10^0`.
    pub const ZERO: Self = Self { mantissa: 0, scale: 0 };

    /// Multiplicative identity: `1 × 10^0`.
    pub const ONE: Self = Self { mantissa: 1, scale: 0 };

    /// Maximum representable value: `i128::MAX × 10^0`.
    pub const MAX: Self = Self { mantissa: i128::MAX, scale: 0 };

    /// Minimum representable value: `i128::MIN × 10^0`.
    pub const MIN: Self = Self { mantissa: i128::MIN, scale: 0 };

    /// Construct a `Decimal` from a raw mantissa and scale.
    ///
    /// Returns `Err(ArithmeticError::ScaleExceeded)` if `scale > MAX_SCALE`.
    #[inline]
    pub fn new(mantissa: i128, scale: u8) -> Result<Self, ArithmeticError> {
        if scale > MAX_SCALE {
            return Err(ArithmeticError::ScaleExceeded);
        }
        Ok(Self { mantissa, scale })
    }

    /// Returns the raw mantissa.
    #[inline]
    pub fn mantissa(self) -> i128 {
        self.mantissa
    }

    /// Returns the scale (number of decimal places).
    #[inline]
    pub fn scale(self) -> u8 {
        self.scale
    }

    /// Returns `true` if the value is exactly zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    /// Returns `true` if the value is strictly positive.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.mantissa > 0
    }

    /// Returns `true` if the value is strictly negative.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.mantissa < 0
    }
}
