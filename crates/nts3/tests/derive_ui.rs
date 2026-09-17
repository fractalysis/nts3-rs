#[test]
fn derive_diagnostics_are_stable_and_actionable() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/parameters-pass.rs");
    tests.compile_fail("tests/ui/parameters-fail-*.rs");
}
