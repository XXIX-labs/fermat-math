//! IEEE 754-2008 rounding modes — placeholder.

/// Rounding mode selector (7 modes per IEEE 754-2008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RoundingMode {
    /// Round toward negative infinity.
    Down,
    /// Round toward positive infinity.
    Up,
    /// Round toward zero (truncate).
    TowardZero,
    /// Round away from zero.
    AwayFromZero,
    /// Round half toward positive infinity ("school" rounding).
    HalfUp,
    /// Round half toward negative infinity.
    HalfDown,
    /// Round half to even (banker's rounding) — default.
    #[default]
    HalfEven,
}
