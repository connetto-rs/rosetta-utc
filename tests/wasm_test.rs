//! WASM tests for rosetta-utc crate.
#![cfg(target_arch = "wasm32")]

use rosetta_utc::TimestampUTC;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_now() {
    let now = TimestampUTC::now();
    assert!(now.as_second() > 0);
}

#[wasm_bindgen_test]
fn test_conversions() {
    let now = TimestampUTC::now();
    let parsed: TimestampUTC = now.to_string().parse().unwrap();
    assert_eq!(now, parsed);
}
