//! Tests for PostgreSQL integration in rosetta_utc
#![cfg(feature = "postgres")]

use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::prelude::*;
use rosetta_utc::TimestampUTC;
use std::env;

diesel::table! {
    /// The test table for PostgreSQL integration tests.
    test_table (id) {
        /// The primary key column.
        id -> Integer,
        /// The UTC timestamp column.
        created_at -> rosetta_utc::diesel_impls::TimestampUTC,
    }
}

mod stored {
    diesel::table! {
        /// The test table seen through diesel's raw `TIMESTAMPTZ` encoding.
        test_table (id) {
            /// The primary key column.
            id -> Integer,
            /// The microseconds since 2000-01-01T00:00:00Z.
            created_at -> Timestamptz,
        }
    }
}

#[derive(Queryable, Insertable, Debug, PartialEq)]
#[diesel(table_name = test_table)]
struct TestEntity {
    id: i32,
    created_at: TimestampUTC,
}

fn with_table(test: impl FnOnce(&mut PgConnection) -> QueryResult<()>) {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url).expect("Error connecting to database");

    conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
        // DDL has no typed form in diesel.
        diesel::sql_query(
            "CREATE TEMPORARY TABLE test_table (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ NOT NULL)",
        )
        .execute(conn)?;
        test(conn)
    });
}

fn timestamp(text: &str) -> TimestampUTC {
    text.parse().unwrap()
}

#[test]
fn test_postgres_roundtrip() {
    with_table(|conn| {
        // PostgreSQL keeps microseconds, so the value starts at that precision.
        let now = jiff::Timestamp::from_microsecond(TimestampUTC::now().as_microsecond()).unwrap();
        let entity = TestEntity {
            id: 1,
            created_at: now.into(),
        };

        diesel::insert_into(test_table::table)
            .values(&entity)
            .execute(conn)?;

        let result: TestEntity = test_table::table.find(1).first(conn)?;
        assert_eq!(result, entity);
        Ok(())
    });
}

/// Writes `value` through `TimestampUTC` and returns the `TIMESTAMPTZ` PostgreSQL stored.
fn stored_micros(conn: &mut PgConnection, id: i32, value: &str) -> QueryResult<PgTimestamp> {
    diesel::insert_into(test_table::table)
        .values(&TestEntity {
            id,
            created_at: timestamp(value),
        })
        .execute(conn)?;
    stored::test_table::table
        .find(id)
        .select(stored::test_table::created_at)
        .first(conn)
}

#[test]
fn test_postgres_encodes_microseconds_since_2000() {
    let cases = [
        ("2000-01-01T00:00:00Z", 0),
        ("2023-10-27T10:00:00.123456Z", 751_716_000_123_456),
        ("1969-12-31T23:59:59.999999Z", -946_684_800_000_001),
    ];

    with_table(|conn| {
        for (id, (value, micros)) in (0..).zip(cases) {
            assert_eq!(
                stored_micros(conn, id, value)?,
                PgTimestamp(micros),
                "writing {value}"
            );
        }

        for (id, (value, micros)) in (100..).zip(cases) {
            diesel::insert_into(stored::test_table::table)
                .values((
                    stored::test_table::id.eq(id),
                    stored::test_table::created_at.eq(PgTimestamp(micros)),
                ))
                .execute(conn)?;
            let result: TestEntity = test_table::table.find(id).first(conn)?;
            assert_eq!(result.created_at, timestamp(value), "reading {micros}");
        }
        Ok(())
    });
}

#[test]
fn test_postgres_truncates_nanoseconds_toward_2000() {
    let cases = [
        ("2023-10-27T10:00:00.123456789Z", 751_716_000_123_456),
        ("1999-12-31T23:59:59.9999995Z", 0),
    ];

    with_table(|conn| {
        for (id, (value, micros)) in (0..).zip(cases) {
            assert_eq!(
                stored_micros(conn, id, value)?,
                PgTimestamp(micros),
                "writing {value}"
            );
        }
        Ok(())
    });
}

#[test]
fn test_postgres_rejects_infinity() {
    with_table(|conn| {
        diesel::insert_into(stored::test_table::table)
            .values((
                stored::test_table::id.eq(1),
                stored::test_table::created_at.eq(PgTimestamp(i64::MAX)),
            ))
            .execute(conn)?;
        assert!(test_table::table.find(1).first::<TestEntity>(conn).is_err());
        Ok(())
    });
}
