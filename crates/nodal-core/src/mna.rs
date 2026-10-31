use crate::error::SimError;
use crate::{Circuit, Part, PartKind};

/// Dense MNA system. Unknowns: node k (k>=1) at index k-1, then one per branch. Ground (node 0) is skipped.
/// shortcut: dense Gaussian elimination, O(n^3); swap in a sparse backend (Task 6) behind the same methods.
#[derive(Clone)]
pub struct Stamper {
    nodes: usize, // non-ground node count
    a: Vec<Vec<f64>>,
    pub rhs: Vec<f64>,
}

impl Stamper {
    pub fn new(node_count: usize, branches: usize) -> Self {
        let nodes = node_count.saturating_sub(1);
        let n = nodes + branches;
        Stamper { nodes, a: vec![vec![0.0; n]; n], rhs: vec![0.0; n] }
    }

    fn idx(node: u32) -> Option<usize> {
        (node != 0).then(|| node as usize - 1)
    }

    pub fn conductance(&mut self, a: u32, b: u32, g: f64) {
        let (ia, ib) = (Self::idx(a), Self::idx(b));
        if let Some(i) = ia { self.a[i][i] += g; }
        if let Some(j) = ib { self.a[j][j] += g; }
        if let (Some(i), Some(j)) = (ia, ib) {
            self.a[i][j] -= g;
            self.a[j][i] -= g;
        }
    }

    /// Current `amps` flows from `from` through the source to `to`.
    pub fn current(&mut self, from: u32, to: u32, amps: f64) {
        if let Some(i) = Self::idx(from) { self.rhs[i] -= amps; }
        if let Some(i) = Self::idx(to) { self.rhs[i] += amps; }
    }

    /// Branch `k` forces v(pos)-v(neg)=volts; its unknown is the current flowing pos -> neg through the element.
    pub fn voltage_source(&mut self, k: usize, pos: u32, neg: u32, volts: f64) {
        let row = self.nodes + k;
        if let Some(i) = Self::idx(pos) { self.a[i][row] += 1.0; self.a[row][i] += 1.0; }
        if let Some(i) = Self::idx(neg) { self.a[i][row] -= 1.0; self.a[row][i] -= 1.0; }
        self.rhs[row] += volts;
    }

    /// Adds -`r` on branch `k`'s diagonal (v(pos)-v(neg)-r*i = rhs), for inductor companion models.
    pub fn branch_resistance(&mut self, k: usize, r: f64) {
        let row = self.nodes + k;
        self.a[row][row] -= r;
    }

    /// LU-factors the matrix (rhs ignored). Reuse the result for many right-hand sides.
    pub fn factor(mut self, cause: &str) -> Result<Factor, SimError> {
        let n = self.rhs.len();
        // Per-column scale makes the pivot test robust to wide resistor ranges.
        let scale: Vec<f64> = (0..n).map(|c| self.a.iter().map(|r| r[c].abs()).fold(0.0, f64::max)).collect();
        let mut perm: Vec<usize> = (0..n).collect();
        for c in 0..n {
            let p = (c..n).max_by(|&x, &y| self.a[x][c].abs().total_cmp(&self.a[y][c].abs())).unwrap();
            if !(self.a[p][c].abs() > 1e-12 * scale[c]) {
                return Err(SimError::Singular(cause.into()));
            }
            self.a.swap(c, p);
            perm.swap(c, p);
            for r in c + 1..n {
                let f = self.a[r][c] / self.a[c][c];
                self.a[r][c] = f; // multiplier kept in the lower triangle
                if f != 0.0 {
                    for k in c + 1..n { self.a[r][k] -= f * self.a[c][k]; }
                }
            }
        }
        Ok(Factor { lu: self.a, perm })
    }

    /// One-shot factor + solve of the stamped rhs; returns unknowns in index order.
    pub fn solve(self, cause: &str) -> Result<Vec<f64>, SimError> {
        let rhs = self.rhs.clone();
        Ok(self.factor(cause)?.solve(&rhs))
    }
}

/// LU factorisation with row permutation.
/// shortcut: dense; Task 6 swaps in a sparse backend behind the same `solve`.
pub struct Factor {
    lu: Vec<Vec<f64>>,
    perm: Vec<usize>,
}

impl Factor {
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let n = self.perm.len();
        let mut x: Vec<f64> = self.perm.iter().map(|&p| rhs[p]).collect();
        for r in 1..n {
            let s: f64 = (0..r).map(|k| self.lu[r][k] * x[k]).sum();
            x[r] -= s;
        }
        for c in (0..n).rev() {
            let s: f64 = (c + 1..n).map(|k| self.lu[c][k] * x[k]).sum();
            x[c] = (x[c] - s) / self.lu[c][c];
        }
        x
    }
}

