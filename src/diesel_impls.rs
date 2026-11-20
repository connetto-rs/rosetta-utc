#![cfg(feature = "diesel")]
//! Submodule proving the implementations of the diesel-related traits.

mod postgres;
mod sqlite;

/// `TIMESTAMPTZ` on PostgreSQL and `TEXT` on SQLite, read and written as
/// [`crate::TimestampUTC`].
#[derive(
    Debug, Clone, Copy, Default, diesel::query_builder::QueryId, diesel::sql_types::SqlType,
)]
#[cfg_attr(
    feature = "postgres",
    diesel(postgres_type(oid = 1184, array_oid = 1185))
)]
#[cfg_attr(feature = "sqlite", diesel(sqlite_type(name = "Text")))]
pub struct TimestampUTC;

/// Lets `max` and `min` run over the column. PostgreSQL orders `TIMESTAMPTZ`
/// by instant. SQLite orders the stored text, which follows instant order for
/// the text this crate writes in years 0 through 9999.
impl diesel::sql_types::SqlOrd for TimestampUTC {}
