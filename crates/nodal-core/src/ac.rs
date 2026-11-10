use crate::dc::{dc_op, validate};
use crate::mna::{linearize, Stamper};
use crate::{Circuit, PartKind::*, SimError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of an AC sweep: node voltages for the AC stimuli (1 V for `ac_sweep`), per frequency.
#[derive(Debug, Clone)]
pub struct AcResult {
    pub freqs: Vec<f64>,
    /// [freq][node] in volts (= gain for a 1 V stimulus); node 0 = ground = 0.0
    pub magnitude: Vec<Vec<f64>>,
    /// [freq][node] in degrees, -180..180
    pub phase_deg: Vec<Vec<f64>>,
}

const MAX_POINTS: f64 = 20_000.0;
const OPEN_SWITCH_SIEMENS: f64 = 1e-12;

/// One voltage source driven at AC: peak `amp` volts at `phase_deg` degrees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcStimulus {
    pub id: String,
    pub amp: f64,
    pub phase_deg: f64,
}

/// `ac_sweep_multi` with voltage source `source` as the only stimulus, 1 V at 0 degrees.
pub fn ac_sweep(c: &Circuit, source: &str, f_start: f64, f_stop: f64, points_per_decade: u32) -> Result<AcResult, SimError> {
    ac_sweep_multi(c, &[AcStimulus { id: source.into(), amp: 1.0, phase_deg: 0.0 }], f_start, f_stop, points_per_decade)
}

