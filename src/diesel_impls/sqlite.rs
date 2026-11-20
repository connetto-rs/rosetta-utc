#![cfg(feature = "sqlite")]
//! Submodule proving the implementations of the traits relative to Diesel with
//! `SQLite` as the backend.
//!
//! Values are written as `YYYY-MM-DD HH:MM:SS.ffffff+00:00`, six fractional
//! digits at PostgreSQL's microsecond precision, so every instant has one
//! spelling. `SQLite` compares these values as text, and `=`, `IN`, `GROUP BY`
//! and `UNIQUE` only match an instant written the same way. Years before 1 AD
//! are written as a minus sign and four digits.
//!
//! Reading also accepts other fraction lengths, any offset, a `T` separator,
//! text without an offset (taken as UTC, as `CURRENT_TIMESTAMP` writes it) and
//! Julian day numbers.

use diesel::sqlite::{Sqlite, SqliteValue};
use jiff::fmt::temporal::Pieces;
use jiff::tz::Offset;

/// The Unix epoch in SQLite Julian days, `julianday('1970-01-01T00:00:00Z')`.
const UNIX_EPOCH_IN_JULIAN_DAYS: f64 = 2_440_587.5;
/// SQLite counts every day as 86400 seconds.
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Parses the text SQLite holds for a `TimestampUTC` column.
fn parse(text: &str) -> Result<jiff::Timestamp, jiff::Error> {
    match Pieces::parse(crate::widen_signed_year(text).as_ref()) {
        Ok(pieces) => pieces
            .to_numeric_offset()
            .unwrap_or(Offset::UTC)
            .to_timestamp(pieces.date().to_datetime(pieces.time().unwrap_or_default())),
        Err(error) => match text.parse::<f64>() {
            Ok(julian_days) => {
                jiff::Timestamp::from_duration(jiff::SignedDuration::try_from_secs_f64(
                    (julian_days - UNIX_EPOCH_IN_JULIAN_DAYS) * SECONDS_PER_DAY,
                )?)
            }
            Err(_) => Err(error),
        },
    }
}

impl diesel::deserialize::FromSql<crate::diesel_impls::TimestampUTC, Sqlite>
    for crate::TimestampUTC
{
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> diesel::deserialize::Result<Self> {
        Ok(Self(parse(value.read_text())?))
    }
}

impl diesel::serialize::ToSql<crate::diesel_impls::TimestampUTC, Sqlite> for crate::TimestampUTC {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Sqlite>,
    ) -> diesel::serialize::Result {
        // The civil time keeps its fraction non-negative, so the division truncates
        // toward the earlier microsecond for instants before 1970 as well.
        let datetime = Offset::UTC.to_datetime(self.0);
        let year = datetime.year();
        out.set_value(format!(
            "{year:0year_width$}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}+00:00",
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
            datetime.second(),
            datetime.subsec_nanosecond() / 1_000,
            // The sign takes one of the padded places.
            year_width = if year < 0 { 5 } else { 4 },
        ));
        Ok(diesel::serialize::IsNull::No)
    }
}
