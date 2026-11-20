# rosetta-utc

[![CI](https://github.com/connetto-rs/rosetta-utc/actions/workflows/ci.yml/badge.svg)](https://github.com/connetto-rs/rosetta-utc/actions/workflows/ci.yml)
[![Security Audit](https://github.com/connetto-rs/rosetta-utc/workflows/Security%20Audit/badge.svg)](https://github.com/connetto-rs/rosetta-utc/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Codecov](https://codecov.io/gh/connetto-rs/rosetta-utc/branch/main/graph/badge.svg)](https://codecov.io/gh/connetto-rs/rosetta-utc)
[![Crates.io](https://img.shields.io/crates/v/rosetta-utc.svg)](https://crates.io/crates/rosetta-utc)
[![Docs.rs](https://docs.rs/rosetta-utc/badge.svg)](https://docs.rs/rosetta-utc)

A wrapper of [`jiff::Timestamp`](https://docs.rs/jiff/latest/jiff/struct.Timestamp.html) providing binary `diesel` bindings for SQLite and PostgreSQL.

## Why it exists

This crate bridges the gap, providing a unified `TimestampUTC` type that:

* In **PostgreSQL**, maps to `TIMESTAMPTZ` (Timestamp with time zone), with [`Timestamptz`](https://docs.diesel.rs/2.0.x/diesel/sql_types/struct.Timestamptz.html) sql type.
* In **SQLite**, maps to `TEXT`, storing `YYYY-MM-DD HH:MM:SS.ffffff+00:00` with six fractional digits, the text a `TIMESTAMPTZ` replicated from PostgreSQL holds, and reading text with any offset or none, and Julian day numbers.

The two SQL types are remapped to the same Rust type, `TimestampUTC`, which internally uses `jiff::Timestamp`.

This ensures consistent UTC handling across both databases, preventing common timezone-related bugs in distributed applications.

## Features

This crate provides a `TimestampUTC` wrapper type that implements various traits based on enabled features:

* **[`diesel`](https://crates.io/crates/diesel)**: Enables Diesel integration.
  * **`postgres`**: Enables `TimestampUTC` support for [PostgreSQL](https://www.postgresql.org/docs/current/datatype-datetime.html).
  * **`sqlite`**: Enables `TimestampUTC` support for [SQLite](https://www.sqlite.org/datatype3.html).
* **[`serde`](https://crates.io/crates/serde)**: Enables serialization and deserialization via [Serde](https://serde.rs/).
* **`wasm`**: Enables support for `TimestampUTC::now()` on `wasm32-unknown-unknown` targets by enabling `jiff/js`.

## Usage

Add this to your `Cargo.toml`. Select the features matching your database requirements.

```toml
[dependencies]
rosetta-utc = { version = "0.2", features = ["diesel", "postgres", "sqlite", "serde"] }
```

### Example

```rust
use rosetta_utc::TimestampUTC;

// Get current UTC time
let now = TimestampUTC::now();

// Parse from string (RFC 3339), any offset is converted to UTC
let parsed: TimestampUTC = "2023-10-27T12:00:00+02:00".parse().unwrap();
assert_eq!(parsed.to_string(), "2023-10-27T10:00:00Z");

// Access the underlying jiff::Timestamp through Deref
let earlier = TimestampUTC::from(*now - jiff::SignedDuration::from_secs(300));
assert!(earlier < now);
assert_eq!(jiff::Timestamp::from(earlier), *earlier);
```
