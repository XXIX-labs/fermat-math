# Fermat Math — POC Product Requirements Document

**Document Type:** Product Requirements Document (PRD)
**Scope:** Pre-Grant Proof of Concept
**Timeline:** 4 weeks
**Author:** Kunal — XXIX Labs (29Projects)
**Contact:** kunal@fermatmath.net
**Website:** https://fermatmath.net
**GitHub:** github.com/xxix-labs/fermat-math
**Date:** March 2026

---

## 1. Purpose

This PRD defines the minimum viable proof of concept for Fermat Math — a precision arithmetic library built from scratch for Solana DeFi programs. The POC will be completed BEFORE submitting the Solana Foundation grant application to demonstrate technical feasibility and execution capability.

The POC exists to answer three questions for the Solana Foundation reviewers:
1. Can a custom 128-bit decimal type compile and execute efficiently on Solana's sBPF runtime?
2. What are the actual compute unit costs vs alternatives (raw u64 math, rust_decimal)?
3. Does a real DeFi use case (lending) work end-to-end with this library on Solana devnet?

---

## 2. What We're Building (and NOT Building)

### Building From Scratch
Fermat Math is a NEW library. We are not forking, wrapping, or adapting any existing codebase. The Dijkstra Keystone project (github.com/dijkstra-keystone/keystone) serves only as a reference for what DeFi math functions are needed — our implementation is original.

### Why From Scratch vs Wrapping rust_decimal
- **CU Optimization**: rust_decimal uses a 96-bit mantissa stored as three u32 words. We use u128 natively, which maps better to sBPF's 64-bit registers and reduces instruction count.
- **Solana-Native**: Built for Borsh serialization from day one (not serde retrofitted). Account data layout is optimized for Solana's rent model.
- **No Panic Guarantee**: rust_decimal's division panics on zero. On Solana, panics lose the full transaction fee. Fermat Math returns Result types for every operation — zero panics by design.
- **Minimal Dependencies**: rust_decimal pulls in serde, num-traits, arrayvec, and optionally libm. Fermat Math targets zero external dependencies for the core crate, minimizing binary size and CU overhead.
- **Grant Credibility**: Building original work is fundamentally different from repackaging existing code. The Solana Foundation funds novel contributions, not wrappers.

---

## 3. Architecture

### 3.1 Core Decimal Type

```
┌──────────────────────────────────────────────┐
│                FermatDecimal                  │
├──────────────────────────────────────────────┤
│  mantissa: i128     (coefficient, signed)    │
│  scale: u8          (decimal places, 0-28)   │
│                                              │
│  Value = mantissa × 10^(-scale)              │
│                                              │
│  Example: 99.99 = { mantissa: 9999,          │
│                      scale: 2 }              │
│                                              │
│  Max precision: 28 significant digits        │
│  Range: ±7.9228162514264337593543950335 ×10²⁸│
│  Size: 17 bytes (i128 + u8)                  │
│  Borsh-serialized: 17 bytes on-chain         │
└──────────────────────────────────────────────┘
```

### 3.2 Why i128 + u8 (not the rust_decimal approach)

rust_decimal stores: `[u32; 3]` mantissa (96-bit) + `u32` flags (sign + scale packed). That's 16 bytes but only 96 bits of mantissa.

Fermat Math stores: `i128` mantissa (128-bit, sign included) + `u8` scale. That's 17 bytes but gives us full 128-bit mantissa, the sign is native to i128 (no bit packing), scale is a clean separate byte, and i128 arithmetic maps directly to sBPF's 64-bit ALU operations (two 64-bit ops per 128-bit op).

### 3.3 Crate Structure (POC Scope)

```
fermat-math/
├── Cargo.toml                    (workspace)
├── crates/
│   ├── fermat-core/              ← POC: Build this
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            (public API)
│   │       ├── decimal.rs        (FermatDecimal type)
│   │       ├── arithmetic.rs     (add, sub, mul, div, mul_div)
│   │       ├── rounding.rs       (7 rounding modes)
│   │       ├── convert.rs        (from/to u64, i64, u128, str)
│   │       ├── compare.rs        (Ord, PartialOrd, Eq)
│   │       ├── display.rs        (Display, Debug formatting)
│   │       └── error.rs          (ArithmeticError type)
│   │
│   └── fermat-solana/            ← POC: Build this
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── token.rs          (SPL token amount ↔ Decimal)
│           └── borsh_impl.rs     (Borsh serialize/deserialize)
│
├── programs/
│   ├── fermat-bench/             ← POC: Build this
│   │   └── src/lib.rs            (CU benchmark program)
│   │
│   └── fermat-lending/           ← POC: Build this
│       └── src/lib.rs            (lending example)
│
└── tests/
    ├── determinism.rs            (cross-platform consistency)
    └── properties.rs             (property-based tests)
```

