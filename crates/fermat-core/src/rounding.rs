//! IEEE 754-2008 rounding modes and the `Decimal::round` method.
//!
//! ## Rounding Modes
//!
//! | Mode          | Description                                    | DeFi Use Case              |
//! |---------------|------------------------------------------------|----------------------------|
//! | `Down`        | Toward −∞ (floor)                              | User withdrawals (safe)    |
//! | `Up`          | Toward +∞ (ceiling)                            | Protocol fees (maximize)   |
//! | `TowardZero`  | Truncate (toward 0)                            | Display / read-only        |
//! | `AwayFromZero`| Away from 0 (magnify)                          | Collateral requirements    |
//! | `HalfUp`      | Round half away from zero ("school" rounding)  | Retail calculations        |
//! | `HalfDown`    | Round half toward zero                         | Interest accrual           |
//! | `HalfEven`    | Round half to even digit (banker's rounding)   | Statistical neutrality (default) |

use crate::arithmetic::pow10;
use crate::decimal::Decimal;
use crate::error::ArithmeticError;

/// Rounding mode selector (7 modes per IEEE 754-2008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RoundingMode {
    /// Round toward negative infinity (floor).
    ///
    /// `-1.5` → `-2`, `1.5` → `1`
    Down,

    /// Round toward positive infinity (ceiling).
    ///
    /// `-1.5` → `-1`, `1.5` → `2`
    Up,

    /// Round toward zero (truncate).
    ///
    /// `-1.9` → `-1`, `1.9` → `1`
    TowardZero,

    /// Round away from zero.
    ///
    /// `-1.1` → `-2`, `1.1` → `2`
    AwayFromZero,

    /// Round half away from zero ("school" rounding).
    ///
    /// `0.5` → `1`, `-0.5` → `-1`
    HalfUp,

    /// Round half toward zero.
    ///
    /// `0.5` → `0`, `-0.5` → `0`
    HalfDown,

    /// Round half to nearest even digit (banker's rounding) — **default**.
    ///
    /// `0.5` → `0`, `1.5` → `2`, `2.5` → `2`, `3.5` → `4`
    ///
    /// Chosen as default because it is statistically unbiased across large
    /// numbers of operations — critical for interest accrual and index updates.
    #[default]
    HalfEven,
}

impl Decimal {
    /// Round `self` to `dp` decimal places using the given rounding `mode`.
    ///
    /// If `dp >= self.scale` no rounding is needed and `self` is returned
    /// unchanged (possibly with a different scale representation).
    ///
    /// # Errors
    ///
    /// Returns `Err(ScaleExceeded)` if `dp > MAX_SCALE`.
    pub fn round(self, dp: u8, mode: RoundingMode) -> Result<Self, ArithmeticError> {
        use crate::decimal::MAX_SCALE;
        if dp > MAX_SCALE {
            return Err(ArithmeticError::ScaleExceeded);
        }
        if dp >= self.scale {
            // No precision is lost — just return as-is.
            return Ok(self);
        }

        let diff = self.scale - dp;
        let factor = pow10(diff)?; // 10^diff, always ≥ 10
        let half = factor / 2;

        let quotient = self.mantissa / factor;
        let remainder = self.mantissa % factor; // sign follows dividend

        let abs_rem = remainder.unsigned_abs() as i128; // magnitude of remainder

        let adjusted = match mode {
            RoundingMode::TowardZero => quotient,

            RoundingMode::AwayFromZero => {
                if remainder != 0 {
                    // Move away from zero: add +1 if positive, -1 if negative
                    quotient + quotient.signum().max(1) * remainder.signum()
                } else {
                    quotient
                }
            }

            RoundingMode::Down => {
                // Floor: subtract 1 when the original value was negative AND
                // there is a fractional part (remainder < 0).
                if remainder < 0 {
                    quotient - 1
                } else {
                    quotient
                }
            }

            RoundingMode::Up => {
                // Ceiling: add 1 when the original value was positive AND
                // there is a fractional part (remainder > 0).
                if remainder > 0 {
                    quotient + 1
                } else {
                    quotient
                }
            }

            RoundingMode::HalfUp => {
                if abs_rem >= half {
                    if self.mantissa >= 0 {
                        quotient + 1
                    } else {
                        quotient - 1
                    }
                } else {
                    quotient
                }
            }

            RoundingMode::HalfDown => {
                if abs_rem > half {
                    if self.mantissa >= 0 {
                        quotient + 1
                    } else {
                        quotient - 1
                    }
                } else {
                    quotient
                }
            }

            RoundingMode::HalfEven => {
                if abs_rem > half {
                    // Past the midpoint → always round away
                    if self.mantissa >= 0 {
                        quotient + 1
                    } else {
                        quotient - 1
                    }
                } else if abs_rem == half {
                    // Exactly at midpoint → round to even
                    if quotient % 2 != 0 {
                        if self.mantissa >= 0 {
                            quotient + 1
                        } else {
                            quotient - 1
                        }
                    } else {
                        quotient
                    }
                } else {
                    quotient
                }
            }
        };

        Decimal::new(adjusted, dp)
    }

