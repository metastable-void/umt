//! Canonical exact-value text codec.
//!
//! This is the crate's single definition of how an exact integer or rational
//! is written and read. Both the serde adapters and any future native
//! container use it, so the wire form does not depend on the arbitrary
//! precision library in use.

use alloc::string::{String, ToString};
use core::str::FromStr;

use crate::algebra::{Q, Z};

/// Writes an exact integer in canonical decimal form.
#[must_use]
pub fn z_to_string(value: &Z) -> String {
    value.to_string()
}

/// Reads an exact integer from canonical decimal form.
///
/// Returns `None` if the text is not a decimal integer.
#[must_use]
pub fn z_from_str(text: &str) -> Option<Z> {
    Z::from_str(text.trim()).ok()
}

/// Writes an exact rational as `"numerator/denominator"`, reduced, with an
/// explicit denominator.
#[must_use]
pub fn q_to_string(value: &Q) -> String {
    let mut out = value.numer().to_string();
    out.push('/');
    out.push_str(&value.denom().to_string());
    out
}

/// Reads an exact rational from `"numerator/denominator"` or a bare integer.
///
/// Returns `None` if the text is malformed or the denominator is zero. The
/// result is reduced to lowest terms with a positive denominator.
#[must_use]
pub fn q_from_str(text: &str) -> Option<Q> {
    let text = text.trim();
    let (numer, denom) = match text.split_once('/') {
        Some((numer, denom)) => (z_from_str(numer)?, z_from_str(denom)?),
        None => (z_from_str(text)?, Z::from(1)),
    };
    if denom == Z::from(0) {
        return None;
    }
    Some(Q::new(numer, denom))
}

#[cfg(test)]
mod tests {
    use super::{q_from_str, q_to_string, z_from_str, z_to_string};
    use crate::algebra::{Q, Z};

    #[test]
    fn rational_round_trip_is_canonical() {
        let value = Q::new(Z::from(-6), Z::from(4));
        let text = q_to_string(&value);
        assert_eq!(text, "-3/2");
        assert_eq!(q_from_str(&text), Some(value));
    }

    #[test]
    fn bare_integers_parse_but_are_written_with_a_denominator() {
        assert_eq!(q_from_str("7"), Some(Q::new(Z::from(7), Z::from(1))));
        assert_eq!(q_to_string(&Q::new(Z::from(7), Z::from(1))), "7/1");
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert!(q_from_str("3/0").is_none());
        assert!(q_from_str("3/").is_none());
        assert!(q_from_str("").is_none());
        assert!(q_from_str("1.5").is_none());
        assert!(q_from_str("3/4/5").is_none());
        assert!(z_from_str("0x10").is_none());
    }

    #[test]
    fn arbitrary_size_integers_survive() {
        let big = Z::from(10).pow(400) + Z::from(7);
        let text = z_to_string(&big);
        assert_eq!(z_from_str(&text), Some(big));
    }
}
