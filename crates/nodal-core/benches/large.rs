//! Timing harness (no criterion): `cargo bench -p nodal-core --bench large`.
use nodal_core::transient::transient;
use nodal_core::{Circuit, NodeId, Part, PartKind::*};
use std::time::Instant;

/// 5 V source on node 1, then R (1 kOhm) from node k to k+1 and C (1 uF) from k+1 to ground.
fn ladder(nodes: u32) -> Circuit {
    let mut parts = vec![Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 5.0 } }];
    for k in 1..nodes {
        parts.push(Part { id: format!("R{k}"), kind: Resistor { a: NodeId(k), b: NodeId(k + 1), ohms: 1000.0, max_watts: 1.0 } });
        parts.push(Part { id: format!("C{k}"), kind: Capacitor { a: NodeId(k + 1), b: NodeId(0), farads: 1e-6 } });
    }
    Circuit { parts }
}

fn main() {
    for nodes in [100, 1_000, 10_000] {
        let c = ladder(nodes);
        let best = (0..5)
            .map(|_| {
                let t = Instant::now();
                std::hint::black_box(transient(&c, 1e-3, 1e-6).unwrap());
                t.elapsed()
            })
            .min()
            .unwrap();
        println!("RC ladder {nodes:>6} nodes, 1000 steps: best of 5 = {best:?}");
    }
}
