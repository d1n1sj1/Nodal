use crate::error::SimError;
use crate::{Circuit, Part, PartKind};
use faer::dyn_stack::{MemBuffer, MemStack};
use faer::perm::{permute_rows_in_place_scratch, PermRef};
use faer::sparse::linalg::{colamd, lu::simplicial};
use faer::sparse::{SparseColMat, Triplet};
use faer::{Conj, MatMut, Par};
use std::cell::Cell;

thread_local! {
    static DENSE_MAX: Cell<usize> = const { Cell::new(50) };
}

/// Test hook: systems with more unknowns than `n` use the sparse LU (default 50). Per thread.
#[doc(hidden)]
pub fn set_dense_max(n: usize) {
    DENSE_MAX.with(|d| d.set(n));
}

/// Small-signal model of every nonlinear part at the operating point `v` (node voltages by NodeId), stamped into
/// `g`: junction and transistor conductances, op-amp gain. The caller stamps the linear parts and each op-amp
/// output `voltage_source(k, out, 0, 0.0)` as in the DC solve; stamped right-hand sides are unused.
pub(crate) fn linearize(g: &mut Stamper, c: &Circuit, v: &[f64], branch_of: &std::collections::HashMap<&str, usize>) {
    for j in junctions(c) {
        g.conductance(j.a, j.k, shockley(v[j.a as usize] - v[j.k as usize], j.nvt).1 + GMIN);
    }
    for q in bjts(c) {
        let (x, y) = q.xy(v);
        let ev = q.eval(x, y);
        q.stamp(g, x, y, &ev);
    }
    for m in mosfets(c) {
        let (x, y) = m.xy(v);
        let ev = m.eval(x, y);
        m.stamp(g, x, y, &ev);
    }
    for o in opamps(c, branch_of) {
        let df = o.f(o.vd(v)).1;
        g.branch_control(o.out_k, o.plus, o.minus, -df, df, 0.0);
    }
}

/// A sparse column order (permutation and inverse), kept between Newton iterations and transient steps.
pub(crate) type ColOrder = Option<(Vec<usize>, Vec<usize>)>;

/// MNA system. Unknowns: node k (k>=1) at index k-1, then one per branch. Ground (node 0) is skipped.
/// The matrix is kept as (row, col, value) stamps (duplicates add up), so large systems never go dense.
#[derive(Clone)]
pub struct Stamper {
    nodes: usize, // non-ground node count
    stamps: Vec<(usize, usize, f64)>,
    pub rhs: Vec<f64>,
}

impl Stamper {
    pub fn new(node_count: usize, branches: usize) -> Self {
        let nodes = node_count.saturating_sub(1);
        Stamper { nodes, stamps: Vec::new(), rhs: vec![0.0; nodes + branches] }
    }

    /// A system built from raw (row, col, value) entries and a right-hand side, for the AC real-block form.
    pub(crate) fn from_entries(stamps: Vec<(usize, usize, f64)>, rhs: Vec<f64>) -> Self {
        Stamper { nodes: 0, stamps, rhs }
    }

    fn idx(node: u32) -> Option<usize> {
        (node != 0).then(|| node as usize - 1)
    }

    pub fn conductance(&mut self, a: u32, b: u32, g: f64) {
        let (ia, ib) = (Self::idx(a), Self::idx(b));
        if let Some(i) = ia { self.stamps.push((i, i, g)); }
        if let Some(j) = ib { self.stamps.push((j, j, g)); }
        if let (Some(i), Some(j)) = (ia, ib) {
            self.stamps.push((i, j, -g));
            self.stamps.push((j, i, -g));
        }
    }

    /// Matrix stamps so far as (row, col, value); duplicates add up. For AC analysis, which builds a complex matrix.
    pub(crate) fn entries(&self) -> &[(usize, usize, f64)] {
        &self.stamps
    }

    /// Adds `g` to the matrix at node row `row`, node column `col` (a dependent-source term). Ground is skipped.
    pub fn transconductance(&mut self, row: u32, col: u32, g: f64) {
        if let (Some(i), Some(j)) = (Self::idx(row), Self::idx(col)) { self.stamps.push((i, j, g)); }
    }

