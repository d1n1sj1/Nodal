use crate::dc::{dc_op, validate};
use crate::mna::{bjts, junctions, mosfets, newton, opamps, BjtModel, Junction, MosModel, Stamper, MAX_NEWTON_ITERS};
use crate::{Circuit, NodeId, PartKind::*, SimError};
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

/// Circuit state at one instant, enough to continue a run: node voltages plus every part's current
/// (capacitor/inductor history comes from these).
#[derive(Debug, Clone)]
pub struct TransientState {
    pub node_voltages: Vec<f64>,
    pub part_currents: HashMap<String, f64>,
}

impl TransientResult {
    /// State at the last time point, for `transient_from`.
    pub fn final_state(&self) -> TransientState {
        TransientState {
            node_voltages: self.node_voltages.last().cloned().unwrap_or_default(),
            part_currents: self.part_currents.iter().map(|(k, v)| (k.clone(), *v.last().unwrap())).collect(),
        }
    }
}

const OPEN_SWITCH_SIEMENS: f64 = 1e-12;
const MAX_STEPS: f64 = 1e7;
const MAX_VALUES: f64 = 5e7;
const CAUSE: &str = "transient matrix is singular: voltage sources in a loop or a source across a zero-ohm path";

/// Trapezoidal integration, started from rest (capacitors 0 V, inductors 0 A, sources switch on at t = 0).
/// Rest state is solved with dc_op on a copy where C is a 0 V source and L a 0 A source.
/// Linear circuits factor the matrix once; circuits with diodes/LEDs/BJTs/MOSFETs run Newton at every step.
pub fn transient(c: &Circuit, stop: f64, step: f64) -> Result<TransientResult, SimError> {
    check_run(c, stop, step)?;
    validate(c)?; // before C and L are swapped for sources below, so their values are still checked
    let mut rest = c.clone();
    for p in &mut rest.parts {
        match p.kind {
            Capacitor { a, b, .. } => p.kind = VSource { pos: a, neg: b, volts: 0.0, wave: None },
            Inductor { a, b, .. } => p.kind = ISource { from: a, to: b, amps: 0.0 },
            _ => {}
        }
    }
    let dc = dc_op(&rest)?;
    run(c, TransientState { node_voltages: dc.node_voltages, part_currents: dc.part_currents }, stop, step)
}

/// Like `transient`, but starts from `initial` (e.g. another run's `final_state()`) instead of rest.
/// The circuit may differ from the one that produced `initial` in values, not in topology or part ids.
pub fn transient_from(c: &Circuit, initial: &TransientState, stop: f64, step: f64) -> Result<TransientResult, SimError> {
    validate(c)?;
    check_run(c, stop, step)?;
    if initial.node_voltages.len() != c.node_count() || c.parts.iter().any(|p| !initial.part_currents.contains_key(&p.id)) {
        return Err(SimError::InvalidInput("initial state does not match the circuit's nodes and parts".into()));
    }
    if initial.node_voltages.iter().chain(initial.part_currents.values()).any(|x| !x.is_finite()) {
        return Err(SimError::InvalidInput("initial state contains a non-finite value".into()));
    }
    run(c, initial.clone(), stop, step)
}

fn check_run(c: &Circuit, stop: f64, step: f64) -> Result<(), SimError> {
    if !(step.is_finite() && step > 0.0 && stop.is_finite() && stop > 0.0) {
        return Err(SimError::InvalidInput("transient stop time and step must be finite and above zero".into()));
    }
    if stop / step > MAX_STEPS {
        return Err(SimError::InvalidInput("transient run would need more than 10 million steps; use a larger step".into()));
    }
    // Bounds stored f64s (node voltages + one current per part), ~400 MB at the cap.
    if (stop / step + 1e-9).floor() * (c.node_count() + c.parts.len()) as f64 > MAX_VALUES {
        return Err(SimError::InvalidInput("transient result would be too large; use a larger step or a shorter run".into()));
    }
    Ok(())
}

