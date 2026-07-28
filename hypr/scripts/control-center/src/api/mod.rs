//! System API Services for Control Center
//!
//! Separates hardware/system interactions (audio, brightness, network, bluetooth, compositor)
//! from GTK4 user interface elements.

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod compositor;
pub mod media;
pub mod network;
