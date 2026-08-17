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

/// A sequence of exact integers as canonical decimal text.
pub mod vec_z {
    use alloc::string::String;
    use alloc::vec::Vec;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::algebra::Z;
    use crate::io::text::{z_from_str, z_to_string};

    /// Writes exact integers as decimal text.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own errors.
    pub fn serialize<S: Serializer>(values: &[Z], serializer: S) -> Result<S::Ok, S::Error> {
        let text: Vec<String> = values.iter().map(z_to_string).collect();
        text.serialize(serializer)
    }

    /// Reads exact integers from decimal text.
    ///
    /// # Errors
    ///
    /// Fails if any entry is not a decimal integer.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Z>, D::Error> {
        let text = Vec::<String>::deserialize(deserializer)?;
        text.iter()
            .map(|entry| {
                z_from_str(entry).ok_or_else(|| serde::de::Error::custom("malformed exact integer"))
            })
            .collect()
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

/// Optional exact rational, as canonical text or `null`.
pub mod option_q {
    use alloc::string::String;
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::algebra::Q;
    use crate::io::text::{q_from_str, q_to_string};

    /// Writes an optional exact rational.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own errors.
    pub fn serialize<S: Serializer>(value: &Option<Q>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serializer.serialize_some(&q_to_string(value)),
            None => serializer.serialize_none(),
        }
    }

    /// Reads an optional exact rational.
    ///
    /// # Errors
    ///
    /// Fails if the text is present and malformed.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Q>, D::Error> {
        let text = Option::<String>::deserialize(deserializer)?;
        text.map(|text| {
            q_from_str(&text).ok_or_else(|| serde::de::Error::custom("malformed exact rational"))
        })
        .transpose()
    }
}

/// A map from string-like keys to exact rationals, with the values as
/// canonical text.
pub mod map_q {
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::algebra::Q;
    use crate::io::text::{q_from_str, q_to_string};

    /// Writes the values as canonical exact text.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's own errors.
    pub fn serialize<S, K>(value: &BTreeMap<K, Q>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + Ord + Clone,
    {
        let encoded: BTreeMap<K, String> = value
            .iter()
            .map(|(key, value)| (key.clone(), q_to_string(value)))
            .collect();
        encoded.serialize(serializer)
    }

    /// Reads the values as canonical exact text.
    ///
    /// # Errors
    ///
    /// Fails if any value is malformed.
    pub fn deserialize<'de, D, K>(deserializer: D) -> Result<BTreeMap<K, Q>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Ord,
    {
        let encoded = BTreeMap::<K, String>::deserialize(deserializer)?;
        encoded
            .into_iter()
            .map(|(key, text)| {
                q_from_str(&text)
                    .map(|value| (key, value))
                    .ok_or_else(|| serde::de::Error::custom("malformed exact rational"))
            })
            .collect()
    }
}
