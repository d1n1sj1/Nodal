use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SimError {
    UnknownPartKind(String),
    NoGround,
    FloatingNode(u32),
    Singular(String),
    InvalidInput(String),
    NoConvergence { iterations: u32 },
    EmptyCircuit,
    BadFile(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPartKind(k) => write!(f, "Unknown part kind \"{k}\". This file may have been made by a newer version of Nodal."),
            Self::NoGround => write!(f, "The circuit has no ground connection."),
            Self::FloatingNode(n) => write!(f, "Node {n} is not connected to ground."),
            Self::Singular(m) => write!(f, "The circuit cannot be solved: {m}"),
            Self::InvalidInput(m) => write!(f, "Invalid input: {m}"),
            Self::NoConvergence { iterations } => write!(f, "The solver did not converge after {iterations} iterations."),
            Self::EmptyCircuit => write!(f, "The circuit is empty."),
            Self::BadFile(m) => write!(f, "Could not read the circuit file: {m}"),
        }
    }
}

impl std::error::Error for SimError {}