    /// Branch `k`'s row gets `g_plus * v(plus) + g_minus * v(minus)` added on the left and `rhs` on the right
    /// (a controlled source: pair with `voltage_source(k, out, 0, 0.0)`, which stamps v(out) itself).
    pub fn branch_control(&mut self, k: usize, plus: u32, minus: u32, g_plus: f64, g_minus: f64, rhs: f64) {
        let row = self.nodes + k;
        if let Some(j) = Self::idx(plus) { self.stamps.push((row, j, g_plus)); }
        if let Some(j) = Self::idx(minus) { self.stamps.push((row, j, g_minus)); }
        self.rhs[row] += rhs;
    }

    /// Current `amps` flows from `from` through the source to `to`.
    pub fn current(&mut self, from: u32, to: u32, amps: f64) {
        if let Some(i) = Self::idx(from) { self.rhs[i] -= amps; }
        if let Some(i) = Self::idx(to) { self.rhs[i] += amps; }
    }

    /// Branch `k` forces v(pos)-v(neg)=volts; its unknown is the current flowing pos -> neg through the element.
    pub fn voltage_source(&mut self, k: usize, pos: u32, neg: u32, volts: f64) {
        let row = self.nodes + k;
        if let Some(i) = Self::idx(pos) { self.stamps.extend([(i, row, 1.0), (row, i, 1.0)]); }
        if let Some(i) = Self::idx(neg) { self.stamps.extend([(i, row, -1.0), (row, i, -1.0)]); }
        self.rhs[row] += volts;
    }

    /// Adds -`r` on branch `k`'s diagonal (v(pos)-v(neg)-r*i = rhs), for inductor companion models.
    pub fn branch_resistance(&mut self, k: usize, r: f64) {
        let row = self.nodes + k;
        self.stamps.push((row, row, -r));
    }

    /// LU-factors the matrix (rhs ignored). Reuse the result for many right-hand sides.
    /// Up to 50 unknowns: dense LU. Above: faer sparse LU (COLAMD column order, partial row pivoting).
    /// Both reject a pivot at or below 1e-12 times its column's largest entry as singular.
    pub fn factor(self, cause: &str) -> Result<Factor, SimError> {
        self.factor_cached(cause, None)
    }

    /// `factor`, reusing a COLAMD column order from `order` (and storing one there when empty). The order only
    /// affects fill-in, not correctness, so it stays valid while the matrix keeps its size and rough pattern.
    pub(crate) fn factor_cached(self, cause: &str, order: Option<&mut ColOrder>) -> Result<Factor, SimError> {
        if self.rhs.len() <= DENSE_MAX.with(Cell::get) { self.factor_dense(cause) } else { self.factor_sparse(cause, order) }
    }

