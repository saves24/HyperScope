// Platform abstraction: same function names for linux/windows; callers use platform::xxx
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
pub(crate) mod service;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::*;
#[cfg(target_os = "windows")]
pub(crate) use windows::*;
