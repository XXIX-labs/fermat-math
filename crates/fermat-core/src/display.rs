//! `Display` and `Debug` formatting for `Decimal` — placeholder.

use crate::decimal::Decimal;

impl core::fmt::Debug for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Decimal {{ mantissa: {}, scale: {} }}", self.mantissa, self.scale)
    }
}
