# `test_docs`

Scan through a directory. For every Rust file, find all the functions marked with `#[test]`. Extract the docstrings for those functions. Emit a JSON object with the information.

To be used to catalog and organize the scientific validation tests encoded in unit tests.

Use `npm run collect-test-docstrings`.