---

## 4. POC Feature Specifications

### 4.1 fermat-core (Week 1-2)

#### FermatDecimal Type
```rust
#![no_std]
#![forbid(unsafe_code)]

/// 128-bit decimal with explicit scale.
/// Designed for Solana sBPF: no alloc, no panic, no std.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Decimal {
    mantissa: i128,
    scale: u8,
}
```

#### Constants
| Constant | Value | Purpose |
|----------|-------|---------|
| `Decimal::ZERO` | 0 (scale 0) | Identity for addition |
| `Decimal::ONE` | 1 (scale 0) | Identity for multiplication |
| `Decimal::MAX` | i128::MAX (scale 0) | Upper bound |
| `Decimal::MIN` | i128::MIN (scale 0) | Lower bound |
| `MAX_SCALE` | 28 | Maximum decimal places |
| `USDC_SCALE` | 6 | SPL USDC/USDT standard |
| `SOL_SCALE` | 9 | Native SOL decimals |

#### Constructors
| Function | Signature | Notes |
|----------|-----------|-------|
| `new` | `(mantissa: i64, scale: u32) -> Self` | Primary constructor |
| `from_scaled` | `(mantissa: i128, scale: u8) -> Result<Self>` | Full i128 range |
| `from_str` | `(&str) -> Result<Self>` | Parse "123.456" |
| `from_u64` | `(value: u64, scale: u8) -> Self` | SPL token amounts |

#### Core Arithmetic (all return `Result<Decimal, ArithmeticError>`)
| Operation | Function | CU Target | Notes |
|-----------|----------|-----------|-------|
| Addition | `checked_add(self, rhs)` | <100 CU | Scale alignment + i128 add |
| Subtraction | `checked_sub(self, rhs)` | <100 CU | Scale alignment + i128 sub |
| Multiplication | `checked_mul(self, rhs)` | <200 CU | i128 mul + scale management |
| Division | `checked_div(self, rhs)` | <500 CU | i128 div + precision scaling |
| MulDiv | `checked_mul_div(a, b, c)` | <600 CU | (a×b)÷c single rounding — the Balancer exploit prevention |
| Negation | `checked_neg(self)` | <50 CU | Sign flip |
| Absolute | `abs(self)` | <50 CU | |

#### Rounding Modes (7 IEEE 754-2008)
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    /// Round toward negative infinity (floor)
    Down,
    /// Round toward positive infinity (ceiling)
    Up,
    /// Round toward zero (truncate)
    TowardZero,
    /// Round away from zero
    AwayFromZero,
    /// Ties go to nearest even digit (banker's rounding) — DEFAULT
    HalfEven,
    /// Ties round away from zero
    HalfUp,
    /// Ties round toward zero
    HalfDown,
}
```

#### Rounding Function
```rust
/// Round to `dp` decimal places using specified mode.
pub fn round(self, dp: u8, mode: RoundingMode) -> Self
```

#### Comparison
Implement `Ord`, `PartialOrd`, `PartialEq`, `Eq` with scale normalization so that `Decimal::new(100, 2) == Decimal::new(1, 0)` (1.00 == 1).

#### Error Type
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticError {
    Overflow,
    Underflow,
    DivisionByZero,
    ScaleExceeded,
    InvalidInput,
}
```

**Key design constraint**: Every operation that can fail returns `Result`. Zero panics. On Solana, a panic = lost transaction fee. This is the #1 differentiator vs rust_decimal.

#### Display
- `Display`: "1234.56" (human readable)
- `Debug`: "Decimal { mantissa: 123456, scale: 2 }" (debug)

### 4.2 fermat-solana (Week 2)

#### SPL Token Conversion
```rust
/// Convert SPL token amount (u64 raw) to Decimal.
/// Example: 1_000_000 USDC (6 decimals) → Decimal 1.000000
pub fn from_token_amount(amount: u64, decimals: u8) -> Decimal

/// Convert Decimal back to SPL token u64.
/// Rounds toward zero (conservative — user never gets more than owed).
/// Returns error if value doesn't fit in u64 or is negative.
pub fn to_token_amount(decimal: Decimal, decimals: u8) -> Result<u64, ConversionError>
```

