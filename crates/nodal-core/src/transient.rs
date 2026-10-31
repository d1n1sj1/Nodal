use crate::dc::dc_op;
use crate::mna::Stamper;
use crate::{Circuit, NodeId, Part, PartKind::*, SimError};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TransientResult {
    /// stop/step + 1 points, including t = 0.
    pub times: Vec<f64>,
    /// [step][node], node 0 = ground = 0.0
    pub node_voltages: Vec<Vec<f64>>,
    /// by part id, one value per time point; same sign conventions as `DcResult`. Capacitor: a->b.
    pub part_currents: HashMap<String, Vec<f64>>,
}

const OPEN_SWITCH_SIEMENS: f64 = 1e-12;
const MAX_STEPS: f64 = 1e7;
const MAX_VALUES: f64 = 5e7;
const CAUSE: &str = "transient matrix is singular: voltage sources in a loop or a source across a zero-ohm path";

/// Trapezoidal integration of a linear circuit, started from rest (capacitors 0 V, inductors 0 A, sources
/// switch on at t = 0). Rest state is solved with dc_op on a copy where C is a 0 V source and L a 0 A source.
/// Diodes/LEDs are skipped until Task 5. The matrix is factored once and reused for every step.
pub fn transient(c: &Circuit, stop: f64, step: f64) -> Result<TransientResult, SimError> {
    if !(step.is_finite() && step > 0.0 && stop.is_finite() && stop > 0.0) {
        return Err(SimError::Singular("transient stop time and step must be finite and above zero".into()));
    }
    if stop / step > MAX_STEPS {
        return Err(SimError::Singular("transient run would need more than 10 million steps; use a larger step".into()));
    }
    let steps = (stop / step + 1e-9).floor() as usize;
    // Bounds stored f64s (node voltages + one current per part), ~400 MB at the cap.
    if steps as f64 * (c.node_count() + c.parts.len()) as f64 > MAX_VALUES {
        return Err(SimError::Singular("transient result would be too large; use a larger step or a shorter run".into()));
    }

    let mut rest = c.clone();
    for p in &mut rest.parts {
        match p.kind {
            Capacitor { farads: x, .. } | Inductor { henries: x, .. } if !(x.is_finite() && x > 0.0) => {
                return Err(SimError::Singular(format!("part {} has an invalid value (must be finite and above zero)", p.id)));
            }
            Capacitor { a, b, .. } => p.kind = VSource { pos: a, neg: b, volts: 0.0 },
            Inductor { a, b, .. } => p.kind = ISource { from: a, to: b, amps: 0.0 },
            _ => {}
        }
    }
    let dc = dc_op(&rest)?;

    let n = c.node_count();
    let mut branch_of = HashMap::new();
    for p in &c.parts {
        if matches!(p.kind, VSource { .. } | Inductor { .. } | Switch { closed: true, .. }) {
            branch_of.insert(p.id.as_str(), branch_of.len());
        }
    }
    let row = |p: &Part| n - 1 + branch_of[p.id.as_str()];
    let vd = |v: &[f64], a: NodeId, b: NodeId| v[a.0 as usize] - v[b.0 as usize];

    let mut s = Stamper::new(n, branch_of.len());
    for p in &c.parts {
        match p.kind {
            Resistor { a, b, ohms, .. } => s.conductance(a.0, b.0, 1.0 / ohms),
            ISource { from, to, amps } => s.current(from.0, to.0, amps),
            VSource { pos, neg, volts } => s.voltage_source(branch_of[p.id.as_str()], pos.0, neg.0, volts),
            Switch { a, b, closed: true } => s.voltage_source(branch_of[p.id.as_str()], a.0, b.0, 0.0),
            Switch { a, b, closed: false } => s.conductance(a.0, b.0, OPEN_SWITCH_SIEMENS),
            Capacitor { a, b, farads } => s.conductance(a.0, b.0, 2.0 * farads / step),
            Inductor { a, b, henries } => {
                let k = branch_of[p.id.as_str()];
                s.voltage_source(k, a.0, b.0, 0.0);
                s.branch_resistance(k, 2.0 * henries / step);
            }
            Diode { .. } | Led { .. } => {}
        }
    }
    let base = s.rhs.clone();
    let f = s.factor(CAUSE)?;

    // Per-part history: current through and voltage across (a - b) at the previous point.
    let mut ist: Vec<f64> = c.parts.iter().map(|p| dc.part_currents[&p.id]).collect();
    let mut vst: Vec<f64> = c.parts.iter().map(|p| match p.kind {
        Capacitor { a, b, .. } | Inductor { a, b, .. } => vd(&dc.node_voltages, a, b),
        _ => 0.0,
    }).collect();

    let mut times = vec![0.0];
    let mut part_currents: HashMap<String, Vec<f64>> = c.parts.iter().map(|p| (p.id.clone(), vec![dc.part_currents[&p.id]])).collect();
    let mut node_voltages = vec![dc.node_voltages];

    for k in 1..=steps {
        let mut rhs = base.clone();
        for (i, p) in c.parts.iter().enumerate() {
            match p.kind {
                Capacitor { a, b, farads } => {
                    let hist = 2.0 * farads / step * vst[i] + ist[i];
                    if a.0 != 0 { rhs[a.0 as usize - 1] += hist; }
                    if b.0 != 0 { rhs[b.0 as usize - 1] -= hist; }
                }
                Inductor { henries, .. } => rhs[row(p)] = -(2.0 * henries / step * ist[i] + vst[i]),
                _ => {}
            }
        }
        let x = f.solve(&rhs);
        let mut v = vec![0.0; n];
        v[1..].copy_from_slice(&x[..n - 1]);
        for (i, p) in c.parts.iter().enumerate() {
            let cur = match p.kind {
                Resistor { a, b, ohms, .. } => vd(&v, a, b) / ohms,
                ISource { amps, .. } => amps,
                Switch { a, b, closed: false } => vd(&v, a, b) * OPEN_SWITCH_SIEMENS,
                VSource { .. } | Switch { .. } => x[row(p)],
                Inductor { a, b, .. } => { vst[i] = vd(&v, a, b); x[row(p)] }
                Capacitor { a, b, farads } => {
                    let vn = vd(&v, a, b);
                    let cur = 2.0 * farads / step * (vn - vst[i]) - ist[i];
                    vst[i] = vn;
                    cur
                }
                Diode { .. } | Led { .. } => 0.0,
            };
            ist[i] = cur;
            part_currents.get_mut(&p.id).unwrap().push(cur);
        }
        times.push(k as f64 * step);
        node_voltages.push(v);
    }
    Ok(TransientResult { times, node_voltages, part_currents })
}
