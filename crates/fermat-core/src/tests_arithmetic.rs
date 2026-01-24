//! Comprehensive arithmetic correctness tests for fermat-core.
//!
//! Covers: checked_add, checked_sub, checked_mul, checked_div,
//!         checked_mul_div, checked_neg, checked_abs, U256 boundary cases.

#[cfg(test)]
mod arithmetic_tests {
    use crate::arithmetic::U256;
    use crate::decimal::Decimal;
    use crate::error::ArithmeticError;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    // ══ checked_add ═══════════════════════════════════════════════════════════

    #[test]
    fn add_same_scale() {
        assert_eq!(d(1, 2).checked_add(d(2, 2)).unwrap(), d(3, 2));
    }

    #[test]
    fn add_different_scales() {
        // 1.0 + 0.05 = 1.05
        assert_eq!(d(10, 1).checked_add(d(5, 2)).unwrap(), d(105, 2));
    }

    #[test]
    fn add_negative() {
        assert_eq!(d(-5, 1).checked_add(d(3, 1)).unwrap(), d(-2, 1));
    }

    #[test]
    fn add_zero_identity() {
        let x = d(12345, 4);
        assert_eq!(x.checked_add(Decimal::ZERO).unwrap(), x);
    }

    #[test]
    fn add_overflow() {
        assert_eq!(
            Decimal::MAX.checked_add(d(1, 0)),
            Err(ArithmeticError::Overflow)
        );
    }

    #[test]
    fn add_opposite_signs_cancel() {
        let x = d(999_999, 6);
        let neg = x.checked_neg().unwrap();
        assert_eq!(x.checked_add(neg).unwrap().mantissa(), 0);
    }

    // ══ checked_sub ═══════════════════════════════════════════════════════════

    #[test]
    fn sub_same_scale() {
        assert_eq!(d(10, 1).checked_sub(d(3, 1)).unwrap(), d(7, 1));
    }

    #[test]
    fn sub_different_scales() {
        // 1.0 - 0.05 = 0.95
        assert_eq!(d(10, 1).checked_sub(d(5, 2)).unwrap(), d(95, 2));
    }

    #[test]
    fn sub_goes_negative() {
        assert_eq!(d(3, 1).checked_sub(d(10, 1)).unwrap(), d(-7, 1));
    }

    #[test]
    fn sub_self_is_zero() {
        let x = d(12345, 4);
        assert_eq!(x.checked_sub(x).unwrap(), d(0, 4));
    }

    #[test]
    fn sub_underflow() {
        assert_eq!(
            Decimal::MIN.checked_sub(d(1, 0)),
            Err(ArithmeticError::Overflow)
        );
    }

    // ══ checked_mul ═══════════════════════════════════════════════════════════

    #[test]
    fn mul_basic() {
        // 1.5 * 2.0 = 3.00
        assert_eq!(d(15, 1).checked_mul(d(20, 1)).unwrap(), d(300, 2));
    }

    #[test]
    fn mul_by_zero() {
        assert_eq!(d(999, 3).checked_mul(Decimal::ZERO).unwrap(), d(0, 3));
    }

    #[test]
    fn mul_by_one() {
        let x = d(12345, 4);
        assert_eq!(x.checked_mul(Decimal::ONE).unwrap(), x);
    }

    #[test]
    fn mul_negative_positive() {
        assert_eq!(d(-3, 0).checked_mul(d(4, 0)).unwrap(), d(-12, 0));
    }

    #[test]
    fn mul_negative_negative() {
        assert_eq!(d(-3, 0).checked_mul(d(-4, 0)).unwrap(), d(12, 0));
    }

    #[test]
    fn mul_scale_overflow() {
        // scale 20 + scale 20 = 40 > MAX_SCALE(28)
        assert_eq!(
            d(1, 20).checked_mul(d(1, 20)),
            Err(ArithmeticError::ScaleExceeded)
        );
    }

    #[test]
    fn mul_mantissa_overflow() {
        assert_eq!(
            Decimal::MAX.checked_mul(d(2, 0)),
            Err(ArithmeticError::Overflow)
        );
    }

    // ══ checked_div ═══════════════════════════════════════════════════════════

    #[test]
    fn div_basic() {
        // 10 / 4 = 2.5 (exact)
        let result = d(10, 0).checked_div(d(4, 0)).unwrap();
        // Result scale will be MAX_SCALE - 0 - 0 = 28
        assert!(result.is_positive());
        // value = mantissa * 10^-28 should equal 2.5
        // mantissa should be 25 * 10^27 = 2.5 * 10^28
        assert_eq!(result.to_i128_truncated(), 2);
    }

    #[test]
    fn div_by_zero() {
        assert_eq!(
            d(1, 0).checked_div(Decimal::ZERO),
            Err(ArithmeticError::DivisionByZero)
        );
    }

    #[test]
    fn div_self_is_near_one() {
        let x = d(12345, 4);
        let result = x.checked_div(x).unwrap();
        // Should be exactly 1.0...00 (truncated to 1)
        assert_eq!(result.to_i128_truncated(), 1);
    }

    #[test]
    fn div_negative_by_positive() {
        let result = d(-10, 0).checked_div(d(4, 0)).unwrap();
        assert!(result.is_negative());
        assert_eq!(result.to_i128_truncated(), -2);
    }

    // ══ checked_neg ═══════════════════════════════════════════════════════════

    #[test]
    fn neg_positive() {
        assert_eq!(d(5, 2).checked_neg().unwrap(), d(-5, 2));
    }

