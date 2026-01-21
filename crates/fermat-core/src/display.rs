//! `Display` and `Debug` formatting for `Decimal`.
//!
//! `Display` renders the value as a human-readable decimal string, padding
//! fractional zeros where necessary:
//!
//! ```text
//! Decimal { mantissa: 1_500_000, scale: 6 }  →  "1.500000"
//! Decimal { mantissa: -42, scale: 0 }        →  "-42"
//! Decimal { mantissa: 5, scale: 2 }          →  "0.05"
//! ```
//!
//! `Debug` renders the internal representation for diagnostic use:
//! ```text
//! Decimal { mantissa: 1500000, scale: 6 }
//! ```

use crate::arithmetic::POW10;
use crate::decimal::Decimal;
use core::fmt;

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{}", self.mantissa);
        }

        let abs_mantissa = self.mantissa.unsigned_abs(); // u128
        let scale = self.scale as usize;
        let factor = POW10[scale] as u128;

        let int_part = abs_mantissa / factor;
        let frac_part = abs_mantissa % factor;

        if self.mantissa < 0 {
            f.write_str("-")?;
        }

        write!(f, "{}", int_part)?;
        write!(f, ".")?;
        // Pad fractional part with leading zeros to reach `scale` digits
        write!(f, "{:0>width$}", frac_part, width = scale)?;

        Ok(())
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Decimal {{ mantissa: {}, scale: {} }}",
            self.mantissa, self.scale
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::decimal::Decimal;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    fn fmt(d: Decimal) -> alloc::string::String {
        alloc::format!("{}", d)
    }

    extern crate alloc;

    #[test]
    fn display_integer() {
        assert_eq!(fmt(d(42, 0)), "42");
    }

    #[test]
    fn display_negative_integer() {
        assert_eq!(fmt(d(-42, 0)), "-42");
    }

    #[test]
    fn display_simple_decimal() {
        assert_eq!(fmt(d(123, 2)), "1.23");
    }

    #[test]
    fn display_leading_zeros_in_frac() {
        // 0.05 → mantissa=5, scale=2
        assert_eq!(fmt(d(5, 2)), "0.05");
    }

    #[test]
    fn display_negative_decimal() {
        assert_eq!(fmt(d(-1_500_000, 6)), "-1.500000");
    }

    #[test]
    fn display_usdc_amount() {
        // 1.500000 USDC
        assert_eq!(fmt(d(1_500_000, 6)), "1.500000");
    }

    #[test]
    fn display_zero() {
        assert_eq!(fmt(Decimal::ZERO), "0");
    }

    #[test]
    fn debug_format() {
        let s = alloc::format!("{:?}", d(42, 2));
        assert!(s.contains("mantissa: 42"));
        assert!(s.contains("scale: 2"));
    }
}
