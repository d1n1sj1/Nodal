use crate::dc::{dc_op, topology, validate};
use crate::mna::{bjts, junctions, mosfets, newton, nvec, opamps, regions, BjtModel, ColOrder, Factor, Junction, MosModel, OpampModel, Stamper, MAX_NEWTON_ITERS};
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

const OPEN_SWITCH_SIEMENS: f64 = 1e-12;
const MAX_STEPS: f64 = 1e7;
const MAX_VALUES: f64 = 5e7;
/// Backward Euler steps in a row after a switching event or source edge.
const BE_STEPS: u32 = 3;
const CAUSE: &str = "transient matrix is singular: voltage sources in a loop or a source across a zero-ohm path";

/// Trapezoidal integration, started from rest (capacitors 0 V, inductors 0 A, sources at their t = 0 value).
/// Rest state is solved with dc_op on a copy where C is a 0 V source and L a 0 A source. A wiring with no such
/// solution (parallel capacitors, a capacitor across a source, series inductors) starts from one tiny backward Euler
/// step instead; capacitor currents at t = 0 are then reported as 0.
/// A step that switches a junction or transistor, or crosses a square or sawtooth edge, is redone with backward
/// Euler, and so are the next two: trapezoidal rings after a discontinuity, and one damped step is not enough
/// when the circuit is stiff compared with the step.
/// Linear circuits factor the matrix once; circuits with diodes/LEDs/BJTs/MOSFETs run Newton at every step.
pub fn transient(c: &Circuit, stop: f64, step: f64) -> Result<TransientResult, SimError> {
    check_run(c, stop, step)?;
    validate(c)?; // before C and L are swapped for sources below, so their values are still checked
    topology(c)?;
    let mut rest = c.clone();
    for p in &mut rest.parts {
        match p.kind {
            Capacitor { a, b, .. } => p.kind = VSource { pos: a, neg: b, volts: 0.0, wave: None },
            Inductor { a, b, .. } => p.kind = ISource { from: a, to: b, amps: 0.0 },
            VSource { pos, neg, volts, wave: Some(w) } => p.kind = VSource { pos, neg, volts: w.at(volts, 0.0), wave: None },
            _ => {}
        }
    }
    let sim = Sim::new(c);
    let mut order = None;
    let (v0, vst, ist, impulse) = match dc_op(&rest) {
        Ok(dc) => {
            let vst = c.parts.iter().map(|p| match p.kind {
                Capacitor { a, b, .. } | Inductor { a, b, .. } => dc.node_voltages[a.0 as usize] - dc.node_voltages[b.0 as usize],
                _ => 0.0,
            }).collect();
            let ist = c.parts.iter().map(|p| dc.part_currents[&p.id]).collect();
            (dc.node_voltages, vst, ist, false)
        }
        Err(e @ (SimError::Singular(_) | SimError::FloatingNode(_))) => {
            let h = step * 1e-6;
            let (mut vst, mut ist) = (vec![0.0; c.parts.len()], vec![0.0; c.parts.len()]);
            let mode = Mode::new(&sim, h, 1.0).map_err(|_| e.clone())?;
            let x = mode.solve(&sim, 1.0, h, 0.0, &vec![0.0; sim.n], &vst, &ist, true, &mut order).map_err(|_| e)?;
            let v = nvec(&x, sim.n);
            sim.update(&x, &v, h, 1.0, &mut vst, &mut ist);
            // the impulse is not a current anyone could measure; the first steps are backward Euler, which ignores it
            for (i, p) in c.parts.iter().enumerate() {
                if matches!(p.kind, Capacitor { .. }) { ist[i] = 0.0; }
            }
            (v, vst, ist, true)
        }
        Err(e) => return Err(e),
    };
    sim.run(v0, vst, ist, impulse, stop, step, order)
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

/// Backward Euler (`a` = 1) or trapezoidal (`a` = 2) companion system for one step size, with its factorisation when linear.
struct Mode {
    s: Stamper,
    fast: Option<Factor>,
}

impl Mode {
    fn new(sim: &Sim, h: f64, a: f64) -> Result<Mode, SimError> {
        let s = sim.stamp(h, a);
        let fast = if sim.linear() { Some(s.clone().factor(CAUSE)?) } else { None };
        Ok(Mode { s, fast })
    }

    /// Solves the step of size `h` ending at `t`. `vprev` = node voltages at the previous point (Newton start),
    /// `vst`/`ist` = voltage across and current through each part there.
    #[allow(clippy::too_many_arguments)]
    fn solve(&self, sim: &Sim, a: f64, h: f64, t: f64, vprev: &[f64], vst: &[f64], ist: &[f64], rails: bool, order: &mut ColOrder) -> Result<Vec<f64>, SimError> {
        let mut rhs = self.s.rhs.clone();
        for (i, p) in sim.c.parts.iter().enumerate() {
            match p.kind {
                Capacitor { a: na, b: nb, farads } => {
                    let hist = a * farads / h * vst[i] + (a - 1.0) * ist[i];
                    if na.0 != 0 { rhs[na.0 as usize - 1] += hist; }
                    if nb.0 != 0 { rhs[nb.0 as usize - 1] -= hist; }
                }
                VSource { volts, wave: Some(w), .. } => rhs[sim.rows[i]] = w.at(volts, t),
                Inductor { henries, .. } => rhs[sim.rows[i]] = -(a * henries / h * ist[i] + (a - 1.0) * vst[i]),
                _ => {}
            }
        }
        match &self.fast {
            Some(f) => Ok(f.solve(&rhs)),
            None => {
                let mut st = self.s.clone();
                st.rhs = rhs;
                newton(&st, &sim.js, &sim.qs, &sim.ms, &sim.os, vprev, MAX_NEWTON_ITERS, CAUSE, rails, order)
            }
        }
    }
}

struct Sim<'a> {
    c: &'a Circuit,
    n: usize,
    branches: usize,
    /// Matrix row of each part's branch unknown (unused for parts without one); avoids per-step map lookups.
    rows: Vec<usize>,
    js: Vec<Junction>,
    qs: Vec<BjtModel>,
    ms: Vec<MosModel>,
    os: Vec<OpampModel>,
}

