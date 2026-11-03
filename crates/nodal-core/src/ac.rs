use crate::dc::{dc_op, validate};
use crate::mna::{linearize, Stamper};
use crate::{Circuit, PartKind::*, SimError};
use std::collections::HashMap;

/// Result of an AC sweep: node voltages for a 1 V AC source, per frequency.
#[derive(Debug, Clone)]
pub struct AcResult {
    pub freqs: Vec<f64>,
    /// [freq][node] in volts (= gain from the source); node 0 = ground = 0.0
    pub magnitude: Vec<Vec<f64>>,
    /// [freq][node] in degrees, -180..180
    pub phase_deg: Vec<Vec<f64>>,
}

const MAX_UNKNOWNS: usize = 300;
const MAX_POINTS: f64 = 20_000.0;
const OPEN_SWITCH_SIEMENS: f64 = 1e-12;

#[derive(Clone, Copy, Default)]
struct Cx { re: f64, im: f64 }

impl Cx {
    fn mul(self, o: Cx) -> Cx { Cx { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re } }
    fn sub(self, o: Cx) -> Cx { Cx { re: self.re - o.re, im: self.im - o.im } }
    fn abs2(self) -> f64 { self.re * self.re + self.im * self.im }
    fn div(self, o: Cx) -> Cx {
        let d = o.abs2();
        Cx { re: (self.re * o.re + self.im * o.im) / d, im: (self.im * o.re - self.re * o.im) / d }
    }
}

/// Dense complex Gaussian elimination with partial pivoting.
/// shortcut: dense, so capped at 300 unknowns; use a sparse complex LU to go larger.
fn solve(mut a: Vec<Vec<Cx>>, mut b: Vec<Cx>) -> Result<Vec<Cx>, SimError> {
    let n = b.len();
    let scale: Vec<f64> = (0..n).map(|c| a.iter().map(|r| r[c].abs2().sqrt()).fold(0.0, f64::max)).collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs2().total_cmp(&a[y][c].abs2())).unwrap();
        if !(a[p][c].abs2().sqrt() > 1e-12 * scale[c]) {
            return Err(SimError::Singular("AC matrix is singular: voltage sources in a loop or a source across a zero-ohm path".into()));
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..n {
            let f = a[r][c].div(a[c][c]);
            if f.abs2() == 0.0 { continue; }
            for k in c..n { let t = f.mul(a[c][k]); a[r][k] = a[r][k].sub(t); }
            let t = f.mul(b[c]);
            b[r] = b[r].sub(t);
        }
    }
    for c in (0..n).rev() {
        let mut s = b[c];
        for k in c + 1..n { s = s.sub(a[c][k].mul(b[k])); }
        b[c] = s.div(a[c][c]);
    }
    Ok(b)
}

/// Small-signal frequency sweep. Solves the DC operating point, linearises every part around it, then for each
/// frequency solves the complex system with voltage source `source` as a 1 V AC stimulus (all other sources
/// zero). `points_per_decade` points per decade from `f_start` to `f_stop` inclusive, log spaced.
pub fn ac_sweep(c: &Circuit, source: &str, f_start: f64, f_stop: f64, points_per_decade: u32) -> Result<AcResult, SimError> {
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
    if !c.parts.iter().any(|p| p.id == source && matches!(p.kind, VSource { .. })) {
        return Err(SimError::InvalidInput(format!("AC source \"{source}\" is not a voltage source in this circuit")));
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
    if size > MAX_UNKNOWNS {
        return Err(SimError::InvalidInput(format!("AC analysis supports up to {MAX_UNKNOWNS} unknowns (this circuit has {size})")));
    }

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
    let mut base = vec![vec![Cx::default(); size]; size];
    for &(r, col, v) in g.entries() { base[r][col].re += v; }
    let mut rhs = vec![Cx::default(); size];
    rhs[n - 1 + branch_of[source]].re = 1.0;

    let points = (decades * points_per_decade as f64).ceil() as usize + 1;
    let ratio = f_stop / f_start;
    let mut out = AcResult { freqs: Vec::with_capacity(points), magnitude: Vec::with_capacity(points), phase_deg: Vec::with_capacity(points) };
    for i in 0..points {
        let f = if i + 1 == points { f_stop } else { f_start * ratio.powf(i as f64 / (points - 1) as f64) };
        let w = 2.0 * std::f64::consts::PI * f;
        let mut a = base.clone();
        for &(na, nb, farads) in &caps {
            let y = w * farads;
            let (ia, ib) = ((na != 0).then(|| na as usize - 1), (nb != 0).then(|| nb as usize - 1));
            if let Some(i) = ia { a[i][i].im += y; }
            if let Some(j) = ib { a[j][j].im += y; }
            if let (Some(i), Some(j)) = (ia, ib) { a[i][j].im -= y; a[j][i].im -= y; }
        }
        for &(row, henries) in &inds { a[row][row].im -= w * henries; }
        let x = solve(a, rhs.clone())?;
        let mut mag = vec![0.0; n];
        let mut ph = vec![0.0; n];
        for (k, v) in x[..n - 1].iter().enumerate() {
            mag[k + 1] = v.abs2().sqrt();
            ph[k + 1] = v.im.atan2(v.re).to_degrees();
        }
        out.freqs.push(f);
        out.magnitude.push(mag);
        out.phase_deg.push(ph);
    }
    Ok(out)
}
