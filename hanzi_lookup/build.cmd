cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/hanzi_lookup.wasm --out-dir ./dist --no-modules --no-typescript
