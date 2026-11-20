#![doc = include_str!("../README.md")]

use core::str::FromStr;

pub mod diesel_impls;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature="diesel", diesel(sql_type = crate::diesel_impls::TimestampUTC))]
/// A wrapper around the `jiff` crate's `Timestamp` type.
pub struct TimestampUTC(jiff::Timestamp);

impl TimestampUTC {
    #[must_use]
    /// Returns the current time in UTC.
    ///
    /// # Examples
    ///
    /// ```
    /// use rosetta_utc::TimestampUTC;
    ///
    /// let now = TimestampUTC::now();
    /// ```
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }
}

impl Default for TimestampUTC {
    fn default() -> Self {
        Self::now()
    }
}

impl From<jiff::Timestamp> for TimestampUTC {
    fn from(value: jiff::Timestamp) -> Self {
        Self(value)
    }
}

impl From<TimestampUTC> for jiff::Timestamp {
    fn from(value: TimestampUTC) -> Self {
        value.0
    }
}

impl AsRef<jiff::Timestamp> for TimestampUTC {
    fn as_ref(&self) -> &jiff::Timestamp {
        &self.0
    }
}

impl AsMut<jiff::Timestamp> for TimestampUTC {
    fn as_mut(&mut self) -> &mut jiff::Timestamp {
        &mut self.0
    }
}

impl core::ops::Deref for TimestampUTC {
    type Target = jiff::Timestamp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for TimestampUTC {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl core::fmt::Display for TimestampUTC {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for TimestampUTC {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TimestampUTC {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Reads the string inside the newtype that the derived `Serialize` writes.
        struct TimestampVisitor;

        impl<'de> serde::de::Visitor<'de> for TimestampVisitor {
            type Value = TimestampUTC;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("an RFC 3339 timestamp")
            }

            fn visit_newtype_struct<D: serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_str(self)
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
                widen_signed_year(text).parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_newtype_struct("TimestampUTC", TimestampVisitor)
    }
}

/// Rewrites a leading year written as a minus sign and four digits, the way
/// rosetta-utc 0.1.0 wrote years before 1 AD, into the six-digit form RFC 9557
/// and jiff require.
#[cfg(any(feature = "serde", feature = "sqlite"))]
fn widen_signed_year(text: &str) -> std::borrow::Cow<'_, str> {
    match text.as_bytes() {
        [b'-', first, second, third, fourth, b'-', ..]
            if [first, second, third, fourth]
                .iter()
                .all(|digit| digit.is_ascii_digit()) =>
        {
            std::borrow::Cow::Owned(format!("-00{}", &text[1..]))
        }
        _ => std::borrow::Cow::Borrowed(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_from_str_converts_offset_to_utc() {
        let t = TimestampUTC::from_str("2023-10-27T12:00:00+02:00").unwrap();
        assert_eq!(t.to_string(), "2023-10-27T10:00:00Z");
    }

    #[test]
    fn test_from_str_rejects_missing_offset() {
        assert!(TimestampUTC::from_str("2023-10-27T10:00:00").is_err());
    }

    #[test]
    fn test_default_is_the_current_time() {
        let before = TimestampUTC::now();
        let default = TimestampUTC::default();
        assert!(before <= default && default <= TimestampUTC::now());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_rejects_a_non_string() {
        let error = serde_json::from_str::<TimestampUTC>("1698400800").unwrap_err();
        assert!(
            error.to_string().contains("an RFC 3339 timestamp"),
            "{error}"
        );
    }

    #[test]
    fn test_from_conversions() {
        let inner = jiff::Timestamp::constant(1_698_400_800, 123_456_789);
        let wrapper: TimestampUTC = inner.into();
        let back: jiff::Timestamp = wrapper.into();
        assert_eq!(back, inner);
        assert_eq!(*wrapper.as_ref(), inner);
    }

    #[test]
    fn test_deref_mut_and_as_mut_modify_inner() {
        let start = jiff::Timestamp::constant(1_698_400_800, 0);
        let mut t = TimestampUTC::from(start);

        *t -= jiff::SignedDuration::from_secs(300);
        assert_eq!(*t, jiff::Timestamp::constant(1_698_400_500, 0));

        *t.as_mut() = start;
        assert_eq!(*t, start);
    }

    #[test]
    fn test_ord_and_hash() {
        let t1 = TimestampUTC::from_str("2023-01-01T00:00:00Z").unwrap();
        let t2 = TimestampUTC::from_str("2023-01-02T00:00:00Z").unwrap();
        assert!(t1 < t2);

        let set: HashSet<_> = [t1, t1, t2].into_iter().collect();
        assert_eq!(set.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_reads_strings_written_by_0_1() {
        // rosetta-utc 0.1.0 serialized with 0, 3, 6 or 9 fractional digits, a `Z`, and
        // years before 1 AD as a sign and four digits.
        for (written, expected) in [
            ("\"2023-10-27T10:00:00Z\"", "2023-10-27T10:00:00Z"),
            ("\"2023-10-27T10:00:00.120Z\"", "2023-10-27T10:00:00.12Z"),
            (
                "\"2023-10-27T10:00:00.123456Z\"",
                "2023-10-27T10:00:00.123456Z",
            ),
            (
                "\"1969-12-31T23:59:59.999999999Z\"",
                "1969-12-31T23:59:59.999999999Z",
            ),
            ("\"-0001-01-01T00:00:00Z\"", "-000001-01-01T00:00:00Z"),
            ("\"-8395-12-17T15:26:43Z\"", "-008395-12-17T15:26:43Z"),
        ] {
            let t: TimestampUTC = serde_json::from_str(written).unwrap();
            assert_eq!(t.to_string(), expected);
        }
    }
}
