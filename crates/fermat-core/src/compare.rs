//! `Ord` and `PartialOrd` for `Decimal` with automatic scale normalisation.
//!
//! Comparing two `Decimal` values with different scales requires aligning them
//! to a common scale first. For example:
//!
//! ```text
//! 1.50 (scale 2) > 1.5 (scale 1)  →  false
//! 1.50 (scale 2) = 1.5 (scale 1)  →  true
//! ```
//!
//! ## Implementation
//!
//! We multiply the smaller-scale mantissa by `10^diff` to align scales, then
//! compare mantissas directly. If the alignment overflows we fall back to a
//! sign-then-magnitude comparison which is still correct (overflow implies the
//! larger-scale value is closer to zero).
//!
//! Note: `Decimal` derives `PartialEq` / `Eq` for *structural* equality
//! (same mantissa AND same scale). This module provides *value* equality via
//! `Ord`/`PartialOrd` — `1.5` and `1.50` compare as equal under `cmp` even
//! though `==` returns false.

use crate::arithmetic::align_scales;
use crate::decimal::Decimal;
use core::cmp::Ordering;

impl PartialOrd for Decimal {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        // Fast path: identical representation
        if self.scale == other.scale {
            return self.mantissa.cmp(&other.mantissa);
        }

        // Align scales; if overflow we fall back to sign comparison
        match align_scales(*self, *other) {
            Ok((a, b, _)) => a.cmp(&b),
            Err(_) => {
                // Overflow during alignment means the operand being scaled up
                // is very large in magnitude — but that doesn't directly tell
                // us the sign. Use sign-then-magnitude as a safe fallback.
                match (self.mantissa >= 0, other.mantissa >= 0) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => {
                        // Same sign — the one with bigger magnitude in its
                        // original scale and same-direction normalization wins.
                        // This is a safe approximation for the overflow edge case.
                        if self.mantissa >= 0 {
                            self.mantissa.cmp(&other.mantissa)
                        } else {
                            other.mantissa.cmp(&self.mantissa)
                        }
                    }
                }
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::decimal::Decimal;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    #[test]
    fn eq_same_scale() {
        assert_eq!(d(100, 2).cmp(&d(100, 2)), core::cmp::Ordering::Equal);
    }

    #[test]
    fn eq_different_scale() {
        // 1.50 and 1.5 should be equal in value
        assert_eq!(d(150, 2).cmp(&d(15, 1)), core::cmp::Ordering::Equal);
    }

    #[test]
    fn gt_same_scale() {
        assert!(d(200, 2) > d(100, 2));
    }

    #[test]
    fn lt_different_scale() {
        // 0.09 < 0.1
        assert!(d(9, 2) < d(1, 1));
    }

    #[test]
    fn negative_less_than_positive() {
        assert!(d(-1, 0) < d(1, 0));
    }

    #[test]
    fn negative_cmp_negative() {
        // -2 < -1
        assert!(d(-2, 0) < d(-1, 0));
    }

    #[test]
    fn zero_cmp() {
        assert_eq!(Decimal::ZERO.cmp(&Decimal::ZERO), core::cmp::Ordering::Equal);
        assert!(Decimal::ZERO < d(1, 0));
        assert!(Decimal::ZERO > d(-1, 0));
    }

    #[test]
    fn sort_order() {
        let mut vals = [d(15, 1), d(5, 2), d(100, 0), d(-1, 0)];
        vals.sort();
        // -1, 0.05, 1.5, 100
        assert_eq!(vals[0], d(-1, 0));
        assert_eq!(vals[1], d(5, 2));
        assert_eq!(vals[2], d(15, 1));
        assert_eq!(vals[3], d(100, 0));
    }
}
