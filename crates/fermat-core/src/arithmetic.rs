//! Arithmetic operations: add, sub, mul, div, mul_div, neg, abs.
//!
//! # Scale Alignment
//!
//! Before addition/subtraction, both operands are scaled to the same number of
//! decimal places (`max(a.scale, b.scale)`) via `align_scales`. This can itself
//! overflow if the mantissa is near `i128::MAX`; that is reported as
//! `ArithmeticError::Overflow`.
//!
//! # mul_div Safety
//!
//! `checked_mul_div` uses a 256-bit intermediate (`U256`) for `(a × b) / c` so
//! that the product does not silently wrap. See [`U256`] for the implementation.

use crate::decimal::{Decimal, MAX_SCALE};
use crate::error::ArithmeticError;

// ─── Powers of 10 table ──────────────────────────────────────────────────────

/// Pre-computed powers of 10 from `10^0` to `10^28`.
///
/// Using a lookup instead of `checked_pow` eliminates runtime loops and
/// guarantees O(1) access — important for CU budget on sBPF.
pub(crate) const POW10: [i128; 29] = [
    1,                                      // 10^0
    10,                                     // 10^1
    100,                                    // 10^2
    1_000,                                  // 10^3
    10_000,                                 // 10^4
    100_000,                                // 10^5
    1_000_000,                              // 10^6
    10_000_000,                             // 10^7
    100_000_000,                            // 10^8
    1_000_000_000,                          // 10^9
    10_000_000_000,                         // 10^10
    100_000_000_000,                        // 10^11
    1_000_000_000_000,                      // 10^12
    10_000_000_000_000,                     // 10^13
    100_000_000_000_000,                    // 10^14
    1_000_000_000_000_000,                  // 10^15
    10_000_000_000_000_000,                 // 10^16
    100_000_000_000_000_000,                // 10^17
    1_000_000_000_000_000_000,              // 10^18
    10_000_000_000_000_000_000,             // 10^19
    100_000_000_000_000_000_000,            // 10^20
    1_000_000_000_000_000_000_000,          // 10^21
    10_000_000_000_000_000_000_000,         // 10^22
    100_000_000_000_000_000_000_000,        // 10^23
    1_000_000_000_000_000_000_000_000,      // 10^24
    10_000_000_000_000_000_000_000_000,     // 10^25
    100_000_000_000_000_000_000_000_000,    // 10^26
    1_000_000_000_000_000_000_000_000_000,  // 10^27
    10_000_000_000_000_000_000_000_000_000, // 10^28
];

/// Return `10^exp` as `i128` or `Err(ScaleExceeded)` if `exp > MAX_SCALE`.
#[inline]
pub(crate) fn pow10(exp: u8) -> Result<i128, ArithmeticError> {
    POW10
        .get(exp as usize)
        .copied()
        .ok_or(ArithmeticError::ScaleExceeded)
}

// ─── Scale alignment ─────────────────────────────────────────────────────────

/// Align two operands to a common scale (`max(a.scale, b.scale)`).
///
/// Returns `(a_mantissa, b_mantissa, common_scale)`.
/// Fails with `Overflow` if multiplying to scale up overflows `i128`.
#[inline]
pub(crate) fn align_scales(a: Decimal, b: Decimal) -> Result<(i128, i128, u8), ArithmeticError> {
    use core::cmp::Ordering;
    match a.scale.cmp(&b.scale) {
        Ordering::Equal => Ok((a.mantissa, b.mantissa, a.scale)),
        Ordering::Less => {
            let diff = b.scale - a.scale;
            let factor = pow10(diff)?;
            let scaled = a
                .mantissa
                .checked_mul(factor)
                .ok_or(ArithmeticError::Overflow)?;
            Ok((scaled, b.mantissa, b.scale))
        }
        Ordering::Greater => {
            let diff = a.scale - b.scale;
            let factor = pow10(diff)?;
            let scaled = b
                .mantissa
                .checked_mul(factor)
                .ok_or(ArithmeticError::Overflow)?;
            Ok((a.mantissa, scaled, a.scale))
        }
    }
}

