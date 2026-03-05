//! Comprehensive rounding tests: 7 modes × multiple scenarios each,
//! plus edge cases and integration with token amount conversions.

#[cfg(test)]
mod rounding_tests {
    use crate::decimal::Decimal;
    use crate::rounding::RoundingMode;

    fn d(m: i128, s: u8) -> Decimal {
        Decimal::new(m, s).unwrap()
    }

    // Helper: round d(m,s) to dp places with given mode
    fn r(m: i128, s: u8, dp: u8, mode: RoundingMode) -> i128 {
        d(m, s).round(dp, mode).unwrap().mantissa()
    }

    // ══ TowardZero ════════════════════════════════════════════════════════════

    #[test]
    fn toward_zero_positive_above() {
        assert_eq!(r(19, 1, 0, RoundingMode::TowardZero), 1);
    }

    #[test]
    fn toward_zero_positive_below() {
        assert_eq!(r(11, 1, 0, RoundingMode::TowardZero), 1);
    }

    #[test]
    fn toward_zero_negative_above() {
        assert_eq!(r(-11, 1, 0, RoundingMode::TowardZero), -1);
    }

    #[test]
    fn toward_zero_negative_below() {
        assert_eq!(r(-19, 1, 0, RoundingMode::TowardZero), -1);
    }

    #[test]
    fn toward_zero_at_midpoint() {
        assert_eq!(r(15, 1, 0, RoundingMode::TowardZero), 1);
    }

    #[test]
    fn toward_zero_exact() {
        assert_eq!(r(20, 1, 0, RoundingMode::TowardZero), 2);
    }

    #[test]
    fn toward_zero_multi_dp() {
        // 1.234 → 1.2 (truncate to 1 dp)
        assert_eq!(r(1234, 3, 1, RoundingMode::TowardZero), 12);
    }

    // ══ AwayFromZero ══════════════════════════════════════════════════════════

    #[test]
    fn away_from_zero_positive() {
        assert_eq!(r(11, 1, 0, RoundingMode::AwayFromZero), 2);
    }

    #[test]
    fn away_from_zero_negative() {
        assert_eq!(r(-11, 1, 0, RoundingMode::AwayFromZero), -2);
    }

    #[test]
    fn away_from_zero_exact_no_change() {
        assert_eq!(r(20, 1, 0, RoundingMode::AwayFromZero), 2);
    }

    #[test]
    fn away_from_zero_at_midpoint_pos() {
        assert_eq!(r(15, 1, 0, RoundingMode::AwayFromZero), 2);
    }

    #[test]
    fn away_from_zero_at_midpoint_neg() {
        assert_eq!(r(-15, 1, 0, RoundingMode::AwayFromZero), -2);
    }

    #[test]
    fn away_from_zero_small_frac() {
        // 0.001 → 1 (away from zero rounds up even tiny fractions)
        assert_eq!(r(1, 3, 0, RoundingMode::AwayFromZero), 1);
    }

    #[test]
    fn away_from_zero_small_frac_neg() {
        assert_eq!(r(-1, 3, 0, RoundingMode::AwayFromZero), -1);
    }

    // ══ Down (floor) ══════════════════════════════════════════════════════════

    #[test]
    fn down_positive() {
        assert_eq!(r(19, 1, 0, RoundingMode::Down), 1);
    }

    #[test]
    fn down_negative_rounds_further_from_zero() {
        assert_eq!(r(-11, 1, 0, RoundingMode::Down), -2);
    }

    #[test]
    fn down_negative_big_frac() {
        assert_eq!(r(-99, 1, 0, RoundingMode::Down), -10);
    }

    #[test]
    fn down_exact() {
        assert_eq!(r(20, 1, 0, RoundingMode::Down), 2);
    }

    #[test]
    fn down_at_midpoint_pos() {
        assert_eq!(r(15, 1, 0, RoundingMode::Down), 1);
    }

    #[test]
    fn down_at_midpoint_neg() {
        assert_eq!(r(-15, 1, 0, RoundingMode::Down), -2);
    }

    #[test]
    fn down_multi_dp() {
        // 1.999 → 1.9 (down)
        assert_eq!(r(1999, 3, 1, RoundingMode::Down), 19);
    }

    // ══ Up (ceiling) ══════════════════════════════════════════════════════════

    #[test]
    fn up_positive_rounds_up() {
        assert_eq!(r(11, 1, 0, RoundingMode::Up), 2);
    }

    #[test]
    fn up_negative_rounds_toward_zero() {
        assert_eq!(r(-11, 1, 0, RoundingMode::Up), -1);
    }

    #[test]
    fn up_exact_no_change() {
        assert_eq!(r(20, 1, 0, RoundingMode::Up), 2);
    }

    #[test]
    fn up_at_midpoint_pos() {
        assert_eq!(r(15, 1, 0, RoundingMode::Up), 2);
    }

    #[test]
    fn up_at_midpoint_neg() {
        assert_eq!(r(-15, 1, 0, RoundingMode::Up), -1);
    }

    #[test]
    fn up_small_fraction() {
        // 1.001 → 2 (ceiling)
        assert_eq!(r(1001, 3, 0, RoundingMode::Up), 2);
    }

    #[test]
    fn up_small_fraction_neg() {
        // -1.001 → -1 (ceiling)
        assert_eq!(r(-1001, 3, 0, RoundingMode::Up), -1);
    }

    // ══ HalfUp ════════════════════════════════════════════════════════════════

    #[test]
    fn half_up_below_mid_positive() {
        assert_eq!(r(14, 1, 0, RoundingMode::HalfUp), 1);
    }

    #[test]
    fn half_up_at_mid_positive() {
        assert_eq!(r(15, 1, 0, RoundingMode::HalfUp), 2);
    }

