//! Determinism tests for fermat-core.
//!
//! These tests verify that arithmetic operations produce identical bit-for-bit
//! results regardless of the order in which they are computed. This is critical
//! for on-chain consensus: all validators must reach the same result.
//!
//! Each test runs the same sequence N times in potentially different orderings
//! and asserts all results are identical.

use fermat_core::{Decimal, RoundingMode};

fn d(m: i128, s: u8) -> Decimal {
    Decimal::new(m, s).unwrap()
}

// ─── Repeated computation ─────────────────────────────────────────────────────

#[test]
fn add_is_deterministic() {
    let a = d(150_000_000, 6);
    let b = d(2_500_000, 6);
    let first = a.checked_add(b).unwrap();
    for _ in 0..100 {
        assert_eq!(a.checked_add(b).unwrap(), first);
    }
}

#[test]
fn sub_is_deterministic() {
    let a = d(150_000_000, 6);
    let b = d(2_500_000, 6);
    let first = a.checked_sub(b).unwrap();
    for _ in 0..100 {
        assert_eq!(a.checked_sub(b).unwrap(), first);
    }
}

#[test]
fn mul_is_deterministic() {
    let a = d(150_000_000, 6);
    let b = d(2_500_000, 6);
    let first = a.checked_mul(b).unwrap();
    for _ in 0..100 {
        assert_eq!(a.checked_mul(b).unwrap(), first);
    }
}

#[test]
fn div_is_deterministic() {
    let a = d(150_000_000, 6);
    let b = d(2_500_000, 6);
    let first = a.checked_div(b).unwrap();
    for _ in 0..100 {
        assert_eq!(a.checked_div(b).unwrap(), first);
    }
}

#[test]
fn mul_div_is_deterministic() {
    let a = d(i128::MAX / 4, 0);
    let b = d(3, 0);
    let c = d(4, 0);
    let first = a.checked_mul_div(b, c).unwrap();
    for _ in 0..100 {
        assert_eq!(a.checked_mul_div(b, c).unwrap(), first);
    }
}

#[test]
fn round_half_even_is_deterministic() {
    let a = d(1_234_567_890, 9);
    let first = a.round(6, RoundingMode::HalfEven).unwrap();
    for _ in 0..100 {
        assert_eq!(a.round(6, RoundingMode::HalfEven).unwrap(), first);
    }
}

// ─── Order independence (where commutativity holds) ───────────────────────────

#[test]
fn add_order_independence() {
    // (a + b) + c == (c + b) + a when no overflow
    let a = d(100_000, 6);
    let b = d(200_000, 6);
    let c = d(300_000, 6);

    let lhs = a.checked_add(b).unwrap().checked_add(c).unwrap();
    let rhs = c.checked_add(b).unwrap().checked_add(a).unwrap();
    assert_eq!(lhs, rhs);
}

#[test]
fn mul_order_independence() {
    // (a * b) * c and (c * b) * a may differ in scale but should have same value
    // Use scale-0 operands to keep scale constant.
    let a = d(7, 0);
    let b = d(11, 0);
    let c = d(13, 0);

    let lhs = a.checked_mul(b).unwrap().checked_mul(c).unwrap();
    let rhs = c.checked_mul(b).unwrap().checked_mul(a).unwrap();
    assert_eq!(lhs, rhs);
}

// ─── Chained operation stability ─────────────────────────────────────────────

#[test]
fn chained_interest_accrual_is_stable() {
    // Simulate 12 monthly interest accruals: index should end up near 1.05 for 5% APR.
    let mut index = d(1_000_000, 6); // 1.000000
                                     // 5% APR / 12 months = 0.4167% per month = 0.004167
    let monthly_rate = d(4_167, 6); // 0.004167

    for _ in 0..12 {
        let interest = index.checked_mul(monthly_rate).unwrap();
        index = index.checked_add(interest).unwrap();
        index = index.round(6, RoundingMode::HalfEven).unwrap();
    }

    // After 12 months at 5% APR: ~1.0511619 (compound)
    // Truncated to 1 integer digit = 1
    assert_eq!(index.to_i128_truncated(), 1);
    // mantissa should be around 1_051_000..1_052_000 (varies slightly by rounding)
    assert!(index.mantissa() > 1_050_000);
    assert!(index.mantissa() < 1_053_000);
}

#[test]
fn repeated_rescale_is_idempotent() {
    let a = d(1_500_000, 6);
    let rescaled = a.rescale_up(6).unwrap(); // no-op: already scale 6
    assert_eq!(rescaled, a);
}