    fn factor_dense(self, cause: &str) -> Result<Factor, SimError> {
        let n = self.rhs.len();
        let mut a = vec![vec![0.0; n]; n];
        for (r, c, v) in self.stamps { a[r][c] += v; }
        // Per-column scale makes the pivot test robust to wide resistor ranges.
        let scale: Vec<f64> = (0..n).map(|c| a.iter().map(|r| r[c].abs()).fold(0.0, f64::max)).collect();
        let mut perm: Vec<usize> = (0..n).collect();
        for c in 0..n {
            let p = (c..n).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs())).unwrap();
            if !(a[p][c].abs() > 1e-12 * scale[c]) {
                return Err(SimError::Singular(cause.into()));
            }
            a.swap(c, p);
            perm.swap(c, p);
            for r in c + 1..n {
                let f = a[r][c] / a[c][c];
                a[r][c] = f; // multiplier kept in the lower triangle
                if f != 0.0 {
                    for k in c + 1..n { a[r][k] -= f * a[c][k]; }
                }
            }
        }
        Ok(Factor::Dense { lu: a, perm })
    }

    fn factor_sparse(self, cause: &str, order: Option<&mut ColOrder>) -> Result<Factor, SimError> {
        let n = self.rhs.len();
        let singular = || SimError::Singular(cause.into());
        let trips: Vec<_> = self.stamps.iter().map(|&(r, c, v)| Triplet::new(r, c, v)).collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(n, n, &trips).map_err(|_| singular())?;
        let (mut cp, mut cpi, mut rp, mut rpi) = (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        match order.as_deref().and_then(|o| o.as_ref()).filter(|(p, _)| p.len() == n) {
            Some((p, pi)) => { cp.copy_from_slice(p); cpi.copy_from_slice(pi); }
            None => {
                let mut buf = MemBuffer::new(colamd::order_scratch::<usize>(n, n, a.compute_nnz()));
                colamd::order(&mut cp, &mut cpi, a.symbolic(), Default::default(), MemStack::new(&mut buf)).map_err(|_| singular())?;
                if let Some(o) = order { *o = Some((cp.clone(), cpi.clone())); }
            }
        }
        let mut lu = simplicial::SimplicialLu::new();
        let mut buf = MemBuffer::new(simplicial::factorize_simplicial_numeric_lu_scratch::<usize, f64>(n, n));
        let col_perm = PermRef::new_checked(&cp, &cpi, n);
        simplicial::factorize_simplicial_numeric_lu(&mut rp, &mut rpi, &mut lu, a.as_ref(), col_perm, MemStack::new(&mut buf))
            .map_err(|_| singular())?;
        // Same relative test as the dense path; U's pivot is the last stored entry of each column (faer simplicial LU).
        let u = lu.u_factor_unsorted();
        for (j, &col) in cp.iter().enumerate() {
            let scale = a.val_of_col(col).iter().fold(0.0, |m: f64, v| m.max(v.abs()));
            if !u.val_of_col(j).last().is_some_and(|p| p.abs() > 1e-12 * scale) {
                return Err(singular());
            }
        }
        Ok(Factor::Sparse { lu, rp, rpi, cp, cpi })
    }

    /// `solve` with a column-order cache (see `factor_cached`).
    pub(crate) fn solve_cached(self, cause: &str, order: &mut ColOrder) -> Result<Vec<f64>, SimError> {
        let rhs = self.rhs.clone();
        Ok(self.factor_cached(cause, Some(order))?.solve(&rhs))
    }

    /// One-shot factor + solve of the stamped rhs; returns unknowns in index order.
    pub fn solve(self, cause: &str) -> Result<Vec<f64>, SimError> {
        let rhs = self.rhs.clone();
        Ok(self.factor(cause)?.solve(&rhs))
    }
}

/// LU factorisation: dense with row permutation, or faer simplicial sparse LU with row and column permutations.
pub enum Factor {
    Dense { lu: Vec<Vec<f64>>, perm: Vec<usize> },
    Sparse { lu: simplicial::SimplicialLu<usize, f64>, rp: Vec<usize>, rpi: Vec<usize>, cp: Vec<usize>, cpi: Vec<usize> },
}

impl Factor {
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        match self {
            Factor::Dense { lu, perm } => {
                let n = perm.len();
                let mut x: Vec<f64> = perm.iter().map(|&p| rhs[p]).collect();
                for r in 1..n {
                    let s: f64 = (0..r).map(|k| lu[r][k] * x[k]).sum();
                    x[r] -= s;
                }
                for c in (0..n).rev() {
                    let s: f64 = (c + 1..n).map(|k| lu[c][k] * x[k]).sum();
                    x[c] = (x[c] - s) / lu[c][c];
                }
                x
            }
            Factor::Sparse { lu, rp, rpi, cp, cpi } => {
                let n = rp.len();
                let mut x = rhs.to_vec();
                let mut buf = MemBuffer::new(permute_rows_in_place_scratch::<usize, f64>(n, 1));
                let (rows, cols) = (PermRef::new_checked(rp, rpi, n), PermRef::new_checked(cp, cpi, n));
                let xm = MatMut::from_column_major_slice_mut(&mut x, n, 1);
                lu.solve_in_place_with_conj(rows, cols, Conj::No, xm, Par::Seq, MemStack::new(&mut buf));
                x
            }
        }
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

/// Past XMAX*nVt (~1 V diode, ~2 V LED, ~2.4 kA) the exponential continues as a straight line. This keeps the
/// companion conductance bounded (~1e5 S), so a bare source across a junction stays well conditioned and
/// solves to a huge but finite current instead of overflowing or tripping the singular-pivot test.
const XMAX: f64 = 40.0;

/// Shockley current and its slope (no gmin), linear past XMAX*nVt.
fn shockley(v: f64, nvt: f64) -> (f64, f64) {
    let x = v / nvt;
    let (e, de) = if x > XMAX { let e = XMAX.exp(); (e * (1.0 + x - XMAX), e) } else { let e = x.exp(); (e, e) };
    (IS * (e - 1.0), IS * de / nvt)
}

/// SPICE pnjlim: keeps a forward junction from jumping far up the exponential in one iteration.
fn pnjlim(vnew: f64, vold: f64, nvt: f64) -> f64 {
    let vcrit = nvt * (nvt / (std::f64::consts::SQRT_2 * IS)).ln();
    // Past the knee the model is linear, so a full Newton step is exact; no limiting needed.
    if vnew <= vcrit || (vnew - vold).abs() <= 2.0 * nvt || vold >= XMAX * nvt {
        return vnew;
    }
    if vold > 0.0 {
        let arg = 1.0 + (vnew - vold) / nvt;
        if arg > 0.0 { vold + nvt * arg.ln() } else { vcrit }
    } else {
        nvt * (vnew / nvt).ln()
    }
}

const BETA_R: f64 = 1.0;

/// An Ebers-Moll BJT. Junction voltages x = p*(vb-ve), y = p*(vb-vc), p = +1 NPN / -1 PNP, so one set of
/// equations serves both polarities. gmin sits across b-e and b-c.
pub(crate) struct BjtModel {
    c: u32,
    b: u32,
    e: u32,
    p: f64,
    beta: f64,
}

/// Collector and base current (into the device) and their partials wrt (x, y).
struct BjtEval { ic: f64, ib: f64, dic: [f64; 2], dib: [f64; 2] }

impl BjtModel {
    pub(crate) fn of(p: &Part) -> Option<BjtModel> {
        match p.kind {
            PartKind::Bjt { c, b, e, pnp, beta, .. } => Some(BjtModel { c: c.0, b: b.0, e: e.0, p: if pnp { -1.0 } else { 1.0 }, beta }),
            _ => None,
        }
    }

