use nodal_core::dc::{dc_op, dc_op_with_limit};
use nodal_core::transient::{transient, transient_from};
use nodal_core::{Circuit, NodeId, Part, PartKind, PartKind::*, SimError};
use std::time::Instant;

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }

fn series(volts: f64, reversed: bool) -> Circuit {
    let (anode, cathode) = if reversed { (n(0), n(2)) } else { (n(2), n(0)) };
    Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 1.0 }),
        part("D1", Diode { anode, cathode }),
    ]}
}

#[test]
fn diode_series_resistor_dc() {
    let r = dc_op(&series(5.0, false)).unwrap();
    let vd = r.node_voltages[2];
    assert!(vd > 0.55 && vd < 0.75, "vd = {vd}");
    let want = (5.0 - vd) / 1000.0;
    assert!((r.part_currents["D1"] - want).abs() < 1e-6, "{} vs {want}", r.part_currents["D1"]);
}

#[test]
fn reverse_diode_blocks() {
    let r = dc_op(&series(5.0, true)).unwrap();
    // D1 anode->cathode current; reversed, so it flows backwards but must be tiny.
    assert!(r.part_currents["D1"].abs() < 1e-9, "{}", r.part_currents["D1"]);
}

#[test]
fn led_drops_more_than_diode() {
    let mut c = series(5.0, false);
    c.parts[2].kind = Led { anode: n(2), cathode: n(0), max_amps: 0.02 };
    let r = dc_op(&c).unwrap();
    assert!(r.node_voltages[2] > 1.0, "{}", r.node_voltages[2]); // n = 2 roughly doubles the drop
}

#[test]
fn nonconvergence_returns_error() {
    // With pnjlim, the linear continuation past the knee and the relative residual, no real circuit
    // tried here fails within 100 iterations (even a 1e9 A source into a diode converges). So this test
    // uses the doc-hidden `dc_op_with_limit` hook. A forward diode starting from 0 V cannot converge in
    // 2 iterations because pnjlim caps the first steps. The test checks the cap is honoured and the call
    // returns a clear error quickly.
    let t = Instant::now();
    let r = dc_op_with_limit(&series(5.0, false), 2);
    assert!(t.elapsed().as_secs_f64() < 1.0);
    assert_eq!(r.unwrap_err(), SimError::NoConvergence { iterations: 2 });
    // Default cap: same circuit converges well inside 100 iterations.
    assert!(dc_op(&series(5.0, false)).is_ok());
}

fn bare(volts: f64, led: bool) -> Circuit {
    let kind = if led { Led { anode: n(1), cathode: n(0), max_amps: 0.02 } } else { Diode { anode: n(1), cathode: n(0) } };
    Circuit { parts: vec![part("V1", VSource { pos: n(1), neg: n(0), volts }), part("D1", kind)] }
}

fn assert_huge_finite(c: &Circuit) {
    let r = dc_op(c).unwrap_or_else(|e| panic!("dc_op: {e}"));
    let i = r.part_currents["D1"];
    assert!(i.is_finite() && i > 1.0, "dc current {i}");
    assert!(r.node_voltages.iter().all(|v| v.is_finite()));
    let t = transient(c, 1e-4, 1e-5).unwrap_or_else(|e| panic!("transient: {e}"));
    assert!(t.part_currents["D1"].iter().all(|i| i.is_finite() && *i > 1.0), "{:?}", t.part_currents["D1"]);
    assert!(t.node_voltages.iter().flatten().all(|v| v.is_finite()));
}

#[test]
fn bare_led_on_5v_gives_huge_finite_current() {
    assert_huge_finite(&bare(5.0, true));
}

#[test]
fn bare_diode_up_to_12v_gives_huge_finite_current() {
    for v in [2.0, 2.5, 3.0, 3.3, 5.0, 12.0] {
        assert_huge_finite(&bare(v, false));
    }
}

#[test]
fn led_with_330_ohm_is_about_10_ma() {
    let mut c = series(5.0, false);
    c.parts[1].kind = Resistor { a: n(1), b: n(2), ohms: 330.0, max_watts: 0.25 };
    c.parts[2].kind = Led { anode: n(2), cathode: n(0), max_amps: 0.02 };
    let i = dc_op(&c).unwrap().part_currents["D1"];
    assert!(i > 9e-3 && i < 11e-3, "{i}");
    let t = transient(&c, 1e-4, 1e-5).unwrap();
    assert!(t.part_currents["D1"].iter().all(|i| *i > 9e-3 && *i < 11e-3));
}

#[test]
fn source_loop_still_singular() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0 }),
        part("V2", VSource { pos: n(1), neg: n(0), volts: 3.0 }),
        part("D1", Diode { anode: n(1), cathode: n(0) }),
    ]};
    assert!(matches!(dc_op(&c), Err(SimError::Singular(_))));
}

#[test]
fn diode_blocks_reverse_step_in_transient() {
    // Source at node 1, diode 1->2, 1 kΩ load 2->0.
    let circuit = |volts| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts }),
        part("D1", Diode { anode: n(1), cathode: n(2) }),
        part("R1", Resistor { a: n(2), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ]};
    let first = transient(&circuit(5.0), 1e-3, 1e-5).unwrap();
    let on = first.node_voltages.last().unwrap()[2];
    assert!(on > 4.0, "forward load voltage {on}");
    assert!(first.part_currents["D1"].last().unwrap() > &4e-3);

    let second = transient_from(&circuit(-5.0), &first.final_state(), 1e-3, 1e-5).unwrap();
    for v in &second.node_voltages[1..] {
        assert!(v[2] >= -0.01, "load went to {}", v[2]);
    }
}

#[test]
fn diode_reached_only_through_diode_is_not_floating() {
    // Node 2 connects to ground only via the diode; must not be FloatingNode.
    let c = Circuit { parts: vec![
        part("I1", ISource { from: n(0), to: n(1), amps: 1e-3 }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 100.0, max_watts: 1.0 }),
        part("D1", Diode { anode: n(2), cathode: n(0) }),
    ]};
    assert!(dc_op(&c).is_ok());
}

#[test]
fn transient_from_rejects_bad_initial_state() {
    let c = series(5.0, false);
    let good = transient(&c, 1e-4, 1e-5).unwrap().final_state();

    let mut s = good.clone();
    s.node_voltages[1] = f64::NAN;
    assert!(matches!(transient_from(&c, &s, 1e-4, 1e-5), Err(SimError::InvalidInput(m)) if m.contains("non-finite")));

    let mut s = good.clone();
    s.part_currents.insert("R1".into(), f64::INFINITY);
    assert!(matches!(transient_from(&c, &s, 1e-4, 1e-5), Err(SimError::InvalidInput(_))));

    let mut s = good;
    s.part_currents.remove("D1");
    assert!(matches!(transient_from(&c, &s, 1e-4, 1e-5), Err(SimError::InvalidInput(m)) if m.contains("does not match")));
}
