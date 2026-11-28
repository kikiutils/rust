//! [`GlobalAllocator`] is jemalloc on Linux, mimalloc on Windows, and the system allocator elsewhere. Enabling
//! the feature does not install it; the binary still declares
//! `#[global_allocator] static GLOBAL: GlobalAllocator = GlobalAllocator;`.

#[cfg(not(any(windows, target_os = "linux")))]
pub use std::alloc::System as GlobalAllocator;

#[cfg(windows)]
pub use mimalloc::MiMalloc as GlobalAllocator;
#[cfg(target_os = "linux")]
pub use tikv_jemallocator::Jemalloc as GlobalAllocator;
