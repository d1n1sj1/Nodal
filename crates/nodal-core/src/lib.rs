mod error;
mod model;
pub use error::SimError;
pub use model::{Circuit, NodeId, Part, PartKind};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
