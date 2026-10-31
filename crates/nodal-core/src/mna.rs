use crate::error::SimError;

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

    /// Zeroes every stamp and the rhs so the stamper can be re-stamped (Newton iterations).
    pub fn reset(&mut self) {
        self.a.iter_mut().for_each(|r| r.fill(0.0));
        self.rhs.fill(0.0);
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
