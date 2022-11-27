# ESP (Ikea) Vindriktning in Rust 🦀

## Overview

TODO

## Notes

- needs custom fork of Rust to support ESP chips <s>https://github.com/esp-rs/rust-build</s> now via https://github.com/esp-rs/espup
- <s>to update Rust ESP version, run `./install-rust-toolchain.sh --toolchain-version 1.xx.0.0`</s> now via `espup`
- `cargo build` needs `source export-esp.sh` first
- to make this work with `rust-analyzer`, add this to your config:

  ```json
    "rust-analyzer.server.extraEnv": {
      "LIBCLANG_PATH": "/Users/tomas.vojtasek/.espressif/tools/xtensa-esp32-elf-clang/esp-14.0.0-20220415-aarch64-apple-darwin/lib/"
    },
  ```

  The path has to be absolute, won't work with the leading `~/`

- binary larger than 1 MB won't flash without `partition.csv` file (1MB is probably a default value)

- `opt-level = "s"` is currenty broken in `rustc 1.65.0` (miscompilation issues)
