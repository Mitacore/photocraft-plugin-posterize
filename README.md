# Posterize: a PhotoCraft plug-in

A WebAssembly filter plug-in (PhotoCraft plug-in ABI v1) built from the `invert-rs` example.
It reduces each colour channel to a chosen number of levels (2 to 32) and leaves alpha alone. Samples outside 0 to 1 (32-bit HDR) are clamped.

## Install

1. Download `posterize.wasm` from the latest Release.
2. Put it in your PhotoCraft plug-ins folder (Edit > Preferences > Plug-ins > Additional Plug-ins Folder), then restart PhotoCraft.
3. Use it from Filter > Plug-ins > Posterize (Plug-in).

Optional: check the file against `posterize.wasm.sha256` (`Get-FileHash posterize.wasm`).

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --locked --target wasm32-unknown-unknown
```

License: MIT.
