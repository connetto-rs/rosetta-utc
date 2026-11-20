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
/// on SQLite, with years before 1 AD as a minus sign and four digits.
const SIX_DIGIT_CASES: [(&str, &str); 8] = [
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
    ("0042-01-01T00:00:00Z", "0042-01-01 00:00:00.000000+00:00"),
    (
        "-000001-01-01T00:00:00Z",
        "-0001-01-01 00:00:00.000000+00:00",
    ),
    (
        "-000690-07-27T11:48:17.22Z",
        "-0690-07-27 11:48:17.220000+00:00",
    ),
    (
        "-008395-12-17T15:26:43Z",
        "-8395-12-17 15:26:43.000000+00:00",
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
        *result.created_at,
        jiff::Timestamp::from_microsecond(entity.created_at.as_microsecond()).unwrap()
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
fn test_sqlite_reads_stored_text_forms() {
    let mut connection = connection();
    let cases = [
        ("2026-11-19 12:34:57+00:00", "2026-11-19T12:34:57Z"),
        ("2026-11-19 12:34:57.120+00:00", "2026-11-19T12:34:57.12Z"),
        (
            "2026-11-19 12:34:57.123456789+00:00",
            "2026-11-19T12:34:57.123456789Z",
        ),
        ("2023-10-27 12:00:00+02:00", "2023-10-27T10:00:00Z"),
        ("2023-10-27T10:00:00.5Z", "2023-10-27T10:00:00.5Z"),
        ("2023-10-27 10:00:00", "2023-10-27T10:00:00Z"),
        ("2023-10-27 10:00", "2023-10-27T10:00:00Z"),
        ("2460245.0", "2023-10-27T12:00:00Z"),
        ("-0001-01-01 00:00:00+00:00", "-000001-01-01T00:00:00Z"),
        ("-8395-12-17 15:26:43.5+00:00", "-008395-12-17T15:26:43.5Z"),
    ];

    for (id, (text, expected)) in (0..).zip(cases) {
        insert_text(&mut connection, id, text);
        let entity: TestEntity = test_table::table.find(id).first(&mut connection).unwrap();
        assert_eq!(entity.created_at, timestamp(expected), "reading {text}");
    }
}

#[test]
fn test_sqlite_rejects_text_that_is_no_timestamp() {
    let mut connection = connection();
    // The second text is past the last instant jiff represents once its offset applies,
    // while the same wall clock read as UTC is still in range.
    for (id, text) in [(1, "yesterday"), (2, "9999-12-30 21:00:00-02:00")] {
        insert_text(&mut connection, id, text);
        let result = test_table::table
            .find(id)
            .first::<TestEntity>(&mut connection);
        assert!(result.is_err(), "reading {text} gave {result:?}");
    }
}
