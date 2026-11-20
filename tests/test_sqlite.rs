//! Tests for SQLite support in rosetta_utc
#![cfg(feature = "sqlite")]

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use rosetta_utc::TimestampUTC;

diesel::table! {
    /// The test table for SQLite integration tests.
    test_table (id) {
        /// The primary key column.
        id -> Integer,
        /// The UTC timestamp column.
        created_at -> rosetta_utc::diesel_impls::TimestampUTC,
    }
}

mod stored {
    diesel::table! {
        /// The test table seen as the text SQLite stores.
        test_table (id) {
            /// The primary key column.
            id -> Integer,
            /// The stored text of the timestamp column.
            created_at -> Text,
        }
    }
}

/// Instants and the text PostgreSQL's microsecond `timestamptz` holds for them
/// on SQLite.
const SIX_DIGIT_CASES: [(&str, &str); 4] = [
    ("2026-11-19T12:34:57Z", "2026-11-19 12:34:57.000000+00:00"),
    (
        "2026-11-19T12:34:57.12Z",
        "2026-11-19 12:34:57.120000+00:00",
    ),
    (
        "2026-11-19T12:34:57.123456789Z",
        "2026-11-19 12:34:57.123456+00:00",
    ),
    (
        "1969-12-31T23:59:59.999999999Z",
        "1969-12-31 23:59:59.999999+00:00",
    ),
];

#[derive(Queryable, Insertable, Debug, PartialEq)]
#[diesel(table_name = test_table)]
struct TestEntity {
    id: i32,
    created_at: TimestampUTC,
}

fn connection() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    // DDL has no typed form in diesel.
    diesel::sql_query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, created_at TEXT NOT NULL)")
        .execute(&mut connection)
        .unwrap();
    connection
}

fn timestamp(text: &str) -> TimestampUTC {
    text.parse().unwrap()
}

fn insert_text(connection: &mut SqliteConnection, id: i32, text: &str) {
    diesel::insert_into(stored::test_table::table)
        .values((
            stored::test_table::id.eq(id),
            stored::test_table::created_at.eq(text),
        ))
        .execute(connection)
        .unwrap();
}

#[test]
fn test_sqlite_roundtrip() {
    let mut connection = connection();
    let entity = TestEntity {
        id: 1,
        created_at: TimestampUTC::now(),
    };

    diesel::insert_into(test_table::table)
        .values(&entity)
        .execute(&mut connection)
        .unwrap();

    let result: TestEntity = test_table::table.find(1).first(&mut connection).unwrap();

    assert_eq!(result.id, entity.id);
    assert_eq!(
        result.created_at,
        TimestampUTC::from(chrono::SubsecRound::trunc_subsecs(*entity.created_at, 6))
    );
}

#[test]
fn test_sqlite_writes_six_fractional_digits() {
    let mut connection = connection();
    for (id, (value, expected)) in (0..).zip(SIX_DIGIT_CASES) {
        diesel::insert_into(test_table::table)
            .values(&TestEntity {
                id,
                created_at: timestamp(value),
            })
            .execute(&mut connection)
            .unwrap();

        let text: String = stored::test_table::table
            .find(id)
            .select(stored::test_table::created_at)
            .first(&mut connection)
            .unwrap();
        assert_eq!(text, expected, "writing {value}");
    }
}

#[test]
fn test_sqlite_equality_matches_six_digit_rows() {
    let mut connection = connection();
    for (id, (_, text)) in (0..).zip(SIX_DIGIT_CASES) {
        insert_text(&mut connection, id, text);
    }

    for (id, (value, _)) in (0..).zip(SIX_DIGIT_CASES) {
        let matches: Vec<i32> = test_table::table
            .filter(test_table::created_at.eq(timestamp(value)))
            .select(test_table::id)
            .load(&mut connection)
            .unwrap();
        assert_eq!(matches, [id], "filtering on {value}");
    }
}

#[test]
fn test_sqlite_reads_variable_fraction_rows() {
    let mut connection = connection();
    let cases = [
        ("2026-11-19 12:34:57+00:00", "2026-11-19T12:34:57Z"),
        ("2026-11-19 12:34:57.120+00:00", "2026-11-19T12:34:57.12Z"),
        (
            "2026-11-19 12:34:57.123456789+00:00",
            "2026-11-19T12:34:57.123456789Z",
        ),
    ];

    for (id, (text, expected)) in (0..).zip(cases) {
        insert_text(&mut connection, id, text);
        let entity: TestEntity = test_table::table.find(id).first(&mut connection).unwrap();
        assert_eq!(entity.created_at, timestamp(expected), "reading {text}");
    }
}
