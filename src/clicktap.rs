//! While the loupe is up, a click should pick the color instead of reaching the app underneath.
//! Each OS has its own backend in `platform/`; all expose `install(tx, ctx)` and `set_sampling(on)`.

#[cfg(target_os = "macos")]
#[path = "platform/macos_clicktap.rs"]
mod imp;
#[cfg(target_os = "windows")]
#[path = "platform/windows_clicktap.rs"]
mod imp;
#[cfg(target_os = "linux")]
#[path = "platform/linux_clicktap.rs"]
mod imp;

pub use imp::*;
