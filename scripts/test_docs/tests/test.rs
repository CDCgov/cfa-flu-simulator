#[allow(dead_code)]

/// This is a top-level function. It is not a test.
fn not_a_test() {}

/// This is a top-level function. It *is* a test.
#[test]
/// And I put some docs below the attribute macro.
fn test_split_docstring() {}

#[test]
#[doc = "This is an explicit attribute macro docstring"]
fn test_explicit_docstring() {}

#[cfg(test)]
mod test {
    #[test]
    /// This is a test function.
    ///
    /// It has a blank line in the middle.
    fn test_in_module_multiline() {}

    #[test]
    fn test_no_docstring() {}
}
