# fermat-core

**128-bit fixed-point decimal arithmetic for Solana's sBPF runtime.**

`fermat-core` is the core arithmetic crate of the [fermat-math](https://github.com/XXIX-labs/fermat-math) project. It provides a `Decimal` type backed by an `i128` mantissa and `u8` scale, with checked arithmetic, 7 IEEE 754-2008 rounding modes, and a 256-bit intermediate for overflow-safe `mul_div`.

[![crates.io](https://img.shields.io/crates/v/fermat-core.svg)](https://crates.io/crates/fermat-core)
[![docs.rs](https://docs.rs/fermat-core/badge.svg)](https://docs.rs/fermat-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-MIT)

## Features

- **`#![no_std]`** — compiles to bare-metal sBPF
- **`#![forbid(unsafe_code)]`** — no unsafe blocks anywhere
- **Zero external dependencies** — minimal binary size
- **Every operation returns `Result`** — no panics, ever
- **7 IEEE 754-2008 rounding modes** — explicit rounding direction
- **`checked_mul_div`** — 256-bit intermediate prevents `(a*b)/c` overflow
- **17 bytes on-chain** — compact `i128` + `u8` Borsh encoding

## Quick Start

```rust
use fermat_core::{Decimal, RoundingMode};

let price  = Decimal::new(150_000_000, 6)?;   // 150.000000
let amount = Decimal::new(2_500_000, 6)?;      //   2.500000

let total  = price.checked_mul(amount)?;
let result = total.round(6, RoundingMode::HalfEven)?;  // 375.000000

// Overflow-safe: (a * b) / c via U256 intermediate
let health = collateral.checked_mul_div(threshold, debt)?;
```

## Installation

```toml
[dependencies]
fermat-core = "0.1"
```

## Modules

| Module | Contents |
|---|---|
| `decimal` | `Decimal` struct, constants (`ZERO`, `ONE`, `MAX`, `MIN`) |
| `arithmetic` | `checked_add/sub/mul/div`, `checked_mul_div` (U256), `checked_neg/abs` |
| `rounding` | 7 modes: `Down`, `Up`, `TowardZero`, `AwayFromZero`, `HalfUp`, `HalfDown`, `HalfEven` |
| `convert` | `from_u64/i64/u128`, `from_str_exact`, `to_token_amount` |
| `compare` | `Ord`/`PartialOrd` with scale normalisation |
| `display` | Human-readable `Display` (e.g. `1.500000`) |
| `error` | `ArithmeticError`: `Overflow`, `DivisionByZero`, `ScaleExceeded`, `InvalidInput` |

## See Also

- [fermat-solana](https://crates.io/crates/fermat-solana) — Borsh + SPL token integration
- [GitHub](https://github.com/XXIX-labs/fermat-math) — full repository with benchmarks and lending example
- [fermatmath.net](https://fermatmath.net) — project website

## License

Licensed under either [MIT](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-APACHE) at your option.

Copyright 2026 XXIX Labs
