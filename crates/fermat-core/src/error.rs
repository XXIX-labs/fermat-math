//! Error types for fermat-core arithmetic operations.

/// All errors that can arise from fixed-point arithmetic in fermat-core.
///
/// Every fallible operation returns `Result<_, ArithmeticError>` instead of
/// panicking, satisfying the sBPF requirement for panic-free on-chain programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticError {
    /// The result exceeded the representable range of `i128`.
    Overflow,
    /// The result fell below the representable range of `i128`.
    Underflow,
    /// A division or modulo operation was attempted with a zero denominator.
    DivisionByZero,
    /// A scale value exceeded `MAX_SCALE` (28) was provided or computed.
    ScaleExceeded,
    /// The input could not be parsed or is otherwise malformed (e.g. bad string).
    InvalidInput,
}

impl core::fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArithmeticError::Overflow      => f.write_str("arithmetic overflow"),
            ArithmeticError::Underflow     => f.write_str("arithmetic underflow"),
            ArithmeticError::DivisionByZero => f.write_str("division by zero"),
            ArithmeticError::ScaleExceeded => f.write_str("scale exceeds maximum (28)"),
            ArithmeticError::InvalidInput  => f.write_str("invalid input"),
        }
    }
}
