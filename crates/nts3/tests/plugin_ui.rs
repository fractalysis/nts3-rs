#[test]
fn plugin_attribute_diagnostics_are_stable_and_actionable() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/plugin-pass.rs");
    tests.compile_fail("tests/ui/plugin-fail-*.rs");
}
