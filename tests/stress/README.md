# Stress Tests

Gateway stress tests are implemented as crate integration tests under:

- `crates/nexis-gateway/tests/stress.rs`
- `crates/nexis-gateway/tests/stress/mod.rs`

Run them with:

```bash
cargo test -p nexis-gateway --test stress -- --ignored
```
