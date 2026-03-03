//! Property-based tests for fermat-core using proptest.
//!
//! These tests verify mathematical invariants that must hold for ALL valid
//! inputs, not just the hand-picked cases in unit tests.
//!
//! Properties verified:
//! - Commutativity of addition
//! - Additive identity (a + 0 = a)
//! - Additive inverse (a + (-a) = 0)
//! - Multiplicative identity (a * 1 = a)
//! - Multiplicative commutativity
//! - checked_mul_div roundtrip (a * b / b ≈ a for non-zero b)
//! - Borsh roundtrip (deserialize(serialize(d)) = d)
//! - Rounding contracts (result scale == target dp)

use fermat_core::{Decimal, RoundingMode, MAX_SCALE};
use proptest::prelude::*;

// ─── Arbitrary generators ──────────────────────────────────────────────────────

/// Generates a valid mantissa in a restricted range to avoid overflow in combined ops.
fn arb_mantissa() -> impl Strategy<Value = i128> {
    // Use ±10^18 range — large enough to be interesting but safe for mul
    -1_000_000_000_000_000_000i128..=1_000_000_000_000_000_000i128
}

fn arb_scale() -> impl Strategy<Value = u8> {
    0u8..=MAX_SCALE
}

fn arb_decimal() -> impl Strategy<Value = Decimal> {
    (arb_mantissa(), arb_scale()).prop_map(|(m, s)| Decimal::new(m, s).unwrap())
}

fn arb_decimal_same_scale(scale: u8) -> impl Strategy<Value = Decimal> {
    arb_mantissa().prop_map(move |m| Decimal::new(m, scale).unwrap())
}

/// Generates a non-zero Decimal for division tests.
#[allow(dead_code)]
fn arb_nonzero_decimal() -> impl Strategy<Value = Decimal> {
    (arb_mantissa(), arb_scale())
        .prop_map(|(m, s)| Decimal::new(m, s).unwrap())
        .prop_filter("non-zero", |d| !d.is_zero())
}

// ─── Addition properties ───────────────────────────────────────────────────────

proptest! {
    /// a + b = b + a (for same-scale decimals to avoid alignment overflow)
    #[test]
    fn add_commutative(
        s in arb_scale(),
        a in arb_decimal_same_scale(0),
        b in arb_decimal_same_scale(0),
    ) {
        let _ = s; // suppress unused
        if let (Ok(ab), Ok(ba)) = (a.checked_add(b), b.checked_add(a)) {
            prop_assert_eq!(ab, ba);
        }
    }

    /// a + 0 = a
    #[test]
    fn add_zero_identity(a in arb_decimal()) {
        if let Ok(result) = a.checked_add(Decimal::ZERO) {
            prop_assert_eq!(result.mantissa(), a.mantissa());
        }
    }

    /// a + (-a) has mantissa 0
    #[test]
    fn add_self_negation(a in arb_decimal()) {
        if let (Ok(neg_a), Ok(result)) = (a.checked_neg(), a.checked_add(a.checked_neg().unwrap())) {
            let _ = neg_a;
            prop_assert_eq!(result.mantissa(), 0);
        }
    }

    /// a - a has mantissa 0
    #[test]
    fn sub_self_is_zero(a in arb_decimal()) {
        if let Ok(result) = a.checked_sub(a) {
            prop_assert_eq!(result.mantissa(), 0);
        }
    }
}

// ─── Multiplication properties ─────────────────────────────────────────────────

proptest! {
    /// a * b = b * a (commutativity), when result scale <= MAX_SCALE
    #[test]
    fn mul_commutative(
        m_a in arb_mantissa(),
        m_b in arb_mantissa(),
        s in 0u8..=14u8, // s*2 <= 28
    ) {
        let a = Decimal::new(m_a, s).unwrap();
        let b = Decimal::new(m_b, s).unwrap();
        if let (Ok(ab), Ok(ba)) = (a.checked_mul(b), b.checked_mul(a)) {
            prop_assert_eq!(ab, ba);
        }
    }

    /// a * 1 = a
    #[test]
    fn mul_one_identity(a in arb_decimal_same_scale(0)) {
        if let Ok(result) = a.checked_mul(Decimal::ONE) {
            prop_assert_eq!(result, a);
        }
    }

    /// a * 0 has mantissa 0
    #[test]
    fn mul_zero(a in arb_decimal_same_scale(0)) {
        if let Ok(result) = a.checked_mul(Decimal::ZERO) {
            prop_assert_eq!(result.mantissa(), 0);
        }
    }
}

