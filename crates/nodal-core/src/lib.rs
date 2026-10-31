pub mod damage;
pub mod dc;
mod error;
pub mod mna;
mod model;
pub mod transient;
pub use damage::{check_damage, DamageKind, DamageWarning};
pub use error::SimError;
pub use model::{Circuit, NodeId, Part, PartKind};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
