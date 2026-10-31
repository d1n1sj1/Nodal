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

const KINDS: [&str; 8] = ["Resistor", "Capacitor", "Inductor", "VSource", "ISource", "Diode", "Led", "Switch"];

impl Circuit {
    /// Highest NodeId used + 1 (ground counts); 0 when empty.
    pub fn node_count(&self) -> usize {
        self.parts.iter().flat_map(|p| p.nodes()).max().map_or(0, |m| m as usize + 1)
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({ "version": 1, "parts": self.parts }).to_string()
    }

    pub fn from_json(s: &str) -> Result<Circuit, SimError> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|e| SimError::BadFile(e.to_string()))?;
        let raw = v.get("parts").and_then(|p| p.as_array()).ok_or_else(|| SimError::BadFile("missing \"parts\" list".into()))?;
        let mut parts = Vec::with_capacity(raw.len());
        for p in raw {
            if let Some(k) = p.get("kind").and_then(|k| k.as_str()) {
                if !KINDS.contains(&k) {
                    return Err(SimError::UnknownPartKind(k.to_string()));
                }
            }
            parts.push(serde_json::from_value(p.clone()).map_err(|e| SimError::BadFile(e.to_string()))?);
        }
        Ok(Circuit { parts })
    }
}

impl Part {
    pub(crate) fn nodes(&self) -> [u32; 2] {
        use PartKind::*;
        let (x, y) = match &self.kind {
            Resistor { a, b, .. } | Capacitor { a, b, .. } | Inductor { a, b, .. } | Switch { a, b, .. } => (a, b),
            VSource { pos, neg, .. } => (pos, neg),
            ISource { from, to, .. } => (from, to),
            Diode { anode, cathode } | Led { anode, cathode, .. } => (anode, cathode),
        };
        [x.0, y.0]
    }
}