#### Borsh Serialization
```rust
/// BorshSerialize and BorshDeserialize for Decimal.
/// On-chain storage format: 17 bytes (i128 LE + u8 scale).
/// This is the native format for Solana account data.
impl borsh::BorshSerialize for Decimal { ... }
impl borsh::BorshDeserialize for Decimal { ... }
```

#### Anchor Integration
```rust
/// Derive AnchorSerialize/AnchorDeserialize so Decimal
/// can be used directly in Anchor account structs.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PriceData {
    pub value: Decimal,
    pub last_update: i64,
}
```

### 4.3 fermat-bench — CU Benchmark Program (Week 2)

An Anchor program deployed to Solana devnet that measures compute unit consumption for each operation.

#### Instructions
| Instruction | What It Measures |
|-------------|-----------------|
| `bench_add` | Addition CU cost |
| `bench_sub` | Subtraction CU cost |
| `bench_mul` | Multiplication CU cost |
| `bench_div` | Division CU cost |
| `bench_mul_div` | MulDiv compound CU cost |
| `bench_round` | Rounding CU cost (all 7 modes) |
| `bench_scale_convert` | Scale alignment CU cost |
| `bench_compound` | 100 sequential mul+add (simulating interest accrual) |
| `bench_rust_decimal_add` | Same operation using raw rust_decimal (comparison baseline) |
| `bench_native_u64` | Same operation using raw u64 math (comparison baseline) |

#### Output
A published benchmark table:
```
| Operation        | Fermat CU | rust_decimal CU | Raw u64 CU | Fermat Overhead |
|------------------|-----------|-----------------|------------|-----------------|
| add              |    ??     |       ??        |     ??     |      ??%        |
| mul              |    ??     |       ??        |     ??     |      ??%        |
| div              |    ??     |       ??        |     ??     |      ??%        |
| mul_div          |    ??     |       ??        |     ??     |      ??%        |
| 100x compound    |    ??     |       ??        |     ??     |      ??%        |
```

### 4.4 fermat-lending — Anchor Lending Example (Week 3)

A minimal but realistic lending protocol demonstrating Fermat Math in a production-like Solana program.

#### Accounts
```rust
#[account]
pub struct LendingMarket {
    pub authority: Pubkey,
    pub collateral_mint: Pubkey,
    pub debt_mint: Pubkey,
    pub deposit_index: Decimal,        // Accumulates interest
    pub borrow_index: Decimal,         // Accumulates interest
    pub interest_rate: Decimal,        // Annual rate (e.g., 0.05 = 5%)
    pub liquidation_threshold: Decimal, // e.g., 0.80 = 80% LTV
    pub last_update: i64,              // Unix timestamp
    pub total_deposits: u64,           // Raw token amount
    pub total_borrows: u64,            // Raw token amount
}

#[account]
pub struct UserPosition {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub deposited_shares: Decimal,     // Share of deposit pool
    pub borrowed_shares: Decimal,      // Share of borrow pool
}
```

#### Instructions
| Instruction | Math Operations Used | Rounding Direction |
|-------------|---------------------|-------------------|
| `deposit` | Share calculation: `amount × total_shares / total_deposits` | Round DOWN (user gets fewer shares — safe for protocol) |
| `withdraw` | Amount calculation: `shares × total_deposits / total_shares` | Round DOWN (user gets less — safe for protocol) |
| `borrow` | Share calculation + health factor check | Round UP for shares (user owes more — safe for protocol) |
| `repay` | Share calculation: `amount × total_shares / total_borrows` | Round DOWN for shares reduced (user repays slightly more) |
| `accrue_interest` | Compound interest on both indices | Round UP (protocol accrues slightly more) |
| `liquidate` | Health factor check + liquidation math | Health rounds DOWN (triggers liquidation sooner — safe) |

This demonstrates the core value proposition: **explicit rounding direction controls prevent the class of exploits that cost DeFi $200M+.**

#### Health Factor Calculation
```rust
/// Health Factor = (collateral_value × liquidation_threshold) / debt_value
/// If health_factor < 1.0, position is liquidatable.
///
/// Rounding: Round DOWN — this is conservative. A borderline position
/// gets liquidated rather than surviving. This prevents the Compound V2
/// fork vulnerability class where precision loss kept unhealthy
/// positions alive.
pub fn calculate_health_factor(
    collateral_value: Decimal,
    debt_value: Decimal,
    liquidation_threshold: Decimal,
) -> Result<Decimal, ArithmeticError> {
    if debt_value.is_zero() {
        return Ok(Decimal::MAX); // No debt = infinitely healthy
    }
    collateral_value
        .checked_mul(liquidation_threshold)?
        .checked_div_rounded(debt_value, RoundingMode::Down)
}
```