impl<'a> Sim<'a> {
    fn new(c: &'a Circuit) -> Self {
        let n = c.node_count();
        let mut branch_of = HashMap::new();
        for p in &c.parts {
            if matches!(p.kind, VSource { .. } | Inductor { .. } | Opamp { .. } | Switch { closed: true, .. }) {
                branch_of.insert(p.id.as_str(), branch_of.len());
            }
        }
        let rows = c.parts.iter().map(|p| branch_of.get(p.id.as_str()).map_or(0, |k| n - 1 + k)).collect();
        Sim { c, n, branches: branch_of.len(), rows, js: junctions(c), qs: bjts(c), ms: mosfets(c), os: opamps(c, &branch_of) }
    }

    fn linear(&self) -> bool {
        self.js.is_empty() && self.qs.is_empty() && self.ms.is_empty() && self.os.is_empty()
    }

    fn stamp(&self, h: f64, a: f64) -> Stamper {
        let mut s = Stamper::new(self.n, self.branches);
        for (i, p) in self.c.parts.iter().enumerate() {
            let k = self.rows[i].wrapping_sub(self.n - 1); // only read for parts that have a branch
            match p.kind {
                Resistor { a, b, ohms, .. } => s.conductance(a.0, b.0, 1.0 / ohms),
                ISource { from, to, amps } => s.current(from.0, to.0, amps),
                VSource { pos, neg, volts, .. } => s.voltage_source(k, pos.0, neg.0, volts),
                Switch { a, b, closed: true } => s.voltage_source(k, a.0, b.0, 0.0),
                Switch { a, b, closed: false } => s.conductance(a.0, b.0, OPEN_SWITCH_SIEMENS),
                Capacitor { a: na, b, farads } => s.conductance(na.0, b.0, a * farads / h),
                Inductor { a: na, b, henries } => {
                    s.voltage_source(k, na.0, b.0, 0.0);
                    s.branch_resistance(k, a * henries / h);
                }
                Opamp { out, .. } => s.voltage_source(k, out.0, 0, 0.0), // control terms stamped per Newton iteration
                Diode { .. } | Led { .. } | Bjt { .. } | Mosfet { .. } => {} // stamped per Newton iteration
            }
        }
        s
    }