    #[test]
    fn neg_negative() {
        assert_eq!(d(-5, 2).checked_neg().unwrap(), d(5, 2));
    }

    #[test]
    fn neg_zero() {
        assert_eq!(Decimal::ZERO.checked_neg().unwrap(), Decimal::ZERO);
    }

    #[test]
    fn neg_min_overflows() {
        // i128::MIN has no positive counterpart
        assert_eq!(Decimal::MIN.checked_neg(), Err(ArithmeticError::Overflow));
    }

    // ══ checked_abs ═══════════════════════════════════════════════════════════

    #[test]
    fn abs_positive() {
        assert_eq!(d(5, 2).checked_abs().unwrap(), d(5, 2));
    }

    #[test]
    fn abs_negative() {
        assert_eq!(d(-5, 2).checked_abs().unwrap(), d(5, 2));
    }

    #[test]
    fn abs_min_overflows() {
        assert_eq!(Decimal::MIN.checked_abs(), Err(ArithmeticError::Overflow));
    }

    // ══ checked_mul_div ════════════════════════════════════════════════════════

    #[test]
    fn mul_div_basic() {
        // (10 * 3) / 5 = 6
        let result = d(10, 0)
            .checked_mul_div(d(3, 0), d(5, 0))
            .unwrap();
        assert_eq!(result, d(6, 0));
    }

    #[test]
    fn mul_div_zero_numerator() {
        // ZERO * numerator / denominator = 0, with result scale = 0+3-0 = 3
        let result = Decimal::ZERO.checked_mul_div(d(999, 3), d(1, 0)).unwrap();
        assert_eq!(result.mantissa(), 0);
        assert_eq!(result.scale(), 3);
    }

    #[test]
    fn mul_div_div_by_zero() {
        assert_eq!(
            d(1, 0).checked_mul_div(d(1, 0), Decimal::ZERO),
            Err(ArithmeticError::DivisionByZero)
        );
    }

    #[test]
    fn mul_div_prevents_naive_overflow() {
        // Without U256, (i128::MAX * 2) would overflow.
        // With U256 intermediate, this should return Overflow only if the
        // *result* doesn't fit, not because of intermediate.
        // Here result = i128::MAX * 2 / 2 = i128::MAX → fits
        let a = Decimal::new(i128::MAX, 0).unwrap();
        let two = d(2, 0);
        let result = a.checked_mul_div(two, two).unwrap();
        assert_eq!(result, a);
    }

    #[test]
    fn mul_div_negative_result() {
        // (-6 * 3) / 9 = -2
        assert_eq!(
            d(-6, 0).checked_mul_div(d(3, 0), d(9, 0)).unwrap(),
            d(-2, 0)
        );
    }

    #[test]
    fn mul_div_three_negatives() {
        // (-6 * -3) / -9 = -2
        assert_eq!(
            d(-6, 0).checked_mul_div(d(-3, 0), d(-9, 0)).unwrap(),
            d(-2, 0)
        );
    }

    // ══ U256 boundary tests ════════════════════════════════════════════════════

    #[test]
    fn u256_zero_times_max() {
        let r = U256::mul(0, u128::MAX);
        assert_eq!(r, U256::ZERO);
    }

    #[test]
    fn u256_one_times_max() {
        let r = U256::mul(1, u128::MAX);
        assert_eq!(r, U256 { lo: u128::MAX, hi: 0 });
    }

    #[test]
    fn u256_large_div_exact() {
        // 2^128 (= U256 { lo: 0, hi: 1 }) / 2 = 2^127
        let n = U256 { lo: 0, hi: 1 };
        let (q, r) = n.checked_div(2).unwrap();
        assert_eq!(r, 0);
        assert_eq!(q, 1u128 << 127);
    }

    #[test]
    fn u256_div_remainder() {
        // 10 / 3 = 3 r 1
        let n = U256 { lo: 10, hi: 0 };
        assert_eq!(n.checked_div(3), Some((3, 1)));
    }

    #[test]
    fn u256_div_large_divisor() {
        // (u128::MAX * u128::MAX) / u128::MAX = u128::MAX  r 0
        let n = U256::mul(u128::MAX, u128::MAX);
        let (q, r) = n.checked_div(u128::MAX).unwrap();
        assert_eq!(q, u128::MAX);
        assert_eq!(r, 0);
    }

    // ══ Edge cases ═════════════════════════════════════════════════════════════

    #[test]
    fn add_at_scale_28() {
        let x = Decimal::new(1, 28).unwrap();
        let y = Decimal::new(2, 28).unwrap();
        assert_eq!(x.checked_add(y).unwrap(), Decimal::new(3, 28).unwrap());
    }

    #[test]
    fn mul_scale_exactly_max() {
        // scale 14 + scale 14 = 28 = MAX_SCALE → allowed
        let x = Decimal::new(1, 14).unwrap();
        assert!(x.checked_mul(x).is_ok());
    }

    #[test]
    fn mul_scale_one_over_max() {
        // scale 14 + scale 15 = 29 > MAX_SCALE → rejected
        let x = Decimal::new(1, 14).unwrap();
        let y = Decimal::new(1, 15).unwrap();
        assert_eq!(x.checked_mul(y), Err(ArithmeticError::ScaleExceeded));
    }

    #[test]
    fn new_scale_exceeded() {
        assert_eq!(Decimal::new(1, 29), Err(ArithmeticError::ScaleExceeded));
    }
}
