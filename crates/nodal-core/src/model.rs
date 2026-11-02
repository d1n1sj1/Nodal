use crate::error::SimError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PartKind {
    Resistor { a: NodeId, b: NodeId, ohms: f64, max_watts: f64 },
    Capacitor { a: NodeId, b: NodeId, farads: f64 },
    Inductor { a: NodeId, b: NodeId, henries: f64 },
    VSource { pos: NodeId, neg: NodeId, volts: f64 },
    ISource { from: NodeId, to: NodeId, amps: f64 },
    Diode { anode: NodeId, cathode: NodeId },
    Led { anode: NodeId, cathode: NodeId, max_amps: f64 },
    Switch { a: NodeId, b: NodeId, closed: bool },
    /// Ebers-Moll transistor (Is = 1e-14 A, reverse beta 1). `beta` = forward current gain; `max_amps` = collector current limit.
    /// Level-1 (square-law) MOSFET, bulk tied to source, no channel-length modulation. `vth` and `kp` (A/V^2) are
    /// magnitudes for both polarities; `max_amps` = drain current limit.
    Mosfet { d: NodeId, g: NodeId, s: NodeId, pmos: bool, vth: f64, kp: f64, max_amps: f64 },
    Bjt { c: NodeId, b: NodeId, e: NodeId, pnp: bool, beta: f64, max_amps: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    #[serde(flatten)]
    pub kind: PartKind,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Circuit {
    pub parts: Vec<Part>,
}

const FILE_VERSION: u64 = 1;
// keep in step with PartKind: tests/model.rs round-trips every kind and fails if one is missing here.
const KINDS: [&str; 10] = ["Resistor", "Capacitor", "Inductor", "VSource", "ISource", "Diode", "Led", "Switch", "Bjt", "Mosfet"];

impl Circuit {
    /// Highest NodeId used + 1 (ground counts); 0 when empty.
    pub fn node_count(&self) -> usize {
        self.parts.iter().flat_map(|p| p.nodes()).max().map_or(0, |m| m as usize + 1)
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({ "version": FILE_VERSION, "parts": self.parts }).to_string()
    }

    pub fn from_json(s: &str) -> Result<Circuit, SimError> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|e| SimError::BadFile(e.to_string()))?;
        if let Some(ver) = v.get("version").and_then(|x| x.as_u64()).filter(|&x| x > FILE_VERSION) {
            return Err(SimError::BadFile(format!("file version {ver} was made by a newer version of Nodal")));
        }
        let raw = v.get("parts").and_then(|p| p.as_array()).ok_or_else(|| SimError::BadFile("missing \"parts\" list".into()))?;
        let mut parts = Vec::with_capacity(raw.len());
        for p in raw {
            if let Some(k) = p.get("kind").and_then(|k| k.as_str()) {
                if !KINDS.contains(&k) {
                    return Err(SimError::UnknownPartKind(k.to_string()));
                }
            }
            let part: Part = serde_json::from_value(p.clone()).map_err(|e| SimError::BadFile(e.to_string()))?;
            // solvers key branch rows by id; a repeat would index past the matrix
            if parts.iter().any(|q: &Part| q.id == part.id) {
                return Err(SimError::BadFile(format!("duplicate part id \"{}\"", part.id)));
            }
            parts.push(part);
        }
        Ok(Circuit { parts })
    }
}

impl Part {
    pub(crate) fn nodes(&self) -> Vec<u32> {
        use PartKind::*;
        let (x, y) = match &self.kind {
            Bjt { c, b, e, .. } => return vec![c.0, b.0, e.0],
            Mosfet { d, g, s, .. } => return vec![d.0, g.0, s.0],
            Resistor { a, b, .. } | Capacitor { a, b, .. } | Inductor { a, b, .. } | Switch { a, b, .. } => (a, b),
            VSource { pos, neg, .. } => (pos, neg),
            ISource { from, to, .. } => (from, to),
            Diode { anode, cathode } | Led { anode, cathode, .. } => (anode, cathode),
        };
        vec![x.0, y.0]
    }
}
