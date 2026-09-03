# Model Regression Snapshots

These fixtures are generated from the native Rust model. Normal test compare
against fixtures but do not regenerate snapshots.

To update the committed fixtures after an intentional model change, run this
from `model/`:

```sh
cargo test update_snapshots -- --ignored --nocapture
```
