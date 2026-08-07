pub mod fieldset;
pub mod connections;
pub mod controls;
pub mod media;
pub mod session;

pub use connections::ConnectionsSection;
pub use controls::{ControlsSection, DragTarget};
pub use media::{MediaSection, MediaClickResult};
pub use session::SessionSection;

#[derive(Debug, PartialEq, Eq)]
pub enum ActionResult {
    None,
    NeedsDraw,
    HidePanel,
}
