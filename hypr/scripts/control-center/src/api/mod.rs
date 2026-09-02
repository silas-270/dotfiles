//! System API Services for Control Center


/// Audio and media live in the shared crate; re-exported so `api::audio::…`
/// and `api::media::…` keep working.
pub use shell_common::api::{audio, connectivity, media};
pub mod bluetooth;
pub mod brightness;
pub mod compositor;
pub mod network;
pub mod session;
