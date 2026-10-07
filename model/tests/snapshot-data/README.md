# Model Regression Snapshots

The test fixture `snapshots.json` is generated from the native Rust model.
To update it after an intentional model change:

```sh
cargo run -p cfa-flu-simulator --bin update_snapshot
```

*Note*: The fixture *generation* is mildly platform-dependent.
Fixture generation will conflate model changes and platform changes.
The test suite uses a finite tolerance that ignores these changes.