// ─── Sign helper for mul_div ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub(crate) enum Sign {
    Positive,
    Negative,
    Zero,
}

/// Compute the sign of `a * b / c` given their `i128` values.
#[inline]
pub(crate) fn sign3(a: i128, b: i128, c: i128) -> Sign {
    if a == 0 || b == 0 {
        return Sign::Zero;
    }
    let neg_a = a < 0;
    let neg_b = b < 0;
    let neg_c = c < 0;
    let negative = (neg_a ^ neg_b) ^ neg_c;
    if negative {
        Sign::Negative
    } else {
        Sign::Positive
    }
}

// ─── 256-bit unsigned integer ─────────────────────────────────────────────────

/// 256-bit unsigned integer represented as two 128-bit limbs.
///
/// Used exclusively as an intermediate type in [`Decimal::checked_mul_div`] to
/// prevent the product `a × b` from silently overflowing `i128` / `u128`.
///
/// Layout: `value = hi * 2^128 + lo`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct U256 {
    /// Least-significant 128 bits.
    pub lo: u128,
    /// Most-significant 128 bits.
    pub hi: u128,
}

impl U256 {
    #[cfg(test)]
    pub const ZERO: Self = Self { lo: 0, hi: 0 };

    /// Exact 128 × 128 → 256-bit multiplication using four 64-bit limbs.
    ///
    /// ```text
    /// a = a_hi * 2^64 + a_lo
    /// b = b_hi * 2^64 + b_lo
    /// a*b = hh * 2^128 + mid * 2^64 + ll
    ///     where mid = a_hi*b_lo + a_lo*b_hi
    /// ```
    pub fn mul(a: u128, b: u128) -> Self {
        const MASK64: u128 = u64::MAX as u128;
        let a_lo = a & MASK64;
        let a_hi = a >> 64;
        let b_lo = b & MASK64;
        let b_hi = b >> 64;

        let ll = a_lo * b_lo;
        let lh = a_lo * b_hi;
        let hl = a_hi * b_lo;
        let hh = a_hi * b_hi;

        let (mid, mid_carry) = lh.overflowing_add(hl);
        let (lo, lo_carry) = ll.overflowing_add(mid << 64);
        let hi = hh
            .wrapping_add(mid >> 64)
            .wrapping_add(if mid_carry { 1u128 << 64 } else { 0 })
            .wrapping_add(lo_carry as u128);

        U256 { lo, hi }
    }