    fn xy(&self, v: &[f64]) -> (f64, f64) {
        let vb = v[self.b as usize];
        (self.p * (vb - v[self.e as usize]), self.p * (vb - v[self.c as usize]))
    }

    fn eval(&self, x: f64, y: f64) -> BjtEval {
        let ((f, gf), (r, gr)) = (shockley(x, VT), shockley(y, VT));
        BjtEval {
            ic: f - r * (1.0 + 1.0 / BETA_R),
            ib: f / self.beta + r / BETA_R,
            dic: [gf, -gr * (1.0 + 1.0 / BETA_R)],
            dib: [gf / self.beta, gr / BETA_R],
        }
    }

    /// Current flowing into the collector, for node voltages indexed by NodeId.
    pub(crate) fn collector_current(&self, v: &[f64]) -> f64 {
        let (x, y) = self.xy(v);
        self.p * self.eval(x, y).ic
    }
}

/// Level-1 MOSFET. Gate-source x = p*(vg-vs), drain-source y = p*(vd-vs), p = +1 N / -1 P. gmin across d-s and g-s.
pub(crate) struct MosModel {
    d: u32,
    g: u32,
    s: u32,
    p: f64,
    vth: f64,
    kp: f64,
}

/// Drain current (into the drain, polarity-normalised) with partials wrt (x, y).
struct MosEval { id: f64, gm: f64, gds: f64 }

impl MosModel {
    pub(crate) fn of(p: &Part) -> Option<MosModel> {
        match p.kind {
            PartKind::Mosfet { d, g, s, pmos, vth, kp, .. } => Some(MosModel { d: d.0, g: g.0, s: s.0, p: if pmos { -1.0 } else { 1.0 }, vth, kp }),
            _ => None,
        }
    }

    fn xy(&self, v: &[f64]) -> (f64, f64) {
        let vs = v[self.s as usize];
        (self.p * (v[self.g as usize] - vs), self.p * (v[self.d as usize] - vs))
    }

    /// Forward-region square law for y >= 0.
    fn forward(&self, x: f64, y: f64) -> MosEval {
        let vov = x - self.vth;
        if vov <= 0.0 {
            MosEval { id: 0.0, gm: 0.0, gds: 0.0 }
        } else if y < vov {
            MosEval { id: self.kp * (vov * y - y * y / 2.0), gm: self.kp * y, gds: self.kp * (vov - y) }
        } else {
            MosEval { id: self.kp / 2.0 * vov * vov, gm: self.kp * vov, gds: 0.0 }
        }
    }

    /// Symmetric device: with y < 0 drain and source swap roles.
    fn eval(&self, x: f64, y: f64) -> MosEval {
        if y >= 0.0 { return self.forward(x, y) }
        let f = self.forward(x - y, -y);
        MosEval { id: -f.id, gm: -f.gm, gds: f.gm + f.gds }
    }

