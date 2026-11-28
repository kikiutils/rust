//! jemalloc on Linux, mimalloc on Windows, and the system allocator on every other target.

#[cfg(not(any(windows, target_os = "linux")))]
pub use std::alloc::System as GlobalAllocator;

#[cfg(windows)]
pub use mimalloc::MiMalloc as GlobalAllocator;
#[cfg(target_os = "linux")]
pub use tikv_jemallocator::Jemalloc as GlobalAllocator;
