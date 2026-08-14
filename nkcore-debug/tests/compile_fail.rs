/// Compiler-facing regression tests for invalid macro inputs.
#[cfg(test)]
mod test {
    /// Verifies invalid macro inputs produce stable compiler diagnostics.
    #[test]
    fn invalid_api_name_inputs_produce_compile_errors() {
        let test_cases = trybuild::TestCases::new();
        test_cases.compile_fail("tests/ui/*.rs");
    }
}