/// Small-signal frequency sweep. Solves the DC operating point, linearises every part around it, then for each
/// frequency solves the complex system with every voltage source in `stimuli` driven at once (all other sources
/// zero), so the result is the sum of their responses. `points_per_decade` points per decade from `f_start` to
/// `f_stop` inclusive, log spaced.
pub fn ac_sweep_multi(c: &Circuit, stimuli: &[AcStimulus], f_start: f64, f_stop: f64, points_per_decade: u32) -> Result<AcResult, SimError> {
    validate(c)?;
    if !(f_start.is_finite() && f_start > 0.0 && f_stop.is_finite() && f_stop > f_start) {
        return Err(SimError::InvalidInput("AC start and stop frequencies must be finite, above zero, and start below stop".into()));
    }
    if points_per_decade == 0 {
        return Err(SimError::InvalidInput("AC sweep needs at least 1 point per decade".into()));
    }
    let decades = (f_stop / f_start).log10();
    if decades * points_per_decade as f64 + 1.0 > MAX_POINTS {
        return Err(SimError::InvalidInput("AC sweep would need more than 20,000 points; use fewer points per decade".into()));
    }
    for (i, s) in stimuli.iter().enumerate() {
        let id = &s.id;
        if !c.parts.iter().any(|p| p.id == *id && matches!(p.kind, VSource { .. })) {
            return Err(SimError::InvalidInput(format!("AC source \"{id}\" is not a voltage source in this circuit")));
        }
        if !(s.amp.is_finite() && s.phase_deg.is_finite()) {
            return Err(SimError::InvalidInput(format!("AC source \"{id}\" needs a finite amplitude and phase")));
        }
        if stimuli[..i].iter().any(|o| o.id == *id) {
            return Err(SimError::InvalidInput(format!("AC source \"{id}\" is listed twice")));
        }
    }
    if !stimuli.iter().any(|s| s.amp != 0.0) {
        return Err(SimError::InvalidInput("AC sweep needs at least one source with a nonzero amplitude".into()));
    }
    let op = dc_op(c)?.node_voltages;
    let n = c.node_count();

    let mut branch_of = HashMap::new();
    for p in &c.parts {
        if matches!(p.kind, VSource { .. } | Inductor { .. } | Opamp { .. } | Switch { closed: true, .. }) {
            branch_of.insert(p.id.as_str(), branch_of.len());
        }
    }
    let size = n - 1 + branch_of.len();

    // Frequency-independent part (real), plus the reactive parts to add per frequency.
    let mut g = Stamper::new(n, branch_of.len());
    let mut caps = Vec::new(); // (a, b, farads)
    let mut inds = Vec::new(); // (branch row, henries)
    for p in &c.parts {
        let k = branch_of.get(p.id.as_str()).copied();
        match p.kind {
            Resistor { a, b, ohms, .. } => g.conductance(a.0, b.0, 1.0 / ohms),
            Capacitor { a, b, farads } => caps.push((a.0, b.0, farads)),
            Inductor { a, b, henries } => {
                g.voltage_source(k.unwrap(), a.0, b.0, 0.0);
                inds.push((n - 1 + k.unwrap(), henries));
            }
            VSource { pos, neg, .. } => g.voltage_source(k.unwrap(), pos.0, neg.0, 0.0),
            Switch { a, b, closed: true } => g.voltage_source(k.unwrap(), a.0, b.0, 0.0),
            Switch { a, b, closed: false } => g.conductance(a.0, b.0, OPEN_SWITCH_SIEMENS),
            Opamp { out, .. } => g.voltage_source(k.unwrap(), out.0, 0, 0.0),
            ISource { .. } | Diode { .. } | Led { .. } | Bjt { .. } | Mosfet { .. } => {} // sources are zero; devices come from linearize
        }
    }
    linearize(&mut g, c, &op, &branch_of);
    // Complex system (Gr + jGi)(xr + jxi) = br as one real system [Gr -Gi; Gi Gr][xr; xi] = [br; 0], so the
    // sparse real LU (and its cached column order) does the work.
    let mut base: Vec<(usize, usize, f64)> = Vec::with_capacity(2 * g.entries().len());
    for &(r, col, v) in g.entries() { base.extend([(r, col, v), (r + size, col + size, v)]); }
    let mut rhs = vec![0.0; 2 * size];
    for s in stimuli {
        let (sin, cos) = s.phase_deg.to_radians().sin_cos();
        let row = n - 1 + branch_of[s.id.as_str()];
        (rhs[row], rhs[row + size]) = (s.amp * cos, s.amp * sin);
    }
    let mut order = None;

    let points = (decades * points_per_decade as f64).ceil() as usize + 1;
    let ratio = f_stop / f_start;
    let mut out = AcResult { freqs: Vec::with_capacity(points), magnitude: Vec::with_capacity(points), phase_deg: Vec::with_capacity(points) };
    for i in 0..points {
        let f = if i + 1 == points { f_stop } else { f_start * ratio.powf(i as f64 / (points - 1) as f64) };
        let w = 2.0 * std::f64::consts::PI * f;
        let mut a = base.clone();
        let mut imag = |r: usize, col: usize, y: f64| a.extend([(r, col + size, -y), (r + size, col, y)]);
        for &(na, nb, farads) in &caps {
            let y = w * farads;
            let (ia, ib) = ((na != 0).then(|| na as usize - 1), (nb != 0).then(|| nb as usize - 1));
            if let Some(i) = ia { imag(i, i, y); }
            if let Some(j) = ib { imag(j, j, y); }
            if let (Some(i), Some(j)) = (ia, ib) { imag(i, j, -y); imag(j, i, -y); }
        }
        for &(row, henries) in &inds { imag(row, row, -w * henries); }
        let cause = "AC matrix is singular: voltage sources in a loop or a source across a zero-ohm path";
        let x = Stamper::from_entries(a, rhs.clone()).solve_cached(cause, &mut order)?;
        let mut mag = vec![0.0; n];
        let mut ph = vec![0.0; n];
        for k in 0..n - 1 {
            let (re, im) = (x[k], x[k + size]);
            mag[k + 1] = re.hypot(im);
            ph[k + 1] = im.atan2(re).to_degrees();
        }
        out.freqs.push(f);
        out.magnitude.push(mag);
        out.phase_deg.push(ph);
    }
    Ok(out)
}