    /// Current flowing into the drain, for node voltages indexed by NodeId.
    pub(crate) fn drain_current(&self, v: &[f64]) -> f64 {
        let (x, y) = self.xy(v);
        self.p * self.eval(x, y).id
    }

    fn stamp(&self, s: &mut Stamper, x0: f64, y0: f64, ev: &MosEval) {
        let nodes = [self.d, self.g, self.s];
        let cd = [ev.gds, ev.gm, -(ev.gm + ev.gds)]; // wrt (vd, vg, vs); p cancels
        let cs = cd.map(|c| -c);
        // Any point with the right junction voltages works because each row of coefs sums to zero.
        let v0 = [self.p * y0, self.p * x0, 0.0];
        let i0 = self.p * ev.id;
        for (n, (co, i)) in nodes.into_iter().zip([(cd, i0), ([0.0; 3], 0.0), (cs, -i0)]) {
            for (k, g) in nodes.into_iter().zip(co) { s.transconductance(n, k, g); }
            s.current(n, 0, i - co.iter().zip(v0).map(|(g, v)| g * v).sum::<f64>());
        }
        s.conductance(self.d, self.s, GMIN);
        s.conductance(self.g, self.s, GMIN);
    }
}

pub(crate) fn mosfets(c: &Circuit) -> Vec<MosModel> {
    c.parts.iter().filter_map(MosModel::of).collect()
}

/// Largest change of a MOSFET junction voltage per Newton step: keeps the square law from jumping between regions.
const MOS_STEP: f64 = 1.0;

/// Op-amp output stage: branch `k` forces v(out) = vsat * tanh(gain * (v(plus) - v(minus)) / vsat).
pub(crate) struct OpampModel {
    out_k: usize,
    plus: u32,
    minus: u32,
    gain: f64,
    vsat: f64,
}

impl OpampModel {
    fn f(&self, vd: f64) -> (f64, f64) {
        let t = (self.gain * vd / self.vsat).tanh();
        (self.vsat * t, self.gain * (1.0 - t * t))
    }

    fn vd(&self, v: &[f64]) -> f64 {
        v[self.plus as usize] - v[self.minus as usize]
    }
}

/// `branch_of` gives each op-amp's branch index (the caller stamps `voltage_source(k, out, 0, 0.0)` for it).
pub(crate) fn opamps(c: &Circuit, branch_of: &std::collections::HashMap<&str, usize>) -> Vec<OpampModel> {
    c.parts.iter().filter_map(|p| match p.kind {
        PartKind::Opamp { plus, minus, gain, vsat, .. } => Some(OpampModel { out_k: branch_of[p.id.as_str()], plus: plus.0, minus: minus.0, gain, vsat }),
        _ => None,
    }).collect()
}

/// Node voltages indexed by NodeId (ground = 0) from the solved unknowns.
pub(crate) fn nvec(x: &[f64], n: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];
    v[1..].copy_from_slice(&x[..n - 1]);
    v
}

/// Which side of its knee each junction, BJT and MOSFET sits on. A change between two transient steps marks a switching event.
pub(crate) fn regions(js: &[Junction], qs: &[BjtModel], ms: &[MosModel], v: &[f64]) -> Vec<bool> {
    let mut r: Vec<bool> = js.iter().map(|j| v[j.a as usize] - v[j.k as usize] > 20.0 * j.nvt).collect();
    for q in qs {
        let (x, y) = q.xy(v);
        r.extend([x > 20.0 * VT, y > 20.0 * VT]);
    }
    for m in ms {
        let (x, y) = m.xy(v);
        r.extend([x > m.vth, y > x - m.vth]);
    }
    r
}

pub(crate) fn bjts(c: &Circuit) -> Vec<BjtModel> {
    c.parts.iter().filter_map(BjtModel::of).collect()
}

