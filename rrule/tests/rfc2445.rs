//! Runs google-rfc-2445's iterator tests (tests/data/rfc2445_iter_tests.txt, Apache-2.0)
//! against this crate. EXRULE cases need `--features exrule`.
mod common;

#[test]
fn rfc2445_corpus() {
    let path = std::env::var("RFC2445_CORPUS").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/rfc2445_iter_tests.txt"
        )
        .into()
    });
    common::corpus::run("rfc2445", &path);
}
