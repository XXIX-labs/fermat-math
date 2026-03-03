# fermat-math

**128-bit fixed-point decimal arithmetic for Solana's sBPF runtime.**

`fermat-math` is a production-grade fixed-point arithmetic library designed for on-chain DeFi
protocols. It eliminates floating-point risk, prevents intermediate-overflow bugs (à la the
Balancer/Mango incidents), and provides deterministic rounding for consensus-critical computations.

[![CI](https://github.com/XXIX-labs/fermat-math/actions/workflows/ci.yml/badge.svg)](https://github.com/XXIX-labs/fermat-math/actions)
[![crates.io](https://img.shields.io/crates/v/fermat-core.svg)](https://crates.io/crates/fermat-core)
[![crates.io](https://img.shields.io/crates/v/fermat-solana.svg)](https://crates.io/crates/fermat-solana)
[![docs.rs](https://docs.rs/fermat-core/badge.svg)](https://docs.rs/fermat-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

## Installation

Add the crates to your `Cargo.toml`:

```toml
[dependencies]
fermat-core = "0.1"            # Core arithmetic (no_std, zero deps)
fermat-solana = "0.1"          # Borsh + SPL token helpers (optional)
```

Or install via the command line:

```bash
cargo add fermat-core
cargo add fermat-solana          # if you need Solana/Anchor integration
```

---

## Why fermat-math?

| Problem | fermat-math's answer |
|---|---|
| Solana sBPF has no hardware float | Pure integer fixed-point (`i128` mantissa) |
| `(a×b)/c` silently wraps in `i128` | `checked_mul_div` uses 256-bit intermediate (`U256`) |
| Silent precision loss from rounding | All rounding requires an explicit `RoundingMode` |
| Panics crash validators | `#![forbid(unsafe_code)]` + every op returns `Result` |
| On-chain borsh scale injection | `BorshDeserialize` validates `scale ≤ MAX_SCALE` |

---

## Core Type

```rust
/// value = mantissa × 10^(−scale)
Decimal { mantissa: i128, scale: u8 /* 0..=28 */ }
```

- **MAX_SCALE = 28** — matches `rust_decimal`'s precision bound.
- **17 bytes on-chain** — 16-byte LE `i128` mantissa + 1-byte `u8` scale (Borsh).
- **USDC_SCALE = 6**, **SOL_SCALE = 9** — convenience constants.

---

## Quick Start

```rust
use fermat_core::{Decimal, RoundingMode};

// Create decimals from raw mantissa + scale
let price  = Decimal::new(150_000_000, 6)?;   // 150.000000
let amount = Decimal::new(2_500_000, 6)?;      //   2.500000

// Arithmetic — every operation returns Result (no panics)
let total  = price.checked_mul(amount)?;        // scale 12
let result = total.round(6, RoundingMode::HalfEven)?;  // 375.000000

// Parse from string
let rate = Decimal::from_str_exact("0.05")?;   // 5% annual rate

// Overflow-safe compound operation: (a × b) / c via U256 intermediate
let health = collateral.checked_mul_div(threshold, debt)?;
```

### On-chain (Anchor / Borsh)

```rust
use fermat_solana::{DecimalBorsh, DECIMAL_SPACE};
use fermat_solana::token::{token_amount_to_decimal, decimal_to_token_amount};

// SPL token amount → Decimal
let price = token_amount_to_decimal(1_500_000u64, 6)?;  // 1.500000

// Decimal → SPL token amount (round down for withdrawals)
let raw = decimal_to_token_amount(price, 6, RoundingMode::Down)?;

// Use in Anchor account structs — 17 bytes on-chain
#[account]
pub struct PriceOracle {
    pub price: DecimalBorsh,       // 17 bytes, validates scale ≤ 28
}
```

---

## Crate Structure

```
fermat-math/
├── crates/
│   ├── fermat-core/        — no_std core library (zero external deps)
│   └── fermat-solana/      — Borsh + SPL token helpers
├── programs/
│   ├── fermat-bench/       — Anchor CU benchmark program
│   └── fermat-lending/     — Reference lending protocol
└── crates/fermat-core/tests/
    ├── properties.rs       — proptest property-based suite
    └── determinism.rs      — bit-for-bit determinism checks
```

### `fermat-core`

`#![no_std]`, `#![forbid(unsafe_code)]`, zero external dependencies.

| Module | Contents |
|---|---|
| `decimal` | `Decimal` struct, constants (`ZERO`, `ONE`, `MAX`, `MIN`) |
| `arithmetic` | `checked_add/sub/mul/div`, `checked_mul_div` (U256), `checked_neg/abs` |
| `rounding` | 7 IEEE 754-2008 modes: `Down`, `Up`, `TowardZero`, `AwayFromZero`, `HalfUp`, `HalfDown`, `HalfEven` |
| `convert` | `from_u64/i64/u128`, `from_str_exact`, `to_token_amount` |
| `compare` | `Ord`/`PartialOrd` with scale normalisation |
| `display` | Human-readable `Display` (e.g. `1.500000`) |
| `error` | `ArithmeticError`: `Overflow`, `DivisionByZero`, `ScaleExceeded`, `InvalidInput` |

### `fermat-solana`

Solana / Anchor integration helpers.

| Module | Contents |
|---|---|
| `borsh_impl` | `DecimalBorsh` wrapper — 17-byte on-chain encoding, adversarial scale validation |
| `token` | `token_amount_to_decimal`, `decimal_to_token_amount`, `align_to_mint` |
| `account` | `DECIMAL_SPACE = 17`, `DecimalBorsh::zero_with_scale` for Anchor `init` |

---

## Arithmetic Operations

### Basic Operations

```rust
let a = Decimal::new(1_500_000, 6)?;  // 1.5
let b = Decimal::new(500_000,   6)?;  // 0.5

a.checked_add(b)?     // 2.000000 (scale 6)
a.checked_sub(b)?     // 1.000000 (scale 6)
a.checked_mul(b)?     // 0.750000 (scale 12, then round as needed)
a.checked_div(b)?     // 3.000... (scale 28)
```

### `checked_mul_div` — The Security-Critical Operation

```rust
// Computes (self × numerator) / denominator
// WITHOUT overflowing i128 on the intermediate product.
//
// DeFi vulnerability prevented:
// If collateral = i128::MAX / 2 and threshold = 2,
// then (collateral × threshold) overflows i128 without U256.

let health = collateral_usd
    .checked_mul_div(liquidation_threshold, total_debt_usd)?;
```

The 256-bit intermediate is computed using four 64-bit partial products — no unsafe, no external
bignum crate. The U256 division uses a fast 4-phase algorithm for denominators ≤ 2⁶⁴ and a
binary long-division fallback for larger denominators.

### Rounding

```rust
use fermat_core::RoundingMode;

let x = Decimal::new(1_234_567, 7)?;  // 0.1234567

x.round(6, RoundingMode::HalfEven)?   // 0.123457 (banker's rounding)
x.round(6, RoundingMode::Down)?        // 0.123456 (truncate toward −∞)
x.round(6, RoundingMode::Up)?          // 0.123457 (toward +∞)
```

All 7 IEEE 754-2008 modes are implemented. `HalfEven` (banker's rounding) is recommended for
financial calculations; it eliminates statistical bias over many rounding operations.

---

## On-Chain Usage (Anchor)

```rust
use anchor_lang::prelude::*;
use fermat_solana::{DecimalBorsh, DECIMAL_SPACE};

#[account]
pub struct PriceOracle {
    pub authority:  Pubkey,        // 32 bytes
    pub price:      DecimalBorsh,  // 17 bytes
    pub confidence: DecimalBorsh,  // 17 bytes
    pub bump:       u8,            //  1 byte
}

impl PriceOracle {
    pub const SPACE: usize = 8 + 32 + DECIMAL_SPACE + DECIMAL_SPACE + 1;
}
```

`DecimalBorsh` validates `scale ≤ 28` on deserialization, preventing adversarial account data
from injecting a `scale = 255` field that would corrupt downstream arithmetic.

---

## SPL Token Conversions

```rust
use fermat_core::RoundingMode;
use fermat_solana::token::{token_amount_to_decimal, decimal_to_token_amount};

// 1_500_000 raw USDC lamports → 1.500000 Decimal
let price = token_amount_to_decimal(1_500_000u64, 6)?;

// Convert back — round down on withdrawal (conservative)
let raw = decimal_to_token_amount(price, 6, RoundingMode::Down)?;
assert_eq!(raw, 1_500_000u64);
```

---

## Security Audit Notes

| ID | Threat | Mitigation |
|---|---|---|
| S-01 | `(a×b)` overflows `i128` in `mul_div` | `U256::mul` + `checked_div` in `checked_mul_div` |
| S-02 | Scale creep in `checked_mul` | Reject if `a.scale + b.scale > MAX_SCALE` |
| S-03 | Scale overflow in align | `pow10` uses const table, `Err(ScaleExceeded)` on bounds |
| S-04 | Division by zero | Explicit zero check before every division |
| S-05 | Borsh injection of `scale = 255` | `scale > MAX_SCALE` check in `BorshDeserialize` |
| S-06 | Panic in sBPF program | `#![no_std]` + `#![forbid(unsafe_code)]` + no `.unwrap()` in lib |
| S-07 | Negative scale | `u8` type makes negative scale impossible at the type level |
| S-08 | Comparing different-scale values | `align_scales` normalises before comparison in `Ord` |
| S-09 | Silent precision loss | All rounding requires explicit `RoundingMode` argument |
| S-10 | `i128::MIN.abs()` panic | Uses `.unsigned_abs()` (returns `u128`) in `checked_mul_div` |

---

## Running Tests

```bash
# Full workspace (215 tests)
cargo test --workspace

# Individual crates
cargo test -p fermat-core       # 160 unit + 15 proptest + 10 determinism + 1 doc
cargo test -p fermat-solana     # 19 unit + 2 doc
cargo test -p fermat-lending    # 8 unit

# sBPF compile check (requires bpfel-unknown-none target)
cargo build --target bpfel-unknown-none -p fermat-core
```

---

## Links

| Resource | URL |
|---|---|
| **fermat-core** on crates.io | [crates.io/crates/fermat-core](https://crates.io/crates/fermat-core) |
| **fermat-solana** on crates.io | [crates.io/crates/fermat-solana](https://crates.io/crates/fermat-solana) |
| API docs (fermat-core) | [docs.rs/fermat-core](https://docs.rs/fermat-core) |
| API docs (fermat-solana) | [docs.rs/fermat-solana](https://docs.rs/fermat-solana) |
| GitHub | [github.com/XXIX-labs/fermat-math](https://github.com/XXIX-labs/fermat-math) |
| Website | [fermatmath.net](https://fermatmath.net) |

---

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

Copyright 2026 XXIX Labs
