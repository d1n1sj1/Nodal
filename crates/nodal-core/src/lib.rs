pub mod ac;
pub mod damage;
pub mod dc;
mod error;
pub mod mna;
mod model;
pub mod spice;
pub mod transient;
pub use damage::{check_damage, DamageKind, DamageWarning};
pub use error::SimError;
pub use model::{Circuit, NodeId, Part, PartKind, Wave};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
