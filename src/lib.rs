// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

/// Define the MAX supported stack depth. TODO: make this variable mutable.
#[cfg(feature = "large-depth")]
pub const MAX_DEPTH: usize = 1024;

#[cfg(all(feature = "huge-depth", not(feature = "large-depth")))]
pub const MAX_DEPTH: usize = 512;

#[cfg(not(any(feature = "large-depth", feature = "huge-depth")))]
pub const MAX_DEPTH: usize = 128;

// todo replace with kindasafe
mod addr_validate;

mod collector;
mod error;
pub mod framehop_unwinder;
mod frames;
mod profiler;
mod shlib;
mod timer;

pub use self::addr_validate::validate;
pub use self::collector::{Collector, HashCounter};
pub use self::error::{Error, Result};
pub use self::frames::Frame;
pub use self::profiler::{ProfilerGuard, ProfilerGuardBuilder};
