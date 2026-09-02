//! System API Services for Control Center


/// Audio lives in the shared crate; re-exported so `api::audio::…` keeps working.
pub use shell_common::api::audio;
pub mod bluetooth;
pub mod brightness;
pub mod compositor;
pub mod media;
pub mod network;
pub mod session;
