// Run the timing test with: cargo test --release -p nodal-core --test speed
use nodal_core::mna::{set_dense_max, Stamper};
use nodal_core::transient::transient;
use nodal_core::{Circuit, NodeId, Part, PartKind::*, SimError};

/// 5 V source on node 1, then R (1 kOhm) from node k to k+1 and C (1 uF) from k+1 to ground, up to node `nodes`.
fn ladder(nodes: u32) -> Circuit {
    let mut parts = vec![Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 5.0 } }];
    for k in 1..nodes {
        parts.push(Part { id: format!("R{k}"), kind: Resistor { a: NodeId(k), b: NodeId(k + 1), ohms: 1000.0, max_watts: 1.0 } });
        parts.push(Part { id: format!("C{k}"), kind: Capacitor { a: NodeId(k + 1), b: NodeId(0), farads: 1e-6 } });
    }
    Circuit { parts }
}

#[cfg(not(debug_assertions))]
#[test]
fn thousand_node_ladder_transient_under_one_second() {
    let c = ladder(1000);
    let t = std::time::Instant::now();
    let r = transient(&c, 1e-3, 1e-6).unwrap();
    let dt = t.elapsed();
    eprintln!("1,000-node ladder, 1,000 steps: {dt:?}");
    assert_eq!(r.times.len(), 1001);
    assert!(r.node_voltages.last().unwrap().iter().all(|v| v.is_finite()));
    assert!(dt.as_secs_f64() < 1.0, "took {dt:?}");
}

#[cfg(not(debug_assertions))]
#[test]
fn thousand_node_ladder_with_diode_transient_under_one_second() {
    let mut c = ladder(1000);
    c.parts.push(Part { id: "D1".into(), kind: Diode { anode: NodeId(500), cathode: NodeId(0) } });
    let t = std::time::Instant::now();
    let r = transient(&c, 1e-3, 1e-6).unwrap();
    let dt = t.elapsed();
    eprintln!("1,000-node ladder + diode, 1,000 steps: {dt:?}");
    assert_eq!(r.times.len(), 1001);
    assert!(r.node_voltages.last().unwrap().iter().all(|v| v.is_finite()));
    assert!(dt.as_secs_f64() < 1.0, "took {dt:?}");
}

#[test]
fn sparse_numeric_singular_pivot_is_reported() {
    // 60 nodes joined only by resistors, no path to ground: every column has entries (not structurally
    // singular), but the Laplacian is singular, so the last pivot is rounding noise.
    let mut s = Stamper::new(61, 0);
    for k in 1..60 { s.conductance(k, k + 1, 1e-3); }
    set_dense_max(0);
    let r = s.factor("island");
    set_dense_max(50);
    assert!(matches!(r, Err(SimError::Singular(_))));
}

#[test]
fn dense_and_sparse_agree() {
    let c = ladder(60); // 61 unknowns: sparse by default
    set_dense_max(usize::MAX);
    let dense = transient(&c, 2e-4, 1e-6).unwrap();
    set_dense_max(0);
    let sparse = transient(&c, 2e-4, 1e-6).unwrap();
    set_dense_max(50);
    for (d, s) in dense.node_voltages.iter().zip(&sparse.node_voltages) {
        for (a, b) in d.iter().zip(s) {
            assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        }
    }
    for (id, d) in &dense.part_currents {
        for (a, b) in d.iter().zip(&sparse.part_currents[id]) {
            assert!((a - b).abs() < 1e-9, "{id}: {a} vs {b}");
        }
    }
}

#[test]
fn sparse_path_reports_singular() {
    let mut c = ladder(60);
    c.parts.push(Part { id: "V2".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 3.0 } });
    for max in [usize::MAX, 0] {
        set_dense_max(max);
        assert!(matches!(transient(&c, 1e-5, 1e-6), Err(SimError::Singular(_))));
    }
    set_dense_max(50);
}