impl BjtModel {
    /// Companion model at (x0, y0): terminal currents I_n ~ I_n0 + sum coef * (v_k - v_k0), coefs summing to zero.
    fn stamp(&self, s: &mut Stamper, x0: f64, y0: f64, ev: &BjtEval) {
        let nodes = [self.c, self.b, self.e];
        let coef = |d: [f64; 2]| [-d[1], d[0] + d[1], -d[0]]; // wrt (vc, vb, ve)
        let (cc, cb) = (coef(ev.dic), coef(ev.dib));
        let ce = [-(cc[0] + cb[0]), -(cc[1] + cb[1]), -(cc[2] + cb[2])];
        // Any point with the right junction voltages works because each row of coefs sums to zero.
        let v0 = [-self.p * y0, 0.0, -self.p * x0];
        let (ic0, ib0) = (self.p * ev.ic, self.p * ev.ib);
        for (n, (co, i0)) in nodes.into_iter().zip([(cc, ic0), (cb, ib0), (ce, -(ic0 + ib0))]) {
            for (k, g) in nodes.into_iter().zip(co) { s.transconductance(n, k, g); }
            s.current(n, 0, i0 - co.iter().zip(v0).map(|(g, v)| g * v).sum::<f64>());
        }
        s.conductance(self.b, self.e, GMIN);
        s.conductance(self.b, self.c, GMIN);
    }
}

/// Newton-Raphson over the junctions and BJTs. `base` holds the linear stamps (and rhs); each iteration clones it,
/// adds each junction's companion model and solves. `v0` = starting node voltages (index = NodeId).
/// Converged when no node moves 1e-6 V or more and every junction's linearisation error is under
/// 1e-9 A + 1e-9 * |I| (relative part keeps huge currents, e.g. a bare LED on 5 V, from failing on rounding).
///
/// `rails`: first try starting every op-amp deep in positive saturation (20 iterations at most). Positive feedback
/// has an unstable solution between the rails that a linear start converges to; a rail start finds a stable one
/// instead. Negative-feedback circuits do not converge from there and fall back to the normal start. Use it for the
/// first solve of a run (DC, first transient step), not every step.
pub(crate) fn newton(base: &Stamper, js: &[Junction], qs: &[BjtModel], ms: &[MosModel], os: &[OpampModel], v0: &[f64], max_iters: u32, cause: &str, rails: bool, order: &mut ColOrder) -> Result<Vec<f64>, SimError> {
    if rails && !os.is_empty() {
        if let Ok(x) = newton_from(base, js, qs, ms, os, v0, max_iters.min(20), cause, true, order) { return Ok(x); }
    }
    match newton_from(base, js, qs, ms, os, v0, max_iters, cause, false, order) {
        Err(SimError::NoConvergence { .. }) if max_iters >= MAX_NEWTON_ITERS => ptc(base, js, qs, ms, os, v0, cause, order),
        r => r,
    }
}

/// Pseudo-transient continuation, for when plain Newton cycles because the answer is a jump away (two coupled
/// transistors flipping: the quasi-static solution folds). Ties every node to its last solution with a conductance
/// `g` and walks the anchor forward while `g` shrinks, as a stray capacitance on each node would; plain Newton
/// then finishes from wherever the anchor got to. A lowered iteration cap (the test hook) skips this.
fn ptc(base: &Stamper, js: &[Junction], qs: &[BjtModel], ms: &[MosModel], os: &[OpampModel], v0: &[f64], cause: &str, order: &mut ColOrder) -> Result<Vec<f64>, SimError> {
    let (mut anchor, mut g) = (v0.to_vec(), 1.0);
    for _ in 0..300 {
        let mut tied = base.clone();
        for (n, &v) in anchor.iter().enumerate().skip(1) {
            tied.conductance(n as u32, 0, g);
            tied.current(0, n as u32, g * v);
        }
        match newton_from(&tied, js, qs, ms, os, &anchor, 30, cause, false, order) {
            Ok(x) => {
                anchor = nvec(&x, v0.len());
                if let Ok(x) = newton_from(base, js, qs, ms, os, &anchor, 15, cause, false, order) { return Ok(x); }
                g /= 3.0;
            }
            Err(SimError::NoConvergence { .. }) if g < 1e9 => g *= 4.0,
            Err(e) => return Err(e),
        }
    }
    Err(SimError::NoConvergence { iterations: 300 })
}

