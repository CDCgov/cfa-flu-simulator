# ASPR-flumodels reference comparison

`generate_reference_aspr.R` creates a Cartesian product of Rust-compatible
model parameter sets and ASPR-specific execution parameter sets, runs
[ASPR-flumodels](https://github.com/HHS/ASPR-flumodels) on every combination,
and outputs the results in `reference_aspr.json`. Each result records its
`rust_parameters` and `aspr_parameters`; the latter currently varies the
solver between `lsoda` and `rk4`.
This test fixture is git-commit.
This update is meant to be infrequent, not run as part of CI.
The test fixture is used by `model/src/reference_aspr.rs` for testing.

From the top level, run:

```bash
Rscript scripts/aspr_reference/generate_reference_aspr.R
```
