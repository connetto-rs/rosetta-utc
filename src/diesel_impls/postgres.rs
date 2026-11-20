#![cfg(feature = "postgres")]
//! Submodule proving the implementations of the traits relative to Diesel with
//! `PostgreSQL` as the backend.
//!
//! `TIMESTAMPTZ` travels as the number of microseconds since
//! `2000-01-01T00:00:00Z`.

use diesel::pg::data_types::PgTimestamp;

/// The `PostgreSQL` epoch, `2000-01-01T00:00:00Z`.
const POSTGRES_EPOCH: jiff::Timestamp = jiff::Timestamp::constant(946_684_800, 0);

impl diesel::deserialize::FromSql<crate::diesel_impls::TimestampUTC, diesel::pg::Pg>
    for crate::TimestampUTC
{
    fn from_sql(value: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let PgTimestamp(micros) = <PgTimestamp as diesel::deserialize::FromSql<
            diesel::sql_types::Timestamptz,
            diesel::pg::Pg,
        >>::from_sql(value)?;
        Ok(Self(
            POSTGRES_EPOCH.checked_add(jiff::SignedDuration::from_micros(micros))?,
        ))
    }
}

impl diesel::serialize::ToSql<crate::diesel_impls::TimestampUTC, diesel::pg::Pg>
    for crate::TimestampUTC
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        // Truncates toward the epoch, the sub-microsecond part has no column to go to.
        let micros = i64::try_from(self.0.duration_since(POSTGRES_EPOCH).as_micros())?;
        <PgTimestamp as diesel::serialize::ToSql<
            diesel::sql_types::Timestamptz,
            diesel::pg::Pg,
        >>::to_sql(&PgTimestamp(micros), &mut out.reborrow())
    }
}
