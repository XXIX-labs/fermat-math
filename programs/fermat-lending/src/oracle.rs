//! Pyth oracle price reading and validation.
//!
//! ## Price flow
//!
//! ```text
//! Pyth price account
//!   → load_price_feed_from_account_info   (account ownership check)
//!   → get_price_no_older_than             (staleness check)
//!   → confidence interval check           (reject wide / illiquid markets)
//!   → expo normalisation                  (convert to Decimal, 6 dp USD)
//! ```
//!
//! ## Safety bounds
//!
//! | Check | Limit | Rationale |
//! |---|---|---|
//! | Staleness | ≤ 60 s | Rejects prices from a halted or lagging oracle |
//! | Confidence | ≤ 2% of \|price\| | Rejects illiquid / manipulable markets |
//! | Price sign | > 0 | Negative / zero oracle prices are never valid USD prices |

use anchor_lang::prelude::*;
use fermat_core::Decimal;
use pyth_sdk_solana::state::SolanaPriceAccount;

use crate::instructions::LendingError;

/// Maximum age of a Pyth price before it is considered stale.
pub const MAX_ORACLE_AGE_SECS: u64 = 60;

/// Read and validate a Pyth price, returning a [`Decimal`] with 6 decimal places (USD).
///
/// # Errors
/// - [`LendingError::OracleError`]             — account is not a valid Pyth feed.
/// - [`LendingError::StaleOraclePrice`]        — publish time older than 60 s.
/// - [`LendingError::OracleConfidenceTooWide`] — confidence > 2% of price.
/// - [`LendingError::MathError`]               — exponent adjustment overflows.
pub fn get_validated_price(price_account: &AccountInfo, clock: &Clock) -> Result<Decimal> {
    let feed = SolanaPriceAccount::account_info_to_feed(price_account)
        .map_err(|_| error!(LendingError::OracleError))?;

    let price = feed
        .get_price_no_older_than(clock.unix_timestamp, MAX_ORACLE_AGE_SECS)
        .ok_or_else(|| error!(LendingError::StaleOraclePrice))?;

    // Reject non-positive prices (USD prices are always positive).
    require!(price.price > 0, LendingError::OracleError);

    // Confidence check using u128 to avoid overflow:
    //   conf / |price| ≤ 2 / 100
    //   ⟺  conf × 100 ≤ |price| × 2
    let conf = price.conf as u128;
    let abs_price = price.price.unsigned_abs() as u128;
    require!(
        conf.saturating_mul(100) <= abs_price.saturating_mul(2),
        LendingError::OracleConfidenceTooWide
    );

    // Convert Pyth (price × 10^expo) to Decimal with scale 6.
    //
    //   target_mantissa = price × 10^(expo + 6)   when expo + 6 ≥ 0 (multiply)
    //                   = price / 10^(−expo − 6)  when expo + 6 < 0 (divide, truncates down)
    //
    // Typical Pyth expo is −8 (USD pairs), giving adjust = −2, so we divide by 100.
    let adjust = price.expo + 6_i32;
    let raw = price.price as i128;

    let mantissa: i128 = if adjust >= 0 {
        let mul = pow10(adjust as u32).ok_or_else(|| error!(LendingError::MathError))?;
        raw.checked_mul(mul)
            .ok_or_else(|| error!(LendingError::MathError))?
    } else {
        let div = pow10((-adjust) as u32).ok_or_else(|| error!(LendingError::MathError))?;
        // Integer division truncates toward zero — round-down for positive prices.
        raw.checked_div(div)
            .ok_or_else(|| error!(LendingError::MathError))?
    };

    Decimal::new(mantissa, 6).map_err(|_| error!(LendingError::MathError))
}

/// Lookup table for 10^n, n ∈ [0, 28].
fn pow10(exp: u32) -> Option<i128> {
    #[rustfmt::skip]
    const TABLE: [i128; 29] = [
        1,
        10,
        100,
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
        1_000_000_000,
        10_000_000_000,
        100_000_000_000,
        1_000_000_000_000,
        10_000_000_000_000,
        100_000_000_000_000,
        1_000_000_000_000_000,
        10_000_000_000_000_000,
        100_000_000_000_000_000,
        1_000_000_000_000_000_000,
        10_000_000_000_000_000_000,
        100_000_000_000_000_000_000_i128,
        1_000_000_000_000_000_000_000_i128,
        10_000_000_000_000_000_000_000_i128,
        100_000_000_000_000_000_000_000_i128,
        1_000_000_000_000_000_000_000_000_i128,
        10_000_000_000_000_000_000_000_000_i128,
        100_000_000_000_000_000_000_000_000_i128,
        1_000_000_000_000_000_000_000_000_000_i128,
        10_000_000_000_000_000_000_000_000_000_i128,
    ];
    TABLE.get(exp as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::pow10;

    #[test]
    fn pow10_table_spot_checks() {
        assert_eq!(pow10(0), Some(1));
        assert_eq!(pow10(2), Some(100));
        assert_eq!(pow10(6), Some(1_000_000));
        assert_eq!(pow10(8), Some(100_000_000));
        assert_eq!(pow10(28), Some(10_000_000_000_000_000_000_000_000_000_i128));
        assert_eq!(pow10(29), None);
    }
}
