#![cfg(feature = "sqlite")]
//! Submodule proving the implementations of the traits relative to Diesel with
//! `SQLite` as the backend.
//!
//! Values are written as `YYYY-MM-DD HH:MM:SS.ffffff+00:00`, six fractional
//! digits at PostgreSQL's microsecond precision, so every instant has one
//! spelling. `SQLite` compares these values as text, and `=`, `IN`, `GROUP BY`
//! and `UNIQUE` only match an instant written the same way.
//! Reading accepts every form diesel's `TimestamptzSqlite` reads, variable
//! fractions included.

/// The text written for a `TimestampUTC`, truncated to PostgreSQL's microsecond
/// precision.
const TEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f+00:00";

impl diesel::deserialize::FromSql<crate::diesel_impls::TimestampUTC, diesel::sqlite::Sqlite>
    for crate::TimestampUTC
{
    fn from_sql(
        value: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> diesel::deserialize::Result<Self> {
        <chrono::DateTime<chrono::Utc> as diesel::deserialize::FromSql<
            diesel::sql_types::TimestamptzSqlite,
            diesel::sqlite::Sqlite,
        >>::from_sql(value)
        .map(Self::from)
    }
}

impl diesel::serialize::ToSql<crate::diesel_impls::TimestampUTC, diesel::sqlite::Sqlite>
    for crate::TimestampUTC
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.format(TEXT_FORMAT).to_string());
        Ok(diesel::serialize::IsNull::No)
    }
}
