use crate::mna::{bjts, junctions, mosfets, newton, opamps, BjtModel, Junction, MosModel, Stamper, MAX_NEWTON_ITERS};
use crate::{Circuit, PartKind::*, SimError};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DcResult {
    /// index = NodeId.0, [0] = 0.0
    pub node_voltages: Vec<f64>,
    /// by part id. Resistor/inductor/switch: a->b. VSource: pos->neg through the source. ISource: from->to. Diode/LED: anode->cathode. Bjt: current into the collector. Mosfet: current into the drain. Opamp: current into the output pin.
    pub part_currents: HashMap<String, f64>,
}

const OPEN_SWITCH_SIEMENS: f64 = 1e-12;

/// Rejects an empty circuit and non-finite / non-positive part values.
pub(crate) fn validate(c: &Circuit) -> Result<(), SimError> {
    if c.parts.is_empty() {
        return Err(SimError::EmptyCircuit);
    }
    for p in &c.parts {
        let ok = match p.kind {
            Resistor { ohms, .. } => ohms.is_finite() && ohms > 0.0,
            Capacitor { farads: x, .. } | Inductor { henries: x, .. } | VSource { volts: x, .. } | ISource { amps: x, .. } => x.is_finite(),
            Bjt { beta, .. } => beta.is_finite() && beta > 0.0,
            Opamp { gain, vsat, .. } => gain.is_finite() && gain > 0.0 && vsat.is_finite() && vsat > 0.0,
            Mosfet { vth, kp, .. } => vth.is_finite() && vth > 0.0 && kp.is_finite() && kp > 0.0,
            _ => true,
        };
        if !ok {
            return Err(SimError::InvalidInput(format!("part {} has an invalid value (must be finite, and resistance above zero)", p.id)));
        }
    }
    Ok(())
}

/// DC operating point. Diodes/LEDs are solved by Newton iteration (see `mna::newton`); a linear circuit is one solve.
pub fn dc_op(c: &Circuit) -> Result<DcResult, SimError> {
    dc_op_with_limit(c, MAX_NEWTON_ITERS)
}

/// `dc_op` with a custom Newton iteration cap. Test hook only: lets tests force `NoConvergence`.
#[doc(hidden)]
pub fn dc_op_with_limit(c: &Circuit, max_iters: u32) -> Result<DcResult, SimError> {
    validate(c)?;
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

    // Every node needs a DC path to ground (capacitors, current sources and open switches carry none; diodes/LEDs count via gmin).
    let mut uf: Vec<usize> = (0..n).collect();
    fn find(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x { uf[x] = uf[uf[x]]; x = uf[x]; }
        x
    }
    for p in &c.parts {
        if matches!(p.kind, Resistor { .. } | Inductor { .. } | VSource { .. } | Diode { .. } | Led { .. } | Bjt { .. } | Mosfet { .. } | Switch { closed: true, .. }) {
            let ns = p.nodes();
            for &y in &ns[1..] {
                let (rx, ry) = (find(&mut uf, ns[0] as usize), find(&mut uf, y as usize));
                uf[rx] = ry;
            }
        }
    }
    for p in &c.parts {
        if let Opamp { out, .. } = p.kind {
            let (ro, r0) = (find(&mut uf, out.0 as usize), find(&mut uf, 0));
            uf[ro] = r0;
        }
    }
    let g = find(&mut uf, 0);
    if let Some(i) = (1..n).find(|&i| find(&mut uf, i) != g) {
        return Err(SimError::FloatingNode(i as u32));
    }

    // Branch unknowns: voltage sources, inductors and closed switches (0 V in DC).
    let mut branch_of = HashMap::new();
    for p in &c.parts {
        if matches!(p.kind, VSource { .. } | Inductor { .. } | Opamp { .. } | Switch { closed: true, .. }) {
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
            Opamp { out, .. } => s.voltage_source(k.unwrap(), out.0, 0, 0.0),
            Capacitor { .. } | Diode { .. } | Led { .. } | Bjt { .. } | Mosfet { .. } => {} // open in DC / junctions stamped by newton
        }
    }
    let cause = "voltage sources in a loop or a source across a zero-ohm path";
    let (js, qs, ms, os) = (junctions(c), bjts(c), mosfets(c), opamps(c, &branch_of));
    let x = if js.is_empty() && qs.is_empty() && ms.is_empty() && os.is_empty() { s.solve(cause)? } else { newton(&s, &js, &qs, &ms, &os, &vec![0.0; n], max_iters, cause)? };

    let mut node_voltages = vec![0.0; n];
    node_voltages[1..].copy_from_slice(&x[..n - 1]);
    let mut part_currents = HashMap::new();
    for p in &c.parts {
        let v = |i: crate::NodeId| node_voltages[i.0 as usize];
        let i = match p.kind {
            Resistor { a, b, ohms, .. } => (v(a) - v(b)) / ohms,
            ISource { amps, .. } => amps,
            Switch { a, b, closed: false } => (v(a) - v(b)) * OPEN_SWITCH_SIEMENS,
            VSource { .. } | Inductor { .. } | Opamp { .. } | Switch { .. } => x[n - 1 + branch_of[p.id.as_str()]],
            Diode { .. } | Led { .. } => Junction::of(p).unwrap().current(&node_voltages),
            Mosfet { .. } => MosModel::of(p).unwrap().drain_current(&node_voltages),
            Bjt { .. } => BjtModel::of(p).unwrap().collector_current(&node_voltages),
            Capacitor { .. } => 0.0,
        };
        part_currents.insert(p.id.clone(), i);
    }
    Ok(DcResult { node_voltages, part_currents })
}
