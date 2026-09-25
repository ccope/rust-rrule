//! Runs ical4j's Recur expansion tests (tests/data/ical4j_recur_tests.txt, BSD-3-Clause)
//! against this crate.
mod common;

#[test]
fn ical4j_corpus() {
    let path = std::env::var("ICAL4J_CORPUS").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/ical4j_recur_tests.txt"
        )
        .into()
    });
    common::corpus::run("ical4j", &path);
}
