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
        if self.rhs.len() <= DENSE_MAX.with(Cell::get) { self.factor_dense(cause) } else { self.factor_sparse(cause) }
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

    fn factor_sparse(self, cause: &str) -> Result<Factor, SimError> {
        let n = self.rhs.len();
        let singular = || SimError::Singular(cause.into());
        let trips: Vec<_> = self.stamps.iter().map(|&(r, c, v)| Triplet::new(r, c, v)).collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(n, n, &trips).map_err(|_| singular())?;
        let (mut cp, mut cpi, mut rp, mut rpi) = (vec![0; n], vec![0; n], vec![0; n], vec![0; n]);
        let mut buf = MemBuffer::new(colamd::order_scratch::<usize>(n, n, a.compute_nnz()));
        colamd::order(&mut cp, &mut cpi, a.symbolic(), Default::default(), MemStack::new(&mut buf)).map_err(|_| singular())?;
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

/// Newton-Raphson over the junctions. `base` holds the linear stamps (and rhs); each iteration clones it,
/// adds each junction's companion model and solves. `v0` = starting node voltages (index = NodeId).
/// Converged when no node moves 1e-6 V or more and every junction's linearisation error is under
/// 1e-9 A + 1e-9 * |I| (relative part keeps huge currents, e.g. a bare LED on 5 V, from failing on rounding).
pub(crate) fn newton(base: &Stamper, js: &[Junction], v0: &[f64], max_iters: u32, cause: &str) -> Result<Vec<f64>, SimError> {
    let nv = |x: &[f64], i: u32| if i == 0 { 0.0 } else { x[i as usize - 1] };
    let mut prev = v0[1..].to_vec();
    let mut vj: Vec<f64> = js.iter().map(|j| v0[j.a as usize] - v0[j.k as usize]).collect();
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
        let x = s.solve(cause)?;
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
        if dv < 1e-6 && resid_ok {
            return Ok(x);
        }
        let m = prev.len();
        prev.copy_from_slice(&x[..m]);
    }
    Err(SimError::NoConvergence { iterations: max_iters })
}
