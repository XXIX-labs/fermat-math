# fermat-solana

**Solana/Anchor integration for [fermat-core](https://crates.io/crates/fermat-core).**

Provides Borsh serialization, SPL token amount conversions, and Anchor account helpers for the `Decimal` type.

[![crates.io](https://img.shields.io/crates/v/fermat-solana.svg)](https://crates.io/crates/fermat-solana)
[![docs.rs](https://docs.rs/fermat-solana/badge.svg)](https://docs.rs/fermat-solana)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-MIT)

## Features

- **`DecimalBorsh`** — 17-byte on-chain encoding (`i128` LE + `u8` scale), validates `scale <= 28` on deserialization
- **SPL token conversions** — `token_amount_to_decimal`, `decimal_to_token_amount` with explicit rounding
- **Anchor account helpers** — `DECIMAL_SPACE = 17`, `DecimalBorsh::zero_with_scale` for account `init`

## Installation

```toml
[dependencies]
fermat-core   = "0.1"
fermat-solana = "0.1"
```

## Quick Start

```rust
use fermat_core::RoundingMode;
use fermat_solana::token::{token_amount_to_decimal, decimal_to_token_amount};

// 1_500_000 raw USDC lamports -> 1.500000 Decimal
let price = token_amount_to_decimal(1_500_000u64, 6)?;

// Convert back — round down on withdrawal (conservative)
let raw = decimal_to_token_amount(price, 6, RoundingMode::Down)?;
assert_eq!(raw, 1_500_000u64);
```

### Anchor Account Usage

```rust
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

## Modules

| Module | Contents |
|---|---|
| `borsh_impl` | `DecimalBorsh` wrapper — 17-byte encoding, adversarial scale validation |
| `token` | `token_amount_to_decimal`, `decimal_to_token_amount`, `align_to_mint` |
| `account` | `DECIMAL_SPACE = 17`, `DecimalBorsh::zero_with_scale` for Anchor `init` |

## See Also

- [fermat-core](https://crates.io/crates/fermat-core) — core arithmetic library
- [GitHub](https://github.com/XXIX-labs/fermat-math) — full repository
- [fermatmath.net](https://fermatmath.net) — project website

## License

Licensed under either [MIT](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/XXIX-labs/fermat-math/blob/main/LICENSE-APACHE) at your option.

Copyright 2026 XXIX Labs
