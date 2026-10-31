use crate::mna::Stamper;
use crate::{Circuit, PartKind::*, SimError};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DcResult {
    /// index = NodeId.0, [0] = 0.0
    pub node_voltages: Vec<f64>,
    /// by part id. Resistor/inductor/switch: a->b. VSource: pos->neg through the source. ISource: from->to.
    pub part_currents: HashMap<String, f64>,
}

const OPEN_SWITCH_SIEMENS: f64 = 1e-12;

pub fn dc_op(c: &Circuit) -> Result<DcResult, SimError> {
    if c.parts.is_empty() {
        return Err(SimError::EmptyCircuit);
    }
    let n = c.node_count();
    let mut pins = vec![0u32; n];
    for p in &c.parts {
        for k in p.nodes() { pins[k as usize] += 1; }
    }
    if pins[0] == 0 {
        return Err(SimError::NoGround);
    }
    if let Some(i) = (1..n).find(|&i| pins[i] < 2) {
        return Err(SimError::FloatingNode(i as u32));
    }

    // Branch unknowns: voltage sources, inductors and closed switches (0 V in DC).
    let mut branch_of = HashMap::new();
    for p in &c.parts {
        if matches!(p.kind, VSource { .. } | Inductor { .. } | Switch { closed: true, .. }) {
            branch_of.insert(p.id.as_str(), branch_of.len());
        }
    }
    let mut s = Stamper::new(n, branch_of.len());
    for p in &c.parts {
        let k = branch_of.get(p.id.as_str()).copied();
        match p.kind {
            Resistor { a, b, ohms, .. } => s.conductance(a.0, b.0, 1.0 / ohms),
            ISource { from, to, amps } => s.current(from.0, to.0, amps),
            VSource { pos, neg, volts } => s.voltage_source(k.unwrap(), pos.0, neg.0, volts),
            Inductor { a, b, .. } | Switch { a, b, closed: true } => s.voltage_source(k.unwrap(), a.0, b.0, 0.0),
            Switch { a, b, closed: false } => s.conductance(a.0, b.0, OPEN_SWITCH_SIEMENS),
            Capacitor { .. } | Diode { .. } | Led { .. } => {} // open in DC / nonlinear parts arrive in Task 5
        }
    }
    let x = s.solve()?;

    let mut node_voltages = vec![0.0; n];
    node_voltages[1..].copy_from_slice(&x[..n - 1]);
    let mut part_currents = HashMap::new();
    for p in &c.parts {
        let v = |i: crate::NodeId| node_voltages[i.0 as usize];
        let i = match p.kind {
            Resistor { a, b, ohms, .. } => (v(a) - v(b)) / ohms,
            ISource { amps, .. } => amps,
            Switch { a, b, closed: false } => (v(a) - v(b)) * OPEN_SWITCH_SIEMENS,
            VSource { .. } | Inductor { .. } | Switch { .. } => x[n - 1 + branch_of[p.id.as_str()]],
            Capacitor { .. } | Diode { .. } | Led { .. } => 0.0,
        };
        part_currents.insert(p.id.clone(), i);
    }
    Ok(DcResult { node_voltages, part_currents })
}
