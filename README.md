# kikiutils

[![crates.io](https://img.shields.io/crates/v/kikiutils)](https://crates.io/crates/kikiutils)
[![docs.rs](https://docs.rs/kikiutils/badge.svg)](https://docs.rs/kikiutils)
[![MSRV](https://img.shields.io/crates/msrv/kikiutils)](./Cargo.toml)
[![codecov][codecov-src]][codecov-href]
[![License][license-src]][license-href]

Feature-gated utilities for Rust services: a `tracing` logger with background writer threads, Tokio task management with graceful shutdown, and small concurrency helpers.

- [✨ Release Notes](./CHANGELOG.md)

## Installation

```bash
cargo add kikiutils
```

## Features

No features are enabled by default.

| Feature | Enables |
| --- | --- |
| `full` | Every feature below |
| `atomic-enum-cell` | `AtomicEnumCell`, an atomic cell for `u8`-backed enums |
| `global-allocator` | A `GlobalAllocator` alias for the platform's preferred allocator |
| `logger` | A `tracing` layer with background console output and per-target file output |
| `signal` | `wait_for_shutdown_signal` for the platform's shutdown signals |
| `task` | `TaskManager`, which tracks Tokio tasks and cancels them on shutdown |

## License

[MIT License](./LICENSE)

<!-- Badges -->
[codecov-href]: https://codecov.io/gh/kikiutils/rust
[codecov-src]: https://codecov.io/gh/kikiutils/rust/graph/badge.svg

[license-href]: https://github.com/kikiutils/rust/blob/main/LICENSE
[license-src]: https://img.shields.io/github/license/kikiutils/rust?colorA=18181b&colorB=28cf8d&style=flat
