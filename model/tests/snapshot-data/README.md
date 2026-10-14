# Model Regression Snapshots

The test fixture `snapshots.json` is generated from the native Rust model.
To update it after an intentional model change:

```sh
cargo run -p cfa-flu-simulator --bin update_snapshot
```

The generator can also write the same JSON document to standard output:

```sh
cargo run -p cfa-flu-simulator --bin update_snapshot -- --stdout
```

*Note*: The fixture *generation* is mildly platform-dependent.
Fixture generation will conflate model changes and platform changes.
The test suite uses a finite tolerance that ignores these changes: each value may differ from the fixture by up to 1e-4 of its series' peak (or of itself, for the summary totals).
Measured cross-platform drift is about 2e-5.