    /// Part currents and C/L voltage history after a solved step of size `h`.
    fn update(&self, x: &[f64], v: &[f64], h: f64, a: f64, vst: &mut [f64], ist: &mut [f64]) {
        let vd = |a: NodeId, b: NodeId| v[a.0 as usize] - v[b.0 as usize];
        for (i, p) in self.c.parts.iter().enumerate() {
            ist[i] = match p.kind {
                Resistor { a, b, ohms, .. } => vd(a, b) / ohms,
                ISource { amps, .. } => amps,
                Switch { a, b, closed: false } => vd(a, b) * OPEN_SWITCH_SIEMENS,
                VSource { .. } | Opamp { .. } | Switch { .. } => x[self.rows[i]],
                Inductor { a, b, .. } => { vst[i] = vd(a, b); x[self.rows[i]] }
                Capacitor { a: na, b, farads } => {
                    let vn = vd(na, b);
                    let cur = a * farads / h * (vn - vst[i]) - (a - 1.0) * ist[i];
                    vst[i] = vn;
                    cur
                }
                Diode { .. } | Led { .. } => Junction::of(p).unwrap().current(v),
                Mosfet { .. } => MosModel::of(p).unwrap().drain_current(v),
                Bjt { .. } => BjtModel::of(p).unwrap().collector_current(v),
            };
        }
    }

    /// True when a square or sawtooth source has an edge between `t0` and `t1`.
    fn edge(&self, t0: f64, t1: f64) -> bool {
        self.c.parts.iter().any(|p| matches!(p.kind, VSource { wave: Some(w), .. } if w.jumps(t0, t1)))
    }

    #[allow(clippy::too_many_arguments)]
    fn run(&self, init_v: Vec<f64>, mut vst: Vec<f64>, mut ist: Vec<f64>, impulse: bool, stop: f64, step: f64, mut order: ColOrder) -> Result<TransientResult, SimError> {
        let steps = (stop / step + 1e-9).floor() as usize;
        let tr = Mode::new(self, step, 2.0)?;
        let mut be: Option<Mode> = None; // built on first use
        let mut be_left = 0;

        let mut times = Vec::with_capacity(steps + 1);
        times.push(0.0);
        let mut currents: Vec<Vec<f64>> = ist.iter().map(|&i| { let mut v = Vec::with_capacity(steps + 1); v.push(i); v }).collect();
        let mut sig = regions(&self.js, &self.qs, &self.ms, &init_v);
        let mut node_voltages = Vec::with_capacity(steps + 1);
        node_voltages.push(init_v);

        for k in 1..=steps {
            let t = k as f64 * step;
            if (k == 1 && impulse) || self.edge(t - step, t) { be_left = BE_STEPS; }
            let v = loop {
                let use_be = be_left > 0;
                let a = if use_be { 1.0 } else { 2.0 };
                if use_be && be.is_none() { be = Some(Mode::new(self, step, 1.0)?); }
                let mode = if use_be { be.as_ref().unwrap() } else { &tr };
                match mode.solve(self, a, step, t, node_voltages.last().unwrap(), &vst, &ist, k == 1, &mut order) {
                    Ok(x) => {
                        let v = nvec(&x, self.n);
                        if !use_be && regions(&self.js, &self.qs, &self.ms, &v) != sig {
                            be_left = BE_STEPS; // a device switched during this step
                            continue;
                        }
                        self.update(&x, &v, step, a, &mut vst, &mut ist);
                        break v;
                    }
                    Err(e) => return Err(e),
                }
            };
            be_left = be_left.saturating_sub(1);
            sig = regions(&self.js, &self.qs, &self.ms, &v);
            for (cur, &i) in currents.iter_mut().zip(&ist) { cur.push(i); }
            times.push(t);
            node_voltages.push(v);
        }
        let part_currents = self.c.parts.iter().map(|p| p.id.clone()).zip(currents).collect();
        Ok(TransientResult { times, node_voltages, part_currents })
    }
}