    /// Rescale `self` to a higher number of decimal places by padding zeros.
    ///
    /// Only increases scale; use `round` to decrease it.
    /// Returns `Err(ScaleExceeded)` if `new_scale > MAX_SCALE` or `Err(Overflow)`
    /// if the mantissa multiplication overflows.
    pub fn rescale_up(self, new_scale: u8) -> Result<Self, ArithmeticError> {
        if new_scale <= self.scale {
            return Ok(self);
        }
        let diff = new_scale - self.scale;
        let factor = pow10(diff)?;
        let mantissa = self
            .mantissa
            .checked_mul(factor)
            .ok_or(ArithmeticError::Overflow)?;
        Decimal::new(mantissa, new_scale)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decimal::Decimal;

    fn d(mantissa: i128, scale: u8) -> Decimal {
        Decimal::new(mantissa, scale).unwrap()
    }

    // ── TowardZero ────────────────────────────────────────────────────────

    #[test]
    fn round_toward_zero_positive() {
        // 1.9 → 1 (truncate)
        assert_eq!(
            d(19, 1).round(0, RoundingMode::TowardZero).unwrap(),
            d(1, 0)
        );
    }

    #[test]
    fn round_toward_zero_negative() {
        // -1.9 → -1 (truncate toward zero)
        assert_eq!(
            d(-19, 1).round(0, RoundingMode::TowardZero).unwrap(),
            d(-1, 0)
        );
    }

    // ── AwayFromZero ──────────────────────────────────────────────────────

    #[test]
    fn round_away_from_zero_positive() {
        assert_eq!(
            d(11, 1).round(0, RoundingMode::AwayFromZero).unwrap(),
            d(2, 0)
        );
    }

    #[test]
    fn round_away_from_zero_negative() {
        assert_eq!(
            d(-11, 1).round(0, RoundingMode::AwayFromZero).unwrap(),
            d(-2, 0)
        );
    }

    #[test]
    fn round_away_from_zero_exact() {
        // 1.0 has no fractional part → unchanged
        assert_eq!(
            d(10, 1).round(0, RoundingMode::AwayFromZero).unwrap(),
            d(1, 0)
        );
    }

    // ── Down (floor) ──────────────────────────────────────────────────────

    #[test]
    fn round_down_positive() {
        assert_eq!(d(19, 1).round(0, RoundingMode::Down).unwrap(), d(1, 0));
    }

    #[test]
    fn round_down_negative() {
        // Floor of -1.9 is -2
        assert_eq!(d(-19, 1).round(0, RoundingMode::Down).unwrap(), d(-2, 0));
    }

    // ── Up (ceiling) ──────────────────────────────────────────────────────

    #[test]
    fn round_up_positive() {
        assert_eq!(d(11, 1).round(0, RoundingMode::Up).unwrap(), d(2, 0));
    }

    #[test]
    fn round_up_negative() {
        // Ceiling of -1.1 is -1
        assert_eq!(d(-11, 1).round(0, RoundingMode::Up).unwrap(), d(-1, 0));
    }

    // ── HalfUp ────────────────────────────────────────────────────────────

    #[test]
    fn round_half_up_at_midpoint() {
        assert_eq!(d(5, 1).round(0, RoundingMode::HalfUp).unwrap(), d(1, 0));
    }

    #[test]
    fn round_half_up_below_midpoint() {
        assert_eq!(d(4, 1).round(0, RoundingMode::HalfUp).unwrap(), d(0, 0));
    }

    #[test]
    fn round_half_up_negative_midpoint() {
        // -0.5 rounds to -1 (HalfUp is away-from-zero)
        assert_eq!(d(-5, 1).round(0, RoundingMode::HalfUp).unwrap(), d(-1, 0));
    }

    // ── HalfDown ──────────────────────────────────────────────────────────

    #[test]
    fn round_half_down_at_midpoint() {
        assert_eq!(d(5, 1).round(0, RoundingMode::HalfDown).unwrap(), d(0, 0));
    }

    #[test]
    fn round_half_down_above_midpoint() {
        assert_eq!(d(6, 1).round(0, RoundingMode::HalfDown).unwrap(), d(1, 0));
    }

    // ── HalfEven (Banker's) ───────────────────────────────────────────────

    #[test]
    fn round_half_even_round_to_even_up() {
        // 1.5 → nearest even = 2
        assert_eq!(d(15, 1).round(0, RoundingMode::HalfEven).unwrap(), d(2, 0));
    }

    #[test]
    fn round_half_even_round_to_even_down() {
        // 2.5 → nearest even = 2
        assert_eq!(d(25, 1).round(0, RoundingMode::HalfEven).unwrap(), d(2, 0));
    }

    #[test]
    fn round_half_even_past_midpoint() {
        // 1.6 → 2 (past midpoint)
        assert_eq!(d(16, 1).round(0, RoundingMode::HalfEven).unwrap(), d(2, 0));
    }

    #[test]
    fn round_no_op_when_dp_equals_scale() {
        let x = d(12345, 3);
        assert_eq!(x.round(3, RoundingMode::HalfEven).unwrap(), x);
    }

    #[test]
    fn round_no_op_when_dp_exceeds_scale() {
        let x = d(12345, 3);
        assert_eq!(x.round(5, RoundingMode::HalfEven).unwrap(), x);
    }

    #[test]
    fn rescale_up_basic() {
        let x = d(1, 0); // 1.0
        let y = x.rescale_up(6).unwrap(); // 1.000000
        assert_eq!(y.mantissa(), 1_000_000);
        assert_eq!(y.scale(), 6);
    }

    #[test]
    fn rescale_up_noop() {
        let x = d(1_000_000, 6);
        assert_eq!(x.rescale_up(3).unwrap(), x); // lower target → no-op
    }
}
