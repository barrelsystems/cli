/// Drives the real compiled `barrel` binary end-to-end via a `tests/cmd/*.trycmd`
/// fixture corpus, mirroring the pattern `clap`'s own repository uses for its
/// `tests/examples.rs`. `trycmd` is parser-agnostic (it drives the process at
/// the stdout/exit-code level via `escargot`), so this exercises real argument
/// parsing and real command dispatch, not internal function calls.
#[test]
fn cli_behavior() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