    #[test]
    fn half_up_above_mid_positive() {
        assert_eq!(r(16, 1, 0, RoundingMode::HalfUp), 2);
    }

    #[test]
    fn half_up_below_mid_negative() {
        // -1.4 → -1
        assert_eq!(r(-14, 1, 0, RoundingMode::HalfUp), -1);
    }

    #[test]
    fn half_up_at_mid_negative() {
        // -1.5 → -2 (HalfUp rounds away from zero for both positive and negative)
        assert_eq!(r(-15, 1, 0, RoundingMode::HalfUp), -2);
    }

    #[test]
    fn half_up_above_mid_negative() {
        assert_eq!(r(-16, 1, 0, RoundingMode::HalfUp), -2);
    }

    #[test]
    fn half_up_two_dp() {
        // 1.235 → 1.24 (round up at midpoint)
        assert_eq!(r(1235, 3, 2, RoundingMode::HalfUp), 124);
    }

    // ══ HalfDown ══════════════════════════════════════════════════════════════

    #[test]
    fn half_down_below_mid() {
        assert_eq!(r(14, 1, 0, RoundingMode::HalfDown), 1);
    }

    #[test]
    fn half_down_at_mid_positive() {
        // 1.5 → 1 (round down at midpoint)
        assert_eq!(r(15, 1, 0, RoundingMode::HalfDown), 1);
    }

    #[test]
    fn half_down_above_mid_positive() {
        assert_eq!(r(16, 1, 0, RoundingMode::HalfDown), 2);
    }

    #[test]
    fn half_down_at_mid_negative() {
        // -1.5 → -1 (HalfDown rounds toward zero: the half-integer is dropped toward 0)
        assert_eq!(r(-15, 1, 0, RoundingMode::HalfDown), -1);
    }

    #[test]
    fn half_down_at_mid_negative_small() {
        // -0.5 → 0 (HalfDown rounds toward zero)
        assert_eq!(r(-5, 1, 0, RoundingMode::HalfDown), 0);
    }

    #[test]
    fn half_down_above_mid_negative() {
        assert_eq!(r(-16, 1, 0, RoundingMode::HalfDown), -2);
    }

    // ══ HalfEven (banker's) ══════════════════════════════════════════════════

    #[test]
    fn half_even_0_5_rounds_to_0() {
        // 0.5 → nearest even = 0
        assert_eq!(r(5, 1, 0, RoundingMode::HalfEven), 0);
    }

    #[test]
    fn half_even_1_5_rounds_to_2() {
        assert_eq!(r(15, 1, 0, RoundingMode::HalfEven), 2);
    }

    #[test]
    fn half_even_2_5_rounds_to_2() {
        assert_eq!(r(25, 1, 0, RoundingMode::HalfEven), 2);
    }

    #[test]
    fn half_even_3_5_rounds_to_4() {
        assert_eq!(r(35, 1, 0, RoundingMode::HalfEven), 4);
    }

    #[test]
    fn half_even_4_5_rounds_to_4() {
        assert_eq!(r(45, 1, 0, RoundingMode::HalfEven), 4);
    }

    #[test]
    fn half_even_below_midpoint() {
        assert_eq!(r(14, 1, 0, RoundingMode::HalfEven), 1);
    }

    #[test]
    fn half_even_above_midpoint() {
        assert_eq!(r(16, 1, 0, RoundingMode::HalfEven), 2);
    }

    #[test]
    fn half_even_negative_midpoint_to_even() {
        // -2.5 → -2 (nearest even is -2)
        assert_eq!(r(-25, 1, 0, RoundingMode::HalfEven), -2);
    }

    #[test]
    fn half_even_negative_midpoint_to_even_odd() {
        // -1.5 → -2 (nearest even is -2)
        assert_eq!(r(-15, 1, 0, RoundingMode::HalfEven), -2);
    }

    // ══ Rounding integration with token amounts ═══════════════════════════════

    #[test]
    fn token_amount_roundtrip_6dp() {
        let amount = 1_234_567u64;
        let dec = Decimal::from_token_amount(amount, 6).unwrap();
        let back = dec.to_token_amount(6, RoundingMode::HalfEven).unwrap();
        assert_eq!(back, amount);
    }

    #[test]
    fn token_amount_round_down_for_withdrawal() {
        // 1.999999 USDC rounded to 5 dp down = 1.99999 = 199999 units at 5dp
        let dec = Decimal::from_token_amount(1_999_999, 6).unwrap(); // 1.999999
        let out = dec.to_token_amount(5, RoundingMode::Down).unwrap();
        assert_eq!(out, 199_999u64); // truncated, not rounded up
    }

    #[test]
    fn rescale_up_then_round() {
        // Start: 1.5 (scale 1), rescale to 3 dp → 1.500, round to 0 dp HalfEven
        let x = d(15, 1).rescale_up(3).unwrap();
        assert_eq!(x.mantissa(), 1500);
        let rounded = x.round(0, RoundingMode::HalfEven).unwrap();
        assert_eq!(rounded.mantissa(), 2); // 1.5 rounds to 2 (nearest even)
    }

    // ══ Scale boundary tests ══════════════════════════════════════════════════

    #[test]
    fn round_scale_28_to_27() {
        let x = Decimal::new(15, 28).unwrap(); // 1.5 × 10^-28
        let rounded = x.round(27, RoundingMode::HalfEven).unwrap();
        // 1.5 × 10^-28 at scale 27 = ? → 1.5 × 10^-28 / 10 = 0.15 → rounds to 0
        assert_eq!(rounded.mantissa(), 2); // HalfEven: 1.5 at scale 27 = round to 2
    }

    #[test]
    fn round_to_scale_exceeded() {
        assert!(d(1, 0).round(29, RoundingMode::HalfEven).is_err());
    }
}