fn run(c: &Circuit, init: TransientState, stop: f64, step: f64) -> Result<TransientResult, SimError> {
    let steps = (stop / step + 1e-9).floor() as usize;
    let n = c.node_count();
    let mut branch_of = HashMap::new();
    for p in &c.parts {
        if matches!(p.kind, VSource { .. } | Inductor { .. } | Opamp { .. } | Switch { closed: true, .. }) {
            branch_of.insert(p.id.as_str(), branch_of.len());
        }
    }
    // Matrix row of each part's branch unknown (unused for parts without one); avoids per-step map lookups.
    let rows: Vec<usize> = c.parts.iter().map(|p| branch_of.get(p.id.as_str()).map_or(0, |k| n - 1 + k)).collect();
    let vd = |v: &[f64], a: NodeId, b: NodeId| v[a.0 as usize] - v[b.0 as usize];

    let mut s = Stamper::new(n, branch_of.len());
    for p in &c.parts {
        match p.kind {
            Resistor { a, b, ohms, .. } => s.conductance(a.0, b.0, 1.0 / ohms),
            ISource { from, to, amps } => s.current(from.0, to.0, amps),
            VSource { pos, neg, volts, .. } => s.voltage_source(branch_of[p.id.as_str()], pos.0, neg.0, volts),
            Switch { a, b, closed: true } => s.voltage_source(branch_of[p.id.as_str()], a.0, b.0, 0.0),
            Switch { a, b, closed: false } => s.conductance(a.0, b.0, OPEN_SWITCH_SIEMENS),
            Capacitor { a, b, farads } => s.conductance(a.0, b.0, 2.0 * farads / step),
            Inductor { a, b, henries } => {
                let k = branch_of[p.id.as_str()];
                s.voltage_source(k, a.0, b.0, 0.0);
                s.branch_resistance(k, 2.0 * henries / step);
            }
            Opamp { out, .. } => s.voltage_source(branch_of[p.id.as_str()], out.0, 0, 0.0), // control terms stamped per Newton iteration
            Diode { .. } | Led { .. } | Bjt { .. } | Mosfet { .. } => {} // stamped per Newton iteration
        }
    }
    let base = s.rhs.clone();
    let mut rhs = base.clone();
    let (js, qs, ms, os) = (junctions(c), bjts(c), mosfets(c), opamps(c, &branch_of));
    // Linear fast path: factor once. Nonlinear: newton re-stamps from a clone of `s` every iteration.
    let fast = if js.is_empty() && qs.is_empty() && ms.is_empty() && os.is_empty() { Some(s.clone().factor(CAUSE)?) } else { None };

    // Per-part history: current through and voltage across (a - b) at the previous point.
    let mut ist: Vec<f64> = c.parts.iter().map(|p| init.part_currents[&p.id]).collect();
    let mut vst: Vec<f64> = c.parts.iter().map(|p| match p.kind {
        Capacitor { a, b, .. } | Inductor { a, b, .. } => vd(&init.node_voltages, a, b),
        _ => 0.0,
    }).collect();

    let mut times = Vec::with_capacity(steps + 1);
    times.push(0.0);
    let mut currents: Vec<Vec<f64>> = ist.iter().map(|&i| { let mut v = Vec::with_capacity(steps + 1); v.push(i); v }).collect();
    let mut node_voltages = Vec::with_capacity(steps + 1);
    node_voltages.push(init.node_voltages);

    let mut order = None; // sparse column order, computed once and reused by every Newton solve
    for k in 1..=steps {
        rhs.copy_from_slice(&base);
        for (i, p) in c.parts.iter().enumerate() {
            match p.kind {
                Capacitor { a, b, farads } => {
                    let hist = 2.0 * farads / step * vst[i] + ist[i];
                    if a.0 != 0 { rhs[a.0 as usize - 1] += hist; }
                    if b.0 != 0 { rhs[b.0 as usize - 1] -= hist; }
                }
                VSource { volts, wave: Some(w), .. } => rhs[rows[i]] = w.at(volts, k as f64 * step),
                Inductor { henries, .. } => rhs[rows[i]] = -(2.0 * henries / step * ist[i] + vst[i]),
                _ => {}
            }
        }
        let x = match &fast {
            Some(f) => f.solve(&rhs),
            None => {
                let mut st = s.clone();
                st.rhs.copy_from_slice(&rhs);
                newton(&st, &js, &qs, &ms, &os, node_voltages.last().unwrap(), MAX_NEWTON_ITERS, CAUSE, k == 1, &mut order)?
            }
        };
        let mut v = vec![0.0; n];
        v[1..].copy_from_slice(&x[..n - 1]);
        for (i, p) in c.parts.iter().enumerate() {
            let cur = match p.kind {
                Resistor { a, b, ohms, .. } => vd(&v, a, b) / ohms,
                ISource { amps, .. } => amps,
                Switch { a, b, closed: false } => vd(&v, a, b) * OPEN_SWITCH_SIEMENS,
                VSource { .. } | Opamp { .. } | Switch { .. } => x[rows[i]],
                Inductor { a, b, .. } => { vst[i] = vd(&v, a, b); x[rows[i]] }
                Capacitor { a, b, farads } => {
                    let vn = vd(&v, a, b);
                    let cur = 2.0 * farads / step * (vn - vst[i]) - ist[i];
                    vst[i] = vn;
                    cur
                }
                Diode { .. } | Led { .. } => Junction::of(p).unwrap().current(&v),
                Mosfet { .. } => MosModel::of(p).unwrap().drain_current(&v),
                Bjt { .. } => BjtModel::of(p).unwrap().collector_current(&v),
            };
            ist[i] = cur;
            currents[i].push(cur);
        }
        times.push(k as f64 * step);
        node_voltages.push(v);
    }
    let part_currents = c.parts.iter().map(|p| p.id.clone()).zip(currents).collect();
    Ok(TransientResult { times, node_voltages, part_currents })
}
