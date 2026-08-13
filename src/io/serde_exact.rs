//! Serde adapters that keep exact values exact.
//!
//! Use with `#[serde(with = "umt::io::serde_exact::z")]` or
//! `#[serde(with = "umt::io::serde_exact::q")]`. Both encode through
//! [`crate::io::text`], so an exact value never passes through a
//! floating-point representation (UMT-3.2 section 8.9).

/// Exact integer as canonical decimal text.
pub mod z {
    use alloc::string::String;
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::algebra::Z;
    use crate::io::text::{z_from_str, z_to_string};

    /// Writes an exact integer as decimal text.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own errors.
    pub fn serialize<S: Serializer>(value: &Z, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&z_to_string(value))
    }

    /// Reads an exact integer from decimal text.
    ///
    /// # Errors
    ///
    /// Fails if the text is not a decimal integer.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Z, D::Error> {
        let text = String::deserialize(deserializer)?;
        z_from_str(&text).ok_or_else(|| serde::de::Error::custom("malformed exact integer"))
    }
}

/// Exact rational as canonical `"numerator/denominator"` text.
pub mod q {
    use alloc::string::String;
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::algebra::Q;
    use crate::io::text::{q_from_str, q_to_string};

    /// Writes an exact rational as `"numerator/denominator"`.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own errors.
    pub fn serialize<S: Serializer>(value: &Q, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&q_to_string(value))
    }

    /// Reads an exact rational.
    ///
    /// # Errors
    ///
    /// Fails if the text is malformed or the denominator is zero.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Q, D::Error> {
        let text = String::deserialize(deserializer)?;
        q_from_str(&text).ok_or_else(|| serde::de::Error::custom("malformed exact rational"))
    }
}