fn newton_from(base: &Stamper, js: &[Junction], qs: &[BjtModel], ms: &[MosModel], os: &[OpampModel], v0: &[f64], max_iters: u32, cause: &str, sat: bool, order: &mut ColOrder) -> Result<Vec<f64>, SimError> {
    let nv = |x: &[f64], i: u32| if i == 0 { 0.0 } else { x[i as usize - 1] };
    let mut prev = v0[1..].to_vec();
    let mut vj: Vec<f64> = js.iter().map(|j| v0[j.a as usize] - v0[j.k as usize]).collect();
    let mut vq: Vec<(f64, f64)> = qs.iter().map(|q| q.xy(v0)).collect();
    let mut vm: Vec<(f64, f64)> = ms.iter().map(|m| m.xy(v0)).collect();
    let mut vo: Vec<f64> = os.iter().map(|o| if sat { 10.0 * o.vsat / o.gain } else { o.vd(v0) }).collect();
    for it in 1..=max_iters {
        let mut s = base.clone();
        let mut lin = Vec::with_capacity(js.len());
        for (j, &v) in js.iter().zip(&vj) {
            let (i, g) = shockley(v, j.nvt);
            let ieq = i - g * v;
            s.conductance(j.a, j.k, g + GMIN);
            s.current(j.a, j.k, ieq);
            lin.push((g, ieq));
        }
        let mut qlin = Vec::with_capacity(qs.len());
        for (q, &(x0, y0)) in qs.iter().zip(&vq) {
            let ev = q.eval(x0, y0);
            q.stamp(&mut s, x0, y0, &ev);
            qlin.push(ev);
        }
        let mut mlin = Vec::with_capacity(ms.len());
        for (m, &(x0, y0)) in ms.iter().zip(&vm) {
            let ev = m.eval(x0, y0);
            m.stamp(&mut s, x0, y0, &ev);
            mlin.push(ev);
        }
        let mut olin = Vec::with_capacity(os.len());
        for (o, &vd0) in os.iter().zip(&vo) {
            let (f0, df) = o.f(vd0);
            s.branch_control(o.out_k, o.plus, o.minus, -df, df, f0 - df * vd0);
            olin.push((f0, df));
        }
        let x = s.solve_cached(cause, order)?;
        if x.iter().any(|v| !v.is_finite()) {
            return Err(SimError::NoConvergence { iterations: it });
        }
        let dv = x[..prev.len()].iter().zip(&prev).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        let mut resid_ok = true;
        for (idx, j) in js.iter().enumerate() {
            let v = nv(&x, j.a) - nv(&x, j.k);
            let (g, ieq) = lin[idx];
            let i = shockley(v, j.nvt).0;
            resid_ok &= (i - (g * v + ieq)).abs() < 1e-9 + 1e-9 * i.abs();
            vj[idx] = pnjlim(v, vj[idx], j.nvt);
        }
        for (idx, q) in qs.iter().enumerate() {
            let (xn, yn) = q.xy(&nvec(&x, v0.len()));
            let (x0, y0) = vq[idx];
            let (lin, ev) = (&qlin[idx], q.eval(xn, yn));
            let (ic, ib) = (lin.ic + lin.dic[0] * (xn - x0) + lin.dic[1] * (yn - y0), lin.ib + lin.dib[0] * (xn - x0) + lin.dib[1] * (yn - y0));
            resid_ok &= (ev.ic - ic).abs() < 1e-9 + 1e-9 * ev.ic.abs() && (ev.ib - ib).abs() < 1e-9 + 1e-9 * ev.ib.abs();
            vq[idx] = (pnjlim(xn, x0, VT), pnjlim(yn, y0, VT));
        }
        for (idx, m) in ms.iter().enumerate() {
            let (xn, yn) = m.xy(&nvec(&x, v0.len()));
            let (x0, y0) = vm[idx];
            let (lin, ev) = (&mlin[idx], m.eval(xn, yn));
            let id = lin.id + lin.gm * (xn - x0) + lin.gds * (yn - y0);
            resid_ok &= (ev.id - id).abs() < 1e-9 + 1e-9 * ev.id.abs();
            vm[idx] = (x0 + (xn - x0).clamp(-MOS_STEP, MOS_STEP), y0 + (yn - y0).clamp(-MOS_STEP, MOS_STEP));
        }
        for (idx, o) in os.iter().enumerate() {
            let vd = o.vd(&nvec(&x, v0.len()));
            let (f0, df) = olin[idx];
            let f = o.f(vd).0;
            resid_ok &= (f - (f0 + df * (vd - vo[idx]))).abs() < 1e-9 + 1e-9 * f.abs();
            vo[idx] = vd;
        }
        if dv < 1e-6 && resid_ok {
            return Ok(x);
        }
        let m = prev.len();
        prev.copy_from_slice(&x[..m]);
    }
    Err(SimError::NoConvergence { iterations: max_iters })
}
