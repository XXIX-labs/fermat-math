//! Borsh serialization / deserialization for `Decimal`.
//!
//! ## Wire Format (17 bytes)
//!
//! ```text
//! bytes  0-15:  mantissa as little-endian i128 (16 bytes)
//! byte   16:    scale as u8 (1 byte)
//! ```
//!
//! ## Security Note
//!
//! `BorshDeserialize` validates that `scale <= MAX_SCALE` before constructing
//! the `Decimal`. This prevents an adversary from submitting an account with
//! `scale = 255`, which would cause silent incorrect arithmetic downstream.

use borsh::{BorshDeserialize, BorshSerialize};
use fermat_core::{ArithmeticError, Decimal, MAX_SCALE};

/// Wrapper implementing Borsh for `Decimal` (17 bytes on-chain).
///
/// Use this when storing `Decimal` fields in Anchor account structs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecimalBorsh(pub Decimal);

impl BorshSerialize for DecimalBorsh {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mantissa_bytes: [u8; 16] = self.0.mantissa().to_le_bytes();
        mantissa_bytes.serialize(writer)?;
        self.0.scale().serialize(writer)
    }
}

impl BorshDeserialize for DecimalBorsh {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mantissa_bytes = <[u8; 16]>::deserialize_reader(reader)?;
        let scale = u8::deserialize_reader(reader)?;

        // ── Security check ────────────────────────────────────────────────────
        // Always validate scale on deserialise. On-chain data is adversarial;
        // a malicious account with scale = 255 would corrupt arithmetic.
        if scale > MAX_SCALE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Decimal scale exceeds MAX_SCALE (28)",
            ));
        }

        let mantissa = i128::from_le_bytes(mantissa_bytes);
        let decimal = Decimal::new(mantissa, scale).map_err(|_: ArithmeticError| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid Decimal encoding",
            )
        })?;

        Ok(DecimalBorsh(decimal))
    }
}

impl From<Decimal> for DecimalBorsh {
    fn from(d: Decimal) -> Self {
        DecimalBorsh(d)
    }
}

impl From<DecimalBorsh> for Decimal {
    fn from(db: DecimalBorsh) -> Self {
        db.0
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    fn roundtrip(d: Decimal) -> Decimal {
        let wrapped = DecimalBorsh(d);
        let mut buf = Vec::new();
        wrapped.serialize(&mut buf).unwrap();
        assert_eq!(buf.len(), 17, "Borsh encoding must be exactly 17 bytes");
        let decoded = DecimalBorsh::try_from_slice(&buf).unwrap();
        decoded.0
    }

    #[test]
    fn roundtrip_zero() {
        assert_eq!(roundtrip(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn roundtrip_one() {
        assert_eq!(roundtrip(Decimal::ONE), Decimal::ONE);
    }

    #[test]
    fn roundtrip_usdc_price() {
        let price = Decimal::new(1_500_000, 6).unwrap();
        assert_eq!(roundtrip(price), price);
    }

    #[test]
    fn roundtrip_negative() {
        let x = Decimal::new(-42_000_000_000i128, 9).unwrap();
        assert_eq!(roundtrip(x), x);
    }

    #[test]
    fn roundtrip_max_scale() {
        let x = Decimal::new(1, 28).unwrap();
        assert_eq!(roundtrip(x), x);
    }

    #[test]
    fn reject_invalid_scale() {
        // Manually craft a 17-byte buffer with scale = 255
        let mut buf = vec![0u8; 17];
        buf[16] = 255;
        let result = DecimalBorsh::try_from_slice(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn encoding_is_17_bytes() {
        let x = Decimal::new(i128::MAX, 28).unwrap();
        let wrapped = DecimalBorsh(x);
        let mut buf = Vec::new();
        wrapped.serialize(&mut buf).unwrap();
        assert_eq!(buf.len(), 17);
    }

    #[test]
    fn little_endian_mantissa_byte_layout() {
        let x = Decimal::new(1, 0).unwrap();
        let wrapped = DecimalBorsh(x);
        let mut buf = Vec::new();
        wrapped.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 1);
        assert!(buf[1..16].iter().all(|&b| b == 0));
        assert_eq!(buf[16], 0);
    }
}