/// Thermal voltage kT/q at about 300 K, in volts.
pub const VT: f64 = 0.02585;
const IS: f64 = 1e-14;
const GMIN: f64 = 1e-12;
pub const MAX_NEWTON_ITERS: u32 = 100;

/// A diode or LED junction (Shockley model, Is = 1e-14 A, n = 1 diode / 2 LED) plus gmin across it.
pub(crate) struct Junction {
    a: u32,
    k: u32,
    nvt: f64,
}

impl Junction {
    pub(crate) fn of(p: &Part) -> Option<Junction> {
        match p.kind {
            PartKind::Diode { anode, cathode } => Some(Junction { a: anode.0, k: cathode.0, nvt: VT }),
            PartKind::Led { anode, cathode, .. } => Some(Junction { a: anode.0, k: cathode.0, nvt: 2.0 * VT }),
            _ => None,
        }
    }

    /// Anode -> cathode current (including gmin) for node voltages indexed by NodeId.
    pub(crate) fn current(&self, v: &[f64]) -> f64 {
        let vd = v[self.a as usize] - v[self.k as usize];
        shockley(vd, self.nvt).0 + GMIN * vd
    }
}

pub(crate) fn junctions(c: &Circuit) -> Vec<Junction> {
    c.parts.iter().filter_map(Junction::of).collect()
}

/// Shockley current and its slope (no gmin). exp is continued linearly past 80*nVt so it never overflows.
fn shockley(v: f64, nvt: f64) -> (f64, f64) {
    const XMAX: f64 = 80.0;
    let x = v / nvt;
    let (e, de) = if x > XMAX { let e = XMAX.exp(); (e * (1.0 + x - XMAX), e) } else { let e = x.exp(); (e, e) };
    (IS * (e - 1.0), IS * de / nvt)
}

/// SPICE pnjlim: keeps a forward junction from jumping far up the exponential in one iteration.
fn pnjlim(vnew: f64, vold: f64, nvt: f64) -> f64 {
    let vcrit = nvt * (nvt / (std::f64::consts::SQRT_2 * IS)).ln();
    if vnew <= vcrit || (vnew - vold).abs() <= 2.0 * nvt {
        return vnew;
    }
    if vold > 0.0 {
        let arg = 1.0 + (vnew - vold) / nvt;
        if arg > 0.0 { vold + nvt * arg.ln() } else { vcrit }
    } else {
        nvt * (vnew / nvt).ln()
    }
}

/// Newton-Raphson over the junctions. `base` holds the linear stamps (and rhs); each iteration clones it,
/// adds each junction's companion model and solves. `v0` = starting node voltages (index = NodeId).
/// Converged when no node moves 1e-6 V or more and every junction's linearisation error is under 1e-9 A.
pub(crate) fn newton(base: &Stamper, js: &[Junction], v0: &[f64], max_iters: u32, cause: &str) -> Result<Vec<f64>, SimError> {
    let nv = |x: &[f64], i: u32| if i == 0 { 0.0 } else { x[i as usize - 1] };
    let mut prev = v0[1..].to_vec();
    let mut vj: Vec<f64> = js.iter().map(|j| v0[j.a as usize] - v0[j.k as usize]).collect();
    for _ in 0..max_iters {
        let mut s = base.clone();
        let mut lin = Vec::with_capacity(js.len());
        for (j, &v) in js.iter().zip(&vj) {
            let (i, g) = shockley(v, j.nvt);
            let ieq = i - g * v;
            s.conductance(j.a, j.k, g + GMIN);
            s.current(j.a, j.k, ieq);
            lin.push((g, ieq));
        }
        let x = s.solve(cause)?;
        if x.iter().any(|v| !v.is_finite()) {
            break;
        }
        let dv = x[..prev.len()].iter().zip(&prev).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        let mut resid = 0.0f64;
        for (idx, j) in js.iter().enumerate() {
            let v = nv(&x, j.a) - nv(&x, j.k);
            let (g, ieq) = lin[idx];
            resid = resid.max((shockley(v, j.nvt).0 - (g * v + ieq)).abs());
            vj[idx] = pnjlim(v, vj[idx], j.nvt);
        }
        if dv < 1e-6 && resid < 1e-9 {
            return Ok(x);
        }
        let m = prev.len();
        prev.copy_from_slice(&x[..m]);
    }
    Err(SimError::NoConvergence { iterations: max_iters })
}