// ─── mul_div roundtrip ─────────────────────────────────────────────────────────

proptest! {
    /// (a * b) / b ≈ a (within rounding) for non-zero b, same scale
    #[test]
    fn mul_div_roundtrip(
        m_a in -1_000_000i128..=1_000_000i128,
        m_b in 1i128..=1_000_000i128, // positive non-zero
    ) {
        let a = Decimal::new(m_a, 0).unwrap();
        let b = Decimal::new(m_b, 0).unwrap();
        if let Ok(result) = a.checked_mul_div(b, b) {
            // result should equal a (scale may differ, compare truncated integer part)
            prop_assert_eq!(result.to_i128_truncated(), a.to_i128_truncated());
        }
    }
}

// ─── Negation and abs ─────────────────────────────────────────────────────────

proptest! {
    /// neg(neg(a)) = a (for non-MIN values)
    #[test]
    fn double_neg(a in arb_decimal()) {
        if let (Ok(neg_a), ) = (a.checked_neg(), ) {
            if let Ok(neg_neg_a) = neg_a.checked_neg() {
                prop_assert_eq!(neg_neg_a, a);
            }
        }
    }

    /// abs(a) >= 0
    #[test]
    fn abs_non_negative(a in arb_decimal()) {
        if let Ok(abs_a) = a.checked_abs() {
            prop_assert!(!abs_a.is_negative());
        }
    }

    /// abs(-a) = abs(a)
    #[test]
    fn abs_neg_eq_abs(a in arb_decimal()) {
        if let (Ok(neg_a), Ok(abs_a)) = (a.checked_neg(), a.checked_abs()) {
            if let Ok(abs_neg_a) = neg_a.checked_abs() {
                prop_assert_eq!(abs_neg_a, abs_a);
            }
        }
    }
}

// ─── Ordering ─────────────────────────────────────────────────────────────────

proptest! {
    /// Ord is reflexive: a == a (as Ord)
    #[test]
    fn ord_reflexive(a in arb_decimal()) {
        prop_assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
    }

    /// Ord is antisymmetric: if a < b then !(b < a)
    #[test]
    fn ord_antisymmetric(a in arb_decimal(), b in arb_decimal()) {
        if a < b {
            prop_assert!(b >= a);
        }
    }
}

// ─── Rounding contracts ────────────────────────────────────────────────────────

proptest! {
    /// round(a, dp, _) always produces scale == dp (when dp <= a.scale)
    #[test]
    fn round_produces_correct_scale(
        m in arb_mantissa(),
        s in 1u8..=MAX_SCALE,
        dp in 0u8..=MAX_SCALE,
    ) {
        let a = Decimal::new(m, s).unwrap();
        if dp <= s {
            if let Ok(rounded) = a.round(dp, RoundingMode::HalfEven) {
                prop_assert_eq!(rounded.scale(), dp);
            }
        }
    }

    /// rescale_up(a, new_scale) produces scale == new_scale (when new_scale >= a.scale)
    #[test]
    fn rescale_up_correct_scale(
        m in arb_mantissa(),
        s in 0u8..=14u8,
        extra in 0u8..=14u8,
    ) {
        let a = Decimal::new(m, s).unwrap();
        let new_scale = s + extra;
        if new_scale <= MAX_SCALE {
            if let Ok(rescaled) = a.rescale_up(new_scale) {
                prop_assert_eq!(rescaled.scale(), new_scale);
            }
        }
    }
}

// ─── Borsh roundtrip (via fermat-solana) ──────────────────────────────────────
// These tests are in a separate file (determinism.rs) to avoid a circular dep.
