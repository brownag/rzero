# Contributing

This project is not actively seeking external contributions. Bug reports and small fixes are welcome.

## Before opening a PR

1. Run the test suite:

   ```bash
   cargo test
   ```

2. If you touch `src/lib.rs` or FFI exports, verify the WebAssembly build:

   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

3. Keep `Cargo.toml` free of third-party dependencies.
4. Match the behavior of base GNU R (`Rscript --vanilla`) for any supported language feature, or trap explicitly as specified in [docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md).
