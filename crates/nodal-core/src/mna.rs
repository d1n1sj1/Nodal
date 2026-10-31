use crate::error::SimError;

/// Dense MNA system. Unknowns: node k (k>=1) at index k-1, then one per branch. Ground (node 0) is skipped.
/// shortcut: dense Gaussian elimination, O(n^3); swap in a sparse backend (Task 6) behind the same methods.
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

    /// Solves in place; returns unknowns in index order.
    pub fn solve(mut self) -> Result<Vec<f64>, SimError> {
        let n = self.rhs.len();
        for c in 0..n {
            let p = (c..n).max_by(|&x, &y| self.a[x][c].abs().total_cmp(&self.a[y][c].abs())).unwrap();
            if self.a[p][c].abs() < 1e-15 {
                return Err(SimError::Singular("voltage sources in a loop or a source across a zero-ohm path".into()));
            }
            self.a.swap(c, p);
            self.rhs.swap(c, p);
            for r in c + 1..n {
                let f = self.a[r][c] / self.a[c][c];
                if f != 0.0 {
                    for k in c..n { self.a[r][k] -= f * self.a[c][k]; }
                    self.rhs[r] -= f * self.rhs[c];
                }
            }
        }
        for c in (0..n).rev() {
            let s: f64 = (c + 1..n).map(|k| self.a[c][k] * self.rhs[k]).sum();
            self.rhs[c] = (self.rhs[c] - s) / self.a[c][c];
        }
        Ok(self.rhs)
    }
}