    /// 256-bit / 128-bit → `(quotient: u128, remainder: u128)`.
    ///
    /// Returns `None` if `d == 0` or `self.hi >= d` (quotient exceeds `u128`).
    ///
    /// ## Algorithm selection
    ///
    /// - **hi == 0**: simple 128-bit division (O(1)).
    /// - **d ≤ u64::MAX**: four-phase 64-bit long-division (O(1), fast path for
    ///   all realistic financial values where `d` is a token amount or scale factor).
    /// - **d > u64::MAX**: binary long-division over 256 bits (O(256), always
    ///   correct; rarely reached in DeFi contexts).
    pub fn checked_div(self, d: u128) -> Option<(u128, u128)> {
        if d == 0 {
            return None;
        }
        // Fast path: numerator fits in 128 bits
        if self.hi == 0 {
            return Some((self.lo / d, self.lo % d));
        }
        // Quotient overflow guard
        if self.hi >= d {
            return None;
        }

        // ── Fast path: d fits in 64 bits ─────────────────────────────────────
        // The four-phase algorithm computes (r * 2^64 + digit) / d in each phase.
        // This is safe iff (r * 2^64) doesn't overflow u128, i.e., r < 2^64.
        // Since r < d and d ≤ 2^64, r < 2^64. ✓
        if d <= u64::MAX as u128 {
            const HALF: u128 = 1u128 << 64;
            const MASK: u128 = HALF - 1;

            let hi_hi = self.hi >> 64;
            let hi_lo = self.hi & MASK;
            let lo_hi = self.lo >> 64;
            let lo_lo = self.lo & MASK;

            let r_a = hi_hi % d;
            let q_a = hi_hi / d;

            let n_b = r_a * HALF + hi_lo;
            let q_b = n_b / d;
            let r_b = n_b % d;

            let n_c = r_b * HALF + lo_hi;
            let q_c = n_c / d;
            let r_c = n_c % d;

            let n_d = r_c * HALF + lo_lo;
            let q_d = n_d / d;
            let r_d = n_d % d;

            if q_a != 0 || q_b != 0 {
                return None; // quotient > u128::MAX
            }

            return Some((q_c * HALF + q_d, r_d));
        }

        // ── General case: d > 2^64, binary long-division ─────────────────────
        //
        // Processes all 256 bits of the numerator from MSB to LSB.
        // Maintains invariant: r < d at the end of every iteration.
        //
        // When `r_hi` (the overflow bit from `r << 1`) is set, the actual
        // remainder is `2^128 + r_new`, which is guaranteed to be ≥ d and
        // < 2d (proved from r < d before shift). The wrapping subtraction
        // `r_new.wrapping_sub(d)` correctly computes `2^128 + r_new - d`.
        let mut q: u128 = 0;
        let mut r: u128 = 0;

        for i in (0..256_u32).rev() {
            let bit: u128 = if i >= 128 {
                (self.hi >> (i - 128)) & 1
            } else {
                (self.lo >> i) & 1
            };

            let r_hi = r >> 127; // top bit of r (will overflow into bit 128 after shift)
            let r_new = (r << 1) | bit;

            if r_hi == 1 {
                // Actual value is 2^128 + r_new; it must be ≥ d (and < 2d).
                // wrapping_sub gives (2^128 + r_new - d) mod 2^128 = correct result.
                r = r_new.wrapping_sub(d);
                if i < 128 {
                    q |= 1u128 << i;
                }
            } else if r_new >= d {
                r = r_new - d;
                if i < 128 {
                    q |= 1u128 << i;
                }
            } else {
                r = r_new;
            }
        }

        Some((q, r))
    }
}

// ─── Decimal: mul, div, neg, abs, mul_div ────────────────────────────────────

impl Decimal {
    /// Checked addition. Aligns scales then adds mantissas.
    pub fn checked_add(self, rhs: Decimal) -> Result<Decimal, ArithmeticError> {
        let (a, b, scale) = align_scales(self, rhs)?;
        let mantissa = a.checked_add(b).ok_or(ArithmeticError::Overflow)?;
        Decimal::new(mantissa, scale)
    }

    /// Checked subtraction. Aligns scales then subtracts mantissas.
    pub fn checked_sub(self, rhs: Decimal) -> Result<Decimal, ArithmeticError> {
        let (a, b, scale) = align_scales(self, rhs)?;
        let mantissa = a.checked_sub(b).ok_or(ArithmeticError::Overflow)?;
        Decimal::new(mantissa, scale)
    }

    /// Checked multiplication: `self * rhs`.
    ///
    /// Result scale = `self.scale + rhs.scale`.
    /// Returns `Err(ScaleExceeded)` if that exceeds `MAX_SCALE`.
    pub fn checked_mul(self, rhs: Decimal) -> Result<Decimal, ArithmeticError> {
        let new_scale = self
            .scale
            .checked_add(rhs.scale)
            .filter(|&s| s <= MAX_SCALE)
            .ok_or(ArithmeticError::ScaleExceeded)?;
        let mantissa = self
            .mantissa
            .checked_mul(rhs.mantissa)
            .ok_or(ArithmeticError::Overflow)?;
        Decimal::new(mantissa, new_scale)
    }

    /// Checked division: `self / rhs`.
    ///
    /// Scales the numerator up by `MAX_SCALE - self.scale` places before
    /// dividing to retain maximum precision.
    pub fn checked_div(self, rhs: Decimal) -> Result<Decimal, ArithmeticError> {
        if rhs.mantissa == 0 {
            return Err(ArithmeticError::DivisionByZero);
        }
        let extra = MAX_SCALE.saturating_sub(self.scale);
        let factor = pow10(extra)?;
        let scaled_num = self
            .mantissa
            .checked_mul(factor)
            .ok_or(ArithmeticError::Overflow)?;
        let mantissa = scaled_num
            .checked_div(rhs.mantissa)
            .ok_or(ArithmeticError::Overflow)?;
        let raw_scale = (self.scale as i32) + (extra as i32) - (rhs.scale as i32);
        if raw_scale < 0 {
            return Err(ArithmeticError::Underflow);
        }
        Decimal::new(mantissa, (raw_scale as u8).min(MAX_SCALE))
    }