---

## 5. Test Requirements

### 5.1 Unit Tests (fermat-core)
| Category | Minimum Count | Description |
|----------|--------------|-------------|
| Arithmetic correctness | 50+ | Basic operations produce correct results |
| Rounding modes | 49+ | 7 operations × 7 modes each |
| Edge cases | 20+ | MAX, MIN, ZERO, scale boundaries |
| Overflow handling | 15+ | Operations that would exceed i128 return Err |
| Division by zero | 5+ | Never panics, always returns Err |
| Scale alignment | 10+ | Adding values with different scales |
| String parsing | 15+ | "0.001", "-99.99", "0", invalid inputs |
| Equality | 10+ | 1.00 == 1.0 == 1, ordering tests |

### 5.2 Property-Based Tests
Using `proptest` crate:
- Commutativity: `a + b == b + a`, `a × b == b × a`
- Associativity: `(a + b) + c == a + (b + c)` (within precision)
- Identity: `a + 0 == a`, `a × 1 == a`
- Inverse: `a - a == 0`, `a / a == 1` (for non-zero)
- Roundtrip: `Decimal::from_str(d.to_string()) == d`
- Borsh roundtrip: `deserialize(serialize(d)) == d`

### 5.3 Solana Integration Tests
- Deploy fermat-bench to localnet, verify CU measurements
- Deploy fermat-lending to localnet, run full deposit→borrow→accrue→repay→liquidate cycle
- Verify Borsh serialization roundtrips through actual Solana account data

### 5.4 Coverage Target
Minimum 90% line coverage for fermat-core.

---

## 6. Implementation Priorities

The POC should be built in this order — each step validates the next:

### Priority 1: Can we even compile to sBPF? (Day 1-2)
Create a minimal Decimal struct with just `checked_add`. Compile with `cargo build-sbf`. If this works with zero external dependencies and `no_std`, we know the approach is viable.

### Priority 2: Core arithmetic (Day 3-7)
Implement all arithmetic operations + rounding. This is the hardest part. The tricky bits:
- **Scale alignment for add/sub**: When adding 1.5 (scale=1) + 0.25 (scale=2), must align to scale=2 first
- **Multiplication scale management**: 1.5 × 0.25 = 0.375 → scale = scale_a + scale_b, then need to manage if it exceeds MAX_SCALE
- **Division precision**: How many decimal places to compute? Use MAX_SCALE then round.
- **mul_div without intermediate overflow**: (a×b)÷c where a×b might overflow i128. Need to use wider intermediate or chunked multiplication.

### Priority 3: Borsh + SPL helpers (Day 8-10)
fermat-solana crate. Quick to build once core works.

### Priority 4: Benchmark program (Day 11-14)
Deploy to devnet, measure CUs, build comparison table.

### Priority 5: Lending example (Day 15-21)
Anchor program demonstrating real DeFi math.

### Priority 6: Tests + documentation (Day 22-28)
Fill out test suite, write README, prepare for grant submission.

---

## 7. POC Deliverables Checklist

By the end of 4 weeks, before submitting the grant application:

- [ ] `fermat-core` crate published on crates.io
- [ ] `fermat-solana` crate published on crates.io
- [ ] Both crates compile for Solana sBPF target
- [ ] 90%+ test coverage on fermat-core
- [ ] Property-based tests passing
- [ ] CU benchmark program deployed to Solana devnet (published program ID)
- [ ] CU comparison table: Fermat vs rust_decimal vs raw u64
- [ ] Lending example deployed to Solana devnet (published program ID)
- [ ] Integration tests for lending program passing
- [ ] GitHub repo at github.com/xxix-labs/fermat-math
- [ ] README with quick start examples
- [ ] GitHub Actions CI pipeline green

---

## 8. What the POC Does NOT Include

These are explicitly deferred to the grant scope:

- Advanced math functions (exp, ln, log2, log10, sqrt, pow)
- AMM example program
- Options pricing example
- Full documentation site
- Security review
- Tutorial blog series
- Protocol outreach
- Community security review program
- npm/WASM bindings