    /// Negation: returns `-self`.
    ///
    /// Fails with `Err(Overflow)` for `Decimal::MIN` (two's-complement has no
    /// positive counterpart for `i128::MIN`).
    pub fn checked_neg(self) -> Result<Decimal, ArithmeticError> {
        let mantissa = self
            .mantissa
            .checked_neg()
            .ok_or(ArithmeticError::Overflow)?;
        Decimal::new(mantissa, self.scale)
    }

    /// Absolute value: returns `|self|`.
    ///
    /// Fails with `Err(Overflow)` for `Decimal::MIN`.
    pub fn checked_abs(self) -> Result<Decimal, ArithmeticError> {
        if self.mantissa >= 0 {
            return Ok(self);
        }
        self.checked_neg()
    }

    /// Compound `(self × numerator) / denominator` with 256-bit intermediate.
    ///
    /// Prevents silent overflow that occurs when `self × numerator` exceeds
    /// `i128::MAX` in a naive two-step `mul` then `div`.
    pub fn checked_mul_div(
        self,
        numerator: Decimal,
        denominator: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        if denominator.mantissa == 0 {
            return Err(ArithmeticError::DivisionByZero);
        }

        let sign = sign3(self.mantissa, numerator.mantissa, denominator.mantissa);

        let a = self.mantissa.unsigned_abs();
        let b = numerator.mantissa.unsigned_abs();
        let c = denominator.mantissa.unsigned_abs();

        let product = U256::mul(a, b);
        let (quotient_u128, _rem) = product.checked_div(c).ok_or(ArithmeticError::Overflow)?;

        let mantissa_abs = i128::try_from(quotient_u128).map_err(|_| ArithmeticError::Overflow)?;

        let signed_mantissa = match sign {
            Sign::Zero => 0i128,
            Sign::Positive => mantissa_abs,
            Sign::Negative => mantissa_abs
                .checked_neg()
                .ok_or(ArithmeticError::Overflow)?,
        };

        let num_scale = self.scale as i32 + numerator.scale as i32;
        let den_scale = denominator.scale as i32;
        let result_scale = num_scale - den_scale;
        if result_scale < 0 || result_scale > MAX_SCALE as i32 {
            return Err(ArithmeticError::ScaleExceeded);
        }

        Decimal::new(signed_mantissa, result_scale as u8)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow10_table_spot_checks() {
        assert_eq!(pow10(0).unwrap(), 1);
        assert_eq!(pow10(6).unwrap(), 1_000_000);
        assert_eq!(pow10(18).unwrap(), 1_000_000_000_000_000_000);
        assert_eq!(
            pow10(28).unwrap(),
            10_000_000_000_000_000_000_000_000_000i128
        );
        assert!(pow10(29).is_err());
    }

    #[test]
    fn u256_mul_small() {
        assert_eq!(U256::mul(3, 7), U256 { lo: 21, hi: 0 });
    }

    #[test]
    fn u256_mul_max_times_max() {
        let r = U256::mul(u128::MAX, u128::MAX);
        assert_eq!(r.lo, 1);
        assert_eq!(r.hi, u128::MAX - 1);
    }

    #[test]
    fn u256_div_basic() {
        assert_eq!(U256 { lo: 21, hi: 0 }.checked_div(7), Some((3, 0)));
    }

    #[test]
    fn u256_div_by_zero() {
        assert_eq!(U256::ZERO.checked_div(0), None);
    }

    #[test]
    fn u256_div_overflow_check() {
        assert_eq!(U256 { lo: 0, hi: 100 }.checked_div(50), None);
    }
}
